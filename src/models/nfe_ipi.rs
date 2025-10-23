use crate::schema::nfe_ipi;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Associations, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_ipi)]
#[diesel(belongs_to(crate::models::nfe_item::NfeItem, foreign_key = nfe_item_id))]
pub struct NfeIpi {
    pub id: i32,
    pub nfe_item_id: i32,
    pub cst: String,
    pub classe_enquadramento: Option<String>,
    pub cnpj_produtor: Option<String>,
    pub codigo_selo_controle: Option<String>,
    pub quantidade_selo: Option<i32>,
    pub modalidade_bc: Option<String>,
    pub valor_bc: Option<Decimal>,
    pub aliquota: Option<Decimal>,
    pub quantidade_unidade: Option<Decimal>,
    pub valor_unidade: Option<Decimal>,
    pub valor: Option<Decimal>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_ipi)]
pub struct NewNfeIpi {
    pub nfe_item_id: i32,
    pub cst: String,
    pub classe_enquadramento: Option<String>,
    pub cnpj_produtor: Option<String>,
    pub codigo_selo_controle: Option<String>,
    pub quantidade_selo: Option<i32>,
    pub modalidade_bc: Option<String>,
    pub valor_bc: Option<Decimal>,
    pub aliquota: Option<Decimal>,
    pub quantidade_unidade: Option<Decimal>,
    pub valor_unidade: Option<Decimal>,
    pub valor: Option<Decimal>,
}

#[diesel(treat_none_as_null = false)]
#[derive(AsChangeset, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_ipi)]
#[diesel(treat_none_as_null = false)]
pub struct UpdateNfeIpi {
    pub cst: Option<String>,
    pub classe_enquadramento: Option<String>,
    pub cnpj_produtor: Option<String>,
    pub codigo_selo_controle: Option<String>,
    pub quantidade_selo: Option<i32>,
    pub modalidade_bc: Option<String>,
    pub valor_bc: Option<Decimal>,
    pub aliquota: Option<Decimal>,
    pub quantidade_unidade: Option<Decimal>,
    pub valor_unidade: Option<Decimal>,
    pub valor: Option<Decimal>,
}
