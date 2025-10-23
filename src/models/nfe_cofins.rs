use crate::schema::nfe_cofins;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Associations, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_cofins)]
#[diesel(belongs_to(crate::models::nfe_item::NfeItem, foreign_key = nfe_item_id))]
pub struct NfeCofins {
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
#[diesel(table_name = nfe_cofins)]
pub struct NewNfeCofins {
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
#[diesel(table_name = nfe_cofins)]
#[diesel(treat_none_as_null = false)]
pub struct UpdateNfeCofins {
    pub cst: Option<String>,
    pub modalidade_bc: Option<String>,
    pub valor_bc: Option<Decimal>,
    pub aliquota_percentual: Option<Decimal>,
    pub aliquota_valor: Option<Decimal>,
    pub quantidade_vendida: Option<Decimal>,
    pub valor: Option<Decimal>,
}
