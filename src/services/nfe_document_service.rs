//! NFE Document Service - Functional Patterns for NFE Document Operations
//!
//! Provides functional programming patterns for NFE document management operations,
//! using QueryReader monads, validators, and composable pipelines.

use crate::{
    config::db::Pool,
    error::{ServiceError, ServiceResult},
    models::nfe_document::{
        operations as nfe_ops,
        validators as nfe_validators,
        NewNfeDocument,
        UpdateNfeDocument,
        NfeDocument,
    },
    services::functional_patterns::{QueryReader, Validator},
};

/// Pagination parameters with functional validation
#[derive(Debug, Clone)]
pub struct PaginationParams {
    pub limit: i64,
    pub offset: i64,
}

impl PaginationParams {
    /// Create pagination params with functional validation and clamping
    pub fn from_query(limit_str: Option<&str>, offset_str: Option<&str>) -> Self {
        let limit = limit_str
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(50)
            .clamp(1, 500);

        let offset = offset_str
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(0)
            .max(0);

        Self { limit, offset }
    }
}

/// Validator for creating new NFE documents
pub fn new_nfe_validator() -> Validator<NewNfeDocument> {
    Validator::new()
        .rule(nfe_validators::validate_new_nfe)
}

/// Validator for updating NFE documents
pub fn update_nfe_validator() -> Validator<UpdateNfeDocument> {
    Validator::new()
        .rule(nfe_validators::validate_update_nfe)
}

/// Build a QueryReader for creating a new NFE document
pub fn create_nfe_reader(
    new_nfe: NewNfeDocument,
) -> Result<QueryReader<NfeDocument>, ServiceError> {
    // Validate the new NFE document first
    new_nfe_validator().validate(&new_nfe)?;

    Ok(QueryReader::new(move |conn| {
        nfe_ops::create_nfe_document(new_nfe.clone(), conn)
            .map_err(|e| e.with_context(|ctx| ctx.with_tag("nfe")))
    }))
}

/// Build a QueryReader for finding an NFE document by ID
pub fn find_nfe_by_id_reader(document_id: i32) -> QueryReader<NfeDocument> {
    QueryReader::new(move |conn| {
        nfe_ops::find_nfe_document_by_id(document_id, conn)
            .map_err(|e| e.with_context(|ctx| ctx.with_tag("nfe")))
    })
}

/// Build a QueryReader for listing NFE documents with pagination
pub fn list_nfe_documents_reader(
    tenant_id: String,
    limit: i64,
    offset: i64,
) -> QueryReader<Vec<NfeDocument>> {
    QueryReader::new(move |conn| {
        nfe_ops::find_nfe_documents_by_tenant(&tenant_id, limit, offset, conn)
            .map_err(|e| e.with_context(|ctx| ctx.with_tag("nfe")))
    })
}

/// Build a QueryReader for updating an NFE document
pub fn update_nfe_reader(
    document_id: i32,
    update_nfe: UpdateNfeDocument,
) -> Result<QueryReader<NfeDocument>, ServiceError> {
    // Validate the update NFE document first
    update_nfe_validator().validate(&update_nfe)?;

    Ok(QueryReader::new(move |conn| {
        nfe_ops::update_nfe_document(document_id, update_nfe.clone(), conn)
            .map_err(|e| e.with_context(|ctx| ctx.with_tag("nfe")))
    }))
}

/// Build a QueryReader for deleting an NFE document
pub fn delete_nfe_reader(document_id: i32) -> QueryReader<usize> {
    QueryReader::new(move |conn| {
        nfe_ops::delete_nfe_document(document_id, conn)
            .map_err(|e| match e {
                ServiceError::NotFound { .. } => e.with_context(|ctx| ctx.with_tag("nfe")),
                _ => ServiceError::internal_server_error(format!("Failed to delete NFE document: {}", e))
                    .with_context(|ctx| ctx.with_tag("nfe")),
            })
    })
}

/// Build a QueryReader for counting NFE documents for a tenant
pub fn count_nfe_documents_reader(tenant_id: String) -> QueryReader<i64> {
    QueryReader::new(move |conn| {
        nfe_ops::count_nfe_documents_by_tenant(&tenant_id, conn)
            .map_err(|e| e.with_context(|ctx| ctx.with_tag("nfe")))
    })
}

/// Execute a QueryReader with a database pool (re-exported for convenience)
pub fn run_query<T>(reader: QueryReader<T>, pool: &Pool) -> ServiceResult<T> {
    crate::services::functional_patterns::run_query(reader, pool)
}
