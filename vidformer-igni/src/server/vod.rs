use super::IgniServerGlobal;
use crate::schema;
use crate::IgniError;
use num::Rational64;
use num::ToPrimitive;
use std::collections::BTreeMap;
use uuid::Uuid;

fn filters() -> BTreeMap<String, Box<dyn vidformer::filter::Filter>> {
    let mut filters: BTreeMap<String, Box<dyn vidformer::filter::Filter>> = BTreeMap::new();
    filters.extend(vidformer::filter::builtin::filters());
    filters.extend(vidformer::filter::cv2::filters());
    filters
}

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
    let terminal = if let Some(pos_terminal) = spec.pos_terminal {
        pos_terminal == spec.pos_discontinuity - 1
    } else {
        false
    };

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
    let terminated = if let Some(pos_terminal) = spec.pos_terminal {
        pos_terminal == spec.pos_discontinuity - 1
    } else {
        false
    };
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
    let terminal = if let Some(pos_terminal) = spec_db.pos_terminal {
        pos_terminal == spec_db.pos_discontinuity - 1
    } else {
        false
    };

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

    assert!(rows.len() == segment.n_frames as usize);

    // map times to rational
    let times: Vec<num_rational::Ratio<i64>> = rows
        .iter()
        .map(|(pos, _)| num_rational::Ratio::from(*pos as i64) * frame_rate.recip())
        .collect();
    let start = *times.first().unwrap();
    let end = *times.last().unwrap();

    let frames = {
        let mut out = vec![];
        for (_, frame) in rows {
            let frame_reader = std::io::Cursor::new(frame);
            let frame_uncompressed = zstd::stream::decode_all(frame_reader).unwrap();
            let feb: crate::feb::FrameBlock =
                serde_json::from_slice(&frame_uncompressed).unwrap();
            let mut f_collection = feb.frames().map_err(|err| {
                IgniError::General(format!("Error decoding frame block: {:?}", err))
            })?;
            assert_eq!(f_collection.len(), 1);
            let f = f_collection.remove(0);
            out.push(f);
        }
        out
    };

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
            });
        }

        out
    };
    transaction.commit().await?;

    let arrays = std::collections::BTreeMap::new();
    let filters = filters();
    let context = vidformer::Context::new(sources, arrays, filters);
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
