use crate::schema::*;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug, ToSchema)]
#[diesel(table_name = nfag)]
pub struct Nfag {
    pub id: i32,
    pub chave: String,
    pub tenant_id: String,
    pub versao: Option<String>,
    pub xml_content: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfag)]
pub struct NewNfag {
    pub chave: String,
    pub tenant_id: String,
    pub versao: Option<String>,
    pub xml_content: Option<String>,
    pub status: Option<String>,
}

#[derive(AsChangeset, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfag)]
pub struct UpdateNfag {
    pub versao: Option<String>,
    pub xml_content: Option<String>,
    pub status: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Validate, Debug, ToSchema)]
pub struct CreateNfagRequest {
    #[validate(length(min = 44, max = 44))]
    pub chave: String,
    #[validate(length(min = 1))]
    pub versao: Option<String>,
    #[validate(length(min = 1))]
    pub xml_content: Option<String>,
    pub status: Option<String>,
}

#[derive(Serialize, Deserialize, Validate, Debug, ToSchema)]
pub struct UpdateNfagRequest {
    #[validate(length(min = 1))]
    pub versao: Option<String>,
    #[validate(length(min = 1))]
    pub xml_content: Option<String>,
    pub status: Option<String>,
}

impl Nfag {
    /// Find NFAg by ID and tenant
    pub fn find_by_id_and_tenant(
        nfag_id: i32,
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<Nfag> {
        nfag::table
            .filter(nfag::id.eq(nfag_id))
            .filter(nfag::tenant_id.eq(tenant_id_))
            .first(conn)
    }

    /// Find all NFAg for a tenant with pagination
    pub fn find_all_by_tenant(
        tenant_id_: &str,
        limit: i64,
        offset: i64,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<Vec<Nfag>> {
        nfag::table
            .filter(nfag::tenant_id.eq(tenant_id_))
            .order(nfag::created_at.desc())
            .limit(limit)
            .offset(offset)
            .load(conn)
    }

    /// Create a new NFAg
    pub fn create(dto: NewNfag, conn: &mut crate::config::db::Connection) -> QueryResult<Nfag> {
        diesel::insert_into(nfag::table)
            .values(&dto)
            .get_result(conn)
    }

    /// Update NFAg by ID and tenant
    pub fn update_by_id_and_tenant(
        nfag_id: i32,
        tenant_id_: &str,
        dto: UpdateNfag,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<Nfag> {
        diesel::update(
            nfag::table
                .filter(nfag::id.eq(nfag_id))
                .filter(nfag::tenant_id.eq(tenant_id_)),
        )
        .set(&dto)
        .get_result(conn)
    }

    /// Delete NFAg by ID and tenant
    pub fn delete_by_id_and_tenant(
        nfag_id: i32,
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<usize> {
        diesel::delete(
            nfag::table
                .filter(nfag::id.eq(nfag_id))
                .filter(nfag::tenant_id.eq(tenant_id_)),
        )
        .execute(conn)
    }

    /// Count NFAg for a tenant
    pub fn count_by_tenant(
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<i64> {
        nfag::table
            .filter(nfag::tenant_id.eq(tenant_id_))
            .count()
            .get_result(conn)
    }
}
