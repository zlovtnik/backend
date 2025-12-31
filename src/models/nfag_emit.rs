use crate::models::nfag::Nfag;
use crate::schema::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Associations, Serialize, Deserialize, Debug)]
#[diesel(belongs_to(Nfag))]
#[diesel(table_name = nfag_emit)]
pub struct NfagEmit {
    pub id: i32,
    pub nfag_id: i32,
    pub cnpj: String,
    pub ie: Option<String>,
    pub xnome: String,
    pub xfant: Option<String>,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfag_emit)]
pub struct NewNfagEmit {
    pub nfag_id: i32,
    pub cnpj: String,
    pub ie: Option<String>,
    pub xnome: String,
    pub xfant: Option<String>,
}
