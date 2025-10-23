use crate::schema::nfe_emitters;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Identifiable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_emitters)]
pub struct NfeEmitter {
    pub id: i32,
    pub tenant_id: String,
    pub cnpj: String,
    pub cpf: Option<String>,
    pub razao_social: String,
    pub nome_fantasia: Option<String>,
    pub inscricao_estadual: Option<String>,
    pub inscricao_estadual_subst_tributario: Option<String>,
    pub inscricao_municipal: Option<String>,
    pub cnae: Option<String>,
    pub regime_tributario: String,
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_emitters)]
pub struct NewNfeEmitter {
    pub tenant_id: String,
    pub cnpj: Option<String>,
    pub cpf: Option<String>,
    pub razao_social: String,
    pub nome_fantasia: Option<String>,
    pub inscricao_estadual: Option<String>,
    pub inscricao_estadual_subst_tributario: Option<String>,
    pub inscricao_municipal: Option<String>,
    pub cnae: Option<String>,
    pub regime_tributario: Option<String>,
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

#[diesel(treat_none_as_null = false)]
#[derive(AsChangeset, Serialize, Deserialize, Debug)]
#[diesel(table_name = nfe_emitters)]
pub struct UpdateNfeEmitter {
    pub nome_fantasia: Option<String>,
    pub inscricao_estadual: Option<String>,
    pub inscricao_estadual_subst_tributario: Option<String>,
    pub inscricao_municipal: Option<String>,
    pub cnae: Option<String>,
    pub regime_tributario: Option<String>,
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
    pub updated_at: Option<NaiveDateTime>,
}
