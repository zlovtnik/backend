use crate::schema::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug, ToSchema)]
#[diesel(table_name = ret_nfag)]
pub struct RetNfag {
    pub id: i32,
    pub tenant_id: String,
    pub tpamb: i32,
    pub cstat: i32,
    pub xmotivo: Option<String>,
    pub versao: Option<String>,
    pub xml_content: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = ret_nfag)]
pub struct NewRetNfag {
    pub tenant_id: String,
    pub tpamb: i32,
    pub cstat: i32,
    pub xmotivo: Option<String>,
    pub versao: Option<String>,
    pub xml_content: Option<String>,
}

#[derive(AsChangeset, Serialize, Deserialize, Debug)]
#[diesel(table_name = ret_nfag)]
pub struct UpdateRetNfag {
    pub xml_content: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Validate, Debug, ToSchema)]
pub struct CreateRetNfagRequest {
    #[validate(range(min = 1, max = 2))]
    pub tpamb: i32,
    #[validate(range(min = 100, max = 999))]
    pub cstat: i32,
    pub xmotivo: Option<String>,
    #[validate(length(min = 1))]
    pub versao: Option<String>,
    #[validate(length(min = 1, max = 10000))]
    pub xml_content: Option<String>,
}

#[derive(Serialize, Deserialize, Validate, Debug, ToSchema)]
pub struct UpdateRetNfagRequest {
    #[validate(length(min = 1, max = 10000))]
    pub xml_content: Option<String>,
}

impl From<(CreateRetNfagRequest, String)> for NewRetNfag {
    fn from((req, tenant_id): (CreateRetNfagRequest, String)) -> Self {
        NewRetNfag {
            tenant_id,
            tpamb: req.tpamb,
            cstat: req.cstat,
            xmotivo: req.xmotivo,
            versao: req.versao,
            xml_content: req.xml_content,
        }
    }
}

impl From<(CreateRetNfagRequest, crate::types::TenantId)> for NewRetNfag {
    fn from((req, tenant_id): (CreateRetNfagRequest, crate::types::TenantId)) -> Self {
        NewRetNfag {
            tenant_id: tenant_id.into_inner(),
            tpamb: req.tpamb,
            cstat: req.cstat,
            xmotivo: req.xmotivo,
            versao: req.versao,
            xml_content: req.xml_content,
        }
    }
}

impl From<UpdateRetNfagRequest> for UpdateRetNfag {
    fn from(req: UpdateRetNfagRequest) -> Self {
        UpdateRetNfag {
            xml_content: req.xml_content,
            updated_at: Some(Utc::now()),
        }
    }
}

impl RetNfag {
    /// Find RetNfag by ID and tenant
    pub fn find_by_id_and_tenant(
        id_: i32,
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<RetNfag> {
        ret_nfag::table
            .filter(ret_nfag::id.eq(id_))
            .filter(ret_nfag::tenant_id.eq(tenant_id_))
            .first(conn)
    }

    /// Find all RetNfag for a tenant with pagination
    pub fn find_all_by_tenant(
        tenant_id_: &str,
        limit: i64,
        offset: i64,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<Vec<RetNfag>> {
        ret_nfag::table
            .filter(ret_nfag::tenant_id.eq(tenant_id_))
            .order(ret_nfag::created_at.desc())
            .limit(limit)
            .offset(offset)
            .load(conn)
    }

    /// Create a new RetNfag
    pub fn create(
        dto: NewRetNfag,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<RetNfag> {
        diesel::insert_into(ret_nfag::table)
            .values(&dto)
            .get_result(conn)
    }

    /// Update RetNfag by ID and tenant
    pub fn update_by_id_and_tenant(
        id_: i32,
        tenant_id_: &str,
        dto: UpdateRetNfag,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<RetNfag> {
        diesel::update(
            ret_nfag::table
                .filter(ret_nfag::id.eq(id_))
                .filter(ret_nfag::tenant_id.eq(tenant_id_))
        )
        .set(&dto)
        .get_result(conn)
    }

    /// Delete RetNfag by ID and tenant
    pub fn delete_by_id_and_tenant(
        id_: i32,
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<usize> {
        diesel::delete(
            ret_nfag::table
                .filter(ret_nfag::id.eq(id_))
                .filter(ret_nfag::tenant_id.eq(tenant_id_))
        )
        .execute(conn)
    }

    /// Count RetNfag for a tenant
    pub fn count_by_tenant(
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<i64> {
        ret_nfag::table
            .filter(ret_nfag::tenant_id.eq(tenant_id_))
            .count()
            .get_result(conn)
    }
}