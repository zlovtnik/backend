use once_cell::sync::OnceCell;

use crate::{
    error::ServiceError,
    models::nfe_document::{NewNfeDocument, UpdateNfeDocument},
    services::functional_patterns::{validation_rules, Validator},
};

/// Validator for creating new NFE documents
pub fn new_nfe_validator() -> Validator<NewNfeDocument> {
    Validator::new()
        .rule(|dto: &NewNfeDocument| validation_rules::required("tenant_id")(&dto.tenant_id))
        .rule(|dto: &NewNfeDocument| validation_rules::max_length("tenant_id", 36)(&dto.tenant_id))
        .rule(|dto: &NewNfeDocument| validation_rules::required("nfe_id")(&dto.nfe_id))
        .rule(|dto: &NewNfeDocument| validation_rules::max_length("nfe_id", 50)(&dto.nfe_id))
        .rule(|dto: &NewNfeDocument| validation_rules::required("serie")(&dto.serie))
        .rule(|dto: &NewNfeDocument| validation_rules::max_length("serie", 3)(&dto.serie))
        .rule(|dto: &NewNfeDocument| validation_rules::required("numero")(&dto.numero))
        .rule(|dto: &NewNfeDocument| validation_rules::max_length("numero", 9)(&dto.numero))
        .rule(|dto: &NewNfeDocument| {
            if dto.valor_total <= rust_decimal::Decimal::ZERO {
                Err(ServiceError::bad_request(
                    "valor_total must be greater than zero",
                ))
            } else {
                Ok(())
            }
        })
        .rule(|dto: &NewNfeDocument| {
            if dto.valor_produtos <= rust_decimal::Decimal::ZERO {
                Err(ServiceError::bad_request(
                    "valor_produtos must be greater than zero",
                ))
            } else {
                Ok(())
            }
        })
}

/// Validator for updating NFE documents
pub fn update_nfe_validator() -> Validator<UpdateNfeDocument> {
    Validator::new()
        .rule(|dto: &UpdateNfeDocument| {
            dto.status
                .as_ref()
                .map_or(Ok(()), |status| {
                    validation_rules::min_length("status", 1)(status)
                })
        })
        .rule(|dto: &UpdateNfeDocument| {
            dto.status
                .as_ref()
                .map_or(Ok(()), |status| {
                    validation_rules::max_length("status", 20)(status)
                })
        })
}

/// Validate a NewNfeDocument
pub fn validate_new_nfe(dto: &NewNfeDocument) -> Result<(), ServiceError> {
    static NEW_NFE_VALIDATOR: OnceCell<Validator<NewNfeDocument>> = OnceCell::new();
    NEW_NFE_VALIDATOR
        .get_or_init(new_nfe_validator)
        .validate(dto)
}

/// Validate an UpdateNfeDocument
pub fn validate_update_nfe(dto: &UpdateNfeDocument) -> Result<(), ServiceError> {
    static UPDATE_NFE_VALIDATOR: OnceCell<Validator<UpdateNfeDocument>> = OnceCell::new();
    UPDATE_NFE_VALIDATOR
        .get_or_init(update_nfe_validator)
        .validate(dto)
}
