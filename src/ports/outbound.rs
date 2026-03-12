use async_trait::async_trait;

#[async_trait]
pub trait ClockPort: Send + Sync + 'static {
    async fn now_rfc3339(&self) -> String;
}

pub struct SystemClock;

#[async_trait]
impl ClockPort for SystemClock {
    async fn now_rfc3339(&self) -> String {
        chrono::Utc::now().to_rfc3339()
    }
}
