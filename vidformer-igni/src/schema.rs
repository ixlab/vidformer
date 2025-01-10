#[derive(sqlx::FromRow)]
pub struct UserRow {
    pub id: uuid::Uuid,
    pub name: String,
    pub api_key: String,
}

#[derive(sqlx::FromRow, Debug)]
pub struct SpecRow {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub width: i32,
    pub height: i32,
    pub pix_fmt: String,
    pub vod_segment_length_num: i64,
    pub vod_segment_length_denom: i64,
    pub frame_rate_num: i64,
    pub frame_rate_denom: i64,
    pub pos_discontinuity: i32,
    pub pos_terminal: Option<i32>,
    pub closed: bool,
    pub ready_hook: Option<String>,
    pub steer_hook: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct SourceRow {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub name: String,
    pub stream_idx: i32,
    pub storage_service: String,
    pub storage_config: serde_json::Value,
    pub codec: String,
    pub pix_fmt: String,
    pub width: i32,
    pub height: i32,
    pub file_size: i64,
}
