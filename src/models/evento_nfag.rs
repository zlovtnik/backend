use crate::schema::*;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::{Validate, ValidationError};

#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug, ToSchema)]
#[diesel(table_name = evento_nfag)]
pub struct EventoNfag {
    pub id: i32,
    pub tenant_id: String,
    pub chnfag: String,
    pub tpevento: i32,
    pub nseqevento: i32,
    pub versao: Option<String>,
    pub xml_content: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = evento_nfag)]
pub struct NewEventoNfag {
    pub tenant_id: String,
    pub chnfag: String,
    pub tpevento: i32,
    pub nseqevento: i32,
    pub versao: Option<String>,
    pub xml_content: Option<String>,
    pub status: Option<String>,
}

#[derive(AsChangeset, Serialize, Deserialize, Debug)]
#[diesel(table_name = evento_nfag)]
pub struct UpdateEventoNfag {
    pub xml_content: Option<String>,
    pub status: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Validate, Debug, Clone, ToSchema)]
pub struct CreateEventoNfagRequest {
    #[validate(length(min = 44, max = 44))]
    pub chnfag: String,
    #[validate(range(min = 1))]
    pub tpevento: i32,
    #[validate(range(min = 1))]
    pub nseqevento: i32,
    #[validate(length(min = 1))]
    pub versao: Option<String>,
    #[validate(length(min = 1))]
    pub xml_content: Option<String>,
}

#[derive(Serialize, Deserialize, Validate, Debug, ToSchema)]
pub struct UpdateEventoNfagRequest {
    #[validate(length(min = 1))]
    pub xml_content: Option<String>,

    pub status: Option<String>,
}

/// Custom validator for evento status values.
/// Validates that the status is one of the allowed values (case-insensitive).
/// Note: Normalization to lowercase is handled in the From<UpdateEventoNfagRequest> impl.
#[allow(dead_code)]
fn validate_evento_status(status: &str) -> Result<(), ValidationError> {
    let valid_statuses = [
        "pending",
        "authorized",
        "rejected",
        "cancelled",
        "processed",
        "failed",
    ];
    if valid_statuses.contains(&status.to_lowercase().as_str()) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_status"))
    }
}

impl From<(CreateEventoNfagRequest, String)> for NewEventoNfag {
    fn from((req, tenant_id): (CreateEventoNfagRequest, String)) -> Self {
        NewEventoNfag {
            tenant_id,
            chnfag: req.chnfag,
            tpevento: req.tpevento,
            nseqevento: req.nseqevento,
            versao: req.versao,
            xml_content: req.xml_content,
            status: Some("pending".to_string()), // Default status
        }
    }
}

impl From<(CreateEventoNfagRequest, crate::types::TenantId)> for NewEventoNfag {
    fn from((req, tenant_id): (CreateEventoNfagRequest, crate::types::TenantId)) -> Self {
        NewEventoNfag {
            tenant_id: tenant_id.into_inner(),
            chnfag: req.chnfag,
            tpevento: req.tpevento,
            nseqevento: req.nseqevento,
            versao: req.versao,
            xml_content: req.xml_content,
            status: Some("pending".to_string()), // Default status
        }
    }
}

impl From<UpdateEventoNfagRequest> for UpdateEventoNfag {
    fn from(req: UpdateEventoNfagRequest) -> Self {
        UpdateEventoNfag {
            xml_content: req.xml_content,
            // Normalize status to lowercase for consistent storage
            status: req.status.map(|s| s.to_lowercase()),
            updated_at: Some(Utc::now()),
        }
    }
}

impl EventoNfag {
    /// Find EventoNfag by ID and tenant
    pub fn find_by_id_and_tenant(
        id_: i32,
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<EventoNfag> {
        evento_nfag::table
            .filter(evento_nfag::id.eq(id_))
            .filter(evento_nfag::tenant_id.eq(tenant_id_))
            .first(conn)
    }

    /// Find all EventoNfag for a tenant with pagination
    pub fn find_all_by_tenant(
        tenant_id_: &str,
        limit: i64,
        offset: i64,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<Vec<EventoNfag>> {
        evento_nfag::table
            .filter(evento_nfag::tenant_id.eq(tenant_id_))
            .order(evento_nfag::created_at.desc())
            .limit(limit)
            .offset(offset)
            .load(conn)
    }

    /// Create a new EventoNfag
    pub fn create(
        dto: NewEventoNfag,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<EventoNfag> {
        diesel::insert_into(evento_nfag::table)
            .values(&dto)
            .get_result(conn)
    }

    /// Update EventoNfag by ID and tenant
    pub fn update_by_id_and_tenant(
        id_: i32,
        tenant_id_: &str,
        dto: UpdateEventoNfag,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<EventoNfag> {
        diesel::update(
            evento_nfag::table
                .filter(evento_nfag::id.eq(id_))
                .filter(evento_nfag::tenant_id.eq(tenant_id_)),
        )
        .set(&dto)
        .get_result(conn)
    }

    /// Delete EventoNfag by ID and tenant
    pub fn delete_by_id_and_tenant(
        id_: i32,
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<usize> {
        diesel::delete(
            evento_nfag::table
                .filter(evento_nfag::id.eq(id_))
                .filter(evento_nfag::tenant_id.eq(tenant_id_)),
        )
        .execute(conn)
    }

    /// Count EventoNfag for a tenant
    pub fn count_by_tenant(
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<i64> {
        evento_nfag::table
            .filter(evento_nfag::tenant_id.eq(tenant_id_))
            .count()
            .get_result(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_evento_nfag_requires_tenant_id() {
        let request = CreateEventoNfagRequest {
            chnfag: "12345678901234567890123456789012345678901234".to_string(),
            tpevento: 110110,
            nseqevento: 1,
            versao: Some("1.00".to_string()),
            xml_content: Some("<xml>test</xml>".to_string()),
        };

        // This should NOT compile - we removed the From<CreateEventoNfagRequest> impl
        // let _invalid: NewEventoNfag = request.into(); // This would fail to compile

        // Instead, we must provide tenant_id explicitly
        let tenant_id = "test-tenant-123".to_string();
        let valid: NewEventoNfag = (request.clone(), tenant_id.clone()).into();

        assert_eq!(valid.tenant_id, tenant_id);
        assert_eq!(valid.chnfag, request.chnfag);
        assert_eq!(valid.tpevento, request.tpevento);
        assert_eq!(valid.nseqevento, request.nseqevento);
        assert_eq!(valid.versao, request.versao);
        assert_eq!(valid.xml_content, request.xml_content);
        assert_eq!(valid.status, Some("pending".to_string()));
    }

    #[test]
    fn test_new_evento_nfag_with_tenant_id_type() {
        use crate::types::TenantId;

        let request = CreateEventoNfagRequest {
            chnfag: "12345678901234567890123456789012345678901234".to_string(),
            tpevento: 110110,
            nseqevento: 1,
            versao: Some("1.00".to_string()),
            xml_content: Some("<xml>test</xml>".to_string()),
        };

        let tenant_id = TenantId::new("test-tenant-456".to_string());
        let valid: NewEventoNfag = (request.clone(), tenant_id.clone()).into();

        assert_eq!(valid.tenant_id, tenant_id.into_inner());
        assert_eq!(valid.status, Some("pending".to_string()));
    }

    #[test]
    fn test_update_evento_status_normalized_to_lowercase() {
        let req_uppercase = UpdateEventoNfagRequest {
            xml_content: None,
            status: Some("AUTHORIZED".to_string()),
        };

        let update: UpdateEventoNfag = req_uppercase.into();
        assert_eq!(update.status, Some("authorized".to_string()));
    }

    #[test]
    fn test_update_evento_status_mixed_case_normalized() {
        let req_mixed = UpdateEventoNfagRequest {
            xml_content: None,
            status: Some("ReJeCTeD".to_string()),
        };

        let update: UpdateEventoNfag = req_mixed.into();
        assert_eq!(update.status, Some("rejected".to_string()));
    }

    #[test]
    fn test_update_evento_status_already_lowercase() {
        let req_lowercase = UpdateEventoNfagRequest {
            xml_content: None,
            status: Some("cancelled".to_string()),
        };

        let update: UpdateEventoNfag = req_lowercase.into();
        assert_eq!(update.status, Some("cancelled".to_string()));
    }

    #[test]
    fn test_update_evento_status_none_remains_none() {
        let req_none = UpdateEventoNfagRequest {
            xml_content: None,
            status: None,
        };

        let update: UpdateEventoNfag = req_none.into();
        assert_eq!(update.status, None);
    }

    #[test]
    fn test_default_status_on_create_is_lowercase() {
        let request = CreateEventoNfagRequest {
            chnfag: "12345678901234567890123456789012345678901234".to_string(),
            tpevento: 110110,
            nseqevento: 1,
            versao: Some("1.00".to_string()),
            xml_content: Some("<xml>test</xml>".to_string()),
        };

        let tenant_id = "test-tenant".to_string();
        let new_evento: NewEventoNfag = (request, tenant_id).into();

        // Default status should always be lowercase "pending"
        assert_eq!(new_evento.status, Some("pending".to_string()));
        // Verify it's not uppercase or mixed case
        assert_ne!(new_evento.status, Some("PENDING".to_string()));
        assert_ne!(new_evento.status, Some("Pending".to_string()));
    }
}
