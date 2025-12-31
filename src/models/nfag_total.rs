use crate::models::nfag::Nfag;
use crate::schema::*;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Associations, Serialize, Deserialize, Debug)]
#[diesel(belongs_to(Nfag))]
#[diesel(table_name = nfag_total)]
pub struct NfagTotal {
    pub id: i32,
    pub nfag_id: i32,
    pub vbc: Option<Decimal>,
    pub vicms: Option<Decimal>,
    pub vicmsdeson: Option<Decimal>,
    pub vfcpufdest: Option<Decimal>,
    pub vicmsufdest: Option<Decimal>,
    pub vicmsufremet: Option<Decimal>,
    pub vfcp: Option<Decimal>,
    pub vbcst: Option<Decimal>,
    pub vst: Option<Decimal>,
    pub vfcpst: Option<Decimal>,
    pub vfcpstret: Option<Decimal>,
    pub vprod: Option<Decimal>,
    pub vfrete: Option<Decimal>,
    pub vseg: Option<Decimal>,
    pub vdesc: Option<Decimal>,
    pub vii: Option<Decimal>,
    pub vipi: Option<Decimal>,
    pub vipidevol: Option<Decimal>,
    pub vpis: Option<Decimal>,
    pub vcofins: Option<Decimal>,
    pub voutro: Option<Decimal>,
    pub vnf: Option<Decimal>,
    pub vtottrib: Option<Decimal>,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfag_total)]
pub struct NewNfagTotal {
    pub nfag_id: i32,
    pub vbc: Option<Decimal>,
    pub vicms: Option<Decimal>,
    pub vicmsdeson: Option<Decimal>,
    pub vfcpufdest: Option<Decimal>,
    pub vicmsufdest: Option<Decimal>,
    pub vicmsufremet: Option<Decimal>,
    pub vfcp: Option<Decimal>,
    pub vbcst: Option<Decimal>,
    pub vst: Option<Decimal>,
    pub vfcpst: Option<Decimal>,
    pub vfcpstret: Option<Decimal>,
    pub vprod: Option<Decimal>,
    pub vfrete: Option<Decimal>,
    pub vseg: Option<Decimal>,
    pub vdesc: Option<Decimal>,
    pub vii: Option<Decimal>,
    pub vipi: Option<Decimal>,
    pub vipidevol: Option<Decimal>,
    pub vpis: Option<Decimal>,
    pub vcofins: Option<Decimal>,
    pub voutro: Option<Decimal>,
    pub vnf: Option<Decimal>,
    pub vtottrib: Option<Decimal>,
}
