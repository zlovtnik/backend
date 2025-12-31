use crate::schema::*;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug, ToSchema)]
#[diesel(table_name = cons_sit_nfag)]
pub struct ConsSitNfag {
    pub id: i32,
    pub tenant_id: String,
    pub tpamb: i32,
    pub xserv: String,
    pub chnfag: String,
    pub versao: Option<String>,
    pub xml_request: Option<String>,
    pub xml_response: Option<String>,
    pub cstat: Option<i32>,
    pub xmotivo: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = cons_sit_nfag)]
pub struct NewConsSitNfag {
    pub tenant_id: String,
    pub tpamb: i32,
    pub xserv: String,
    pub chnfag: String,
    pub versao: Option<String>,
    pub xml_request: Option<String>,
    pub xml_response: Option<String>,
    pub cstat: Option<i32>,
    pub xmotivo: Option<String>,
}

#[derive(AsChangeset, Serialize, Deserialize, Debug)]
#[diesel(table_name = cons_sit_nfag)]
pub struct UpdateConsSitNfag {
    pub xml_response: Option<String>,
    pub cstat: Option<i32>,
    pub xmotivo: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Validate, Debug, ToSchema)]
pub struct CreateConsSitNfagRequest {
    #[validate(range(min = 1, max = 2))]
    pub tpamb: i32,
    #[validate(length(min = 1))]
    pub xserv: String,
    #[validate(length(min = 44, max = 44))]
    pub chnfag: String,
    #[validate(length(min = 1))]
    pub versao: Option<String>,
    pub xml_request: Option<String>,
}

#[derive(Serialize, Deserialize, Validate, Debug, ToSchema)]
pub struct UpdateConsSitNfagRequest {
    pub xml_response: Option<String>,
    #[validate(range(min = 100, max = 999))]
    pub cstat: Option<i32>,
    pub xmotivo: Option<String>,
}

impl From<UpdateConsSitNfagRequest> for UpdateConsSitNfag {
    fn from(request: UpdateConsSitNfagRequest) -> Self {
        UpdateConsSitNfag {
            xml_response: request.xml_response.map(|s| s.trim().to_string()),
            cstat: request.cstat,
            xmotivo: request.xmotivo.map(|s| s.trim().to_string()),
            updated_at: Some(Utc::now()),
        }
    }
}

impl ConsSitNfag {
    /// Find ConsSitNfag by ID and tenant
    pub fn find_by_id_and_tenant(
        id_: i32,
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<ConsSitNfag> {
        cons_sit_nfag::table
            .filter(cons_sit_nfag::id.eq(id_))
            .filter(cons_sit_nfag::tenant_id.eq(tenant_id_))
            .first(conn)
    }

    /// Find all ConsSitNfag for a tenant with pagination
    pub fn find_all_by_tenant(
        tenant_id_: &str,
        limit: i64,
        offset: i64,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<Vec<ConsSitNfag>> {
        cons_sit_nfag::table
            .filter(cons_sit_nfag::tenant_id.eq(tenant_id_))
            .order(cons_sit_nfag::created_at.desc())
            .limit(limit)
            .offset(offset)
            .load(conn)
    }

    /// Create a new ConsSitNfag
    pub fn create(
        dto: NewConsSitNfag,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<ConsSitNfag> {
        diesel::insert_into(cons_sit_nfag::table)
            .values(&dto)
            .get_result(conn)
    }

    /// Update ConsSitNfag by ID and tenant
    pub fn update_by_id_and_tenant(
        id_: i32,
        tenant_id_: &str,
        dto: UpdateConsSitNfag,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<ConsSitNfag> {
        diesel::update(
            cons_sit_nfag::table
                .filter(cons_sit_nfag::id.eq(id_))
                .filter(cons_sit_nfag::tenant_id.eq(tenant_id_)),
        )
        .set(&dto)
        .get_result(conn)
    }

    /// Delete ConsSitNfag by ID and tenant
    pub fn delete_by_id_and_tenant(
        id_: i32,
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<usize> {
        diesel::delete(
            cons_sit_nfag::table
                .filter(cons_sit_nfag::id.eq(id_))
                .filter(cons_sit_nfag::tenant_id.eq(tenant_id_)),
        )
        .execute(conn)
    }

    /// Count ConsSitNfag for a tenant
    pub fn count_by_tenant(
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<i64> {
        cons_sit_nfag::table
            .filter(cons_sit_nfag::tenant_id.eq(tenant_id_))
            .count()
            .get_result(conn)
    }
}
