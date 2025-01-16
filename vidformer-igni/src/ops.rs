use std::collections::HashMap;

use crate::server;

use super::IgniError;

pub(crate) async fn ping(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), IgniError> {
    let row: (i64,) = sqlx::query_as("SELECT $1")
        .bind(1_i64)
        .fetch_one(pool)
        .await?;
    assert_eq!(row.0, 1);
    Ok(())
}

pub(crate) fn parse_storage_config(
    storage: &str,
) -> Result<(serde_json::Value, HashMap<String, String>), IgniError> {
    let storage_json: serde_json::Value = match serde_json::from_str(storage) {
        Ok(v) => v,
        Err(e) => {
            return Err(IgniError::General(format!(
                "Failed to parse storage JSON: {}",
                e
            )));
        }
    };

    let storage_map = match storage_json.clone() {
        serde_json::Value::Object(m) => m,
        _ => {
            return Err(IgniError::General(
                "Storage JSON must be an object".to_string(),
            ));
        }
    };

    let mut out = std::collections::HashMap::new();
    for (k, v) in storage_map {
        match v {
            serde_json::Value::String(s) => {
                out.insert(k, s);
            }
            _ => {
                return Err(IgniError::General(
                    "Storage JSON values must be strings".to_string(),
                ));
            }
        }
    }

    Ok((storage_json, out))
}

pub(crate) async fn add_user(
    pool: &sqlx::Pool<sqlx::Postgres>,
    name: &str,
    api_key: &str,
    permissions: &server::UserPermissions,
) -> Result<uuid::Uuid, IgniError> {
    let user_id = uuid::Uuid::new_v4();
    let permissions = permissions.json_value();
    sqlx::query("INSERT INTO \"user\" (id, name, api_key, permissions) VALUES ($1, $2, $3, $4)")
        .bind(user_id)
        .bind(name)
        .bind(api_key)
        .bind(permissions)
        .execute(pool)
        .await?;
    Ok(user_id)
}

pub(crate) async fn profile_and_add_source(
    pool: &sqlx::Pool<sqlx::Postgres>,
    user_id: &uuid::Uuid,
    source_name: String,
    stream_idx: usize,
    storage_service: &str,
    storage_config_json: &str,
) -> Result<uuid::Uuid, IgniError> {
    let storage: (serde_json::Value, HashMap<String, String>) =
        parse_storage_config(storage_config_json)?;
    let service = vidformer::service::Service::new(storage_service.to_string(), storage.1);
    let source_name2 = source_name.clone();
    // run profile in a blocking thread
    let profile: vidformer::source::SourceVideoStreamMeta =
        tokio::task::spawn_blocking(move || {
            vidformer::source::SourceVideoStreamMeta::profile(
                &source_name,
                &source_name,
                stream_idx,
                &service,
            )
        })
        .await
        .expect("Failed joining blocking thread")?;

    let mut transaction = pool.begin().await?;
    let source_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO source (id, user_id, name, stream_idx, storage_service, storage_config, codec, pix_fmt, width, height, file_size) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)")
        .bind(source_id)
        .bind(user_id)
        .bind(&source_name2)
        .bind(stream_idx as i32)
        .bind(storage_service)
        .bind(storage.0)
        .bind(profile.codec)
        .bind(profile.pix_fmt)
        .bind(profile.resolution.0 as i32)
        .bind(profile.resolution.1 as i32)
        .bind(profile.file_size as i64)
        .execute(&mut *transaction)
        .await?;
    let source_ids = vec![source_id; profile.ts.len()];
    let pos = (0..profile.ts.len()).map(|i| i as i32).collect::<Vec<_>>();
    let keys = profile
        .ts
        .iter()
        .map(|t| profile.keys.binary_search(t).is_ok())
        .collect::<Vec<_>>();
    let t_num = profile
        .ts
        .iter()
        .map(|t| *t.numer() as i32)
        .collect::<Vec<_>>();
    let t_denom = profile
        .ts
        .iter()
        .map(|t| *t.denom() as i32)
        .collect::<Vec<_>>();
    sqlx::query("INSERT INTO source_t (source_id, pos, key, t_num, t_denom) SELECT * FROM UNNEST($1::UUID[], $2::INT[], $3::BOOLEAN[], $4::INT[], $5::INT[])")
        .bind(&source_ids)
        .bind(&pos)
        .bind(&keys)
        .bind(&t_num)
        .bind(&t_denom)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(source_id)
}

pub(crate) async fn add_spec(
    pool: &sqlx::Pool<sqlx::Postgres>,
    user_id: &uuid::Uuid,
    segment_length: (i32, i32),
    frame_rate: (i32, i32),
    height: i32,
    width: i32,
    pix_fmt: String,
    ready_hook: Option<String>,
    steer_hook: Option<String>,
) -> Result<uuid::Uuid, IgniError> {
    let spec_id = uuid::Uuid::new_v4();

    sqlx::query("INSERT INTO spec (id, user_id, width, height, pix_fmt, vod_segment_length_num, vod_segment_length_denom, frame_rate_num, frame_rate_denom, pos_discontinuity, closed, ready_hook, steer_hook) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)")
        .bind(spec_id)
        .bind(user_id)
        .bind(width)
        .bind(height)
        .bind(pix_fmt)
        .bind(segment_length.0)
        .bind(segment_length.1)
        .bind(frame_rate.0)
        .bind(frame_rate.1)
        .bind(0)
        .bind(false)
        .bind(ready_hook)
        .bind(steer_hook)
        .execute(pool)
        .await?;

    Ok(spec_id)
}
