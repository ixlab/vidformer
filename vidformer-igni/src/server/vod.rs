use super::IgniServerGlobal;
use crate::IgniError;
use log::*;
use num::Rational64;
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

pub(crate) async fn get_stream(
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

pub(crate) async fn get_segment(
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
