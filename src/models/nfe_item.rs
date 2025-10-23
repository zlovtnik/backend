use crate::schema::nfe_items;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Associations, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_items)]
#[diesel(belongs_to(crate::models::nfe_document::NfeDocument, foreign_key = nfe_document_id))]
#[diesel(belongs_to(crate::models::nfe_product::NfeProduct, foreign_key = product_id))]
pub struct NfeItem {
    pub id: i32,
    pub nfe_document_id: i32,
    pub numero_item: i32,
    pub product_id: Option<i32>,
    pub codigo: String,
    pub ean: Option<String>,
    pub descricao: String,
    pub ncm: Option<String>,
    pub cfop: String,
    pub unidade: String,
    pub quantidade: Decimal,
    pub valor_unitario: Decimal,
    pub valor_total: Decimal,
    pub valor_desconto: Option<Decimal>,
    pub valor_frete: Option<Decimal>,
    pub valor_seguro: Option<Decimal>,
    pub valor_outras_despesas: Option<Decimal>,
    pub valor_bc_icms: Option<Decimal>,
    pub valor_icms: Option<Decimal>,
    pub valor_bc_icms_st: Option<Decimal>,
    pub valor_icms_st: Option<Decimal>,
    pub valor_bc_ipi: Option<Decimal>,
    pub valor_ipi: Option<Decimal>,
    pub valor_bc_pis: Option<Decimal>,
    pub valor_pis: Option<Decimal>,
    pub valor_bc_cofins: Option<Decimal>,
    pub valor_cofins: Option<Decimal>,
    pub informacoes_adicionais: Option<String>,
    pub numero_pedido_compra: Option<String>,
    pub item_pedido_compra: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_items)]
pub struct NewNfeItem {
    pub nfe_document_id: i32,
    pub numero_item: i32,
    pub product_id: Option<i32>,
    pub codigo: String,
    pub ean: Option<String>,
    pub descricao: String,
    pub ncm: Option<String>,
    pub cfop: String,
    pub unidade: String,
    pub quantidade: Decimal,
    pub valor_unitario: Decimal,
    pub valor_total: Decimal,
    pub valor_desconto: Option<Decimal>,
    pub valor_frete: Option<Decimal>,
    pub valor_seguro: Option<Decimal>,
    pub valor_outras_despesas: Option<Decimal>,
    pub valor_bc_icms: Option<Decimal>,
    pub valor_icms: Option<Decimal>,
    pub valor_bc_icms_st: Option<Decimal>,
    pub valor_icms_st: Option<Decimal>,
    pub valor_bc_ipi: Option<Decimal>,
    pub valor_ipi: Option<Decimal>,
    pub valor_bc_pis: Option<Decimal>,
    pub valor_pis: Option<Decimal>,
    pub valor_bc_cofins: Option<Decimal>,
    pub valor_cofins: Option<Decimal>,
    pub informacoes_adicionais: Option<String>,
    pub numero_pedido_compra: Option<String>,
    pub item_pedido_compra: Option<String>,
}

#[derive(AsChangeset, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_items)]
pub struct UpdateNfeItem {
    pub quantidade: Option<Decimal>,
    pub valor_unitario: Option<Decimal>,
    pub valor_total: Option<Decimal>,
    pub valor_desconto: Option<Decimal>,
    pub valor_frete: Option<Decimal>,
    pub valor_seguro: Option<Decimal>,
    pub valor_outras_despesas: Option<Decimal>,
    pub valor_bc_icms: Option<Decimal>,
    pub valor_icms: Option<Decimal>,
    pub valor_bc_icms_st: Option<Decimal>,
    pub valor_icms_st: Option<Decimal>,
    pub valor_bc_ipi: Option<Decimal>,
    pub valor_ipi: Option<Decimal>,
    pub valor_bc_pis: Option<Decimal>,
    pub valor_pis: Option<Decimal>,
    pub valor_bc_cofins: Option<Decimal>,
    pub valor_cofins: Option<Decimal>,
    pub informacoes_adicionais: Option<String>,
    pub numero_pedido_compra: Option<String>,
    pub item_pedido_compra: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}
