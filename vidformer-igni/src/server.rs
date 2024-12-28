use super::IgniError;
use http_body_util::BodyExt;
use log::*;
use num::Rational64;
use regex::Regex;
use std::collections::BTreeMap;
use uuid::Uuid;

use super::ServerOpt;

fn filters() -> BTreeMap<String, Box<dyn vidformer::filter::Filter>> {
    let mut filters: BTreeMap<String, Box<dyn vidformer::filter::Filter>> = BTreeMap::new();
    filters.extend(vidformer::filter::builtin::filters());
    filters.extend(vidformer::filter::cv2::filters());
    filters
}

struct IgniServerGlobal {
    pool: sqlx::Pool<sqlx::Postgres>,
}

pub(crate) async fn cmd_server(
    pool: sqlx::Pool<sqlx::Postgres>,
    opt: ServerOpt,
) -> Result<(), IgniError> {
    use hyper::server::conn::http1;
    use hyper_util::rt::TokioIo;

    let global = std::sync::Arc::new(IgniServerGlobal { pool });
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
                error!("Error serving connection: {:?}", err);
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
            get_playlist(req, global, spec_id).await
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
            get_stream(req, global, spec_id).await
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
            get_segment(req, global, spec_id, segment_number).await
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
            get_source(req, global, source_id).await
        }
        (hyper::Method::POST, "/v2/source") // /v2/source
        => {
            push_source(req, global).await
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
            get_spec(req, global, spec_id).await
        }
        (hyper::Method::POST, "/v2/spec") // /v2/spec
        => {
            push_spec(req, global).await
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
            push_part(req, global, spec_id).await
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

async fn get_playlist(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    spec_id: &str,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let spec_id = Uuid::parse_str(spec_id).unwrap();

    let mut transaction = global.pool.begin().await?;
    let row: Option<bool> = sqlx::query_scalar("SELECT closed FROM spec WHERE id = $1")
        .bind(spec_id)
        .fetch_optional(&mut *transaction)
        .await?;

    match row {
        None => {
            transaction.commit().await?;
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::NOT_FOUND)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Not found",
                )))?);
        }
        Some(closed) => {
            if closed {
                transaction.commit().await?;
                return Ok(hyper::Response::builder()
                    .status(hyper::StatusCode::FORBIDDEN)
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        "VOD is closed",
                    )))?);
            }
        }
    }

    let playlist_text = format!(
        "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=640000\nhttp://localhost:8080/vod/{}/stream.m3u8\n",
        spec_id
    );

    transaction.commit().await?;

    Ok(hyper::Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "application/vnd.apple.mpegURL")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            playlist_text,
        )))?)
}

async fn get_stream(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    spec_id: &str,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let spec_id = Uuid::parse_str(spec_id).unwrap();

    let mut transaction = global.pool.begin().await?;
    let row: Option<(bool, i32, i32)> = sqlx::query_as("SELECT closed, vod_segment_length_num n, vod_segment_length_denom d FROM spec WHERE id = $1")
        .bind(spec_id)
        .fetch_optional(&mut *transaction)
        .await?;

    match row {
        None => {
            transaction.commit().await?;
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::NOT_FOUND)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Not found",
                )))?);
        }
        Some((closed, _, _)) => {
            if closed {
                transaction.commit().await?;
                return Ok(hyper::Response::builder()
                    .status(hyper::StatusCode::FORBIDDEN)
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        "VOD is closed",
                    )))?);
            }
        }
    }

    // Get vod_segment rows
    let rows: Vec<(i32,)> = sqlx::query_as(
        "SELECT segment_number FROM vod_segment WHERE spec_id = $1 ORDER BY segment_number",
    )
    .bind(spec_id)
    .fetch_all(&mut *transaction)
    .await?;
    transaction.commit().await?;

    let mut stream_text =
        "#EXTM3U\n#EXT-X-PLAYLIST-TYPE:VOD\n#EXT-X-TARGETDURATION:2\n#EXT-X-VERSION:4\n#EXT-X-MEDIA-SEQUENCE:0\n".to_string();
    for (segment_number,) in rows {
        stream_text.push_str(&format!(
            "#EXTINF:2.0,\nhttp://localhost:8080/vod/{}/segment-{}.ts\n",
            spec_id, segment_number
        ));
    }
    stream_text.push_str("#EXT-X-ENDLIST\n");

    Ok(hyper::Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "application/vnd.apple.mpegURL")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            stream_text,
        )))?)
}

async fn get_segment(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    spec_id: &str,
    segment_number: i32,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let spec_id = Uuid::parse_str(spec_id).unwrap();

    // Check that the spec exists, is not closed, and grab the width, height, and pix_fmt
    let mut transaction = global.pool.begin().await?;
    let row: Option<(i32, i32, String, bool)> =
        sqlx::query_as("SELECT width, height, pix_fmt, closed FROM spec WHERE id = $1")
            .bind(spec_id)
            .fetch_optional(&mut *transaction)
            .await?;

    let (width, height, pix_fmt) = match row {
        None => {
            transaction.commit().await?;
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::NOT_FOUND)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Not found",
                )))?);
        }
        Some((width, height, pix_fmt, closed)) => {
            if closed {
                transaction.commit().await?;
                return Ok(hyper::Response::builder()
                    .status(hyper::StatusCode::FORBIDDEN)
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        "VOD is closed",
                    )))?);
            }

            (width, height, pix_fmt)
        }
    };

    // Check that the segment exists, also grab the first_t and last_t
    let row: Option<(i32, i32)> = sqlx::query_as(
        "SELECT first_t, last_t FROM vod_segment WHERE spec_id = $1 AND segment_number = $2",
    )
    .bind(spec_id)
    .bind(segment_number)
    .fetch_optional(&mut *transaction)
    .await?;

    let (first_t, last_t) = match row {
        None => {
            transaction.commit().await?;
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::NOT_FOUND)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Not found",
                )))?);
        }
        Some(row) => row,
    };

    // Get the frames from spec_t that are in the segment (pos between first_t and last_t)
    let rows: Vec<(i64, i64, serde_json::Value)> = sqlx::query_as(
        "SELECT t_numer, t_denom, frame FROM spec_t WHERE spec_id = $1 AND pos BETWEEN $2 AND $3",
    )
    .bind(spec_id)
    .bind(first_t)
    .bind(last_t)
    .fetch_all(&mut *transaction)
    .await?;

    // map times to rational
    let times: Vec<num_rational::Ratio<i64>> = rows
        .iter()
        .map(|(t_numer, t_denom, _)| num_rational::Ratio::new(*t_numer, *t_denom))
        .collect();
    let start = *times.first().unwrap();
    let end = *times.last().unwrap();

    // Map json values to FrameExpr
    let frames: Vec<vidformer::sir::FrameExpr> = rows
        .iter()
        .map(|(_, _, frame)| serde_json::from_value(frame.clone()).unwrap())
        .collect();

    struct IgniSpec {
        times: Vec<num_rational::Ratio<i64>>,
        frames: Vec<vidformer::sir::FrameExpr>,
    }

    impl vidformer::spec::Spec for IgniSpec {
        fn domain(&self, _: &dyn vidformer::spec::SpecContext) -> Vec<num_rational::Ratio<i64>> {
            self.times.clone()
        }

        fn render(
            &self,
            _: &dyn vidformer::spec::SpecContext,
            t: &num_rational::Ratio<i64>,
        ) -> vidformer::sir::FrameExpr {
            let idx = self.times.binary_search(t).unwrap();
            self.frames[idx].clone()
        }
    }

    let spec = IgniSpec { times, frames };
    let spec = std::sync::Arc::new(std::boxed::Box::new(spec) as Box<dyn vidformer::spec::Spec>);
    let sources = {
        let mut out = vec![];

        // load all data from source
        let rows: Vec<(uuid::Uuid, String, i32, String, serde_json::Value, String, String, i32, i32, i64)> = sqlx::query_as("SELECT id, name, stream_idx, storage_service, storage_config, codec, pix_fmt, width, height, file_size FROM source")
            .fetch_all(&mut *transaction)
            .await
            ?;

        for (
            source_id,
            name,
            stream_idx,
            storage_service,
            storage_config,
            codec,
            pix_fmt,
            width,
            height,
            file_size,
        ) in rows
        {
            let (ts, keys): (Vec<Rational64>, Vec<Rational64>) = {
                let rows: Vec<(i64, i64, bool)> = sqlx::query_as(
                    "SELECT t_num, t_denom, key FROM source_t WHERE source_id = $1 ORDER BY pos",
                )
                .bind(source_id)
                .fetch_all(&mut *transaction)
                .await?;

                let ts: Vec<Rational64> = rows
                    .iter()
                    .map(|(t_num, t_denom, _)| Rational64::new(*t_num, *t_denom))
                    .collect();

                let keys: Vec<Rational64> = rows
                    .iter()
                    .map(|(t_num, t_denom, key)| {
                        if *key {
                            Rational64::new(*t_num, *t_denom)
                        } else {
                            Rational64::new(0, 1)
                        }
                    })
                    .collect();

                (ts, keys)
            };

            let storage_config_json = serde_json::to_string(&storage_config).unwrap();
            let service = crate::ops::parse_storage_config(&storage_config_json).unwrap();
            let service = vidformer::service::Service::new(storage_service, service.1);

            out.push(vidformer::source::SourceVideoStreamMeta {
                name: source_id.to_string(),
                file_path: name,
                stream_idx: stream_idx as usize,
                file_size: file_size as u64,
                codec,
                pix_fmt,
                service,
                resolution: (width as usize, height as usize),
                ts,
                keys,
            });
        }

        out
    };
    transaction.commit().await?;

    {
        // Print the sources, but not the ts and keys
        for source in &sources {
            info!(
                "source: {} {} {} {} {} {} {:?} {} {}",
                source.name,
                source.file_path,
                source.stream_idx,
                source.file_size,
                source.codec,
                source.pix_fmt,
                source.service,
                source.resolution.0,
                source.resolution.1
            );
        }
    }

    let arrays = std::collections::BTreeMap::new();
    let filters = filters();
    let context = vidformer::Context::new(sources, arrays, filters);
    let context = std::sync::Arc::new(context);

    let dve_config: vidformer::Config = vidformer::Config {
        decode_pool_size: 50,
        decoder_view: 50,
        decoders: u16::MAX as usize,
        filterers: 8,
        output_width: width as usize,
        output_height: height as usize,
        output_pix_fmt: pix_fmt,
        encoder: None,
        format: None,
    };

    let output_path = format!("/tmp/{}.ts", Uuid::new_v4());
    let output_path2 = output_path.clone();

    // Run the spec in a blocking task
    let context = std::sync::Arc::new(context);
    let dve_config = std::sync::Arc::new(dve_config);
    let output_path = std::sync::Arc::new(output_path);

    let dve_range_config = vidformer::Range {
        start,
        end,
        ts_format: vidformer::RangeTsFormat::StreamLocal,
    };

    let stats = tokio::task::spawn_blocking(move || {
        vidformer::run(
            &spec,
            &output_path,
            &context,
            &dve_config,
            &Some(dve_range_config),
        )
    })
    .await
    .expect("Error joining blocking task");

    if let Err(err) = stats {
        return Err(IgniError::General(format!(
            "Error running vidformer spec: {:?}",
            err
        )));
    }

    let stats = stats.unwrap();
    dbg! {&stats};

    let output = match tokio::fs::read(output_path2.as_str()).await {
        Ok(ok) => ok,
        Err(err) => {
            return Err(IgniError::General(format!(
                "Failed to read temporary file: {}",
                err
            )))
        }
    };

    let res = hyper::Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "video/MP2T")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(output)))?;

    match tokio::fs::remove_file(output_path2.as_str()).await {
        Ok(_) => Ok(res),
        Err(err) => Err(IgniError::General(format!(
            "Failed to remove temporary file: {}",
            err
        ))),
    }
}

async fn get_source(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    source_id: &str,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let source_id = Uuid::parse_str(source_id).unwrap();

    let mut transaction = global.pool.begin().await?;
    let row: Option<(String, i32, String, serde_json::Value, String, String, i32, i32)> =
        sqlx::query_as("SELECT name, stream_idx, storage_service, storage_config, codec, pix_fmt, width, height FROM source WHERE id = $1")
            .bind(source_id)
            .fetch_optional(&mut *transaction)
            .await
            ?;

    if row.is_none() {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::NOT_FOUND)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Not found",
            )))?);
    }

    let row = row.unwrap();
    let name = row.0;
    let stream_idx = row.1;
    let storage_service = row.2;
    let storage_config = row.3;
    let codec = row.4;
    let pix_fmt = row.5;
    let width = row.6;
    let height = row.7;

    let row: Vec<(i64, i64, bool)> = sqlx::query_as(
        "SELECT t_num, t_denom, key FROM source_t WHERE source_id = $1 ORDER BY pos",
    )
    .bind(source_id)
    .fetch_all(&mut *transaction)
    .await?;

    transaction.commit().await?;

    let ts: Vec<Vec<serde_json::Value>> = row
        .iter()
        .map(|(t_num, t_denom, key)| {
            vec![
                serde_json::Value::Number(serde_json::Number::from(*t_num)),
                serde_json::Value::Number(serde_json::Number::from(*t_denom)),
                serde_json::Value::Bool(*key),
            ]
        })
        .collect();

    let res = serde_json::json!({
        "id": source_id,
        "name": name,
        "stream_idx": stream_idx,
        "storage_service": storage_service,
        "storage_config": storage_config,
        "codec": codec,
        "pix_fmt": pix_fmt,
        "width": width,
        "height": height,
        "ts": ts,
    });

    Ok(hyper::Response::builder()
        .header("Content-Type", "application/json")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            serde_json::to_string(&res).unwrap(),
        )))?)
}

async fn push_source(
    req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let req: Vec<u8> = match req.collect().await {
        Err(_err) => {
            error!("Error reading request body");
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Bad request",
                )))?);
        }
        Ok(req) => req.to_bytes().to_vec(),
    };

    #[derive(serde::Deserialize)]
    struct RequestContent {
        name: String,
        stream_idx: i32,
        storage_service: String,
        storage_config: serde_json::Value,
    }

    let req: RequestContent = match serde_json::from_slice(&req) {
        Err(err) => {
            error!("Error parsing request body");
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    format!("Bad request: {}", err),
                )))?);
        }
        Ok(req) => req,
    };

    let name = req.name;
    let stream_idx = req.stream_idx;
    let storage_service = req.storage_service;
    let storage_config_json = serde_json::to_string(&req.storage_config).unwrap();

    let uuid = crate::ops::profile_and_add_source(
        &global.pool,
        name,
        stream_idx as usize,
        &storage_service,
        &storage_config_json,
    )
    .await;

    match uuid {
        Ok(uuid) => {
            let res = serde_json::json!({
                "status": "ok",
                "id": uuid,
            });

            Ok(hyper::Response::builder()
                .header("Content-Type", "application/json")
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    serde_json::to_string(&res).unwrap(),
                )))?)
        }
        Err(err) => {
            // TODO: This should have a more specific error message
            error!("Error profiling and adding source: {:?}", err);
            Ok(hyper::Response::builder()
                .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Internal server error",
                )))?)
        }
    }
}

async fn get_spec(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    source_id: &str,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let source_id = Uuid::parse_str(source_id).unwrap();

    let mut transaction = global.pool.begin().await?;
    let row: Option<(i32, i32, String, i32, i32, Option<String>, Option<String>, i32, bool, bool)> =
        sqlx::query_as("SELECT width, height, pix_fmt, vod_segment_length_num, vod_segment_length_denom, ready_hook, steer_hook, applied_parts, terminated, closed FROM spec WHERE id = $1")
            .bind(source_id)
            .fetch_optional(&mut *transaction)
            .await
            ?;

    if row.is_none() {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::NOT_FOUND)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Not found",
            )))?);
    }

    let row = row.unwrap();
    let width = row.0;
    let height = row.1;
    let pix_fmt = row.2;
    let vod_segment_length_num = row.3;
    let vod_segment_length_denom = row.4;
    let ready_hook = row.5;
    let steer_hook = row.6;
    let applied_parts = row.7;
    let terminated = row.8;
    let closed = row.9;

    let res = serde_json::json!({
        "id": source_id,
        "width": width,
        "height": height,
        "pix_fmt": pix_fmt,
        "vod_segment_length": [vod_segment_length_num, vod_segment_length_denom],
        "ready_hook": ready_hook,
        "steer_hook": steer_hook,
        "applied_parts": applied_parts,
        "terminated": terminated,
        "closed": closed,
        "playlist": format!("http://localhost:8080/vod/{}/playlist.m3u8", source_id),
    });

    transaction.commit().await?;

    Ok(hyper::Response::builder()
        .header("Content-Type", "application/json")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            serde_json::to_string(&res).unwrap(),
        )))?)
}

async fn push_spec(
    req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let req: Vec<u8> = match req.collect().await {
        Err(_err) => {
            error!("Error reading request body");
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Bad request",
                )))?);
        }
        Ok(req) => req.to_bytes().to_vec(),
    };

    #[derive(serde::Deserialize)]
    struct RequestContent {
        width: i32,
        height: i32,
        pix_fmt: String,
        vod_segment_length: [i32; 2],
        ready_hook: Option<String>,
        steer_hook: Option<String>,
    }

    let req: RequestContent = match serde_json::from_slice(&req) {
        Err(err) => {
            error!("Error parsing request body");
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    format!("Bad request: {}", err),
                )))?);
        }
        Ok(req) => req,
    };

    let vod_segment_length_num = req.vod_segment_length[0];
    let vod_segment_length_denom = req.vod_segment_length[1];

    let spec = crate::ops::add_spec(
        &global.pool,
        vod_segment_length_num,
        vod_segment_length_denom,
        req.height,
        req.width,
        req.pix_fmt,
        req.ready_hook,
        req.steer_hook,
    )
    .await;

    match spec {
        Ok(spec) => {
            let res = serde_json::json!({
                "status": "ok",
                "id": spec,
            });

            Ok(hyper::Response::builder()
                .header("Content-Type", "application/json")
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    serde_json::to_string(&res).unwrap(),
                )))?)
        }
        Err(err) => {
            error!("Error adding spec: {:?}", err);
            Ok(hyper::Response::builder()
                .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Internal server error",
                )))?)
        }
    }
}

async fn push_part(
    req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    spec_id: &str,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let spec_id = Uuid::parse_str(spec_id).unwrap();

    let req: Vec<u8> = match req.collect().await {
        Err(_err) => {
            error!("Error reading request body");
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Bad request",
                )))?);
        }
        Ok(req) => req.to_bytes().to_vec(),
    };

    #[derive(serde::Deserialize, serde::Serialize)]
    struct RequestContent {
        pos: usize,
        terminal: bool,
        frames: Vec<((i64, i64), Option<vidformer::sir::FrameExpr>)>,
    }

    let req: RequestContent = match serde_json::from_slice(&req) {
        Err(err) => {
            error!("Error parsing request body");
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    format!("Bad request: {}", err),
                )))?);
        }
        Ok(req) => req,
    };

    if req.frames.is_empty() && !req.terminal {
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::BAD_REQUEST)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Cannot push a non-terminal part with no frames",
            )))?);
    }

    // stage frames
    let spec_ids = vec![spec_id; req.frames.len()];
    let pos = vec![req.pos as i32; req.frames.len()];
    let mut in_part_poss = Vec::with_capacity(req.frames.len());
    let mut t_numers = Vec::with_capacity(req.frames.len());
    let mut t_denoms = Vec::with_capacity(req.frames.len());
    let mut frames: Vec<Option<serde_json::Value>> = Vec::with_capacity(req.frames.len());
    for (idx, ((numer, denom), frame)) in req.frames.iter().enumerate() {
        in_part_poss.push(idx as i32);
        t_numers.push(numer);
        t_denoms.push(denom);
        if let Some(expr) = frame {
            frames.push(Some(serde_json::to_value(expr).unwrap()));
        } else {
            frames.push(None);
        }
    }

    let mut transaction = global.pool.begin().await?;
    let row: Option<(Uuid, bool, i32, i32, i32)> =
        sqlx::query_as("SELECT id, terminated OR closed, applied_parts, vod_segment_length_num, vod_segment_length_denom FROM spec WHERE id = $1")
            .bind(spec_id)
            .fetch_optional(&mut *transaction)
            .await
            ?;

    if row.is_none() {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::NOT_FOUND)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Not found",
            )))?);
    }
    let row = row.unwrap();

    if row.1 {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::FORBIDDEN)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Forbidden to push to a terminated or closed spec",
            )))?);
    }
    let applied_parts = row.2;
    let vod_segment_length_num = row.3;
    let vod_segment_length_denom = row.4;

    // check if the part is already in the database
    if req.pos < applied_parts as usize {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::BAD_REQUEST)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Part already exists",
            )))?);
    }

    let row: Option<i32> =
        sqlx::query_scalar("SELECT pos FROM spec_part_staged WHERE spec_id = $1 AND pos = $2")
            .bind(spec_id)
            .bind(req.pos as i32)
            .fetch_optional(&mut *transaction)
            .await?;

    if row.is_some() {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::BAD_REQUEST)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Part already exists",
            )))?);
    }

    // check if there is a terminal part before this part, if so, reject
    let row: Option<i32> =
        sqlx::query_scalar("SELECT MIN(pos) FROM spec_part_staged WHERE spec_id = $1 AND terminal")
            .bind(spec_id)
            .bind(req.pos as i32)
            .fetch_one(&mut *transaction)
            .await?;

    if let Some(min_terminal_pos) = row {
        if req.terminal {
            transaction.commit().await?;
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Cannot push a second terminal part",
                )))?);
        }

        if min_terminal_pos < req.pos as i32 {
            transaction.commit().await?;
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Cannot push a part after a terminal part",
                )))?);
        }
    }

    // insert the part into spec_part_staged
    sqlx::query("INSERT INTO spec_part_staged (spec_id, pos, terminal) VALUES ($1, $2, $3)")
        .bind(spec_id)
        .bind(req.pos as i32)
        .bind(req.terminal)
        .execute(&mut *transaction)
        .await?;

    sqlx::query("INSERT INTO spec_part_staged_t (spec_id, pos, in_part_pos, t_numer, t_denom, frame) VALUES (unnest($1::uuid[]), unnest($2::int[]), unnest($3::int[]), unnest($4::bigint[]), unnest($5::bigint[]), unnest($6::jsonb[]))")
        .bind(&spec_ids)
        .bind(&pos)
        .bind(&in_part_poss)
        .bind(&t_numers)
        .bind(&t_denoms)
        .bind(&frames)
        .execute(&mut *transaction)
        .await
        ?;

    // Check if this part makes a contiguous sequence of parts that can be applied
    let row: Option<i32> = sqlx::query_scalar("WITH ordered AS (SELECT pos, ROW_NUMBER() OVER (ORDER BY pos) AS rn FROM spec_part_staged WHERE spec_id = $1) SELECT MAX(pos) AS max_ready_pos FROM ordered WHERE (pos - rn) = ($2 - 1)")
        .bind(spec_id)
        .bind(applied_parts)
        .fetch_one(&mut *transaction)
        .await
        ?;

    if let Some(max_ready_pos) = row {
        info!(
            "Applying parts [{}, {}] on spec {}",
            applied_parts, max_ready_pos, spec_id
        );

        // Insert spec_part_staged_t values into spec_t
        // A new global pos is assigned to each part from the part-local in_part_pos value
        // After this operation, the spec_t pos values are contiguous
        sqlx::query("WITH max_pos AS (SELECT COALESCE(MAX(pos), -1) AS mp FROM spec_t WHERE spec_id = $1) INSERT INTO spec_t (spec_id, pos, t_numer, t_denom, frame) SELECT spec_id, ROW_NUMBER() OVER (ORDER BY pos, in_part_pos) + max_pos.mp AS pos, t_numer, t_denom, frame FROM spec_part_staged_t, max_pos WHERE spec_id = $1 AND pos <= $2;")
            .bind(spec_id)
            .bind(max_ready_pos)
            .execute(&mut *transaction)
            .await
            ?;

        // If the last part is terminal, update the terminal field in spec
        sqlx::query("UPDATE spec SET terminated = TRUE WHERE id = $1 AND (SELECT terminal FROM spec_part_staged WHERE spec_id = $1 AND pos = $2)")
            .bind(spec_id)
            .bind(max_ready_pos)
            .execute(&mut *transaction)
            .await
            ?;

        // Delete the parts up to and including max_ready_pos from spec_part_staged_t
        sqlx::query("DELETE FROM spec_part_staged_t WHERE spec_id = $1 AND pos <= $2")
            .bind(spec_id)
            .bind(max_ready_pos)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM spec_part_staged WHERE spec_id = $1 AND pos <= $2")
            .bind(spec_id)
            .bind(max_ready_pos)
            .execute(&mut *transaction)
            .await?;

        // Update the applied_parts field in spec
        sqlx::query("UPDATE spec SET applied_parts = $1 + 1 WHERE id = $2")
            .bind(max_ready_pos)
            .bind(spec_id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query("INSERT INTO vod_segment (spec_id, segment_number, first_t, last_t) WITH expected_t_segments AS (
    SELECT 
        spec_id, 
        pos, 
        (spec_t.t_numer * $2) / $1 / spec_t.t_denom AS expected_segment_idx
    FROM 
        spec_t
), 
expected_segments AS (
    SELECT 
        spec_id, 
        expected_segment_idx AS segment_idx, 
        MIN(pos) AS first_t, 
        MAX(pos) AS last_t
    FROM 
        expected_t_segments
    GROUP BY 
        spec_id, expected_segment_idx
), 
max_vod_segments AS (
    SELECT 
        spec_id, 
        COALESCE(MAX(segment_number), -1) AS max_segment
    FROM 
        vod_segment
    GROUP BY 
        spec_id
)
SELECT 
    es.spec_id, 
    es.segment_idx segment_number, 
    es.first_t, 
    es.last_t
FROM 
    expected_segments es
LEFT JOIN 
    max_vod_segments mvs
ON 
    es.spec_id = mvs.spec_id
WHERE 
    es.segment_idx > COALESCE(mvs.max_segment, -1);")
    .bind(vod_segment_length_num)
    .bind(vod_segment_length_denom)

            .execute(&mut *transaction)
            .await?;
    }

    transaction.commit().await?;

    Ok(hyper::Response::builder()
        .status(hyper::StatusCode::OK)
        .body(http_body_util::Full::new(hyper::body::Bytes::from("")))?)
}
