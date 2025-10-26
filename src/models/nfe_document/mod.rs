//! NFE Document Module
//!
//! This module provides the NFE Document model and related functionality.

use crate::schema::nfe_documents;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = nfe_documents)]
pub struct NfeDocument {
	pub id: i32,
	pub tenant_id: String,
	pub nfe_id: String,
	pub serie: String,
	pub numero: String,
	pub modelo: String,
	pub versao: String,
	pub status: String,
	pub tipo_operacao: String,
	pub tipo_emissao: String,
	pub finalidade: String,
	pub indicador_presencial: String,
	pub data_emissao: DateTime<Utc>,
	pub data_saida_entrada: Option<DateTime<Utc>>,
	pub data_autorizacao: Option<DateTime<Utc>>,
	pub data_cancelamento: Option<DateTime<Utc>>,
	pub valor_total: Decimal,
	pub valor_desconto: Option<Decimal>,
	pub valor_frete: Option<Decimal>,
	pub valor_seguro: Option<Decimal>,
	pub valor_outras_despesas: Option<Decimal>,
	pub valor_produtos: Decimal,
	pub valor_impostos: Decimal,
	pub pedido_compra: Option<String>,
	pub contrato: Option<String>,
	pub informacoes_adicionais: Option<String>,
	pub informacoes_fisco: Option<String>,
	pub protocolo_autorizacao: Option<String>,
	pub motivo_cancelamento: Option<String>,
	pub justificativa_contingencia: Option<String>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = nfe_documents)]
pub struct NewNfeDocument {
	pub tenant_id: String,
	pub nfe_id: String,
	pub serie: String,
	pub numero: String,
	pub modelo: Option<String>,
	pub versao: Option<String>,
	pub status: Option<String>,
	pub tipo_operacao: Option<String>,
	pub tipo_emissao: Option<String>,
	pub finalidade: Option<String>,
	pub indicador_presencial: Option<String>,
	pub data_emissao: Option<DateTime<Utc>>,
	pub data_saida_entrada: Option<DateTime<Utc>>,
	pub data_autorizacao: Option<DateTime<Utc>>,
	pub data_cancelamento: Option<DateTime<Utc>>,
	pub valor_total: Decimal,
	pub valor_desconto: Option<Decimal>,
	pub valor_frete: Option<Decimal>,
	pub valor_seguro: Option<Decimal>,
	pub valor_outras_despesas: Option<Decimal>,
	pub valor_produtos: Decimal,
	pub valor_impostos: Decimal,
	pub pedido_compra: Option<String>,
	pub contrato: Option<String>,
	pub informacoes_adicionais: Option<String>,
	pub informacoes_fisco: Option<String>,
	pub protocolo_autorizacao: Option<String>,
	pub motivo_cancelamento: Option<String>,
	pub justificativa_contingencia: Option<String>,
}

#[derive(AsChangeset, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = nfe_documents)]
#[diesel(treat_none_as_null = false)]
pub struct UpdateNfeDocument {
	pub modelo: Option<String>,
	pub versao: Option<String>,
	pub status: Option<String>,
	pub tipo_operacao: Option<String>,
	pub tipo_emissao: Option<String>,
	pub finalidade: Option<String>,
	pub indicador_presencial: Option<String>,
	pub data_emissao: Option<DateTime<Utc>>,
	pub data_saida_entrada: Option<DateTime<Utc>>,
	pub data_autorizacao: Option<DateTime<Utc>>,
	pub data_cancelamento: Option<DateTime<Utc>>,
	pub valor_total: Option<Decimal>,
	pub valor_desconto: Option<Decimal>,
	pub valor_frete: Option<Decimal>,
	pub valor_seguro: Option<Decimal>,
	pub valor_outras_despesas: Option<Decimal>,
	pub valor_produtos: Option<Decimal>,
	pub valor_impostos: Option<Decimal>,
	pub pedido_compra: Option<String>,
	pub contrato: Option<String>,
	pub informacoes_adicionais: Option<String>,
	pub informacoes_fisco: Option<String>,
	pub protocolo_autorizacao: Option<String>,
	pub motivo_cancelamento: Option<String>,
	pub justificativa_contingencia: Option<String>,
	pub updated_at: Option<DateTime<Utc>>,
}

pub mod operations;
pub mod validators;
