use crate::models::nfag::Nfag;
use crate::schema::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum TaxIdentifier {
    Cnpj(String),
    Cpf(String),
    Idestrangeiro(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaxIdentifierDto {
    pub r#type: String,
    pub value: String,
}

#[derive(Debug)]
pub enum NfagDestConversionError {
    MissingIdentifier,
    InvalidIdentifierType(String),
}

impl std::fmt::Display for NfagDestConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NfagDestConversionError::MissingIdentifier => {
                write!(f, "NfagDest must have an identifier")
            }
            NfagDestConversionError::InvalidIdentifierType(type_str) => {
                write!(f, "Invalid identifier type: {}", type_str)
            }
        }
    }
}

impl std::error::Error for NfagDestConversionError {}

#[derive(Queryable, Identifiable, Associations, Debug)]
#[diesel(belongs_to(Nfag))]
#[diesel(table_name = nfag_dest)]
pub struct NfagDest {
    pub id: i32,
    pub nfag_id: i32,
    pub cnpj: Option<String>,
    pub cpf: Option<String>,
    pub idestrangeiro: Option<String>,
    pub xnome: Option<String>,
    pub ie: Option<String>,
    pub im: Option<String>,
    pub email: Option<String>,
    pub telefone: Option<String>,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NfagDest {
    pub fn identifier(&self) -> Option<TaxIdentifier> {
        if let Some(cnpj) = &self.cnpj {
            Some(TaxIdentifier::Cnpj(cnpj.clone()))
        } else if let Some(cpf) = &self.cpf {
            Some(TaxIdentifier::Cpf(cpf.clone()))
        } else if let Some(idestrangeiro) = &self.idestrangeiro {
            Some(TaxIdentifier::Idestrangeiro(idestrangeiro.clone()))
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NfagDestDto {
    pub id: i32,
    pub nfag_id: i32,
    pub identifier: TaxIdentifierDto,
    pub xnome: Option<String>,
}

impl TryFrom<NfagDest> for NfagDestDto {
    type Error = NfagDestConversionError;

    fn try_from(dest: NfagDest) -> Result<Self, Self::Error> {
        let identifier = match dest.identifier() {
            Some(TaxIdentifier::Cnpj(value)) => TaxIdentifierDto { r#type: "cnpj".to_string(), value },
            Some(TaxIdentifier::Cpf(value)) => TaxIdentifierDto { r#type: "cpf".to_string(), value },
            Some(TaxIdentifier::Idestrangeiro(value)) => TaxIdentifierDto { r#type: "idestrangeiro".to_string(), value },
            None => return Err(NfagDestConversionError::MissingIdentifier),
        };
        Ok(NfagDestDto {
            id: dest.id,
            nfag_id: dest.nfag_id,
            identifier,
            xnome: dest.xnome,
        })
    }
}

#[derive(Insertable, Debug)]
#[diesel(table_name = nfag_dest)]
pub struct NewNfagDest {
    pub nfag_id: i32,
    pub cnpj: Option<String>,
    pub cpf: Option<String>,
    pub idestrangeiro: Option<String>,
    pub xnome: Option<String>,
    pub ie: Option<String>,
    pub im: Option<String>,
    pub email: Option<String>,
    pub telefone: Option<String>,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewNfagDestDto {
    pub nfag_id: i32,
    pub identifier: TaxIdentifierDto,
    pub xnome: String,
    pub ie: Option<String>,
    pub im: Option<String>,
    pub email: Option<String>,
    pub telefone: Option<String>,
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
}

impl TryFrom<NewNfagDestDto> for NewNfagDest {
    type Error = NfagDestConversionError;

    fn try_from(dto: NewNfagDestDto) -> Result<Self, Self::Error> {
        let (cnpj, cpf, idestrangeiro) = match dto.identifier.r#type.as_str() {
            "cnpj" => (Some(dto.identifier.value), None, None),
            "cpf" => (None, Some(dto.identifier.value), None),
            "idestrangeiro" => (None, None, Some(dto.identifier.value)),
            invalid_type => return Err(NfagDestConversionError::InvalidIdentifierType(invalid_type.to_string())),
        };
        let now = Utc::now();
        Ok(NewNfagDest {
            nfag_id: dto.nfag_id,
            cnpj,
            cpf,
            idestrangeiro,
            xnome: Some(dto.xnome),
            ie: dto.ie,
            im: dto.im,
            email: dto.email,
            telefone: dto.telefone,
            logradouro: dto.logradouro,
            numero: dto.numero,
            complemento: dto.complemento,
            bairro: dto.bairro,
            codigo_municipio: dto.codigo_municipio,
            municipio: dto.municipio,
            uf: dto.uf,
            cep: dto.cep,
            codigo_pais: dto.codigo_pais,
            pais: dto.pais,
            created_at: now,
            updated_at: now,
        })
    }
}