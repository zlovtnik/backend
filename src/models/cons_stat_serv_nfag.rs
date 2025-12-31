use crate::schema::*;
use chrono::{DateTime, Utc};
use diesel::deserialize::FromSql;
use diesel::prelude::*;
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::Integer;
use serde::{Deserialize, Serialize};
use std::io::Write;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, diesel::AsExpression, diesel::FromSqlRow, ToSchema)]
#[diesel(sql_type = diesel::sql_types::Integer)]
#[schema(as = i32, example = 1)]
pub enum Tpamb {
    Production = 1,
    Staging = 2,
}

impl Serialize for Tpamb {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i32(*self as i32)
    }
}

impl<'de> Deserialize<'de> for Tpamb {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = i32::deserialize(deserializer)?;
        Tpamb::try_from(value).map_err(serde::de::Error::custom)
    }
}

impl Tpamb {
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

impl TryFrom<i32> for Tpamb {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Tpamb::Production),
            2 => Ok(Tpamb::Staging),
            _ => Err(format!(
                "Invalid tpamb value: {}. Must be 1 (Production) or 2 (Staging)",
                value
            )),
        }
    }
}

impl From<Tpamb> for i32 {
    fn from(tpamb: Tpamb) -> Self {
        tpamb.as_i32()
    }
}

impl ToSql<Integer, diesel::pg::Pg> for Tpamb {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> diesel::serialize::Result {
        out.write_all(&(*self as i32).to_be_bytes())?;
        Ok(diesel::serialize::IsNull::No)
    }
}

impl FromSql<Integer, diesel::pg::Pg> for Tpamb {
    fn from_sql(bytes: diesel::pg::PgValue) -> diesel::deserialize::Result<Self> {
        let value = <i32 as FromSql<Integer, diesel::pg::Pg>>::from_sql(bytes)?;
        Tpamb::try_from(value).map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                as Box<dyn std::error::Error + Send + Sync>
        })
    }
}

fn deserialize_tpamb<'de, D>(deserializer: D) -> Result<Tpamb, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = i32::deserialize(deserializer)?;
    Tpamb::try_from(value).map_err(serde::de::Error::custom)
}

#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug, ToSchema)]
#[diesel(table_name = cons_stat_serv_nfag)]
pub struct ConsStatServNfag {
    pub id: i32,
    pub tenant_id: String,
    #[serde(deserialize_with = "deserialize_tpamb")]
    pub tpamb: Tpamb,
    pub xserv: String,
    pub versao: Option<String>,
    pub xml_request: Option<String>,
    pub xml_response: Option<String>,
    pub cstat: Option<i32>,
    pub xmotivo: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = cons_stat_serv_nfag)]
pub struct NewConsStatServNfag {
    pub tenant_id: String,
    #[serde(deserialize_with = "deserialize_tpamb")]
    pub tpamb: Tpamb,
    pub xserv: String,
    pub versao: Option<String>,
    pub xml_request: Option<String>,
    pub xml_response: Option<String>,
    pub cstat: Option<i32>,
    pub xmotivo: Option<String>,
}

#[derive(AsChangeset, Serialize, Deserialize, Debug)]
#[diesel(table_name = cons_stat_serv_nfag)]
pub struct UpdateConsStatServNfag {
    pub xml_response: Option<String>,
    pub cstat: Option<i32>,
    pub xmotivo: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Validate, Debug, ToSchema)]
pub struct CreateConsStatServNfagRequest {
    pub tpamb: Tpamb,
    #[validate(length(min = 1))]
    pub xserv: String,
    #[validate(length(min = 1))]
    pub versao: Option<String>,
    pub xml_request: Option<String>,
}

#[derive(Serialize, Deserialize, Validate, Debug, ToSchema)]
pub struct UpdateConsStatServNfagRequest {
    pub xml_response: Option<String>,
    #[validate(range(min = 100, max = 999))]
    pub cstat: Option<i32>,
    pub xmotivo: Option<String>,
}

impl ConsStatServNfag {
    /// Find ConsStatServNfag by ID and tenant
    pub fn find_by_id_and_tenant(
        id_: i32,
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<ConsStatServNfag> {
        cons_stat_serv_nfag::table
            .filter(cons_stat_serv_nfag::id.eq(id_))
            .filter(cons_stat_serv_nfag::tenant_id.eq(tenant_id_))
            .first(conn)
    }

    /// Find all ConsStatServNfag for a tenant with pagination
    pub fn find_all_by_tenant(
        tenant_id_: &str,
        limit: i64,
        offset: i64,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<Vec<ConsStatServNfag>> {
        cons_stat_serv_nfag::table
            .filter(cons_stat_serv_nfag::tenant_id.eq(tenant_id_))
            .order(cons_stat_serv_nfag::created_at.desc())
            .limit(limit)
            .offset(offset)
            .load(conn)
    }

    /// Create a new ConsStatServNfag
    pub fn create(
        dto: NewConsStatServNfag,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<ConsStatServNfag> {
        diesel::insert_into(cons_stat_serv_nfag::table)
            .values(&dto)
            .get_result(conn)
    }

    /// Update ConsStatServNfag by ID and tenant
    pub fn update_by_id_and_tenant(
        id_: i32,
        tenant_id_: &str,
        dto: UpdateConsStatServNfag,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<ConsStatServNfag> {
        diesel::update(
            cons_stat_serv_nfag::table
                .filter(cons_stat_serv_nfag::id.eq(id_))
                .filter(cons_stat_serv_nfag::tenant_id.eq(tenant_id_)),
        )
        .set(&dto)
        .get_result(conn)
    }

    /// Delete ConsStatServNfag by ID and tenant
    pub fn delete_by_id_and_tenant(
        id_: i32,
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<usize> {
        diesel::delete(
            cons_stat_serv_nfag::table
                .filter(cons_stat_serv_nfag::id.eq(id_))
                .filter(cons_stat_serv_nfag::tenant_id.eq(tenant_id_)),
        )
        .execute(conn)
    }

    /// Count ConsStatServNfag for a tenant
    pub fn count_by_tenant(
        tenant_id_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<i64> {
        cons_stat_serv_nfag::table
            .filter(cons_stat_serv_nfag::tenant_id.eq(tenant_id_))
            .count()
            .get_result(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tpamb_try_from_i32() {
        assert_eq!(Tpamb::try_from(1).unwrap(), Tpamb::Production);
        assert_eq!(Tpamb::try_from(2).unwrap(), Tpamb::Staging);
        assert!(Tpamb::try_from(0).is_err());
        assert!(Tpamb::try_from(3).is_err());
    }

    #[test]
    fn test_tpamb_from_i32() {
        assert_eq!(i32::from(Tpamb::Production), 1);
        assert_eq!(i32::from(Tpamb::Staging), 2);
    }

    #[test]
    fn test_tpamb_as_i32() {
        assert_eq!(Tpamb::Production.as_i32(), 1);
        assert_eq!(Tpamb::Staging.as_i32(), 2);
    }

    #[test]
    fn test_tpamb_serde() {
        // Test serialization
        let production_json = serde_json::to_string(&Tpamb::Production).unwrap();
        assert_eq!(production_json, "1");

        let staging_json = serde_json::to_string(&Tpamb::Staging).unwrap();
        assert_eq!(staging_json, "2");

        // Test deserialization
        let production: Tpamb = serde_json::from_str("1").unwrap();
        assert_eq!(production, Tpamb::Production);

        let staging: Tpamb = serde_json::from_str("2").unwrap();
        assert_eq!(staging, Tpamb::Staging);

        // Test invalid deserialization
        assert!(serde_json::from_str::<Tpamb>("0").is_err());
        assert!(serde_json::from_str::<Tpamb>("3").is_err());
        assert!(serde_json::from_str::<Tpamb>("\"invalid\"").is_err());
    }
}
