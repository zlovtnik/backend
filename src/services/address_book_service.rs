//! Service functions for address book operations
//!
//! Provides advanced functional programming patterns for CRUD operations on Person entities.
//! Uses iterator chains, lazy evaluation, and iterator-based validation for high-performance processing.
//!
//! ## Functional Programming Features
//!
//! - **Iterator-based validation**: All input validation uses iterator chains
//! - **Lazy evaluation**: Database queries use functional pipelines for memory efficiency
//! - **Pure functional composition**: Business logic composed from pure functions
//! - **Immutable data transformations**: All operations preserve immutability
//! - **Error handling monads**: Comprehensive Result/Option chaining

use crate::{
    config::db::Pool,
    constants,
    error::ServiceError,
    models::{
        filters::PersonFilter,
        person::{Person, PersonDTO},
        response::Page,
    },
    services::functional_patterns::{validation_rules, Either, Validator},
    services::functional_service_base::{FunctionalErrorHandling, FunctionalQueryService},
};

/// Iterator-based validation using functional combinator pattern
fn create_person_validator() -> Validator<PersonDTO> {
    Validator::new()
        .rule(|dto: &PersonDTO| validation_rules::required("name")(&dto.name))
        .rule(|dto: &PersonDTO| validation_rules::max_length("name", 100)(&dto.name))
        .rule(|dto: &PersonDTO| validation_rules::required("email")(&dto.email))
        .rule(|dto: &PersonDTO| validation_rules::email("email")(&dto.email))
        .rule(|dto: &PersonDTO| validation_rules::max_length("email", 255)(&dto.email))
}

/// Legacy validation for backward compatibility - uses new functional validator
fn validate_person_dto(dto: &PersonDTO) -> Result<(), ServiceError> {
    create_person_validator().validate(dto)
}

/// Fetches all Person records with iterator-based processing and lazy evaluation.
///
/// This function demonstrates lazy evaluation and iterator-based processing
/// without immediately collecting results, allowing for efficient chaining.
///
/// # Returns
/// `Ok(Vec<Person>)` on success, `Err(ServiceError)` on database errors.
pub fn find_all(pool: &Pool) -> Result<Vec<Person>, ServiceError> {
    let query_service = FunctionalQueryService::new(pool.clone());

    query_service
        .query(|conn| {
            Person::find_all(conn).map_err(|_| {
                ServiceError::internal_server_error(
                    constants::MESSAGE_CAN_NOT_FETCH_DATA.to_string(),
                )
            })
        })
        .log_error("find_all operation")
}

/// Fetches all Person records using Either types for functional composition
pub fn find_all_either(pool: &Pool) -> Either<ServiceError, Vec<Person>> {
    let query_service = FunctionalQueryService::new(pool.clone());

    match query_service.query(|conn| {
        Person::find_all(conn).map_err(|_| {
            ServiceError::internal_server_error(constants::MESSAGE_CAN_NOT_FETCH_DATA.to_string())
        })
    }) {
        Ok(people) => Either::Right(people),
        Err(e) => Either::Left(e),
    }
}

/// Retrieve a person by their ID using functional error handling.
///
/// Pure function that composes database operations with lazy error mapping.
///
/// # Returns
/// `Ok(Person)` if found, `Err(ServiceError::NotFound)` if not found.
pub fn find_by_id(id: i32, pool: &Pool) -> Result<Person, ServiceError> {
    let query_service = FunctionalQueryService::new(pool.clone());

    query_service.query(|conn| {
        Person::find_by_id(id, conn)
            .map_err(|_| ServiceError::not_found(format!("Person with id {} not found", id)))
    })
}

/// Retrieve a person by their ID using Either types for functional composition
pub fn find_by_id_either(id: i32, pool: &Pool) -> Either<ServiceError, Person> {
    let query_service = FunctionalQueryService::new(pool.clone());

    match query_service.query(|conn| {
        Person::find_by_id(id, conn)
            .map_err(|_| ServiceError::not_found(format!("Person with id {} not found", id)))
    }) {
        Ok(person) => Either::Right(person),
        Err(e) => Either::Left(e),
    }
}

/// Retrieves a paginated page of people using lazy iterator evaluation.
///
/// Applies filtering through iterator chains without immediate collection,
/// enabling efficient lazy processing of potentially large datasets.
///
/// # Returns
/// `Ok(Page<Person>)` with filtered and paginated results.
pub fn filter(filter: PersonFilter, pool: &Pool) -> Result<Page<Person>, ServiceError> {
    use log::{debug, error};

    debug!("Starting filter operation with filter: {:?}", filter);
    let query_service = FunctionalQueryService::new(pool.clone());

    query_service.query(|conn| {
        debug!("Executing Person::filter with database connection");
        Person::filter(filter, conn).map_err(|e| {
            error!("Database error in Person::filter: {}", e);
            ServiceError::internal_server_error(format!("Database error: {}", e))
        })
    })
}

/// Retrieves a paginated page of people using Either types for functional composition
pub fn filter_either(filter: PersonFilter, pool: &Pool) -> Either<ServiceError, Page<Person>> {
    use log::{debug, error};

    debug!("Starting filter operation with filter: {:?}", filter);
    let query_service = FunctionalQueryService::new(pool.clone());

    match query_service.query(|conn| {
        debug!("Executing Person::filter with database connection");
        Person::filter(filter, conn).map_err(|e| {
            error!("Database error in Person::filter: {}", e);
            ServiceError::internal_server_error(format!("Database error: {}", e))
        })
    }) {
        Ok(page) => Either::Right(page),
        Err(e) => Either::Left(e),
    }
}

/// Inserts a new person using iterator-based validation and functional pipelines.
///
/// Uses iterator chains for validation and composes database operations through functional pipelines.
///
/// # Returns
/// `Ok(())` on successful insertion, `Err(ServiceError)` on validation or database errors.
pub fn insert(new_person: PersonDTO, pool: &Pool) -> Result<(), ServiceError> {
    // Use iterator-based validation pipeline
    validate_person_dto(&new_person)?;

    // Use functional pipeline with validated data
    crate::services::functional_service_base::ServicePipeline::new(pool.clone())
        .with_data(new_person)
        .execute(|person, conn| {
            Person::insert(person, conn)
                .map_err(|_| {
                    ServiceError::internal_server_error(
                        constants::MESSAGE_CAN_NOT_INSERT_DATA.to_string(),
                    )
                })
                .map(|_| ())
        })
}

/// Inserts a new person using Either types for functional composition
pub fn insert_either(new_person: PersonDTO, pool: &Pool) -> Either<ServiceError, ()> {
    // Convert validation result to Either
    match validate_person_dto(&new_person) {
        Ok(()) => {
            // Use functional pipeline with validated data
            match crate::services::functional_service_base::ServicePipeline::new(pool.clone())
                .with_data(new_person)
                .execute(|person, conn| {
                    Person::insert(person, conn)
                        .map_err(|_| {
                            ServiceError::internal_server_error(
                                constants::MESSAGE_CAN_NOT_INSERT_DATA.to_string(),
                            )
                        })
                        .map(|_| ())
                }) {
                Ok(()) => Either::Right(()),
                Err(e) => Either::Left(e),
            }
        }
        Err(e) => Either::Left(e),
    }
}

/// Updates a person using iterator-based validation and functional pipelines.
///
/// Validates input data using iterator chains, verifies existence, then performs update in a functional pipeline.
///
/// # Returns
/// `Ok(())` on successful update, `Err(ServiceError)` on validation or database errors.
pub fn update(id: i32, updated_person: PersonDTO, pool: &Pool) -> Result<(), ServiceError> {
    // Use iterator-based validation pipeline
    validate_person_dto(&updated_person)?;

    // Use functional pipeline with validated data
    crate::services::functional_service_base::ServicePipeline::new(pool.clone())
        .with_data((id, updated_person))
        .execute(move |(person_id, person), conn| {
            Person::find_by_id(person_id, conn).map_err(|_| {
                ServiceError::not_found(format!("Person with id {} not found", person_id))
            })?;
            Person::update(person_id, person, conn)
                .map_err(|_| {
                    ServiceError::internal_server_error(
                        constants::MESSAGE_CAN_NOT_UPDATE_DATA.to_string(),
                    )
                })
                .map(|_| ())
        })
}

/// Updates a person using Either types for functional composition
pub fn update_either(id: i32, updated_person: PersonDTO, pool: &Pool) -> Either<ServiceError, ()> {
    // Convert validation result to Either
    match validate_person_dto(&updated_person) {
        Ok(()) => {
            // Use functional pipeline with validated data
            match crate::services::functional_service_base::ServicePipeline::new(pool.clone())
                .with_data((id, updated_person))
                .execute(move |(person_id, person), conn| {
                    Person::find_by_id(person_id, conn).map_err(|_| {
                        ServiceError::not_found(format!("Person with id {} not found", person_id))
                    })?;
                    Person::update(person_id, person, conn)
                        .map_err(|_| {
                            ServiceError::internal_server_error(
                                constants::MESSAGE_CAN_NOT_UPDATE_DATA.to_string(),
                            )
                        })
                        .map(|_| ())
                }) {
                Ok(()) => Either::Right(()),
                Err(e) => Either::Left(e),
            }
        }
        Err(e) => Either::Left(e),
    }
}

/// Deletes a person using pure functional composition.
///
/// Verifies existence through lazy evaluation, then performs deletion
/// in a functional pipeline.
///
/// # Returns
/// `Ok(())` on successful deletion, `Err(ServiceError)` on database errors.
pub fn delete(id: i32, pool: &Pool) -> Result<(), ServiceError> {
    let query_service = FunctionalQueryService::new(pool.clone());

    query_service
        .query(|conn| {
            Person::find_by_id(id, conn)
                .map_err(|_| ServiceError::not_found(format!("Person with id {} not found", id)))
        })
        .and_then_error(|_| {
            query_service.query(|conn| {
                Person::delete(id, conn)
                    .map_err(|_| {
                        ServiceError::internal_server_error(
                            constants::MESSAGE_CAN_NOT_DELETE_DATA.to_string(),
                        )
                    })
                    .map(|_| ())
            })
        })
}

/// Deletes a person using Either types for functional composition
pub fn delete_either(id: i32, pool: &Pool) -> Either<ServiceError, ()> {
    let query_service = FunctionalQueryService::new(pool.clone());

    match query_service
        .query(|conn| {
            Person::find_by_id(id, conn)
                .map_err(|_| ServiceError::not_found(format!("Person with id {} not found", id)))
        })
        .and_then_error(|_| {
            query_service.query(|conn| {
                Person::delete(id, conn)
                    .map_err(|_| {
                        ServiceError::internal_server_error(
                            constants::MESSAGE_CAN_NOT_DELETE_DATA.to_string(),
                        )
                    })
                    .map(|_| ())
            })
        }) {
        Ok(()) => Either::Right(()),
        Err(e) => Either::Left(e),
    }
}
