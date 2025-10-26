use crate::{
    error::ServiceError,
    models::person::PersonDTO,
    services::functional_patterns::{validation_rules, Validator},
};

/// Build a validator for `PersonDTO` enforcing the constraints documented in FP-013 benchmarks.
pub fn person_validator() -> Validator<PersonDTO> {
    Validator::new()
        .rule(|dto: &PersonDTO| validation_rules::required("name")(&dto.name))
        .rule(|dto: &PersonDTO| validation_rules::min_length("name", 2)(&dto.name))
        .rule(|dto: &PersonDTO| validation_rules::max_length("name", 100)(&dto.name))
        .rule(|dto: &PersonDTO| validation_rules::required("email")(&dto.email))
        .rule(|dto: &PersonDTO| validation_rules::email("email")(&dto.email))
        .rule(|dto: &PersonDTO| validation_rules::max_length("email", 255)(&dto.email))
        .rule(|dto: &PersonDTO| validation_rules::required("phone")(&dto.phone))
        .rule(|dto: &PersonDTO| validation_rules::min_length("phone", 10)(&dto.phone))
        .rule(|dto: &PersonDTO| validation_rules::max_length("phone", 20)(&dto.phone))
        .rule(|dto: &PersonDTO| validation_rules::pattern("phone", r"^[0-9()+\-\s]+$")(&dto.phone))
        .rule(|dto: &PersonDTO| validation_rules::required("address")(&dto.address))
        .rule(|dto: &PersonDTO| validation_rules::max_length("address", 500)(&dto.address))
        .rule(|dto: &PersonDTO| validation_rules::range("age", 0, 150)(&dto.age))
}

/// Validate a `PersonDTO` using the reusable validator combinator.
pub fn validate_person(dto: &PersonDTO) -> Result<(), ServiceError> {
    person_validator().validate(dto)
}
