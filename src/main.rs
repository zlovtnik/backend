use anyhow::Context;

use rcs::{
    adapters::{grpc::server::run_grpc_server, http::server::run_http_server},
    config::runtime::AppConfig,
    state::AppState,
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let cfg = AppConfig::from_env().context("failed to load application configuration")?;
    let state = AppState::new(cfg)
        .await
        .context("failed to initialize application state")?;

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let mut tasks = tokio::task::JoinSet::new();

    // HTTP transport
    let http_state = state.clone();
    let http_shutdown = shutdown_rx.clone();
    tasks.spawn(async move { run_http_server(http_state, http_shutdown).await });

    // gRPC transport
    let grpc_state = state.clone();
    let grpc_shutdown = shutdown_rx.clone();
    tasks.spawn(async move { run_grpc_server(grpc_state, grpc_shutdown).await });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutdown signal received");
            let _ = shutdown_tx.send(true);
            while let Some(result) = tasks.join_next().await {
                let task_result = result.context("server task join failure")?;
                task_result.map_err(anyhow::Error::from)?;
            }
        }
        result = tasks.join_next() => {
            let _ = shutdown_tx.send(true);
            match result {
                Some(join_res) => {
                    let task_result = join_res.context("server task join failure")?;
                    task_result.map_err(anyhow::Error::from)?;
                }
                None => {
                    anyhow::bail!("both server tasks stopped unexpectedly");
                }
            }

            while let Some(pending) = tasks.join_next().await {
                let task_result = pending.context("server task join failure")?;
                task_result.map_err(anyhow::Error::from)?;
            }
        }
    }

    Ok(())
}
