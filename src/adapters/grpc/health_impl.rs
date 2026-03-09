use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::adapters::grpc::core::health_service_server::HealthService;
use crate::adapters::grpc::core::{HealthCheckRequest, HealthCheckResponse};
use crate::services::core::health_service::HealthService as CoreHealthService;

#[derive(Clone)]
pub struct HealthGrpcService {
    service: Arc<CoreHealthService>,
}

impl HealthGrpcService {
    pub fn new(service: Arc<CoreHealthService>) -> Self {
        Self { service }
    }
}

#[tonic::async_trait]
impl HealthService for HealthGrpcService {
    async fn check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let snapshot = self
            .service
            .check()
            .await
            .map_err(Status::from)?;

        Ok(Response::new(HealthCheckResponse {
            status: snapshot.status,
            service: snapshot.service,
            timestamp: snapshot.timestamp,
        }))
    }
}
