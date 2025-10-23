use crate::schema::nfe_icms;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Associations, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_icms)]
#[diesel(belongs_to(crate::models::nfe_item::NfeItem, foreign_key = nfe_item_id))]
pub struct NfeIcms {
    pub id: i32,
    pub nfe_item_id: i32,
    pub cst: String,
    pub modalidade_bc: Option<String>,
    pub valor_bc: Option<Decimal>,
    pub aliquota: Option<Decimal>,
    pub valor: Option<Decimal>,
    pub modalidade_bc_st: Option<String>,
    pub percentual_mva_st: Option<Decimal>,
    pub percentual_reducao_bc_st: Option<Decimal>,
    pub valor_bc_st: Option<Decimal>,
    pub aliquota_st: Option<Decimal>,
    pub valor_st: Option<Decimal>,
    pub percentual_reducao_bc_efetiva: Option<Decimal>,
    pub valor_bc_efetiva: Option<Decimal>,
    pub aliquota_efetiva: Option<Decimal>,
    pub valor_efetivo: Option<Decimal>,
    pub codigo_beneficio_fiscal: Option<String>,
    pub percentual_diferimento: Option<Decimal>,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_icms)]
pub struct NewNfeIcms {
    pub nfe_item_id: i32,
    pub cst: String,
    pub modalidade_bc: Option<String>,
    pub valor_bc: Option<Decimal>,
    pub aliquota: Option<Decimal>,
    pub valor: Option<Decimal>,
    pub modalidade_bc_st: Option<String>,
    pub percentual_mva_st: Option<Decimal>,
    pub percentual_reducao_bc_st: Option<Decimal>,
    pub valor_bc_st: Option<Decimal>,
    pub aliquota_st: Option<Decimal>,
    pub valor_st: Option<Decimal>,
    pub percentual_reducao_bc_efetiva: Option<Decimal>,
    pub valor_bc_efetiva: Option<Decimal>,
    pub aliquota_efetiva: Option<Decimal>,
    pub valor_efetivo: Option<Decimal>,
    pub codigo_beneficio_fiscal: Option<String>,
    pub percentual_diferimento: Option<Decimal>,
}

#[derive(AsChangeset, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_icms)]
pub struct UpdateNfeIcms {
    pub cst: Option<String>,
    pub modalidade_bc: Option<String>,
    pub valor_bc: Option<Decimal>,
    pub aliquota: Option<Decimal>,
    pub valor: Option<Decimal>,
    pub modalidade_bc_st: Option<String>,
    pub percentual_mva_st: Option<Decimal>,
    pub percentual_reducao_bc_st: Option<Decimal>,
    pub valor_bc_st: Option<Decimal>,
    pub aliquota_st: Option<Decimal>,
    pub valor_st: Option<Decimal>,
    pub percentual_reducao_bc_efetiva: Option<Decimal>,
    pub valor_bc_efetiva: Option<Decimal>,
    pub aliquota_efetiva: Option<Decimal>,
    pub valor_efetivo: Option<Decimal>,
    pub codigo_beneficio_fiscal: Option<String>,
    pub percentual_diferimento: Option<Decimal>,
}
