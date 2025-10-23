use crate::schema::nfe_recipients;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_recipients)]
pub struct NfeRecipient {
    pub id: i32,
    pub tenant_id: String,
    pub tipo_pessoa: String,
    pub cnpj: Option<String>,
    pub cpf: Option<String>,
    pub id_estrangeiro: Option<String>,
    pub razao_social: String,
    pub nome_fantasia: Option<String>,
    pub inscricao_estadual: Option<String>,
    pub inscricao_municipal: Option<String>,
    pub inscricao_suframa: Option<String>,
    pub email: Option<String>,
    pub logradouro: Option<String>,
    pub numero: Option<String>,
    pub complemento: Option<String>,
    pub bairro: Option<String>,
    pub codigo_municipio: Option<String>,
    pub municipio: Option<String>,
    pub uf: Option<String>,
    pub cep: Option<String>,
    pub codigo_pais: Option<String>,
    pub pais: Option<String>,
    pub telefone: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_recipients)]
pub struct NewNfeRecipient {
    pub tenant_id: String,
    pub tipo_pessoa: String,
    pub cnpj: Option<String>,
    pub cpf: Option<String>,
    pub id_estrangeiro: Option<String>,
    pub razao_social: String,
    pub nome_fantasia: Option<String>,
    pub inscricao_estadual: Option<String>,
    pub inscricao_municipal: Option<String>,
    pub inscricao_suframa: Option<String>,
    pub email: Option<String>,
    pub logradouro: Option<String>,
    pub numero: Option<String>,
    pub complemento: Option<String>,
    pub bairro: Option<String>,
    pub codigo_municipio: Option<String>,
    pub municipio: Option<String>,
    pub uf: Option<String>,
    pub cep: Option<String>,
    pub codigo_pais: Option<String>,
    pub pais: Option<String>,
    pub telefone: Option<String>,
}

#[derive(AsChangeset, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_recipients)]
pub struct UpdateNfeRecipient {
    pub nome_fantasia: Option<String>,
    pub inscricao_estadual: Option<String>,
    pub inscricao_municipal: Option<String>,
    pub inscricao_suframa: Option<String>,
    pub email: Option<String>,
    pub logradouro: Option<String>,
    pub numero: Option<String>,
    pub complemento: Option<String>,
    pub bairro: Option<String>,
    pub codigo_municipio: Option<String>,
    pub municipio: Option<String>,
    pub uf: Option<String>,
    pub cep: Option<String>,
    pub codigo_pais: Option<String>,
    pub pais: Option<String>,
    pub telefone: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}
