use super::super::IgniError;
use super::IgniServerGlobal;
use log::*;
use uuid::Uuid;

pub(crate) async fn gc_main(global: std::sync::Arc<IgniServerGlobal>) -> Result<(), IgniError> {
    let period = global.config.gc_period;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(period as u64)).await;
        let gc_result = gc(global.clone()).await;
        if let Err(err) = gc_result {
            error!("Garbage collector failure: {:?}", err);
        }
    }
}

pub(crate) async fn gc(global: std::sync::Arc<IgniServerGlobal>) -> Result<(), IgniError> {
    info!("Running garbage collector");
    let now = chrono::Utc::now();

    let mut transaction = global.pool.begin().await?;
    let spec_ids_to_delete: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM spec WHERE NOT closed AND expires_at IS NOT NULL AND expires_at < $1",
    )
    .bind(now)
    .fetch_all(&mut *transaction)
    .await?;

    for (spec_id,) in &spec_ids_to_delete {
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
    }

    transaction.commit().await?;

    if !spec_ids_to_delete.is_empty() {
        info!("Deleted {} expired specs", spec_ids_to_delete.len());
    } else {
        info!("No expired specs to delete");
    }

    Ok(())
}
