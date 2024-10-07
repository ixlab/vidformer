use super::YrdenCmd;
use crate::source;
use crate::spec;
use base64::Engine;
use log::*;
use num::Rational64;
use rayon::prelude::*;
use regex::Regex;
use rusqlite::{Connection, Result};
use std::collections::BTreeMap;
use tokio::io::AsyncReadExt;

pub(crate) fn cmd_yrden(opt: &YrdenCmd) {
    let host_prefix = format!("http://localhost:{}", opt.port);
    let global = YrdenGlobal {
        host_prefix: host_prefix.clone(),
        port: opt.port,
        print_url: opt.print_url,
        namespaces: BTreeMap::new(),
        db_path: opt.db_path.clone(),
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(yrden_main(global));
}

struct YrdenGlobal {
    host_prefix: String,
    port: u16,
    print_url: bool,
    namespaces: BTreeMap<String, std::sync::Arc<YrdenNamespace>>,
    db_path: String,
}

impl YrdenGlobal {
    fn init_db(&self) {
        let conn = Connection::open(&self.db_path).unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sources (
                name TEXT PRIMARY KEY,
                meta TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
    }
}

fn load_source_meta(
    db_path: &str,
    name: &str,
    path: &str,
    stream: usize,
    service: Option<&vidformer::service::Service>,
) -> Result<source::SourceVideoStreamMeta, vidformer::Error> {
    let conn = Connection::open(db_path).unwrap();
    let mut stmt = conn
        .prepare("SELECT meta FROM sources WHERE name = ?")
        .unwrap();

    let mut rows = stmt.query([name]).unwrap();
    if let Some(row) = rows.next().unwrap() {
        let meta: String = row.get(0).unwrap();
        let meta: source::SourceVideoStreamMeta = serde_json::from_str(&meta).unwrap();
        return Ok(meta);
    }
    drop(rows);
    drop(stmt);
    drop(conn);

    let new_source = crate::opendal_source(name, path, stream, service)?;

    let conn: Connection = Connection::open(db_path).unwrap();
    let meta = serde_json::to_string(&new_source).unwrap();
    conn.execute(
        "INSERT INTO sources (name, meta) VALUES (?, ?)",
        [name, &meta],
    )
    .unwrap();

    Ok(new_source)
}

struct YrdenNamespace {
    context: std::sync::Arc<vidformer::Context>,
    dve_config: std::sync::Arc<vidformer::Config>,
    spec: std::sync::Arc<Box<dyn crate::spec::Spec>>,
    playlist: String,
    stream: String,
    ranges: Vec<(Rational64, Rational64)>,
}

async fn yrden_main(global: YrdenGlobal) {
    use hyper::server::conn::http1;
    use hyper_util::rt::TokioIo;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    let addr: SocketAddr = format!("[::]:{}", global.port).parse().unwrap();
    let listener = TcpListener::bind(addr).await.unwrap();

    global.init_db();

    if global.print_url {
        // VSCode looks for urls in the output to forward ports.
        println!("vidformer-yrden server open at {}", global.host_prefix);
    }

    let global = std::sync::Arc::new(std::sync::Mutex::new(global));

    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let io = TokioIo::new(stream);
        let global = global.clone();

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    hyper::service::service_fn(|req| yrden_http_req(req, global.clone())),
                )
                .await
            {
                debug!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn yrden_http_req(
    req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<std::sync::Mutex<YrdenGlobal>>,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, std::convert::Infallible> {
    let uri = req.uri().path();
    debug!("Request: {}", uri);

    use http_body_util::BodyExt;

    #[derive(Debug, serde::Deserialize)]
    struct YrdenSource {
        name: String,
        stream: usize,
        path: String,
        service: Option<vidformer::service::Service>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct YrdenFilter {
        filter: String,
        args: BTreeMap<String, String>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct YrdenRequest {
        spec: String,
        sources: Vec<YrdenSource>,
        filters: BTreeMap<String, YrdenFilter>,
        arrays: Vec<String>,
        width: u32,
        height: u32,
        pix_fmt: String,
        output_path: Option<String>,
        encoder: Option<String>,
        encoder_opts: Option<BTreeMap<String, String>>,
        format: Option<String>,
    }

    #[derive(Debug, serde::Serialize)]
    struct YrdenResponse {
        namespace: String,
        playlist_url: String,
        stream_url: String,
        player_url: String,
    }

    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/") => Ok(hyper::Response::builder()
            .header("Access-Control-Allow-Origin", "*")
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                format!("vidformer-yrden v{}\n", env!("CARGO_PKG_VERSION")),
            )))
            .unwrap()),
        (&hyper::Method::POST, "/source") => {
            // For example: curl localhost:8000/source -X POST -H "Content-Type: application/json" -d "{\"name\":\"tos\", \"path\":\"tos.mp4\", \"stream\":0}"
            let whole_body = req.collect().await;
            let body: Vec<u8> = match whole_body {
                Ok(body) => body,
                Err(_err) => {
                    return Ok(hyper::Response::builder()
                        .status(hyper::StatusCode::BAD_REQUEST)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(
                            "Error reading body".to_string(),
                        )))
                        .unwrap());
                }
            }
            .to_bytes()
            .into();

            #[derive(Debug, serde::Deserialize)]
            struct SourceRequest {
                name: String,
                stream: usize,
                path: String,
                service: Option<vidformer::service::Service>,
            }

            let request: SourceRequest = match serde_json::from_slice(&body) {
                Ok(body) => body,
                Err(err) => {
                    return Ok(hyper::Response::builder()
                        .status(hyper::StatusCode::BAD_REQUEST)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(
                            format!("Error parsing body: {}", err),
                        )))
                        .unwrap());
                }
            };

            let service = request.service;

            let db_path = {
                let global: std::sync::MutexGuard<'_, YrdenGlobal> = global.lock().unwrap();
                global.db_path.clone()
            };

            let source = tokio::task::spawn_blocking(move || {
                load_source_meta(
                    &db_path,
                    &request.name,
                    &request.path,
                    request.stream,
                    service.as_ref(),
                )
            })
            .await;

            let source = match source {
                Ok(source) => source,
                Err(err) => {
                    return Ok(hyper::Response::builder()
                        .status(hyper::StatusCode::BAD_REQUEST)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(
                            format!("Unknown failure when loading source: {}", err),
                        )))
                        .unwrap());
                }
            };

            let source = match source {
                Ok(source) => source,
                Err(err) => {
                    return Ok(hyper::Response::builder()
                        .status(hyper::StatusCode::BAD_REQUEST)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(
                            format!("Error loading source: {}", err),
                        )))
                        .unwrap());
                }
            };

            #[derive(Debug, serde::Serialize)]
            struct SourceResponse {
                name: String,
                path: String,
                stream: usize,
                width: usize,
                height: usize,
                pix_fmt: String,
                ts: Vec<Rational64>,
            }

            let response = SourceResponse {
                name: source.name.clone(),
                path: source.file_path.clone(),
                stream: source.stream_idx,
                width: source.resolution.0,
                height: source.resolution.1,
                pix_fmt: source.pix_fmt.clone(),
                ts: source.ts.clone(),
            };

            let response = serde_json::to_string(&response).unwrap();
            Ok(hyper::Response::builder()
                .header("Access-Control-Allow-Origin", "*")
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    response,
                )))
                .unwrap())
        }
        (&hyper::Method::POST, "/new") => {
            // For example: curl localhost:8000/new -X POST -H "Content-Type: application/json" -d "{\"spec\":\"myspec.json\", \"sources\":[\"tos:tos.mp4:0\"], \"arrays\":[], \"width\":3840, \"height\":1714, \"pix_fmt\":\"yuv420p\"}"
            let whole_body = req.collect().await;
            let body: Vec<u8> = match whole_body {
                Ok(body) => body,
                Err(_err) => {
                    return Ok(hyper::Response::builder()
                        .status(hyper::StatusCode::BAD_REQUEST)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(
                            "Error reading body".to_string(),
                        )))
                        .unwrap());
                }
            }
            .to_bytes()
            .into();

            let start_time = std::time::Instant::now();
            let request: YrdenRequest = match serde_json::from_slice(&body) {
                Ok(body) => body,
                Err(err) => {
                    return Ok(hyper::Response::builder()
                        .status(hyper::StatusCode::BAD_REQUEST)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(
                            format!("Error parsing body: {}", err),
                        )))
                        .unwrap());
                }
            };
            let end_time = std::time::Instant::now();
            debug!("Parsed request in {:?}", end_time - start_time);

            let db_path = {
                let global: std::sync::MutexGuard<'_, YrdenGlobal> = global.lock().unwrap();
                global.db_path.clone()
            };

            let start_time = std::time::Instant::now();
            let sources: Vec<Result<source::SourceVideoStreamMeta, vidformer::Error>> =
                tokio::task::spawn_blocking(move || {
                    request
                        .sources
                        .par_iter()
                        .map(|source| {
                            load_source_meta(
                                &db_path,
                                &source.name,
                                &source.path,
                                source.stream,
                                source.service.as_ref(),
                            )
                        })
                        .collect()
                })
                .await
                .unwrap();

            if sources.iter().any(|s| s.is_err()) {
                return Ok(hyper::Response::builder()
                    .status(hyper::StatusCode::BAD_REQUEST)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        "Error loading sources".to_string(),
                    )))
                    .unwrap());
            }
            let sources = sources.into_iter().map(|s| s.unwrap()).collect();

            let end_time = std::time::Instant::now();
            debug!("Loaded sources in {:?}", end_time - start_time);

            let start_time = std::time::Instant::now();
            let arrays: BTreeMap<String, Box<dyn vidformer::array::Array>> = request
                .arrays
                .par_iter()
                .map(|path| {
                    let split = path.split(':').collect::<Vec<&str>>();
                    assert_eq!(2, split.len());
                    let (name, path) = (split[0], split[1]);

                    let array_file = std::fs::File::open(path).unwrap();
                    let array: vidformer::array::JsonArary =
                        serde_json::from_reader(array_file).unwrap();
                    let array: Box<dyn vidformer::array::Array> = Box::new(array);
                    (name.to_string(), array)
                })
                .collect();
            let end_time = std::time::Instant::now();
            debug!("Loaded arrays in {:?}", end_time - start_time);

            let spec_json_gzip_base64 = &request.spec;
            let spec_content = base64::prelude::BASE64_STANDARD
                .decode(spec_json_gzip_base64.as_bytes())
                .unwrap();
            let spec_content = flate2::read::GzDecoder::new(&spec_content[..]);
            let spec_content = std::io::BufReader::new(spec_content);
            let spec: spec::JsonSpec =
                serde_json::from_reader(spec_content).expect("Unable to parse JSON");

            let spec: Box<dyn spec::Spec> = Box::new(spec);
            let spec = std::sync::Arc::new(spec);
            let mut filters = crate::default_filters();

            for (name, filter) in request.filters {
                if let std::collections::btree_map::Entry::Vacant(e) = filters.entry(name) {
                    assert!(filter.filter == "IPC");
                    let filter = crate::filter::builtin::IPC::via_map(&filter.args);

                    let filter = match filter {
                        Ok(filter) => filter,
                        Err(err) => {
                            return Ok(hyper::Response::builder()
                                .status(hyper::StatusCode::BAD_REQUEST)
                                .header("Access-Control-Allow-Origin", "*")
                                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                                    format!("Error establishing UDF: {}", err),
                                )))
                                .unwrap());
                        }
                    };

                    e.insert(Box::new(filter));
                }
            }

            let context: vidformer::Context = vidformer::Context::new(sources, arrays, filters);
            let context = std::sync::Arc::new(context);

            let dve_config: vidformer::Config = vidformer::Config {
                decode_pool_size: 50,
                decoder_view: usize::MAX,
                decoders: u16::MAX as usize,
                filterers: 8,
                output_width: request.width as usize,
                output_height: request.height as usize,
                output_pix_fmt: request.pix_fmt,

                encoder: None,
                format: None,
            };
            let dve_config = std::sync::Arc::new(dve_config);

            // Validate the spec
            let validation_status = vidformer::validate(&spec, &context, &dve_config);
            if let Err(err) = validation_status {
                return Ok(hyper::Response::builder()
                    .status(hyper::StatusCode::BAD_REQUEST)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        format!("Error validating spec: {}", err),
                    )))
                    .unwrap());
            }

            let host_prefix = {
                let global: std::sync::MutexGuard<'_, YrdenGlobal> = global.lock().unwrap();
                global.host_prefix.clone()
            };

            let (namespace_id, playlist, stream, ranges) = vidformer::create_spec_hls(
                spec.as_ref().as_ref(),
                &host_prefix,
                &context,
                &dve_config,
            );

            let namespace = YrdenNamespace {
                context,
                dve_config,
                spec,
                playlist,
                stream,
                ranges,
            };

            {
                let mut g = global.lock().unwrap();
                g.namespaces
                    .insert(namespace_id.clone(), std::sync::Arc::new(namespace));
            }
            let response = YrdenResponse {
                namespace: namespace_id.clone(),
                playlist_url: format!("{}/{}/playlist.m3u8", host_prefix, namespace_id),
                stream_url: format!("{}/{}/stream.m3u8", host_prefix, namespace_id),
                player_url: format!("{}/{}/player.html", host_prefix, namespace_id),
            };
            let response = serde_json::to_string(&response).unwrap();

            Ok(hyper::Response::builder()
                .header("Access-Control-Allow-Origin", "*")
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    response,
                )))
                .unwrap())
        }
        (&hyper::Method::POST, "/export") => {
            let whole_body = req.collect().await;
            let body: Vec<u8> = match whole_body {
                Ok(body) => body,
                Err(_err) => {
                    return Ok(hyper::Response::builder()
                        .status(hyper::StatusCode::BAD_REQUEST)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(
                            "Error reading body".to_string(),
                        )))
                        .unwrap());
                }
            }
            .to_bytes()
            .into();

            let start_time = std::time::Instant::now();
            let request: YrdenRequest = match serde_json::from_slice(&body) {
                Ok(body) => body,
                Err(err) => {
                    return Ok(hyper::Response::builder()
                        .status(hyper::StatusCode::BAD_REQUEST)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(
                            format!("Error parsing body: {}", err),
                        )))
                        .unwrap());
                }
            };
            let end_time = std::time::Instant::now();
            debug!("Parsed request in {:?}", end_time - start_time);

            let db_path = {
                let global: std::sync::MutexGuard<'_, YrdenGlobal> = global.lock().unwrap();
                global.db_path.clone()
            };

            let start_time = std::time::Instant::now();
            let sources: Vec<Result<source::SourceVideoStreamMeta, vidformer::Error>> =
                tokio::task::spawn_blocking(move || {
                    request
                        .sources
                        .par_iter()
                        .map(|source| {
                            load_source_meta(
                                &db_path,
                                &source.name,
                                &source.path,
                                source.stream,
                                source.service.as_ref(),
                            )
                        })
                        .collect()
                })
                .await
                .unwrap();

            if sources.iter().any(|s| s.is_err()) {
                return Ok(hyper::Response::builder()
                    .status(hyper::StatusCode::BAD_REQUEST)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        "Error loading sources".to_string(),
                    )))
                    .unwrap());
            }
            let sources = sources.into_iter().map(|s| s.unwrap()).collect();

            let end_time = std::time::Instant::now();
            debug!("Loaded sources in {:?}", end_time - start_time);

            let start_time = std::time::Instant::now();
            let arrays: BTreeMap<String, Box<dyn vidformer::array::Array>> = request
                .arrays
                .par_iter()
                .map(|path| {
                    let split = path.split(':').collect::<Vec<&str>>();
                    assert_eq!(2, split.len());
                    let (name, path) = (split[0], split[1]);

                    let array_file = std::fs::File::open(path).unwrap();
                    let array: vidformer::array::JsonArary =
                        serde_json::from_reader(array_file).unwrap();
                    let array: Box<dyn vidformer::array::Array> = Box::new(array);
                    (name.to_string(), array)
                })
                .collect();
            let end_time = std::time::Instant::now();
            debug!("Loaded arrays in {:?}", end_time - start_time);

            let spec_json_gzip_base64 = &request.spec;
            let spec_content = base64::prelude::BASE64_STANDARD
                .decode(spec_json_gzip_base64.as_bytes())
                .unwrap();
            let spec_content = flate2::read::GzDecoder::new(&spec_content[..]);
            let spec_content = std::io::BufReader::new(spec_content);
            let spec: spec::JsonSpec =
                serde_json::from_reader(spec_content).expect("Unable to parse JSON");

            let spec: Box<dyn spec::Spec> = Box::new(spec);
            let mut filters = crate::default_filters();

            for (name, filter) in request.filters {
                if let std::collections::btree_map::Entry::Vacant(e) = filters.entry(name) {
                    assert!(filter.filter == "IPC");
                    let filter = crate::filter::builtin::IPC::via_map(&filter.args).unwrap();
                    e.insert(Box::new(filter));
                }
            }

            let enc_config = match request.encoder {
                Some(encoder_name) => {
                    let opts: Vec<(String, String)> = match &request.encoder_opts {
                        Some(req_opts) => req_opts
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                        None => vec![],
                    };

                    Some(vidformer::EncoderConfig {
                        codec_name: encoder_name,
                        opts,
                    })
                }
                None => None,
            };
            let format: Option<String> = request.format;

            let context: vidformer::Context = vidformer::Context::new(sources, arrays, filters);
            let context = std::sync::Arc::new(context);

            let dve_config: vidformer::Config = vidformer::Config {
                decode_pool_size: 50,
                decoder_view: 50,
                decoders: u16::MAX as usize,
                filterers: 8,
                output_width: request.width as usize,
                output_height: request.height as usize,
                output_pix_fmt: request.pix_fmt,

                encoder: enc_config,
                format,
            };
            let dve_config = std::sync::Arc::new(dve_config);

            let output_path = match request.output_path {
                Some(output_path) => output_path,
                None => {
                    return Ok(hyper::Response::builder()
                        .status(hyper::StatusCode::BAD_REQUEST)
                        .header("Access-Control-Allow-Origin", "*")
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(
                            "output_path is required".to_string(),
                        )))
                        .unwrap());
                }
            };

            let output_path2 = output_path.clone();

            let spec = std::sync::Arc::new(spec);

            let spec_result = tokio::task::spawn_blocking(move || {
                vidformer::run(&spec, &output_path2, &context, &dve_config, &None)
            })
            .await
            .unwrap();

            if let Err(err) = spec_result {
                warn!("Error running spec: {}", err);
                return Ok(hyper::Response::builder()
                    .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        format!("Error running spec: {}", err),
                    )))
                    .unwrap());
            }

            #[derive(Debug, serde::Serialize)]
            struct ExportResponse {
                path: String,
            }

            let response = ExportResponse { path: output_path };

            let response = serde_json::to_string(&response).unwrap();

            Ok(hyper::Response::builder()
                .header("Access-Control-Allow-Origin", "*")
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    response,
                )))
                .unwrap())
        }
        (&hyper::Method::GET, _)
            if req.uri().path().ends_with("/playlist.m3u8")
                || req.uri().path().ends_with("/stream.m3u8") =>
        {
            let parts = uri.split('/').collect::<Vec<&str>>();
            assert!(parts[0].is_empty());
            let namespace_id = parts[1];
            assert!(parts[2] == "playlist.m3u8" || parts[2] == "stream.m3u8");

            let namespace = {
                let global: std::sync::MutexGuard<'_, YrdenGlobal> = global.lock().unwrap();
                let namespace = global.namespaces.get(namespace_id);

                match namespace {
                    Some(namespace) => namespace.clone(),
                    None => {
                        return Ok(hyper::Response::builder()
                            .status(hyper::StatusCode::NOT_FOUND)
                            .header("Access-Control-Allow-Origin", "*")
                            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                                "Namespace not found".to_string(),
                            )))
                            .unwrap());
                    }
                }
            };

            let response = match parts[2] {
                "playlist.m3u8" => hyper::Response::builder()
                    .header("Access-Control-Allow-Origin", "*")
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        namespace.playlist.clone(),
                    )))
                    .unwrap(),
                "stream.m3u8" => hyper::Response::builder()
                    .header("Access-Control-Allow-Origin", "*")
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        namespace.stream.clone(),
                    )))
                    .unwrap(),
                _ => unreachable!(),
            };

            Ok(response)
        }
        (&hyper::Method::GET, _) if req.uri().path().ends_with(".ts") => {
            let parts = uri.split('/').collect::<Vec<&str>>();
            assert!(parts[0].is_empty());
            let namespace_id = parts[1];
            let path = parts[2];
            assert!(path.starts_with("segment-"));
            assert!(path.ends_with(".ts"));
            let segment_number = path["segment-".len()..path.len() - ".ts".len()]
                .parse::<u32>()
                .unwrap();

            let namespace = {
                let global: std::sync::MutexGuard<'_, YrdenGlobal> = global.lock().unwrap();
                let namespace = global.namespaces.get(namespace_id);

                match namespace {
                    Some(namespace) => namespace.clone(),
                    None => {
                        return Ok(hyper::Response::builder()
                            .status(hyper::StatusCode::NOT_FOUND)
                            .header("Access-Control-Allow-Origin", "*")
                            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                                "Namespace not found".to_string(),
                            )))
                            .unwrap());
                    }
                }
            };

            let (start, end) = &namespace.ranges[segment_number as usize];
            let context = namespace.context.clone();
            let spec = namespace.spec.clone();
            let config = &namespace.dve_config;
            let config = config.clone(); // todo, don't clone

            let dve_range_config = vidformer::Range {
                start: *start,
                end: *end,
                ts_format: vidformer::RangeTsFormat::StreamLocal,
            };

            let tmp_path = format!("/tmp/{}.ts", uuid::Uuid::new_v4());
            let tmp_path_2 = tmp_path.clone(); // copy that gets sent to the thread

            // todo, don't unwrap
            let spec_result = tokio::task::spawn_blocking(move || {
                vidformer::run(
                    &spec,
                    &tmp_path_2,
                    &context,
                    &config,
                    &Some(dve_range_config),
                )
            })
            .await
            .unwrap();

            if let Err(err) = spec_result {
                warn!("Error running spec: {}", err);
                return Ok(hyper::Response::builder()
                    .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        format!("Error running spec: {}", err),
                    )))
                    .unwrap());
            }

            let mut file = tokio::fs::File::open(&tmp_path).await.unwrap();
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).await.unwrap();
            let response = hyper::Response::builder()
                .header("Access-Control-Allow-Origin", "*")
                .body(http_body_util::Full::new(hyper::body::Bytes::from(buf)))
                .unwrap();

            // delete file
            tokio::fs::remove_file(tmp_path).await.unwrap();

            Ok(response)
        }
        (&hyper::Method::GET, _)
            if {
                Regex::new(r"^/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}/raw/\d+-\d+$").unwrap().is_match(req.uri().path())
            } =>
        {
            let re = Regex::new(r"^/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})/raw/(\d+)-(\d+)$").unwrap();
            let caps = re.captures(req.uri().path()).unwrap();
            let namespace_id = caps.get(1).unwrap().as_str();
            let start_frame_idx = caps.get(2).unwrap().as_str().parse::<usize>().unwrap();
            let end_frame_idx = caps.get(3).unwrap().as_str().parse::<usize>().unwrap();

            let namespace = {
                let global: std::sync::MutexGuard<'_, YrdenGlobal> = global.lock().unwrap();
                let namespace = global.namespaces.get(namespace_id);

                match namespace {
                    Some(namespace) => namespace.clone(),
                    None => {
                        return Ok(hyper::Response::builder()
                            .status(hyper::StatusCode::NOT_FOUND)
                            .header("Access-Control-Allow-Origin", "*")
                            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                                "Namespace not found".to_string(),
                            )))
                            .unwrap());
                    }
                }
            };

            let context = namespace.context.clone();
            let spec = namespace.spec.clone();
            let config: vidformer::Config = vidformer::Config {
                decode_pool_size: 50,
                decoder_view: usize::MAX,
                decoders: u16::MAX as usize,
                filterers: 8,
                output_width: namespace.dve_config.output_width,
                output_height: namespace.dve_config.output_height,
                output_pix_fmt: namespace.dve_config.output_pix_fmt.clone(),

                encoder: Some(vidformer::EncoderConfig {
                    codec_name: "rawvideo".to_string(),
                    opts: vec![],
                }),
                format: Some("rawvideo".to_string()),
            };
            let config = std::sync::Arc::new(config);

            let (start, end) = {
                let domain = spec.domain(&context.spec_ctx());
                (domain[start_frame_idx], domain[end_frame_idx])
            };

            let dve_range_config = vidformer::Range {
                start: start,
                end: end,
                ts_format: vidformer::RangeTsFormat::StreamLocal,
            };

            let tmp_path = format!("/tmp/{}.raw", uuid::Uuid::new_v4());
            let tmp_path_2 = tmp_path.clone(); // copy that gets sent to the thread

            // todo, don't unwrap
            let spec_result = tokio::task::spawn_blocking(move || {
                vidformer::run(
                    &spec,
                    &tmp_path_2,
                    &context,
                    &config,
                    &Some(dve_range_config),
                )
            })
            .await
            .unwrap();

            if let Err(err) = spec_result {
                warn!("Error running spec: {}", err);
                return Ok(hyper::Response::builder()
                    .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Access-Control-Allow-Origin", "*")
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        format!("Error running spec: {}", err),
                    )))
                    .unwrap());
            }

            let mut file = tokio::fs::File::open(&tmp_path).await.unwrap();
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).await.unwrap();
            let response = hyper::Response::builder()
                .header("Access-Control-Allow-Origin", "*")
                .body(http_body_util::Full::new(hyper::body::Bytes::from(buf)))
                .unwrap();

            // delete file
            tokio::fs::remove_file(tmp_path).await.unwrap();

            Ok(response)
        }
        (&hyper::Method::GET, _) if req.uri().path().ends_with("player.html") => {
            let parts = uri.split('/').collect::<Vec<&str>>();
            assert!(parts[0].is_empty());
            let namespace_id = parts[1];
            assert!(parts[2] == "player.html");

            let stream_url = format!(
                "{}/{}/stream.m3u8",
                global.lock().unwrap().host_prefix,
                namespace_id
            );

            let hls_js_url = format!("{}/hls.js", global.lock().unwrap().host_prefix);
            let html = format!(
                r#"<!DOCTYPE html>
<html>
<head>
    <title>HLS Video Player</title>
    <!-- Include hls.js library -->
    <script src="{}"></script>
</head>
<body>
    <!-- Video element -->
    <video id="video" controls width="640" height="360" autoplay></video>
    <script>
        var video = document.getElementById('video');
        var videoSrc = '{}';
        var hls = new Hls();
        hls.loadSource(videoSrc);
        hls.attachMedia(video);
        hls.on(Hls.Events.MANIFEST_PARSED, function() {{
            video.play();
        }});
    </script>
</body>
</html>
"#,
                hls_js_url, stream_url
            );

            Ok(hyper::Response::builder()
                .header("Access-Control-Allow-Origin", "*")
                .body(http_body_util::Full::new(hyper::body::Bytes::from(html)))
                .unwrap())
        }
        (&hyper::Method::GET, "/hls.js") => Ok(hyper::Response::builder()
            .header("Access-Control-Allow-Origin", "*")
            .header("Content-Type", "text/javascript")
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                include_str!("hls.js"),
            )))
            .unwrap()),
        (method, pth) => {
            warn!("Unhandled request: {} {}", method, pth);
            Ok(hyper::Response::builder()
                .status(hyper::StatusCode::NOT_FOUND)
                .header("Access-Control-Allow-Origin", "*")
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Not found".to_string(),
                )))
                .unwrap())
        }
    }
}
