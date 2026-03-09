use async_trait::async_trait;

use crate::ports::repository::TenantSqlxRepository;

#[derive(Clone)]
pub struct TenantSqlxRepositoryImpl {
    pool: sqlx::PgPool,
}

impl TenantSqlxRepositoryImpl {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantSqlxRepository for TenantSqlxRepositoryImpl {
    async fn health_check(&self) -> Result<(), crate::app_error::AppError> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map(|_| ())
            .map_err(crate::app_error::AppError::from)
    }
}
