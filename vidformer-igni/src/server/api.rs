use std::collections::BTreeSet;

use super::super::IgniError;
use crate::schema;
use http_body_util::BodyExt;
use log::*;
use num_rational::Rational64;
use uuid::Uuid;

use super::IgniServerGlobal;

pub(crate) async fn auth(
    _req: hyper::Request<impl hyper::body::Body>,
    _global: std::sync::Arc<IgniServerGlobal>,
    _user: &super::UserAuth,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let res = serde_json::json!({
        "status": "ok",
    });
    Ok(hyper::Response::builder()
        .header("Content-Type", "application/json")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            serde_json::to_string(&res).unwrap(),
        )))?)
}

pub(crate) async fn list_sources(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    user: &super::UserAuth,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM source WHERE user_id = $1")
        .bind(user.user_id)
        .fetch_all(&global.pool)
        .await?;

    let res = rows.iter().map(|(id,)| id).collect::<Vec<_>>();

    Ok(hyper::Response::builder()
        .header("Content-Type", "application/json")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            serde_json::to_string(&res).unwrap(),
        )))?)
}

pub(crate) async fn get_source(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    source_id: &str,
    user: &super::UserAuth,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let source_id = Uuid::parse_str(source_id).unwrap();

    let mut transaction = global.pool.begin().await?;
    let row: Option<(String, i32, String, serde_json::Value, String, String, i32, i32)> =
        sqlx::query_as("SELECT name, stream_idx, storage_service, storage_config, codec, pix_fmt, width, height FROM source WHERE id = $1 AND user_id = $2")
            .bind(source_id)
            .bind(user.user_id)
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

pub(crate) async fn search_source(
    req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    user: &super::UserAuth,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    #[derive(serde::Deserialize)]
    struct Request {
        name: String,
        stream_idx: i32,
        storage_service: String,
        storage_config: serde_json::Value,
    }

    let req: Request = match req.collect().await {
        Err(_err) => {
            error!("Error reading request body");
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Error reading request body",
                )))?);
        }
        Ok(req) => match serde_json::from_slice(&req.to_bytes().to_vec()) {
            Err(err) => {
                error!("Error parsing request body");
                return Ok(hyper::Response::builder()
                    .status(hyper::StatusCode::BAD_REQUEST)
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        format!("Bad request: {}", err),
                    )))?);
            }
            Ok(req) => req,
        },
    };

    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM source WHERE name = $1 AND stream_idx = $2 AND storage_service = $3 AND storage_config = $4 AND user_id = $5")
        .bind(req.name)
        .bind(req.stream_idx)
        .bind(req.storage_service)
        .bind(req.storage_config)
        .bind(user.user_id)
        .fetch_all(&global.pool)
        .await?;

    let res: Vec<String> = rows.iter().map(|(id,)| id.to_string()).collect();

    Ok(hyper::Response::builder()
        .header("Content-Type", "application/json")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            serde_json::to_string(&res).unwrap(),
        )))?)
}

pub(crate) async fn delete_source(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    source_id: &str,
    user: &super::UserAuth,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let source_id = Uuid::parse_str(source_id).unwrap();

    let mut transaction = global.pool.begin().await?;
    // Check if the source exists
    let source_exists: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM source WHERE id = $1 AND user_id = $2")
            .bind(source_id)
            .bind(user.user_id)
            .fetch_optional(&mut *transaction)
            .await?;

    if source_exists.is_none() {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::NOT_FOUND)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Not found",
            )))?);
    }

    // Check if the source is referenced by any specs
    let source_referenced: Option<(Uuid,)> =
        sqlx::query_as("SELECT spec_id FROM spec_source_dependency WHERE source_id = $1")
            .bind(source_id)
            .fetch_optional(&mut *transaction)
            .await?;

    if let Some((spec_id,)) = source_referenced {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::BAD_REQUEST)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                format!("Source is referenced by a spec ({})", spec_id),
            )))?);
    }

    // Delete source_t entries
    sqlx::query("DELETE FROM source_t WHERE source_id = $1")
        .bind(source_id)
        .execute(&mut *transaction)
        .await?;

    // Delete the source
    sqlx::query("DELETE FROM source WHERE id = $1")
        .bind(source_id)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;

    Ok(hyper::Response::builder()
        .header("Content-Type", "application/json")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            serde_json::to_string(&serde_json::json!({"status": "ok"})).unwrap(),
        )))?)
}

pub(crate) async fn create_source(
    req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    user: &super::UserAuth,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let req: Vec<u8> = match req.collect().await {
        Err(_err) => {
            error!("Error reading request body");
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Error reading request body",
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
        &user.user_id,
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

pub(crate) async fn list_specs(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    user: &super::UserAuth,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let rows: Vec<(Uuid,)> =
        sqlx::query_as("SELECT id FROM spec WHERE NOT closed AND user_id = $1")
            .bind(user.user_id)
            .fetch_all(&global.pool)
            .await?;

    let res = rows.iter().map(|(id,)| id).collect::<Vec<_>>();

    Ok(hyper::Response::builder()
        .header("Content-Type", "application/json")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            serde_json::to_string(&res).unwrap(),
        )))?)
}

pub(crate) async fn get_spec(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    source_id: &str,
    user: &super::UserAuth,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let source_id = Uuid::parse_str(source_id).unwrap();

    let row: Option<schema::SpecRow> =
        sqlx::query_as("SELECT * FROM spec WHERE id = $1 AND NOT closed AND user_id = $2")
            .bind(source_id)
            .bind(user.user_id)
            .fetch_optional(&global.pool)
            .await?;

    if row.is_none() {
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::NOT_FOUND)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Not found",
            )))?);
    }

    let row = row.unwrap();
    let res = serde_json::json!({
        "id": source_id,
        "width": row.width,
        "height": row.height,
        "pix_fmt": row.pix_fmt,
        "vod_segment_length": [row.vod_segment_length_num, row.vod_segment_length_denom],
        "ready_hook": row.ready_hook,
        "steer_hook": row.steer_hook,
        "terminated": if let Some(pos_terminal) = row.pos_terminal { pos_terminal == row.pos_discontinuity - 1 } else { false },
        "frames_applied": row.pos_discontinuity,
        "closed": row.closed,
        "vod_endpoint": format!("{}{}/", global.config.vod_prefix, source_id), // TODO: This should be configurable
    });

    Ok(hyper::Response::builder()
        .header("Content-Type", "application/json")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            serde_json::to_string(&res).unwrap(),
        )))?)
}

pub(crate) async fn delete_spec(
    _req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    spec_id: &str,
    user: &super::UserAuth,
) -> Result<hyper::Response<http_body_util::Full<hyper::body::Bytes>>, IgniError> {
    let spec_id = Uuid::parse_str(spec_id).unwrap();

    let mut transaction = global.pool.begin().await?;
    // Check if the spec exists
    let spec_exists: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM spec WHERE id = $1 AND NOT closed AND user_id = $2")
            .bind(spec_id)
            .bind(user.user_id)
            .fetch_optional(&mut *transaction)
            .await?;

    if spec_exists.is_none() {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::NOT_FOUND)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Not found",
            )))?);
    }

    // Remove any spec dependencies
    sqlx::query("DELETE FROM spec_source_dependency WHERE spec_id = $1")
        .bind(spec_id)
        .execute(&mut *transaction)
        .await?;

    // Remove any spec_t entries
    sqlx::query("DELETE FROM spec_t WHERE spec_id = $1")
        .bind(spec_id)
        .execute(&mut *transaction)
        .await?;

    // Mark the spec as closed
    sqlx::query("UPDATE spec SET closed = true WHERE id = $1")
        .bind(spec_id)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;

    Ok(hyper::Response::builder()
        .header("Content-Type", "application/json")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            serde_json::to_string(&serde_json::json!({"status": "ok"})).unwrap(),
        )))?)
}

pub(crate) async fn push_spec(
    req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    user: &super::UserAuth,
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
        frame_rate: [i32; 2],
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

    if let Some(err) = user
        .permissions
        .limit_err_max("spec:max_width", req.width as i64)
    {
        return Ok(err);
    }
    if let Some(err) = user
        .permissions
        .limit_err_max("spec:max_height", req.height as i64)
    {
        return Ok(err);
    }
    if let Some(err) = user.permissions.valset_err("spec:pix_fmt", &req.pix_fmt) {
        return Ok(err);
    };
    let vod_segment_length: Rational64 = Rational64::new(
        req.vod_segment_length[0] as i64,
        req.vod_segment_length[1] as i64,
    );
    if let Some(err) = user
        .permissions
        .limit_frac_err_max("spec:max_vod_segment_length", vod_segment_length)
    {
        return Ok(err);
    }
    if let Some(err) = user
        .permissions
        .limit_frac_err_min("spec:min_vod_segment_length", vod_segment_length)
    {
        return Ok(err);
    }
    let frame_rate = Rational64::new(req.frame_rate[0] as i64, req.frame_rate[1] as i64);
    if let Some(err) = user
        .permissions
        .limit_frac_err_max("spec:max_frame_rate", frame_rate)
    {
        return Ok(err);
    }
    if let Some(err) = user
        .permissions
        .limit_frac_err_min("spec:min_frame_rate", frame_rate)
    {
        return Ok(err);
    }

    let spec = crate::ops::add_spec(
        &global.pool,
        &user.user_id,
        (req.vod_segment_length[0], req.vod_segment_length[1]),
        (req.frame_rate[0], req.frame_rate[1]),
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

pub(crate) async fn push_part(
    req: hyper::Request<impl hyper::body::Body>,
    global: std::sync::Arc<IgniServerGlobal>,
    spec_id: &str,
    user: &super::UserAuth,
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
        pos: i32,
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

    let pos = req.pos as i32;
    let n_frames = req.frames.len();

    // stage inserted rows before beginning transaction
    let insert_spec_ids = vec![spec_id; req.frames.len()];
    let mut insert_pos: Vec<i32> = Vec::with_capacity(req.frames.len());
    let mut insert_frames: Vec<Option<String>> = Vec::with_capacity(req.frames.len());

    let mut referenced_source_frames = BTreeSet::new();
    for (frame_idx, ((_numer, _denom), frame)) in req.frames.iter().enumerate() {
        // TODO: Check numer and denom are correct?

        insert_pos.push(pos + frame_idx as i32);
        if let Some(expr) = frame {
            insert_frames.push(Some(serde_json::to_string(expr).unwrap()));
        } else {
            insert_frames.push(None);
        }

        if let Some(frame) = frame {
            frame.add_source_deps(&mut referenced_source_frames);
        }
    }

    let (frame_ref_by_pos, frame_ref_by_ts) = {
        let mut ref_by_pos = vec![];
        let mut ref_by_ts = vec![];
        for frame_ref in referenced_source_frames {
            let source_id: Uuid = match frame_ref.video().parse() {
                Ok(source_id) => source_id,
                Err(_) => {
                    return Ok(hyper::Response::builder()
                        .status(hyper::StatusCode::BAD_REQUEST)
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(
                            "Invalid source id",
                        )))?);
                }
            };

            match frame_ref.index() {
                vidformer::sir::IndexConst::ILoc(pos) => {
                    ref_by_pos.push((source_id, pos));
                }
                vidformer::sir::IndexConst::T(t) => {
                    ref_by_ts.push((source_id, t));
                }
            }
        }

        (ref_by_pos, ref_by_ts)
    };

    let mut transaction = global.pool.begin().await?;

    let spec: Option<schema::SpecRow> =
        sqlx::query_as("SELECT * FROM spec WHERE id = $1 AND NOT closed AND user_id = $2")
            .bind(spec_id)
            .bind(user.user_id)
            .fetch_optional(&mut *transaction)
            .await?;

    if spec.is_none() {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::NOT_FOUND)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Spec not found",
            )))?);
    }
    let spec = spec.unwrap();

    // Check we are not pushing a terminal onto an already terminal spec
    if req.terminal && spec.pos_terminal.is_some() {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::BAD_REQUEST)
            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                "Can not push a terminal part onto a terminal spec",
            )))?);
    }

    // Check if we are pushing past the terminal
    if let Some(pos_terminal) = spec.pos_terminal {
        if pos + req.frames.len() as i32 > pos_terminal + 1 {
            transaction.commit().await?;
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    "Can not push past the terminal frame",
                )))?);
        }
    }

    // Check source references are valid
    {
        // Check all sources both exist and are owned by the user
        let source_ids = frame_ref_by_pos
            .iter()
            .map(|(source_id, _)| *source_id)
            .chain(frame_ref_by_ts.iter().map(|(source_id, _)| *source_id))
            .collect::<Vec<_>>();
        if !source_ids.is_empty() {
            let db_sources: Vec<(Uuid,)> =
                sqlx::query_as("SELECT id FROM source WHERE id = ANY($1::UUID[]) AND user_id = $2")
                    .bind(&source_ids)
                    .bind(user.user_id)
                    .fetch_all(&mut *transaction)
                    .await?;

            for source_id in &source_ids {
                if !db_sources.iter().any(|(id,)| id == source_id) {
                    transaction.commit().await?;
                    return Ok(hyper::Response::builder()
                        .status(hyper::StatusCode::NOT_FOUND)
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(
                            format!("Source {} not found", source_id),
                        )))?);
                }
            }
        }

        // Check by pos
        if !frame_ref_by_pos.is_empty() {
            let source_ids = frame_ref_by_pos
                .iter()
                .map(|(source_id, _)| *source_id)
                .collect::<Vec<_>>();
            let pos = frame_ref_by_pos
                .iter()
                .map(|(_, pos)| **pos as i32)
                .collect::<Vec<_>>();

            let missing_ref: Option<(Uuid, i32)> = sqlx::query_as(
    "WITH needed_refs AS (SELECT UNNEST($1::UUID[]) AS source_id, UNNEST($2::INT[]) AS pos) SELECT source_id, pos FROM needed_refs WHERE NOT EXISTS (SELECT 1 FROM source_t WHERE source_id = needed_refs.source_id AND pos = needed_refs.pos) LIMIT 1")
    .bind(&source_ids)
    .bind(&pos)
    .fetch_optional(&mut *transaction)
    .await?;

            if let Some((source_id, pos)) = missing_ref {
                transaction.commit().await?;
                return Ok(hyper::Response::builder()
                    .status(hyper::StatusCode::BAD_REQUEST)
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        format!("Missing reference to source {} at pos {}", source_id, pos),
                    )))?);
            }
        }

        // Check by ts
        if !frame_ref_by_ts.is_empty() {
            let source_ids = frame_ref_by_ts
                .iter()
                .map(|(source_id, _)| *source_id)
                .collect::<Vec<_>>();
            let ts_num = frame_ref_by_ts
                .iter()
                .map(|(_, ts)| *ts.numer() as i32)
                .collect::<Vec<_>>();
            let ts_denom = frame_ref_by_ts
                .iter()
                .map(|(_, ts)| *ts.denom() as i32)
                .collect::<Vec<_>>();

            let missing_ref: Option<(Uuid, i32, i32)> = sqlx::query_as(
                "WITH needed_refs AS (SELECT UNNEST($1::UUID[]) AS source_id, UNNEST($2::INT[]) AS ts_num, UNNEST($3::INT[]) AS ts_denom) SELECT source_id, ts_num, ts_denom FROM needed_refs WHERE NOT EXISTS (SELECT 1 FROM source_t WHERE source_id = needed_refs.source_id AND t_num = needed_refs.ts_num AND t_denom = needed_refs.ts_denom) LIMIT 1")
                .bind(&source_ids)
                .bind(&ts_num)
                .bind(&ts_denom)
                .fetch_optional(&mut *transaction)
                .await?;

            if let Some((source_id, ts_num, ts_denom)) = missing_ref {
                transaction.commit().await?;
                return Ok(hyper::Response::builder()
                    .status(hyper::StatusCode::BAD_REQUEST)
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(
                        format!(
                            "Missing reference to source {} at ts {}/{}",
                            source_id, ts_num, ts_denom
                        ),
                    )))?);
            }
        }
    }

    if req.terminal {
        // If the part is terminal, make sure there are no existing values in spec_t with pos > req.pos
        let existing: Option<(i32,)> =
            sqlx::query_as("SELECT pos FROM spec_t WHERE spec_id = $1 AND pos > $2 LIMIT 1")
                .bind(spec_id)
                .bind(pos)
                .fetch_optional(&mut *transaction)
                .await?;
        if let Some((pos,)) = existing {
            transaction.commit().await?;
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    format!(
                        "Can not terminate middle of stream (position {} exists)",
                        pos
                    ),
                )))?);
        }
    } else {
        // Make sure there are no existing values in spec_t with pos between pos and pos + n_frames
        let existing: Option<(i32,)> = sqlx::query_as(
            "SELECT pos FROM spec_t WHERE spec_id = $1 AND pos >= $2 AND pos < $3 LIMIT 1",
        )
        .bind(spec_id)
        .bind(pos)
        .bind(pos + n_frames as i32)
        .fetch_optional(&mut *transaction)
        .await?;
        if let Some((pos,)) = existing {
            transaction.commit().await?;
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::BAD_REQUEST)
                .body(http_body_util::Full::new(hyper::body::Bytes::from(
                    format!("Can not push to an existing position (position {})", pos),
                )))?);
        }
    }

    if !insert_spec_ids.is_empty() {
        // Only insert if there are frames to insert
        sqlx::query("INSERT INTO spec_t (spec_id, pos, frame) VALUES (UNNEST($1::UUID[]), UNNEST($2::INT[]), UNNEST($3::TEXT[]))")
            .bind(&insert_spec_ids)
            .bind(&insert_pos)
            .bind(&insert_frames)
            .execute(&mut *transaction)
            .await?;

        // Check if we need to update the spec's pos_discontinuity
        // TODO: We can restrict this to only scan frames after pos_discontinuity
        let x: (Option<i32>,) = sqlx::query_as(
            "WITH cte AS (
    SELECT
        pos,
        (row_number() OVER (ORDER BY pos) - 1)::INT4 AS rn
    FROM spec_t
    WHERE spec_id = $1
)
SELECT CASE WHEN (SELECT MIN(cte.rn) FROM cte
WHERE cte.pos <> cte.rn) IS NULL THEN (SELECT MAX(pos) + 1 FROM cte) ELSE (SELECT MIN(cte.rn) FROM cte
WHERE cte.pos <> cte.rn) END AS first_missing_pos
",
        )
        .bind(spec_id)
        .fetch_one(&mut *transaction)
        .await?;

        if let (Some(first_missing_pos),) = x {
            assert!(first_missing_pos >= spec.pos_discontinuity);
            if first_missing_pos > spec.pos_discontinuity {
                sqlx::query("UPDATE spec SET pos_discontinuity = $1 WHERE id = $2")
                    .bind(first_missing_pos)
                    .bind(spec_id)
                    .execute(&mut *transaction)
                    .await?;
            }
        } else {
            // If there are no missing frames, set pos_discontinuity to the next frame
            sqlx::query("UPDATE spec SET pos_discontinuity = $1 WHERE id = $2")
                .bind(pos + n_frames as i32)
                .bind(spec_id)
                .execute(&mut *transaction)
                .await?;
        }

        // Make sure we track the source dependencies
        let dependent_source_ids = frame_ref_by_pos
            .iter()
            .map(|(source_id, _)| *source_id)
            .chain(frame_ref_by_ts.iter().map(|(source_id, _)| *source_id))
            .collect::<Vec<_>>();
        if !dependent_source_ids.is_empty() {
            sqlx::query("INSERT INTO spec_source_dependency (spec_id, source_id) VALUES (UNNEST($1::UUID[]), UNNEST($2::UUID[])) ON CONFLICT (spec_id, source_id) DO NOTHING")
            .bind(vec![spec_id; dependent_source_ids.len()])
            .bind(&dependent_source_ids)
            .execute(&mut *transaction)
            .await?;
        }
    }

    if req.terminal {
        sqlx::query("UPDATE spec SET pos_terminal = $1 WHERE id = $2")
            .bind(pos + n_frames as i32 - 1)
            .bind(spec_id)
            .execute(&mut *transaction)
            .await?;
    }

    transaction.commit().await?;

    let response = serde_json::json!({
        "status": "ok"
    });
    let response = serde_json::to_string(&response).unwrap();
    Ok(hyper::Response::builder()
        .header("Content-Type", "application/json")
        .body(http_body_util::Full::new(hyper::body::Bytes::from(
            response,
        )))?)
}
