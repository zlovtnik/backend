use crate::models::nfag::Nfag;
use crate::schema::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Queryable, Identifiable, Associations, Serialize, Deserialize, Debug)]
#[diesel(belongs_to(Nfag))]
#[diesel(table_name = nfag_ide)]
pub struct NfagIde {
    pub id: i32,
    pub nfag_id: i32,
    pub cuf: i32,
    pub tpamb: i32,
    pub mod_: i32,
    pub serie: i32,
    pub nnf: i64,
    pub cnf: String,
    pub cdv: String,
    pub dhemi: DateTime<Utc>,
    pub tpemis: i32,
    pub nsiteautoriz: i32,
    pub cmunfg: i32,
    pub finnfag: i32,
    pub tpfat: i32,
    pub verproc: String,
    pub dhcont: Option<DateTime<Utc>>,
    pub xjust: Option<String>,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfag_ide)]
pub struct NewNfagIde {
    pub nfag_id: i32,
    pub cuf: i32,
    pub tpamb: i32,
    pub mod_: i32,
    pub serie: i32,
    pub nnf: i64,
    pub cnf: String,
    pub cdv: String,
    pub dhemi: DateTime<Utc>,
    pub tpemis: i32,
    pub nsiteautoriz: i32,
    pub cmunfg: i32,
    pub finnfag: i32,
    pub tpfat: i32,
    pub verproc: String,
    pub dhcont: Option<DateTime<Utc>>,
    pub xjust: Option<String>,
}