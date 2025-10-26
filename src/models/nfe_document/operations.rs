//! Pure functional operations for NFE Document model
//!
//! This module contains all database and business logic operations for NFE Documents,
//! implemented as pure functions with functional composition patterns.

use diesel::{prelude::*, result::DatabaseErrorKind};

use crate::{
    config::db::Connection,
    error::ServiceError,
    models::nfe_document::{NewNfeDocument, NfeDocument, UpdateNfeDocument},
    schema::nfe_documents::dsl::*,
};

/// Creates a new NFE document in the database.
///
/// # Returns
///
/// `Ok(NfeDocument)` with the created document on success.
/// `Err(ServiceError)` with appropriate error type on failure.
pub fn create_nfe_document(
    new_nfe: NewNfeDocument,
    conn: &mut Connection,
) -> Result<NfeDocument, ServiceError> {
    diesel::insert_into(nfe_documents)
        .values(new_nfe)
        .get_result::<NfeDocument>(conn)
        .map_err(|err| {
            log::error!("Failed to create NFE document: {}", err);
            if let diesel::result::Error::DatabaseError(kind, info) = &err {
                let constraint = info.constraint_name().map(str::to_owned);
                let detail = info.details().map(str::to_owned);
                let base_message = info.message().to_string();

                let mut service_error = match kind {
                    DatabaseErrorKind::UniqueViolation => {
                        ServiceError::conflict(base_message)
                    }
                    DatabaseErrorKind::ForeignKeyViolation
                    | DatabaseErrorKind::CheckViolation
                    | DatabaseErrorKind::NotNullViolation => {
                        ServiceError::bad_request(base_message)
                    }
                    DatabaseErrorKind::SerializationFailure => {
                        ServiceError::internal_server_error(
                            "Failed to create NFE document due to concurrent access"
                                .to_string(),
                        )
                    }
                    _ => ServiceError::internal_server_error(
                        "Failed to create NFE document".to_string(),
                    ),
                };

                if let Some(details) = detail {
                    service_error =
                        service_error.with_context(|ctx| ctx.with_detail(details));
                }

                if let Some(constraint_name) = constraint {
                    service_error = service_error
                        .with_context(|ctx| ctx.with_metadata("constraint", constraint_name));
                }

                return service_error.with_context(|ctx| ctx.with_tag("nfe"));
            }

            ServiceError::internal_server_error("Failed to create NFE document".to_string())
                .with_context(|ctx| ctx.with_tag("nfe").with_detail(err.to_string()))
        })
}

/// Retrieves an NFE document by its ID.
///
/// # Returns
///
/// `Ok(NfeDocument)` with the found document on success.
/// `Err(ServiceError::NotFound)` if no document with the given ID exists.
/// `Err(ServiceError::InternalServerError)` for other database errors.
pub fn find_nfe_document_by_id(
    document_id: i32,
    conn: &mut Connection,
) -> Result<NfeDocument, ServiceError> {
    nfe_documents
        .filter(id.eq(document_id))
        .get_result::<NfeDocument>(conn)
        .map_err(|err| match err {
            diesel::result::Error::NotFound => {
                ServiceError::not_found(format!("NFE document with id {} not found", document_id))
                    .with_context(|ctx| ctx.with_tag("nfe"))
            }
            _ => {
                log::error!("Failed to find NFE document: {}", err);
                ServiceError::internal_server_error("Failed to find NFE document".to_string())
                    .with_context(|ctx| ctx.with_tag("nfe").with_detail(err.to_string()))
            }
        })
}

/// Retrieves NFE documents for a tenant with pagination.
///
/// # Returns
///
/// `Ok(Vec<NfeDocument>)` with the found documents on success.
/// `Err(ServiceError::InternalServerError)` for database errors.
///
/// The pagination inputs are clamped to ensure `0 <= offset` and `0 < limit <= MAX_LIMIT`.
pub fn find_nfe_documents_by_tenant(
    tenant_id_str: &str,
    limit: i64,
    offset: i64,
    conn: &mut Connection,
) -> Result<Vec<NfeDocument>, ServiceError> {
    // clamp pagination inputs to reasonable bounds
    let safe_limit = if limit <= 0 {
        50
    } else {
        limit.min(500)
    };

    let safe_offset = offset.max(0);

    nfe_documents
        .filter(tenant_id.eq(tenant_id_str))
        .order(id.desc())
        .limit(safe_limit)
        .offset(safe_offset)
        .load::<NfeDocument>(conn)
        .map_err(|err| {
            log::error!("Failed to find NFE documents: {}", err);
            ServiceError::internal_server_error("Failed to find NFE documents".to_string())
                .with_context(|ctx| ctx.with_tag("nfe").with_detail(err.to_string()))
        })
}

/// Updates an NFE document by its ID.
///
/// # Returns
///
/// `Ok(NfeDocument)` with the updated document on success.
/// `Err(ServiceError::NotFound)` if no document with the given ID exists.
/// `Err(ServiceError::InternalServerError)` for other database errors.
pub fn update_nfe_document(
    document_id: i32,
    update_nfe: UpdateNfeDocument,
    conn: &mut Connection,
) -> Result<NfeDocument, ServiceError> {
    diesel::update(nfe_documents.filter(id.eq(document_id)))
        .set(update_nfe)
        .get_result::<NfeDocument>(conn)
        .map_err(|err| match err {
            diesel::result::Error::NotFound => {
                ServiceError::not_found(format!("NFE document with id {} not found", document_id))
                    .with_context(|ctx| ctx.with_tag("nfe"))
            }
            _ => {
                log::error!("Failed to update NFE document: {}", err);
                ServiceError::internal_server_error("Failed to update NFE document".to_string())
                    .with_context(|ctx| ctx.with_tag("nfe").with_detail(err.to_string()))
            }
        })
}

/// Deletes an NFE document by its ID.
///
/// # Returns
///
/// `Ok(usize)` with the number of deleted rows on success.
/// `Err(ServiceError::NotFound)` if no document with the given ID exists.
/// `Err(ServiceError::InternalServerError)` for other database errors.
pub fn delete_nfe_document(document_id: i32, conn: &mut Connection) -> Result<usize, ServiceError> {
    let deleted = diesel::delete(nfe_documents.filter(id.eq(document_id)))
        .execute(conn)
        .map_err(|err| {
            log::error!("Failed to delete NFE document: {}", err);
            ServiceError::internal_server_error("Failed to delete NFE document".to_string())
                .with_context(|ctx| ctx.with_tag("nfe").with_detail(err.to_string()))
        })?;
    
    if deleted == 0 {
        Err(ServiceError::not_found(format!("NFE document with id {} not found", document_id))
            .with_context(|ctx| ctx.with_tag("nfe")))
    } else {
        Ok(deleted)
    }
}

/// Counts NFE documents for a tenant.
///
/// # Returns
///
/// `Ok(i64)` with the count of documents on success.
/// `Err(ServiceError::InternalServerError)` for database errors.
pub fn count_nfe_documents_by_tenant(
    tenant_id_str: &str,
    conn: &mut Connection,
) -> Result<i64, ServiceError> {
    nfe_documents
        .filter(tenant_id.eq(tenant_id_str))
        .count()
        .get_result(conn)
        .map_err(|err| {
            log::error!("Failed to count NFE documents: {}", err);
            ServiceError::internal_server_error("Failed to count NFE documents".to_string())
                .with_context(|ctx| ctx.with_tag("nfe").with_detail(err.to_string()))
        })
}
