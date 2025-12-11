use crate::schema::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;
use validator::{Validate, ValidationError};

#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug)]
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
    pub created_at: Option<NaiveDateTime>,
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
}

#[derive(Serialize, Deserialize, Validate, Debug)]
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
    pub xml_content: Option<String>, // TODO: Add XML validation once XML parsing library is added (e.g., #[validate(custom = "validate_xml")])
}

#[derive(Serialize, Deserialize, Validate, Debug)]
pub struct UpdateEventoNfagRequest {
    #[validate(length(min = 1))]
    pub xml_content: Option<String>,
    #[validate(custom = "validate_evento_status")]
    pub status: Option<String>,
}

/// Custom validator for evento status values
fn validate_evento_status(status: &str) -> Result<(), ValidationError> {
    let valid_statuses = ["pending", "authorized", "rejected", "cancelled", "processed", "failed"];
    if valid_statuses.contains(&status.to_lowercase().as_str()) {
        Ok(())
    } else {
        Err(ValidationError::new("invalid_status"))
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
                .filter(evento_nfag::tenant_id.eq(tenant_id_))
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
                .filter(evento_nfag::tenant_id.eq(tenant_id_))
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