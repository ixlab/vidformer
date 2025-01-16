use clap::{Parser, Subcommand};
use rand::Rng;
use sqlx::postgres::PgPoolOptions;
use tabled::{Table, Tabled};

mod ops;
mod schema;
mod segment;
mod server;

#[derive(thiserror::Error, Debug)]
enum IgniError {
    #[error("{0}")]
    General(String),
    #[error("Sqlx error: {0}")]
    Sqlx(sqlx::Error),
    #[error("Vidformer error: {0}")]
    Vidformer(vidformer::Error),
}

impl From<sqlx::Error> for IgniError {
    fn from(e: sqlx::Error) -> Self {
        IgniError::Sqlx(e)
    }
}

impl From<vidformer::Error> for IgniError {
    fn from(e: vidformer::Error) -> Self {
        IgniError::Vidformer(e)
    }
}

impl From<hyper::http::Error> for IgniError {
    fn from(e: hyper::http::Error) -> Self {
        IgniError::General(format!("Hyper error: {}", e))
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: ArgCmd,
}

#[derive(Subcommand, Debug)]
enum ArgCmd {
    Ping,
    Server(ServerOpt),
    #[command(subcommand)]
    Source(SourceCmd),
    #[command(subcommand)]
    Spec(SpecCmd),
    #[command(subcommand)]
    User(UserCmd),
}

#[derive(Parser, Debug)]
struct ServerOpt {
    #[clap(long, default_value = "8080")]
    port: u16,
    #[clap(long)]
    config: String,
}

#[derive(Parser, Debug)]
enum SpecCmd {
    Ls,
    Add(SpecAddOpt),
}

#[derive(Parser, Debug)]
struct SpecAddOpt {
    #[clap(long)]
    user_id: String,
    #[clap(long)]
    width: usize,
    #[clap(long)]
    height: usize,
    #[clap(long)]
    pix_fmt: String,
    #[clap(long)]
    segment_length: String,
    #[clap(long)]
    frame_rate: String,
    #[clap(long)]
    ready_hook: Option<String>,
    #[clap(long)]
    steer_hook: Option<String>,
}

#[derive(Parser, Debug)]
enum SourceCmd {
    Ls,
    Add(SourceAddOpt),
    Rm(SourceRmOpt),
}

#[derive(Parser, Debug)]
struct SourceAddOpt {
    #[clap(long)]
    user_id: String,
    #[clap(long)]
    name: String,
    #[clap(long)]
    stream_idx: usize,
    #[clap(long)]
    storage_service: String,
    #[clap(long)]
    storage_config: String,
}

#[derive(Parser, Debug)]
struct SourceRmOpt {
    id: Vec<String>,
}

#[derive(Parser, Debug)]
enum UserCmd {
    Ls,
    Add(UserAddOpt),
    Rm(UserRmOpt),
}

#[derive(clap::ValueEnum, Debug, Clone)]
enum UserPermissionLevel {
    Regular,
    Test,
    Guest,
}

#[derive(Parser, Debug)]
struct UserAddOpt {
    #[clap(long)]
    name: String,
    #[clap(long)]
    api_key: Option<String>,
    #[clap(long)]
    permissions: UserPermissionLevel,
}

#[derive(Parser, Debug)]
struct UserRmOpt {
    id: Vec<String>,
}

fn main() {
    pretty_env_logger::init();
    vidformer::init();

    let args = Args::parse();

    // Run the async main function
    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(async_main(args)) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn db_connect() -> Result<sqlx::Pool<sqlx::Postgres>, IgniError> {
    let timeout = std::time::Duration::from_secs(10);
    let start_time = std::time::Instant::now();
    loop {
        // Pull connection string from environment
        let conn_str = match std::env::var("IGNI_DB") {
            Ok(s) => s,
            Err(_) => {
                return Err(IgniError::General(
                    "Environment variable IGNI_DB not set".to_string(),
                ));
            }
        };

        match PgPoolOptions::new()
            .max_connections(10)
            .connect(&conn_str)
            .await
        {
            Ok(pool) => {
                ops::ping(&pool).await?;
                return Ok(pool);
            }
            Err(e) => {
                if start_time.elapsed() > timeout {
                    return Err(IgniError::General(format!(
                        "Failed to connect to database: {}",
                        e
                    )));
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}

async fn async_main(args: Args) -> Result<(), IgniError> {
    let pool: sqlx::Pool<sqlx::Postgres> = db_connect().await?;

    match args.cmd {
        ArgCmd::Ping => {
            cmd_ping(pool).await?;
        }
        ArgCmd::Server(server_cmd) => {
            server::cmd_server(pool, server_cmd).await?;
        }
        ArgCmd::Source(source_cmd) => {
            cmd_source(pool, source_cmd).await?;
        }
        ArgCmd::Spec(spec_cmd) => {
            cmd_spec(pool, spec_cmd).await?;
        }
        ArgCmd::User(user_cmd) => {
            cmd_user(pool, user_cmd).await?;
        }
    }

    Ok(())
}

async fn cmd_ping(pool: sqlx::Pool<sqlx::Postgres>) -> Result<(), IgniError> {
    ops::ping(&pool).await?;
    println!("pong!");
    Ok(())
}

async fn cmd_source(
    pool: sqlx::Pool<sqlx::Postgres>,
    source_cmd: SourceCmd,
) -> Result<(), IgniError> {
    match source_cmd {
        SourceCmd::Ls => {
            cmd_source_ls(pool).await?;
        }
        SourceCmd::Add(add_source) => {
            cmd_source_add(pool, add_source).await?;
        }
        SourceCmd::Rm(del_source) => {
            cmd_source_del(pool, del_source).await?;
        }
    }
    Ok(())
}

async fn cmd_source_ls(pool: sqlx::Pool<sqlx::Postgres>) -> Result<(), IgniError> {
    let rows: Vec<schema::SourceRow> = sqlx::query_as("SELECT * FROM source ORDER BY id")
        .fetch_all(&pool)
        .await?;

    #[derive(Tabled)]
    struct Row {
        id: String,
        user_id: String,
        name: String,
        stream_idx: usize,
        storage_service: String,
        storage_config: String,
        codec: String,
        pix_fmt: String,
        width: String,
        height: String,
        file_size: i64,
    }

    let rows = rows
        .iter()
        .map(|row| Row {
            id: row.id.to_string(),
            user_id: row.user_id.to_string(),
            name: row.name.clone(),
            stream_idx: row.stream_idx as usize,
            storage_service: row.storage_service.clone(),
            storage_config: row.storage_config.to_string(),
            codec: row.codec.clone(),
            pix_fmt: row.pix_fmt.clone(),
            width: row.width.to_string(),
            height: row.height.to_string(),
            file_size: row.file_size,
        })
        .collect::<Vec<_>>();

    let table = Table::new(&rows);
    println!("{}", table);
    Ok(())
}

async fn cmd_source_add(
    pool: sqlx::Pool<sqlx::Postgres>,
    add_source: SourceAddOpt,
) -> Result<(), IgniError> {
    let user_id = match uuid::Uuid::parse_str(&add_source.user_id) {
        Ok(u) => u,
        Err(e) => {
            return Err(IgniError::General(format!("Invalid user id: {}", e)));
        }
    };
    let profile = ops::profile_source(
        &add_source.name,
        add_source.stream_idx,
        &add_source.storage_service,
        &add_source.storage_config,
    )
    .await?;

    let storage_config_json_value: serde_json::Value =
        serde_json::from_str(&add_source.storage_config).unwrap();

    let source_id = {
        let mut transaction = pool.begin().await?;
        let source_id = uuid::Uuid::new_v4();
        sqlx::query("INSERT INTO source (id, user_id, name, stream_idx, storage_service, storage_config, codec, pix_fmt, width, height, file_size) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)")
        .bind(source_id)
        .bind(user_id)
        .bind(&add_source.name)
        .bind(add_source.stream_idx as i32)
        .bind(&add_source.storage_service)
        .bind(&storage_config_json_value)
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

        source_id
    };

    println!("{}", source_id);
    Ok(())
}

async fn cmd_source_del(
    pool: sqlx::Pool<sqlx::Postgres>,
    del_source: SourceRmOpt,
) -> Result<(), IgniError> {
    for source_id in &del_source.id {
        let source_id = match uuid::Uuid::parse_str(source_id) {
            Ok(u) => u,
            Err(e) => {
                return Err(IgniError::General(format!("Invalid source id: {}", e)));
            }
        };

        let conflicting_spec: Option<uuid::Uuid> =
            sqlx::query_scalar("SELECT spec_id FROM spec_source_dependency WHERE source_id = $1")
                .bind(source_id)
                .fetch_optional(&pool)
                .await?;

        if let Some(spec_id) = conflicting_spec {
            return Err(IgniError::General(format!(
                "Spec {} depends on source {}",
                spec_id, source_id
            )));
        }

        let resp = sqlx::query("DELETE FROM source WHERE id = $1")
            .bind(source_id)
            .execute(&pool)
            .await?;

        if resp.rows_affected() == 0 {
            return Err(IgniError::General(format!(
                "Source not found: {}",
                source_id
            )));
        }

        println!("{}", source_id);
    }

    Ok(())
}

async fn cmd_spec(pool: sqlx::Pool<sqlx::Postgres>, source_cmd: SpecCmd) -> Result<(), IgniError> {
    match source_cmd {
        SpecCmd::Ls => {
            cmd_spec_ls(pool).await?;
        }
        SpecCmd::Add(add_spec) => {
            cmd_spec_add(pool, add_spec).await?;
        }
    }
    Ok(())
}

async fn cmd_spec_ls(pool: sqlx::Pool<sqlx::Postgres>) -> Result<(), IgniError> {
    let rows: Vec<schema::SpecRow> = sqlx::query_as("SELECT * FROM spec ORDER BY id")
        .fetch_all(&pool)
        .await?;

    #[derive(Tabled)]
    struct Row {
        id: String,
        user_id: String,
        width: i32,
        height: i32,
        pix_fmt: String,
        segment_length: String,
        frame_rate: String,
        pos_discontinuity: i32,
        pos_terminal: String,
        closed: bool,
        ready_hook: String,
        steer_hook: String,
    }

    let rows = rows
        .iter()
        .map(|row| Row {
            id: row.id.to_string(),
            user_id: row.user_id.to_string(),
            width: row.width,
            height: row.height,
            pix_fmt: row.pix_fmt.clone(),
            segment_length: format!(
                "{}/{}",
                row.vod_segment_length_num, row.vod_segment_length_denom
            ),
            frame_rate: format!("{}/{}", row.frame_rate_num, row.frame_rate_denom),
            pos_discontinuity: row.pos_discontinuity,
            pos_terminal: row.pos_terminal.map(|t| t.to_string()).unwrap_or_default(),
            closed: row.closed,
            ready_hook: row.ready_hook.clone().unwrap_or_default(),
            steer_hook: row.steer_hook.clone().unwrap_or_default(),
        })
        .collect::<Vec<_>>();

    let table = Table::new(&rows);
    println!("{}", table);
    Ok(())
}

fn parse_frac(s: &str) -> Result<(i32, i32), IgniError> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 2 {
        return Err(IgniError::General(format!("Invalid fraction: {}", s)));
    }

    let num = if let Ok(n) = parts[0].parse::<i32>() {
        n
    } else {
        return Err(IgniError::General(format!("Invalid fraction: {}", s)));
    };
    let denom = if let Ok(d) = parts[1].parse::<i32>() {
        d
    } else {
        return Err(IgniError::General(format!("Invalid fraction: {}", s)));
    };

    Ok((num, denom))
}

async fn cmd_spec_add(pool: sqlx::Pool<sqlx::Postgres>, opt: SpecAddOpt) -> Result<(), IgniError> {
    let (seg_num, seg_denom) = parse_frac(&opt.segment_length)?;
    let (frame_num, frame_denom) = parse_frac(&opt.frame_rate)?;

    let user_id = match uuid::Uuid::parse_str(&opt.user_id) {
        Ok(u) => u,
        Err(e) => {
            return Err(IgniError::General(format!("Invalid user id: {}", e)));
        }
    };
    let height = opt.height as i32;
    let width = opt.width as i32;
    let pix_fmt = opt.pix_fmt.clone();
    let ready_hook = opt.ready_hook.clone();
    let steer_hook = opt.steer_hook.clone();

    let spec_id = ops::add_spec(
        &pool,
        &user_id,
        (seg_num, seg_denom),
        (frame_num, frame_denom),
        height,
        width,
        pix_fmt,
        ready_hook,
        steer_hook,
    )
    .await?;

    println!("{}", spec_id);
    Ok(())
}

async fn cmd_user(pool: sqlx::Pool<sqlx::Postgres>, user_cmd: UserCmd) -> Result<(), IgniError> {
    match user_cmd {
        UserCmd::Ls => {
            cmd_user_ls(pool).await?;
        }
        UserCmd::Add(add_user) => {
            cmd_user_add(pool, add_user).await?;
        }
        UserCmd::Rm(del_user) => {
            cmd_user_del(pool, del_user).await?;
        }
    }
    Ok(())
}

async fn cmd_user_ls(pool: sqlx::Pool<sqlx::Postgres>) -> Result<(), IgniError> {
    let rows: Vec<schema::UserRow> = sqlx::query_as("SELECT * FROM \"user\" ORDER BY id")
        .fetch_all(&pool)
        .await?;

    #[derive(Tabled)]
    struct Row {
        id: String,
        name: String,
        api_key: String,
    }

    let rows = rows
        .iter()
        .map(|row| Row {
            id: row.id.to_string(),
            name: row.name.clone(),
            api_key: row.api_key.clone(),
        })
        .collect::<Vec<_>>();

    let table = Table::new(&rows);
    println!("{}", table);
    Ok(())
}

async fn cmd_user_add(
    pool: sqlx::Pool<sqlx::Postgres>,
    add_user: UserAddOpt,
) -> Result<(), IgniError> {
    let name = add_user.name.clone();
    let api_key = match add_user.api_key.clone() {
        Some(key) => key,
        None => rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(30)
            .map(char::from)
            .collect(),
    };
    let permissions = match add_user.permissions {
        UserPermissionLevel::Regular => server::UserPermissions::default_regular(),
        UserPermissionLevel::Test => server::UserPermissions::default_test(),
        UserPermissionLevel::Guest => server::UserPermissions::default_guest(),
    };
    let user_id = ops::add_user(&pool, &name, &api_key, &permissions).await?;
    println!("{}", user_id);
    if add_user.api_key.is_none() {
        println!("{}", api_key);
    }
    Ok(())
}

async fn cmd_user_del(
    pool: sqlx::Pool<sqlx::Postgres>,
    del_user: UserRmOpt,
) -> Result<(), IgniError> {
    for user_id in &del_user.id {
        let user_id = match uuid::Uuid::parse_str(user_id) {
            Ok(u) => u,
            Err(e) => {
                return Err(IgniError::General(format!("Invalid user id: {}", e)));
            }
        };

        let resp = sqlx::query("DELETE FROM \"user\" WHERE id = $1")
            .bind(user_id)
            .execute(&pool)
            .await?;

        if resp.rows_affected() == 0 {
            return Err(IgniError::General(format!("User not found: {}", user_id)));
        }

        println!("{}", user_id);
    }

    Ok(())
}
