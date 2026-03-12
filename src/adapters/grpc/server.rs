use std::net::SocketAddr;
use std::sync::Arc;

use tonic::transport::Server;

use crate::adapters::grpc::core::health_service_server::HealthServiceServer;
use crate::adapters::grpc::health_impl::HealthGrpcService;
use crate::app_error::AppError;
use crate::ports::outbound::SystemClock;
use crate::services::core::health_service::HealthService;
use crate::state::AppState;

pub async fn run_grpc_server(
    state: AppState,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), AppError> {
    let addr: SocketAddr = state
        .config()
        .grpc
        .bind_addr()
        .parse()
        .map_err(|e| AppError::Validation(format!("invalid gRPC bind address: {e}")))?;

    let health = Arc::new(HealthService::new(Arc::new(SystemClock)));
    let health_grpc = HealthGrpcService::new(health);

    Server::builder()
        .add_service(HealthServiceServer::new(health_grpc))
        .serve_with_shutdown(addr, async move {
            let _ = shutdown.changed().await;
        })
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
}
