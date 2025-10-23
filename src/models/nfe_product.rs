use crate::schema::nfe_products;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_products)]
pub struct NfeProduct {
    pub id: i32,
    pub tenant_id: String,
    pub codigo: String,
    pub ean: Option<String>,
    pub descricao: String,
    pub ncm: Option<String>,
    pub cfop: Option<String>,
    pub unidade: String,
    pub valor_unitario: Decimal,
    pub valor_frete: Option<Decimal>,
    pub valor_seguro: Option<Decimal>,
    pub valor_desconto: Option<Decimal>,
    pub valor_outras_despesas: Option<Decimal>,
    pub icms_cst: Option<String>,
    pub icms_aliquota: Option<Decimal>,
    pub ipi_cst: Option<String>,
    pub ipi_aliquota: Option<Decimal>,
    pub pis_cst: Option<String>,
    pub pis_aliquota: Option<Decimal>,
    pub cofins_cst: Option<String>,
    pub cofins_aliquota: Option<Decimal>,
    pub informacoes_adicionais: Option<String>,
    pub ativo: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_products)]
pub struct NewNfeProduct {
    pub tenant_id: String,
    pub codigo: String,
    pub ean: Option<String>,
    pub descricao: String,
    pub ncm: Option<String>,
    pub cfop: Option<String>,
    pub unidade: Option<String>,
    pub valor_unitario: Decimal,
    pub valor_frete: Option<Decimal>,
    pub valor_seguro: Option<Decimal>,
    pub valor_desconto: Option<Decimal>,
    pub valor_outras_despesas: Option<Decimal>,
    pub icms_cst: Option<String>,
    pub icms_aliquota: Option<Decimal>,
    pub ipi_cst: Option<String>,
    pub ipi_aliquota: Option<Decimal>,
    pub pis_cst: Option<String>,
    pub pis_aliquota: Option<Decimal>,
    pub cofins_cst: Option<String>,
    pub cofins_aliquota: Option<Decimal>,
    pub informacoes_adicionais: Option<String>,
    pub ativo: Option<bool>,
}

#[diesel(treat_none_as_null = false)]
#[derive(AsChangeset, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_products)]
pub struct UpdateNfeProduct {
    pub ean: Option<String>,
    pub descricao: Option<String>,
    pub ncm: Option<String>,
    pub cfop: Option<String>,
    pub unidade: Option<String>,
    pub valor_unitario: Option<Decimal>,
    pub valor_frete: Option<Decimal>,
    pub valor_seguro: Option<Decimal>,
    pub valor_desconto: Option<Decimal>,
    pub valor_outras_despesas: Option<Decimal>,
    pub icms_cst: Option<String>,
    pub icms_aliquota: Option<Decimal>,
    pub ipi_cst: Option<String>,
    pub ipi_aliquota: Option<Decimal>,
    pub pis_cst: Option<String>,
    pub pis_aliquota: Option<Decimal>,
    pub cofins_cst: Option<String>,
    pub cofins_aliquota: Option<Decimal>,
    pub informacoes_adicionais: Option<String>,
    pub ativo: Option<bool>,
    pub updated_at: Option<DateTime<Utc>>,
}
