use crate::schema;

use super::IgniError;
use super::ServerOpt;
use log::*;
use num_rational::Rational64;
use regex::Regex;

mod api;
mod gc;
pub mod io_cache;
mod vod;

struct IgniServerGlobal {
    config: ServerConfig,
    pool: sqlx::Pool<sqlx::Postgres>,
}

impl IgniServerGlobal {
    fn io_wrapper(&self) -> Option<Box<dyn vidformer::io::IoWrapper>> {
        match &self.config.io_cache_valkey_url {
            Some(url) => Some(Box::new(io_cache::IgniIoWrapper {
                url: url.clone(),
                chunk_size: self.config.io_cache_block_size,
            })),
            None => None,
        }
    }
}

#[derive(Debug)]
struct UserAuth {
    user_id: uuid::Uuid,
    permissions: UserPermissions,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct UserPermissions {
    flags: Vec<String>,
    valsets: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    limits_int: std::collections::BTreeMap<String, i64>,
    limits_frac: std::collections::BTreeMap<String, Rational64>,
}

impl UserPermissions {
    pub fn json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

impl UserPermissions {
    pub fn default_regular() -> UserPermissions {
        let limits_int = [
            ("spec:max_width", 4096),       // DCI 4K
            ("spec:max_height", 2160),      // DCI 4K
            ("spec:max_frames", 432000),    // 4 hours @ 30 fps / 5 hours @ 24 fps
            ("spec:max_ttl", 24 * 60 * 60), // 24 hours
            ("source:max_width", 4096),
            ("source:max_height", 2160),
        ]
        .iter()
        .map(|(key, value)| (key.to_string(), *value))
        .collect();

        let limits_frac = [
            ("spec:max_vod_segment_length", (3, 1)),
            ("spec:min_vod_segment_legth", (1, 1)),
            ("spec:max_frame_rate", (60, 1)),
            ("spec:min_frame_rate", (1, 1)),
        ]
        .iter()
        .map(|(key, value)| (key.to_string(), Rational64::new(value.0, value.1)))
        .collect();

        let valsets = [
            ("spec:pix_fmt", vec!["yuv420p"]),
            ("source:storage_service", vec!["http", "s3"]),
            ("frame:pix_fmt", vec!["rgb24", "gray"]),
        ]
        .iter()
        .map(|(key, values)| {
            let values = values.iter().map(|v| v.to_string()).collect();
            (key.to_string(), values)
        })
        .collect();

        let flags = vec![
            // Source permissions
            "source:create",
            "source:get",
            "source:list",
            "source:search",
            "source:delete",
            // Spec permissions
            "spec:create",
            "spec:get",
            "spec:list",
            "spec:push_part",
            "spec:delete",
            // "spec:deferred_frames" - do not enable until stable
            // Frame permissions
            "frame:get",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        UserPermissions {
            flags,
            valsets,
            limits_int,
            limits_frac,
        }
    }

    pub fn default_guest() -> UserPermissions {
        let limits_int = [
            ("spec:max_width", 1280),
            ("spec:max_height", 720),
            ("spec:max_frames", 162000),   // 90 minutes @ 30 fps
            ("spec:max_ttl", 1 * 60 * 60), // 1 hours
        ]
        .iter()
        .map(|(key, value)| (key.to_string(), *value))
        .collect();

        let limits_frac = [
            ("spec:max_vod_segment_length", (3, 1)),
            ("spec:min_vod_segment_legth", (1, 1)),
            ("spec:max_frame_rate", (30, 1)),
            ("spec:min_frame_rate", (1, 1)),
        ]
        .iter()
        .map(|(key, value)| (key.to_string(), Rational64::new(value.0, value.1)))
        .collect();

        let valsets = [
            ("spec:pix_fmt", vec!["yuv420p"]),
            ("frame:pix_fmt", vec!["rgb24", "gray"]),
        ]
        .iter()
        .map(|(key, values)| {
            let values = values.iter().map(|v| v.to_string()).collect();
            (key.to_string(), values)
        })
        .collect();

        let flags = vec![
            // Source permissions
            "source:get",
            "source:list",
            "source:search",
            // Spec permissions
            "spec:create",
            "spec:get",
            "spec:push_part",
            "spec:delete",
            // Frame permissions
            "frame:get",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        UserPermissions {
            flags,
            valsets,
            limits_int,
            limits_frac,
        }
    }

    pub fn default_test() -> UserPermissions {
        let mut out = UserPermissions::default_regular();

        // Allow local fs storage for testing
        out.valsets
            .get_mut("source:storage_service")
            .unwrap()
            .insert("fs".to_string());

        // Increase source:max_height to 4000x4000 so apollo.jpg can be used in tests
        out.limits_int.insert("source:max_height".to_string(), 4000);
        out.limits_int.insert("source:max_width".to_string(), 4000);

        out
    }

    pub fn flag(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }

    pub fn flag_err(
        &self,
        flag: &str,
    ) -> Option<hyper::Response<http_body_util::Full<hyper::body::Bytes>>> {
        if !self.flag(flag) {
            let mut res = hyper::Response::new(http_body_util::Full::new(
                hyper::body::Bytes::from(format!("Permission denied - {}", flag)),
            ));
            *res.status_mut() = hyper::StatusCode::FORBIDDEN;
            Some(res)
        } else {
            None
        }
    }

    pub fn limit(&self, limit: &str) -> Option<i64> {
        self.limits_int.get(limit).cloned()
    }

    pub fn limit_err_max(
        &self,
        limit: &str,
        value: i64,
    ) -> Option<hyper::Response<http_body_util::Full<hyper::body::Bytes>>> {
        if let Some(limit_value) = self.limit(limit) {
            if value > limit_value {
                let mut res =
                    hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(
                        format!("Limit {} exceeded - {} > {}", limit, value, limit_value),
                    )));
                *res.status_mut() = hyper::StatusCode::FORBIDDEN;
                Some(res)
            } else {
                None
            }
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn limit_err_min(
        &self,
        limit: &str,
        value: i64,
    ) -> Option<hyper::Response<http_body_util::Full<hyper::body::Bytes>>> {
        if let Some(limit_value) = self.limit(limit) {
            if value < limit_value {
                let mut res =
                    hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(
                        format!("Limit {} exceeded - {} < {}", limit, value, limit_value),
                    )));
                *res.status_mut() = hyper::StatusCode::FORBIDDEN;
                Some(res)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn limit_frac(&self, limit: &str) -> Option<Rational64> {
        self.limits_frac.get(limit).cloned()
    }

    pub fn limit_frac_err_max(
        &self,
        limit: &str,
        value: Rational64,
    ) -> Option<hyper::Response<http_body_util::Full<hyper::body::Bytes>>> {
        if let Some(limit_value) = self.limit_frac(limit) {
            if value > limit_value {
                let mut res =
                    hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(
                        format!("Limit {} exceeded - {} > {}", limit, value, limit_value),
                    )));
                *res.status_mut() = hyper::StatusCode::FORBIDDEN;
                Some(res)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn limit_frac_err_min(
        &self,
        limit: &str,
        value: Rational64,
    ) -> Option<hyper::Response<http_body_util::Full<hyper::body::Bytes>>> {
        if let Some(limit_value) = self.limit_frac(limit) {
            if value < limit_value {
                let mut res =
                    hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(
                        format!("Limit {} exceeded - {} < {}", limit, value, limit_value),
                    )));
                *res.status_mut() = hyper::StatusCode::FORBIDDEN;
                Some(res)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn valset_err(
        &self,
        valset: &str,
        value: &str,
    ) -> Option<hyper::Response<http_body_util::Full<hyper::body::Bytes>>> {
        if let Some(allowed_values) = self.valsets.get(valset) {
            if !allowed_values.contains(value) {
                let mut res = hyper::Response::new(http_body_util::Full::new(
                    hyper::body::Bytes::from(format!(
                        "Invalid value for {} - {} not in {{{}}}",
                        valset,
                        value,
                        allowed_values
                            .iter()
                            .cloned()
                            .collect::<Vec<String>>()
                            .join(", ")
                    )),
                ));
                *res.status_mut() = hyper::StatusCode::FORBIDDEN;
                Some(res)
            } else {
                None
            }
        } else {
            None
        }
    }
}

fn load_config(path: &String) -> Result<ServerConfig, IgniError> {
    let config_string = std::fs::read_to_string(path)
        .map_err(|e| IgniError::General(format!("Failed to read config file: {}", e)))?;
    let config: ServerConfig = toml::from_str(&config_string).map_err(|e: toml::de::Error| {
        IgniError::General(format!("Failed to parse config file: {}", e))
    })?;

    // Validate the config
    if !config.vod_prefix.starts_with("http://") && !config.vod_prefix.starts_with("https://") {
        return Err(IgniError::General(
            "vod_prefix must start with http:// or https://".to_string(),
        ));
    }
    if !config.vod_prefix.ends_with("/") {
        return Err(IgniError::General("vod_prefix must end with /".to_string()));
    }

    Ok(config)
}

#[derive(serde::Deserialize, Debug)]
struct ServerConfig {
    vod_prefix: String,
    gc_period: i64,
    io_cache_valkey_url: Option<String>,
    io_cache_block_size: usize,
}

pub(crate) async fn cmd_server(
    pool: sqlx::Pool<sqlx::Postgres>,
    opt: ServerOpt,
) -> Result<(), IgniError> {
    use hyper::server::conn::http1;
    use hyper_util::rt::TokioIo;

    let config = load_config(&opt.config)?;
    let global = std::sync::Arc::new(IgniServerGlobal { config, pool });
    let addr: std::net::SocketAddr = format!("[::]:{}", opt.port).parse().unwrap();
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| IgniError::General(format!("Failed to bind to {}: {}", addr, e)))?;

    if global.config.gc_period > 0 {
        let global_gc = global.clone();
        tokio::task::spawn(async move {
            if let Err(err) = gc::gc_main(global_gc).await {
                error!("Garbage collector failed: {:?}", err);
            }
        });
    }

    println!("Opened igni server on {}", addr);

    loop {
        let (stream, client_addr) = match listener.accept().await {
            Ok(ok) => ok,
            Err(err) => {
                warn!("Error accepting connection: {:?}", err);
                continue;
            }
        };
        trace!("Accepted connection from {}", client_addr);

        let io = TokioIo::new(stream);
        let global = global.clone();

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    hyper::service::service_fn(|req| {
                        igni_http_req_error_handler(req, global.clone())
                    }),
                )
                .await
            {
                debug!("Dropped connection: {:?}", err);
            }
        });
    }
}

async fn igni_http_req_error_handler(
    req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, std::convert::Infallible> {
    let method_copy_for_err = req.method().clone();
    let uri_copy_for_err = req.uri().clone();
    match igni_http_req(req, global).await {
        Ok(ok) => Ok(ok),
        Err(err) => {
            // An error occured which is not an explicitly handled error
            // Log the error and return a 500 response
            // Do not leak the error to the client
            error!(
                "Error handling request {} {}: {:?}",
                method_copy_for_err, &uri_copy_for_err, err
            );
            Ok(hyper::Response::builder()
                .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Internal server error",
                )))
                .unwrap())
        }
    }
}

async fn igni_http_req(
    req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let uri = req.uri().path().to_string();
    let method = req.method().clone();

    match (method, uri.as_str()) {
        (hyper::Method::GET, "/") => Ok(hyper::Response::builder()
            .header("Access-Control-Allow-Origin", "*")
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                format!("vidformer-igni v{}\n", env!("CARGO_PKG_VERSION")),
            )))
            ?),
        (hyper::Method::GET, "/hls.js") => {
            Ok(hyper::Response::builder()
            .header("Access-Control-Allow-Origin", "*")
            .header("Content-Type", "text/javascript")
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                include_str!("server/hls.js"),
            )))
            .unwrap())
        }
        (hyper::Method::GET, _) // playlist.m3u8
            if {
                Regex::new(r"^/vod/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/playlist.m3u8$").unwrap().is_match(req.uri().path())
            } =>
        {
            let r = Regex::new(
                r"^/vod/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})/playlist.m3u8$",
            );
            let uri = req.uri().path().to_string();
            let spec_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            vod::get_playlist(req, global, spec_id).await
        }
        (hyper::Method::GET, _) // stream.m3u8
            if {
                Regex::new(r"^/vod/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/stream.m3u8$").unwrap().is_match(req.uri().path())
            } =>
        {
            let r = Regex::new(
                r"^/vod/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})/stream.m3u8$",
            );
            let uri = req.uri().path().to_string();
            let spec_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            vod::get_stream(req, global, spec_id).await
        }
        (hyper::Method::GET, _) // status
            if {
                Regex::new(r"^/vod/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/status$").unwrap().is_match(req.uri().path())
            } =>
        {
            let r = Regex::new(
                r"^/vod/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})/status$",
            );
            let uri = req.uri().path().to_string();
            let spec_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            vod::get_status(req, global, spec_id).await
        }
        (_, uri) if {
            uri.starts_with("/v2/")
        } => {
            igni_http_req_api(req, global).await
        }
        (hyper::Method::GET, _) // segment-$n.ts
            if {
                Regex::new(r"^/vod/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/segment-[0-9]+.ts$").unwrap().is_match(req.uri().path())
            } => {
            let r = Regex::new(
                r"^/vod/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})/segment-([0-9]+).ts$",
            ).unwrap();
            let uri = req.uri().path().to_string();
            let matches = r.captures(&uri).unwrap();
            let spec_id = matches.get(1).unwrap().as_str();
            let segment_number = matches.get(2).unwrap().as_str().parse().unwrap();
            vod::get_segment(req, global, spec_id, segment_number).await
        }
        (method, uri) => {
            warn!("404 Not found: {} {}", method, uri);
            let mut res = hyper::Response::new(http_body_util::Full::new(
                hyper::body::Bytes::from("Not found"),
            ));
            *res.status_mut() = hyper::StatusCode::NOT_FOUND;
            Ok(res)
        }
    }
}

async fn igni_http_req_api(
    req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let uri = req.uri().path().to_string();
    let method = req.method().clone();

    let api_key = req
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "));

    let api_key = match api_key {
        Some(api_key) => api_key,
        None => {
            let mut res = hyper::Response::new(http_body_util::Full::new(
                hyper::body::Bytes::from("Unauthorized"),
            ));
            *res.status_mut() = hyper::StatusCode::UNAUTHORIZED;
            return Ok(res);
        }
    };

    let user: Option<schema::UserRow> = sqlx::query_as("SELECT * FROM \"user\" WHERE api_key = $1")
        .bind(api_key)
        .fetch_optional(&global.pool)
        .await?;

    let user = match user {
        Some(user) => user,
        None => {
            let mut res = hyper::Response::new(http_body_util::Full::new(
                hyper::body::Bytes::from("Unauthorized"),
            ));
            *res.status_mut() = hyper::StatusCode::UNAUTHORIZED;
            return Ok(res);
        }
    };
    let user_permissions: UserPermissions =
        serde_json::from_value(user.permissions).map_err(|e| {
            IgniError::General(format!(
                "Failed to parse user permissions for user {}: {}",
                user.id, e
            ))
        })?;
    let user_auth = UserAuth {
        user_id: user.id,
        permissions: user_permissions,
    };

    match (method, uri.as_str()) {
        (hyper::Method::GET, "/v2/auth") // /v2/auth (for checking auth success)
        => {
            api::auth(req, global, &user_auth).await
        }
        (hyper::Method::GET, "/v2/source") // /v2/source (list)
        => {
            if let Some(res) = user_auth.permissions.flag_err("source:list") {
                return Ok(res);
            }
            api::list_sources(req, global, &user_auth).await
        }
        (hyper::Method::GET, _) // /v2/source/<uuid>
            if {
                Regex::new(r"^/v2/source/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$").unwrap().is_match(req.uri().path())
            } =>
        {
            if let Some(res) = user_auth.permissions.flag_err("source:get") {
                return Ok(res);
            }
            let r = Regex::new(
                r"^/v2/source/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$",
            );
            let uri = req.uri().path().to_string();
            let source_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            api::get_source(req, global, source_id, &user_auth).await
        }
        (hyper::Method::POST, "/v2/source/search") => {
            if let Some(res) = user_auth.permissions.flag_err("source:search") {
                return Ok(res);
            }
            api::search_source(req, global, &user_auth).await
        }
        (hyper::Method::POST, "/v2/source") // /v2/source
        => {
            if let Some(res) = user_auth.permissions.flag_err("source:create") {
                return Ok(res);
            }
            api::create_source(req, global, &user_auth).await
        }
        (hyper::Method::DELETE, _) // /v2/source/<uuid>
            if {
                Regex::new(r"^/v2/source/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$").unwrap().is_match(req.uri().path())
            } =>
        {
            if let Some(res) = user_auth.permissions.flag_err("source:delete") {
                return Ok(res);
            }
            let r = Regex::new(
                r"^/v2/source/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$",
            );
            let uri = req.uri().path().to_string();
            let source_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            api::delete_source(req, global, source_id, &user_auth).await
        }
        (hyper::Method::GET, "/v2/spec") // /v2/spec (list)
        => {
            if let Some(res) = user_auth.permissions.flag_err("spec:list") {
                return Ok(res);
            }
            api::list_specs(req, global, &user_auth).await
        }
        (hyper::Method::GET, _) // /v2/spec/<uuid>
            if {
                Regex::new(r"^/v2/spec/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$").unwrap().is_match(req.uri().path())
            } =>
        {
            if let Some(res) = user_auth.permissions.flag_err("spec:get") {
                return Ok(res);
            }
            let r = Regex::new(
                r"^/v2/spec/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$",
            );
            let uri = req.uri().path().to_string();
            let spec_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            api::get_spec(req, global, spec_id, &user_auth).await
        }
        (hyper::Method::DELETE, _) // /v2/spec/<uuid>
            if {
                Regex::new(r"^/v2/spec/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$").unwrap().is_match(req.uri().path())
            } =>
        {
            if let Some(res) = user_auth.permissions.flag_err("spec:delete") {
                return Ok(res);
            }
            let r = Regex::new(
                r"^/v2/spec/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$",
            );
            let uri = req.uri().path().to_string();
            let spec_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            api::delete_spec(req, global, spec_id, &user_auth).await
        }
        (hyper::Method::POST, "/v2/spec") // /v2/spec
        => {
            if let Some(res) = user_auth.permissions.flag_err("spec:create") {
                return Ok(res);
            }
            api::push_spec(req, global, &user_auth).await
        }
        (hyper::Method::POST, _) // /v2/spec/<uuid>/part
            if {
                Regex::new(r"^/v2/spec/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/part$").unwrap().is_match(req.uri().path())
            } =>
        {
            if let Some(res) = user_auth.permissions.flag_err("spec:push_part") {
                return Ok(res);
            }
            let r = Regex::new(
                r"^/v2/spec/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})/part$",
            );
            let uri = req.uri().path().to_string();
            let spec_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            api::push_part(req, global, spec_id, &user_auth).await
        }
        (hyper::Method::POST, _) // /v2/spec/<uuid>/part_block
            if {
                Regex::new(r"^/v2/spec/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/part_block$").unwrap().is_match(req.uri().path())
            } =>
        {
            if let Some(res) = user_auth.permissions.flag_err("spec:push_part") {
                return Ok(res);
            }
            let r = Regex::new(
                r"^/v2/spec/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})/part_block$",
            );
            let uri = req.uri().path().to_string();
            let spec_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            api::push_part_block(req, global, spec_id, &user_auth).await
        }
        (hyper::Method::POST, "/v2/frame") => {
            if let Some(res) = user_auth.permissions.flag_err("frame:get") {
                return Ok(res);
            }
            api::get_frame(req, global, &user_auth).await
        }
        (method, uri) => {
            warn!("404 Not found: {} {}", method, uri);
            let mut res = hyper::Response::new(http_body_util::Full::new(
                hyper::body::Bytes::from("Not found"),
            ));
            *res.status_mut() = hyper::StatusCode::NOT_FOUND;
            Ok(res)
        }
    }
}
