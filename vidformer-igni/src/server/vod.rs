use super::IgniServerGlobal;
use crate::schema;
use crate::IgniError;
use num::Rational64;
use num::ToPrimitive;
use rayon::prelude::*;
use std::collections::BTreeSet;
use uuid::Uuid;

pub(crate) async fn get_playlist(
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
        "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=640000\n{}{}/stream.m3u8\n",
        global.config.vod_prefix, spec_id
    );

    transaction.commit().await?;

    Ok(hyper::Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "application/vnd.apple.mpegURL")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            playlist_text,
        )))?)
}

pub(crate) async fn get_stream(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    spec_id: &str,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let spec_id = Uuid::parse_str(spec_id).unwrap();

    let row: Option<schema::SpecRow> = sqlx::query_as("SELECT * FROM spec WHERE id = $1")
        .bind(spec_id)
        .fetch_optional(&global.pool)
        .await?;

    let spec = match row {
        None => {
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::NOT_FOUND)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Not found",
                )))?);
        }
        Some(spec) => {
            if spec.closed {
                return Ok(hyper::Response::builder()
                    .status(hyper::StatusCode::FORBIDDEN)
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        "VOD is closed",
                    )))?);
            }

            spec
        }
    };

    let segment_length =
        num_rational::Ratio::new(spec.vod_segment_length_num, spec.vod_segment_length_denom);
    let frame_rate = num_rational::Ratio::new(spec.frame_rate_num, spec.frame_rate_denom);
    let n_frames: i32 = spec.pos_discontinuity;
    let terminal = spec.is_terminated();

    let segments = crate::segment::segments(n_frames, &segment_length, &frame_rate, terminal);

    let mut stream_text =
        "#EXTM3U\n#EXT-X-PLAYLIST-TYPE:EVENT\n#EXT-X-TARGETDURATION:2\n#EXT-X-VERSION:4\n#EXT-X-MEDIA-SEQUENCE:0\n#EXT-X-START:TIME-OFFSET=0\n".to_string();
    for (segment_number, segment) in segments.iter().enumerate() {
        let duration: Rational64 = segment.duration(&frame_rate);
        stream_text.push_str(&format!(
            "#EXTINF:{},\n{}{}/segment-{}.ts\n", // TODO: Make configurable
            duration.to_f32().unwrap(),
            global.config.vod_prefix,
            spec_id,
            segment_number
        ));
    }
    if terminal {
        stream_text.push_str("#EXT-X-ENDLIST\n");
    }

    Ok(hyper::Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "application/vnd.apple.mpegURL")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            stream_text,
        )))?)
}

pub(crate) async fn get_status(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    spec_id: &str,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let spec_id = Uuid::parse_str(spec_id).unwrap();

    #[derive(serde::Serialize)]
    struct Response {
        closed: bool,
        terminated: bool,
        ready: bool,
    }

    let row: Option<schema::SpecRow> = sqlx::query_as("SELECT * FROM spec WHERE id = $1")
        .bind(spec_id)
        .fetch_optional(&global.pool)
        .await?;

    let spec = match row {
        None => {
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::NOT_FOUND)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Spec not found",
                )))?);
        }
        Some(spec) => spec,
    };

    let frame_rate = num_rational::Ratio::new(spec.frame_rate_num, spec.frame_rate_denom);
    let segment_length =
        num_rational::Ratio::new(spec.vod_segment_length_num, spec.vod_segment_length_denom);
    let n_frames: i32 = spec.pos_discontinuity;
    let closed = spec.closed;
    let terminated = spec.is_terminated();
    let ready =
        crate::segment::num_segments(n_frames, &segment_length, &frame_rate, terminated) > 0;

    let response = Response {
        closed,
        terminated,
        ready,
    };

    let body = serde_json::to_string(&response).unwrap();
    Ok(hyper::Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "application/json")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(body)))?)
}

pub(crate) async fn get_embedded_player(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    spec_id: &str,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let spec_id = Uuid::parse_str(spec_id).unwrap();

    let row: Option<schema::SpecRow> = sqlx::query_as("SELECT * FROM spec WHERE id = $1")
        .bind(spec_id)
        .fetch_optional(&global.pool)
        .await?;

    let spec = match row {
        None => {
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::NOT_FOUND)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Spec not found",
                )))?);
        }
        Some(spec) => spec,
    };

    if spec.closed {
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::FORBIDDEN)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "VOD is closed",
            )))?);
    }

    let vod_prefix = &global.config.vod_prefix;

    // TODO: Hacky
    let hls_js_path = if vod_prefix.ends_with("/vod/") {
        let mut s = vod_prefix.clone();
        s.truncate(s.len() - 4);
        s + "hls.js"
    } else {
        vod_prefix.clone() + "hls.js"
    };

    let template = include_str!("embedded-player.html");
    let html = template
        .replace("{{UUID}}", &spec_id.to_string())
        .replace("{{VOD_PREFIX}}", vod_prefix)
        .replace("{{HLS_JS_PATH}}", &hls_js_path)
        .replace("{{MAX_WIDTH}}", &spec.width.to_string());

    Ok(hyper::Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "text/html")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(html)))?)
}

pub(crate) async fn get_segment(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    spec_id: &str,
    segment_number: i32,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let spec_id = Uuid::parse_str(spec_id).unwrap();

    let mut transaction = global.pool.begin().await?;

    let row: Option<schema::SpecRow> = sqlx::query_as("SELECT * FROM spec WHERE id = $1")
        .bind(spec_id)
        .fetch_optional(&mut *transaction)
        .await?;

    let spec_db = match row {
        None => {
            transaction.commit().await?;
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::NOT_FOUND)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Not found",
                )))?);
        }
        Some(spec) => spec,
    };

    if spec_db.closed {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::FORBIDDEN)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "VOD is closed",
            )))?);
    }

    let segment_length = num_rational::Ratio::new(
        spec_db.vod_segment_length_num,
        spec_db.vod_segment_length_denom,
    );
    let frame_rate = num_rational::Ratio::new(spec_db.frame_rate_num, spec_db.frame_rate_denom);
    let n_frames: i32 = spec_db.pos_discontinuity;
    let terminal = spec_db.is_terminated();

    let num_segments =
        crate::segment::num_segments(n_frames, &segment_length, &frame_rate, terminal);

    if segment_number >= num_segments {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::NOT_FOUND)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Segment not found",
            )))?);
    }

    let segment = crate::segment::segment(segment_number, n_frames, &segment_length, &frame_rate);

    let first_t = segment.start_frame;
    let last_t = first_t + segment.n_frames - 1;

    // Get the frames from spec_t that are in the segment (pos between first_t and last_t)
    let rows: Vec<(i32, Vec<u8>)> = sqlx::query_as(
        "SELECT pos, frame FROM spec_t WHERE spec_id = $1 AND pos BETWEEN $2 AND $3 ORDER BY pos",
    )
    .bind(spec_id)
    .bind(first_t)
    .bind(last_t)
    .fetch_all(&mut *transaction)
    .await?;

    if rows.len() != segment.n_frames as usize {
        transaction.commit().await?;
        return Err(IgniError::General(format!(
            "segment {} of spec {} has {} stored frames, expected {}",
            segment_number,
            spec_id,
            rows.len(),
            segment.n_frames
        )));
    }

    // map times to rational
    let times: Vec<num_rational::Ratio<i64>> = rows
        .iter()
        .map(|(pos, _)| num_rational::Ratio::from(*pos as i64) * frame_rate.recip())
        .collect();
    let start = *times.first().unwrap();
    let end = *times.last().unwrap();

    let mut needed_source_ids: BTreeSet<Uuid> = BTreeSet::new();

    let frames: Vec<vidformer::sir::FrameExpr> = rows
        .par_iter()
        .map(|(_, frame)| crate::feb::decode_frame_block(frame).map_err(IgniError::General))
        .collect::<Result<Vec<_>, IgniError>>()?;

    for f in &frames {
        let mut referenced_source_frames: BTreeSet<&vidformer::sir::FrameSource> = BTreeSet::new();
        f.add_source_deps(&mut referenced_source_frames);
        for src in &referenced_source_frames {
            needed_source_ids.insert(Uuid::parse_str(src.video()).unwrap());
        }
    }

    let needed_source_ids: Vec<Uuid> = needed_source_ids.into_iter().collect();

    struct IgniSpec {
        times: Vec<num_rational::Ratio<i64>>,
        frames: Vec<vidformer::sir::FrameExpr>,
    }

    impl vidformer::spec::Spec for IgniSpec {
        fn timestamps(
            &self,
            _: &dyn vidformer::spec::SpecContext,
        ) -> Vec<num_rational::Ratio<i64>> {
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
        let rows: Vec<(uuid::Uuid, String, i32, String, serde_json::Value, String, String, i32, i32, i64)> = sqlx::query_as("SELECT id, name, stream_idx, storage_service, storage_config, codec, pix_fmt, width, height, file_size FROM source WHERE id = ANY($1::uuid[])")
            .bind(&needed_source_ids)
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
                    .filter(|(_, _, key)| *key)
                    .map(|(t_num, t_denom, _)| Rational64::new(*t_num, *t_denom))
                    .collect();

                // debug_assert!(ts.is_sorted());
                // debug_assert!(keys.is_sorted());

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
                fuid: Some(source_id.to_string()),
            });
        }

        out
    };
    transaction.commit().await?;

    let io_wrapper = global.io_wrapper();

    let filters = vidformer::filter::default_filters();
    let context = vidformer::Context::new(sources, filters, io_wrapper);
    let context = std::sync::Arc::new(context);

    let dve_config: vidformer::Config = vidformer::Config {
        decode_pool_size: 50,
        decoder_view: 50,
        decoders: u16::MAX as usize,
        filterers: 8,
        output_width: spec_db.width as usize,
        output_height: spec_db.height as usize,
        output_pix_fmt: spec_db.pix_fmt,
        encoder: None,
        format: None,
    };

    let output_path = format!("/tmp/{}.ts", Uuid::new_v4());
    let _tmp_guard = super::TempFileGuard(output_path.clone());
    let output_path2 = output_path.clone();

    // Run the spec in a blocking task
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
    let _stats = stats.unwrap();

    let output = match tokio::fs::read(output_path2.as_str()).await {
        Ok(ok) => ok,
        Err(err) => {
            return Err(IgniError::General(format!(
                "Failed to read temporary file: {}",
                err
            )))
        }
    };

    Ok(hyper::Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "video/MP2T")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(output)))?)
}
