use std::sync::Arc;

use crate::domain::health::HealthSnapshot;
use crate::ports::outbound::ClockPort;

#[derive(Clone)]
pub struct HealthService {
    clock: Arc<dyn ClockPort>,
}

impl HealthService {
    pub fn new(clock: Arc<dyn ClockPort>) -> Self {
        Self { clock }
    }

    pub async fn check(&self) -> Result<HealthSnapshot, crate::app_error::AppError> {
        Ok(HealthSnapshot {
            status: "ok".to_string(),
            service: "nexus-core".to_string(),
            timestamp: self.clock.now_rfc3339().await,
        })
    }
}
