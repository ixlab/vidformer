use clap::{Parser, Subcommand};
use sqlx::postgres::PgPoolOptions;
use tabled::{Table, Tabled};

mod ops;
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
}

#[derive(Parser, Debug)]
struct ServerOpt {
    #[clap(long, default_value = "8080")]
    port: u16,
}

#[derive(Parser, Debug)]
enum SpecCmd {
    Ls,
    Add(SpecAddOpt),
}

#[derive(Parser, Debug)]
struct SpecAddOpt {
    #[clap(long)]
    width: usize,
    #[clap(long)]
    height: usize,
    #[clap(long)]
    pix_fmt: String,
    #[clap(long)]
    segment_length: String,
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

async fn async_main(args: Args) -> Result<(), IgniError> {
    let pool: sqlx::Pool<sqlx::Postgres> = PgPoolOptions::new()
        .max_connections(1)
        .connect("postgres://igni:igni@localhost/igni")
        .await?;

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
    }

    Ok(())
}

async fn cmd_ping(pool: sqlx::Pool<sqlx::Postgres>) -> Result<(), sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT $1")
        .bind(1_i64)
        .fetch_one(&pool)
        .await?;

    assert_eq!(row.0, 1);
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
    let rows: Vec<(uuid::Uuid, String, i32, Option<String>, sqlx::types::Json<serde_json::Value>, Option<String>, Option<String>, Option<i32>, Option<i32>)> = sqlx::query_as("SELECT id, name, stream_idx, storage_service, storage_config, codec, pix_fmt, width, height FROM source ORDER BY id")
        .fetch_all(&pool)
        .await?;

    #[derive(Tabled)]
    struct Row {
        id: String,
        name: String,
        stream_idx: usize,
        storage_service: String,
        storage_config: String,
        codec: String,
        pix_fmt: String,
        width: String,
        height: String,
    }
    let rows = rows
        .iter()
        .map(
            |(
                id,
                name,
                stream_idx,
                storage_service,
                storage_config,
                codec,
                pix_fmt,
                width,
                height,
            )| Row {
                id: id.to_string(),
                name: name.to_string(),
                stream_idx: *stream_idx as usize,
                storage_service: storage_service.clone().unwrap_or_else(|| "".to_string()),
                storage_config: storage_config.to_string(),
                codec: codec.clone().unwrap_or_else(|| "".to_string()),
                pix_fmt: pix_fmt.clone().unwrap_or_else(|| "".to_string()),
                width: width
                    .map(|w| w.to_string())
                    .unwrap_or_else(|| "".to_string()),
                height: height
                    .map(|h| h.to_string())
                    .unwrap_or_else(|| "".to_string()),
            },
        )
        .collect::<Vec<_>>();
    let table = Table::new(&rows);
    println!("{}", table);
    Ok(())
}

async fn cmd_source_add(
    pool: sqlx::Pool<sqlx::Postgres>,
    add_source: SourceAddOpt,
) -> Result<(), IgniError> {
    let source_id = ops::profile_and_add_source(
        &pool,
        add_source.name.to_string(),
        add_source.stream_idx,
        &add_source.storage_service,
        &add_source.storage_config,
    )
    .await?;

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
    let rows: Vec<(uuid::Uuid, i32, i32, String, i32, i32, Option<String>, Option<String>, i32, bool, bool)> = sqlx::query_as("SELECT id, width, height, pix_fmt, vod_segment_length_num, vod_segment_length_denom, ready_hook, steer_hook, applied_parts, terminated, closed FROM spec ORDER BY id")
        .fetch_all(&pool)
        .await?;

    #[derive(Tabled)]
    struct Row {
        id: String,
        width: i32,
        height: i32,
        pix_fmt: String,
        segment_length: String,
        ready_hook: String,
        steer_hook: String,
        applied_parts: i32,
        terminated: bool,
        closed: bool,
    }
    let rows = rows
        .iter()
        .map(
            |(
                id,
                width,
                height,
                pix_fmt,
                vod_segment_length_num,
                vod_segment_length_denom,
                ready_hook,
                steer_hook,
                applied_parts,
                terminated,
                closed,
            )| Row {
                id: id.to_string(),
                width: *width,
                height: *height,
                pix_fmt: pix_fmt.clone(),
                segment_length: format!("{}/{}", vod_segment_length_num, vod_segment_length_denom),
                ready_hook: ready_hook.clone().unwrap_or_else(|| "".to_string()),
                steer_hook: steer_hook.clone().unwrap_or_else(|| "".to_string()),
                applied_parts: *applied_parts,
                terminated: *terminated,
                closed: *closed,
            },
        )
        .collect::<Vec<_>>();
    let table = Table::new(&rows);
    println!("{}", table);
    Ok(())
}

async fn cmd_spec_add(pool: sqlx::Pool<sqlx::Postgres>, opt: SpecAddOpt) -> Result<(), IgniError> {
    let (num, denom) = match opt.segment_length.as_str() {
        "2/1" => (2, 1),
        _ => {
            return Err(IgniError::General(format!(
                "Invalid segment length: {}",
                opt.segment_length
            )));
        }
    };

    let height = opt.height as i32;
    let width = opt.width as i32;
    let pix_fmt = opt.pix_fmt.clone();
    let ready_hook = opt.ready_hook.clone();
    let steer_hook = opt.steer_hook.clone();

    let spec_id = ops::add_spec(
        &pool, num, denom, height, width, pix_fmt, ready_hook, steer_hook,
    )
    .await?;

    println!("{}", spec_id);
    Ok(())
}
