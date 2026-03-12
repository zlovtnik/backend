use std::collections::HashMap;

use actix_web::{http::header::HeaderValue, HttpMessage, HttpRequest};

use crate::{
    config::db::{Pool, TenantPoolManager},
    constants,
    error::ServiceError,
    services::functional_patterns::{run_query, QueryReader},
    state::AppState,
    types::TenantId,
};

use crate::functional::pagination::Pagination;

#[derive(Clone)]
pub struct DatabaseContext {
    pool: Pool,
    tenant_id: Option<String>,
}

impl DatabaseContext {
    pub fn from_request(req: &HttpRequest) -> Result<Self, ServiceError> {
        let pool = req.extensions().get::<Pool>().cloned().ok_or_else(|| {
            ServiceError::internal_server_error("Pool not found").with_context(|ctx| {
                ctx.with_tag("tenant")
                    .with_detail("Missing tenant pool in request extensions")
            })
        })?;

        // Auth middleware stores the tenant in request extensions as TenantId.
        let tenant_id = req
            .extensions()
            .get::<TenantId>()
            .map(|tenant_id| tenant_id.as_str().to_string())
            .or_else(|| req.extensions().get::<String>().cloned());

        Ok(Self { pool, tenant_id })
    }

    pub fn from_state(
        state: &AppState,
        tenant_id: impl Into<String>,
    ) -> Result<Self, ServiceError> {
        Self::from_manager(&state.tenant_manager(), tenant_id)
    }

    pub fn from_manager(
        manager: &TenantPoolManager,
        tenant_id: impl Into<String>,
    ) -> Result<Self, ServiceError> {
        let tenant_id = tenant_id.into();
        manager
            .get_tenant_pool(&tenant_id)
            .map(|pool| Self {
                pool,
                tenant_id: Some(tenant_id.clone()),
            })
            .ok_or_else(|| {
                ServiceError::bad_request("Tenant not found").with_context(|ctx| {
                    ctx.with_metadata("tenant_id", tenant_id).with_tag("tenant")
                })
            })
    }

    pub fn with_pool(pool: Pool) -> Self {
        Self {
            pool,
            tenant_id: None,
        }
    }

    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    pub fn tenant_id(&self) -> Option<&str> {
        self.tenant_id.as_deref()
    }

    pub fn run_query<T>(&self, reader: QueryReader<T>) -> Result<T, ServiceError> {
        run_query(reader, &self.pool)
    }
}

#[derive(Clone)]
pub struct AuthContext {
    header: HeaderValue,
}

impl AuthContext {
    pub fn new(header: HeaderValue) -> Self {
        Self { header }
    }

    pub fn from_request(req: &HttpRequest) -> Option<Self> {
        req.headers()
            .get(constants::AUTHORIZATION)
            .cloned()
            .map(Self::new)
    }

    pub fn header(&self) -> &HeaderValue {
        &self.header
    }
}

#[derive(Clone)]
pub struct PaginationContext {
    pagination: Pagination,
    raw_cursor: Option<i64>,
    raw_limit: Option<i64>,
}

impl PaginationContext {
    pub fn from_query_map(
        query: &HashMap<String, String>,
        default_page_size: usize,
        max_page_size: usize,
    ) -> Self {
        let cursor = query
            .get("cursor")
            .or_else(|| query.get("offset"))
            .and_then(|value| value.parse::<i64>().ok());

        let raw_limit = query
            .get("limit")
            .and_then(|value| value.parse::<i64>().ok());

        let limit = raw_limit.map(|value| value.max(1).min(max_page_size as i64));

        let pagination = Pagination::from_optional(cursor, limit, default_page_size);

        Self {
            pagination,
            raw_cursor: cursor,
            raw_limit,
        }
    }

    pub fn pagination(&self) -> Pagination {
        self.pagination
    }

    pub fn raw_cursor(&self) -> Option<i64> {
        self.raw_cursor
    }

    pub fn raw_limit(&self) -> Option<i64> {
        self.raw_limit
    }
}

pub struct ControllerContext {
    database: DatabaseContext,
    auth: Option<AuthContext>,
    pagination: Option<PaginationContext>,
}

impl ControllerContext {
    pub fn new(database: DatabaseContext) -> Self {
        Self {
            database,
            auth: None,
            pagination: None,
        }
    }

    pub fn with_auth(mut self, auth: Option<AuthContext>) -> Self {
        self.auth = auth;
        self
    }

    pub fn with_pagination(mut self, pagination: Option<PaginationContext>) -> Self {
        self.pagination = pagination;
        self
    }

    pub fn database(&self) -> &DatabaseContext {
        &self.database
    }

    pub fn auth(&self) -> Option<&AuthContext> {
        self.auth.as_ref()
    }

    pub fn pagination(&self) -> Option<&PaginationContext> {
        self.pagination.as_ref()
    }

    pub fn tenant_id(&self) -> Option<&str> {
        self.database.tenant_id()
    }

    pub fn run_query<T>(&self, reader: QueryReader<T>) -> Result<T, ServiceError> {
        self.database.run_query(reader)
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{test::TestRequest, HttpMessage};
    use diesel::r2d2::ConnectionManager;

    use super::DatabaseContext;
    use crate::config::db;
    use crate::types::TenantId;

    #[test]
    fn from_request_reads_tenant_id_newtype() {
        let manager = ConnectionManager::<db::Connection>::new("postgres://localhost/test");
        let pool = db::Pool::builder().max_size(1).build_unchecked(manager);

        let req = TestRequest::default().to_http_request();
        req.extensions_mut().insert(pool.clone());
        req.extensions_mut().insert(TenantId::from("tenant-42"));

        let ctx = DatabaseContext::from_request(&req).expect("request context should build");

        let _ = ctx.pool();
        assert_eq!(ctx.tenant_id(), Some("tenant-42"));
    }
}
