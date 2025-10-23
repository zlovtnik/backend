use crate::schema::nfe_documents;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug)]
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
    pub data_emissao: NaiveDateTime,
    pub data_saida_entrada: Option<NaiveDateTime>,
    pub data_autorizacao: Option<NaiveDateTime>,
    pub data_cancelamento: Option<NaiveDateTime>,
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
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
    pub data_emissao: Option<NaiveDateTime>,
    pub data_saida_entrada: Option<NaiveDateTime>,
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
}

#[derive(AsChangeset, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_documents)]
pub struct UpdateNfeDocument {
    pub status: Option<String>,
    pub data_autorizacao: Option<NaiveDateTime>,
    pub data_cancelamento: Option<NaiveDateTime>,
    pub protocolo_autorizacao: Option<String>,
    pub motivo_cancelamento: Option<String>,
    pub justificativa_contingencia: Option<String>,
    pub informacoes_adicionais: Option<String>,
    pub informacoes_fisco: Option<String>,
    pub updated_at: Option<NaiveDateTime>,
}
