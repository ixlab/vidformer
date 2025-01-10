use super::IgniError;
use super::ServerOpt;
use log::*;
use regex::Regex;

mod api;
mod vod;

struct IgniServerGlobal {
    config: ServerConfig,
    pool: sqlx::Pool<sqlx::Postgres>,
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

    info!("Opened server on {}", addr);

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
    match igni_http_req(req, global).await {
        Ok(ok) => Ok(ok),
        Err(err) => {
            // An error occured which is not an explicitly handled error
            // Log the error and return a 500 response
            // Do not leak the error to the client
            error!("Error handling request: {:?}", err);
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
        (hyper::Method::GET, "/v2/source") // /v2/source (list)
        => {
            api::list_sources(req, global).await
        }
        (hyper::Method::GET, _) // /v2/source/<uuid>
            if {
                Regex::new(r"^/v2/source/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$").unwrap().is_match(req.uri().path())
            } =>
        {
            let r = Regex::new(
                r"^/v2/source/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$",
            );
            let uri = req.uri().path().to_string();
            let source_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            api::get_source(req, global, source_id).await
        }
        (hyper::Method::POST, "/v2/source") // /v2/source
        => {
            api::push_source(req, global).await
        }
        (hyper::Method::DELETE, _) // /v2/source/<uuid>
            if {
                Regex::new(r"^/v2/source/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$").unwrap().is_match(req.uri().path())
            } =>
        {
            let r = Regex::new(
                r"^/v2/source/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$",
            );
            let uri = req.uri().path().to_string();
            let source_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            api::delete_source(req, global, source_id).await
        }
        (hyper::Method::GET, "/v2/spec") // /v2/spec (list)
        => {
            api::list_specs(req, global).await
        }
        (hyper::Method::GET, _) // /v2/spec/<uuid>
            if {
                Regex::new(r"^/v2/spec/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$").unwrap().is_match(req.uri().path())
            } =>
        {
            let r = Regex::new(
                r"^/v2/spec/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$",
            );
            let uri = req.uri().path().to_string();
            let spec_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            api::get_spec(req, global, spec_id).await
        }
        (hyper::Method::DELETE, _) // /v2/spec/<uuid>
            if {
                Regex::new(r"^/v2/spec/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$").unwrap().is_match(req.uri().path())
            } =>
        {
            let r = Regex::new(
                r"^/v2/spec/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$",
            );
            let uri = req.uri().path().to_string();
            let spec_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            api::delete_spec(req, global, spec_id).await
        }
        (hyper::Method::POST, "/v2/spec") // /v2/spec
        => {
            api::push_spec(req, global).await
        }
        (hyper::Method::POST, _) // /v2/spec/<uuid>/part
            if {
                Regex::new(r"^/v2/spec/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/part$").unwrap().is_match(req.uri().path())
            } =>
        {
            let r = Regex::new(
                r"^/v2/spec/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})/part$",
            );
            let uri = req.uri().path().to_string();
            let spec_id = r.unwrap().captures(&uri).unwrap().get(1).unwrap().as_str();
            api::push_part(req, global, spec_id).await
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
