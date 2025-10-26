use crate::{
    error::ServiceError,
    models::user::{LoginDTO, SignupDTO, UserDTO},
    services::functional_patterns::{validation_rules, Validator},
};

fn validate_password(password: &String) -> Result<(), ServiceError> {
    let count = password.chars().count();
    if count < 8 {
        Err(ServiceError::bad_request(
            "Password too short (min 8 characters)",
        ))
    } else if count > 64 {
        Err(ServiceError::bad_request(
            "Password too long (max 64 characters)",
        ))
    } else if !password.chars().any(|c| c.is_uppercase()) {
        Err(ServiceError::bad_request(
            "Password must contain at least one uppercase letter",
        ))
    } else if !password.chars().any(|c| c.is_lowercase()) {
        Err(ServiceError::bad_request(
            "Password must contain at least one lowercase letter",
        ))
    } else if !password.chars().any(|c| c.is_numeric()) {
        Err(ServiceError::bad_request(
            "Password must contain at least one number",
        ))
    } else {
        Ok(())
    }
}

pub fn user_validator() -> Validator<UserDTO> {
    Validator::new()
        .rule(|dto: &UserDTO| validation_rules::required("username")(&dto.username))
        .rule(|dto: &UserDTO| validation_rules::min_length("username", 3)(&dto.username))
        .rule(|dto: &UserDTO| validation_rules::max_length("username", 50)(&dto.username))
        .rule(|dto: &UserDTO| validate_password(&dto.password))
        .rule(|dto: &UserDTO| validation_rules::required("email")(&dto.email))
        .rule(|dto: &UserDTO| validation_rules::email("email")(&dto.email))
        .rule(|dto: &UserDTO| validation_rules::max_length("email", 255)(&dto.email))
}

pub fn signup_validator() -> Validator<SignupDTO> {
    Validator::new()
        .rule(|dto: &SignupDTO| validation_rules::required("username")(&dto.username))
        .rule(|dto: &SignupDTO| validation_rules::min_length("username", 3)(&dto.username))
        .rule(|dto: &SignupDTO| validation_rules::max_length("username", 50)(&dto.username))
        .rule(|dto: &SignupDTO| validate_password(&dto.password))
        .rule(|dto: &SignupDTO| validation_rules::required("email")(&dto.email))
        .rule(|dto: &SignupDTO| validation_rules::email("email")(&dto.email))
        .rule(|dto: &SignupDTO| validation_rules::max_length("email", 255)(&dto.email))
        .rule(|dto: &SignupDTO| validation_rules::required("tenant_id")(&dto.tenant_id))
        .rule(|dto: &SignupDTO| validation_rules::min_length("tenant_id", 1)(&dto.tenant_id))
        .rule(|dto: &SignupDTO| validation_rules::max_length("tenant_id", 64)(&dto.tenant_id))
}

pub fn login_validator() -> Validator<LoginDTO> {
    Validator::new()
        .rule(|dto: &LoginDTO| {
            validation_rules::required("username_or_email")(&dto.username_or_email)
        })
        .rule(|dto: &LoginDTO| {
            validation_rules::max_length("username_or_email", 255)(&dto.username_or_email)
        })
        .rule(|dto: &LoginDTO| validation_rules::required("password")(&dto.password))
        .rule(|dto: &LoginDTO| validation_rules::max_length("password", 128)(&dto.password))
        .rule(|dto: &LoginDTO| validation_rules::required("tenant_id")(&dto.tenant_id))
        .rule(|dto: &LoginDTO| validation_rules::min_length("tenant_id", 1)(&dto.tenant_id))
        .rule(|dto: &LoginDTO| validation_rules::max_length("tenant_id", 64)(&dto.tenant_id))
}

pub fn validate_user(dto: &UserDTO) -> Result<(), ServiceError> {
    user_validator().validate(dto)
}

pub fn validate_signup(dto: &SignupDTO) -> Result<(), ServiceError> {
    signup_validator().validate(dto)
}

pub fn validate_login(dto: &LoginDTO) -> Result<(), ServiceError> {
    login_validator().validate(dto)
}
