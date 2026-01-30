use std::collections::HashMap;

use crate::{schema, server};

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

pub(crate) async fn profile_source(
    source_name: &str,
    stream_idx: usize,
    storage_service: &str,
    storage_config_json: &str,
    io_cache: Option<(Box<dyn vidformer::io::IoWrapper>, String)>,
) -> Result<vidformer::source::SourceVideoStreamMeta, IgniError> {
    let storage: (serde_json::Value, HashMap<String, String>) =
        parse_storage_config(storage_config_json)?;
    let service = vidformer::service::Service::new(storage_service.to_string(), storage.1);
    let source_name = source_name.to_string();

    // run profile in a blocking thread
    let profile: vidformer::source::SourceVideoStreamMeta =
        tokio::task::spawn_blocking(move || {
            vidformer::source::SourceVideoStreamMeta::profile(
                &source_name,
                &source_name,
                stream_idx,
                &service,
                match io_cache {
                    Some((ref wrapper, ref ns)) => Some((wrapper.as_ref(), ns.as_str())),
                    None => None,
                },
            )
        })
        .await
        .map_err(|e| IgniError::General(format!("Failed to join blocking thread: {}", e)))??;

    Ok(profile)
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
    ttl: Option<i64>,
) -> Result<uuid::Uuid, IgniError> {
    let spec_id = uuid::Uuid::new_v4();

    sqlx::query("INSERT INTO spec (id, user_id, width, height, pix_fmt, vod_segment_length_num, vod_segment_length_denom, frame_rate_num, frame_rate_denom, pos_discontinuity, closed, ready_hook, steer_hook, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)")
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
        .bind(ttl.map(|ttl| chrono::Utc::now() + chrono::Duration::seconds(ttl)))
        .execute(pool)
        .await?;

    Ok(spec_id)
}

pub(crate) async fn update_users(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    users: &[schema::UserRow],
) -> Result<(), IgniError> {
    // Make the users table match the provided list
    let mut user_ids = Vec::new();
    for user in users {
        assert!(!user_ids.contains(&user.id));
        user_ids.push(user.id);
        sqlx::query("INSERT INTO \"user\" (id, name, api_key, permissions) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET name = $2, api_key = $3, permissions = $4")
            .bind(&user.id)
            .bind(&user.name)
            .bind(&user.api_key)
            .bind(&user.permissions)
            .execute(&mut **transaction)
            .await?;
    }

    // Remove any users not in the list
    let mut rows = sqlx::query_as::<_, schema::UserRow>("SELECT * FROM \"user\"")
        .fetch_all(&mut **transaction)
        .await?;

    for row in rows.drain(..) {
        if !user_ids.contains(&row.id) {
            sqlx::query("DELETE FROM \"user\" WHERE id = $1")
                .bind(&row.id)
                .execute(&mut **transaction)
                .await?;
        }
    }

    Ok(())
}
