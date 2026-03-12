use async_trait::async_trait;

#[async_trait]
pub trait TenantSqlxRepository: Send + Sync + 'static {
    async fn health_check(&self) -> Result<(), crate::app_error::AppError>;
}
