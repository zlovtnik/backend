use crate::schema::nfe_pis;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Associations, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_pis)]
#[diesel(belongs_to(crate::models::nfe_item::NfeItem, foreign_key = nfe_item_id))]
pub struct NfePis {
    pub id: i32,
    pub nfe_item_id: i32,
    pub cst: String,
    pub modalidade_bc: Option<String>,
    pub valor_bc: Option<Decimal>,
    pub aliquota_percentual: Option<Decimal>,
    pub aliquota_valor: Option<Decimal>,
    pub quantidade_vendida: Option<Decimal>,
    pub valor: Option<Decimal>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_pis)]
pub struct NewNfePis {
    pub nfe_item_id: i32,
    pub cst: String,
    pub modalidade_bc: Option<String>,
    pub valor_bc: Option<Decimal>,
    pub aliquota_percentual: Option<Decimal>,
    pub aliquota_valor: Option<Decimal>,
    pub quantidade_vendida: Option<Decimal>,
    pub valor: Option<Decimal>,
}

#[derive(AsChangeset, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_pis)]
pub struct UpdateNfePis {
    pub cst: Option<String>,
    pub modalidade_bc: Option<String>,
    pub valor_bc: Option<Decimal>,
    pub aliquota_percentual: Option<Decimal>,
    pub aliquota_valor: Option<Decimal>,
    pub quantidade_vendida: Option<Decimal>,
    pub valor: Option<Decimal>,
}
