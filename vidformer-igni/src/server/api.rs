use super::super::IgniError;
use http_body_util::BodyExt;
use log::*;
use uuid::Uuid;

use super::IgniServerGlobal;

pub(crate) async fn get_source(
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

pub(crate) async fn push_source(
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

pub(crate) async fn get_spec(
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

pub(crate) async fn push_spec(
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

pub(crate) async fn push_part(
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
                "Spec not found",
            )))?);
    }
    let row = row.unwrap();

    if row.1 {
        transaction.commit().await?;
        return Ok(hyper::Response::builder()
            .status(hyper::StatusCode::BAD_REQUEST)
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
        debug!(
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
        let terminated_state: Option<bool> = sqlx::query_scalar("UPDATE spec SET terminated = TRUE WHERE id = $1 AND (SELECT terminal FROM spec_part_staged WHERE spec_id = $1 AND pos = $2) RETURNING terminated")
            .bind(spec_id)
            .bind(max_ready_pos)
            .fetch_optional(&mut *transaction)
            .await
            ?;
        if let Some(terminated) = terminated_state {
            assert!(terminated);
        }

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

        if terminated_state.is_some() {
            sqlx::query("INSERT INTO vod_segment (spec_id, segment_number, first_t, last_t) WITH expected_t_segments AS (
                    SELECT
                        spec_id,
                        pos,
                        (spec_t.t_numer * $2) / $1 / spec_t.t_denom AS expected_segment_idx
                    FROM
                        spec_t
                    WHERE spec_id = $3
                ),
                expected_segments AS (
                    SELECT
                        spec_id,
                        expected_segment_idx AS segment_idx,
                        MIN(pos) AS first_t,
                        MAX(pos) AS last_t
                    FROM
                        expected_t_segments
                    WHERE spec_id = $3
                    GROUP BY
                        spec_id, expected_segment_idx
                ),
                max_vod_segments AS (
                    SELECT
                        spec_id,
                        COALESCE(MAX(segment_number), -1) AS max_segment
                    FROM
                        vod_segment
                    WHERE spec_id = $3
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
                    es.segment_idx > COALESCE(mvs.max_segment, -1) AND es.spec_id = $3")
                        .bind(vod_segment_length_num)
                        .bind(vod_segment_length_denom)
                        .bind(spec_id)
                                .execute(&mut *transaction)
                                .await?;
        } else {
            sqlx::query("INSERT INTO vod_segment (spec_id, segment_number, first_t, last_t) WITH expected_t_segments AS (
                    SELECT
                        spec_id,
                        pos,
                        (spec_t.t_numer * $2) / $1 / spec_t.t_denom AS expected_segment_idx
                    FROM
                        spec_t
                    WHERE spec_id = $3
                ),
                expected_segments AS (
                    SELECT
                        spec_id,
                        expected_segment_idx AS segment_idx,
                        MIN(pos) AS first_t,
                        MAX(pos) AS last_t
                    FROM
                        expected_t_segments
                    WHERE spec_id = $3
                    GROUP BY
                        spec_id, expected_segment_idx
                ),
                max_expected_segments AS (
                    SELECT
                        spec_id,
                        MAX(segment_idx) AS max_segment
                    FROM
                        expected_segments
                    WHERE spec_id = $3
                    GROUP BY
                        spec_id
                ),
                max_vod_segments AS (
                    SELECT
                        spec_id,
                        COALESCE(MAX(segment_number), -1) AS max_segment
                    FROM
                        vod_segment
                    WHERE spec_id = $3
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
                LEFT JOIN
                    max_expected_segments mes
                ON
                    es.spec_id = mes.spec_id
                WHERE
                    es.segment_idx > COALESCE(mvs.max_segment, -1)
                    AND es.segment_idx < mes.max_segment
                    AND es.spec_id = $3")
                        .bind(vod_segment_length_num)
                        .bind(vod_segment_length_denom)
                        .bind(spec_id)
                                .execute(&mut *transaction)
                                .await?;
        }
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
