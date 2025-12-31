use actix_web::{get, post, web, HttpResponse};
use serde::{Deserialize, Serialize};
use log::info;

use crate::error::ServiceError;
use crate::functional::chain_builder::ChainBuilder;

/// Represents a single step in the functional transformation pipeline for visualization
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransformationStep {
    /// Step identifier (e.g., "filter", "map", "reduce")
    pub operation: String,
    /// Description of what this step does
    pub description: String,
    /// Input data for this step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Vec<i32>>,
    /// Output data after this step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Vec<i32>>,
    /// Processing time in milliseconds
    pub duration_ms: u64,
}

/// Complete visualization of a functional pipeline execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineVisualization {
    /// Unique ID for this pipeline execution
    pub pipeline_id: String,
    /// Name of the pipeline demonstration
    pub name: String,
    /// Complete sequence of transformation steps
    pub steps: Vec<TransformationStep>,
    /// Initial input data
    pub initial_data: Vec<i32>,
    /// Final result after all transformations
    pub final_result: Vec<i32>,
    /// Total execution time in milliseconds
    pub total_duration_ms: u64,
    /// Timestamp when the pipeline was executed
    pub executed_at: String,
}

/// Request to demonstrate a filter operation
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FilterDemoRequest {
    /// Input data to filter
    pub data: Vec<i32>,
    /// Filter condition (e.g., "even" or "greater_than_5")
    pub condition: String,
}

/// Request to demonstrate a map transformation
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapDemoRequest {
    /// Input data to transform
    pub data: Vec<i32>,
    /// Transformation type (e.g., "double", "square", "increment")
    pub transformation: String,
}

/// Request to demonstrate a chain of operations
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChainDemoRequest {
    /// Input data for the chain
    pub data: Vec<i32>,
    /// Sequence of operations to apply
    pub operations: Vec<ChainOperation>,
}

/// A single operation in a chain
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChainOperation {
    /// Type of operation: "filter", "map", "take", "skip"
    pub op_type: String,
    /// Parameter for the operation (e.g., filter condition, map function, count)
    #[serde(default)]
    pub param: String,
}

/// Request to demonstrate state transitions
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StateTransitionDemoRequest {
    /// Initial state value
    pub initial_value: i32,
    /// Sequence of transitions to apply
    pub transitions: Vec<StateMutation>,
}

/// A state mutation operation
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StateMutation {
    /// Operation type: "increment", "decrement", "multiply", "divide", "set"
    pub mutation_type: String,
    /// Parameter value for the mutation
    pub value: i32,
}

/// Demonstrates a simple filter operation with step-by-step visualization
///
/// This endpoint shows how data flows through a filter operation,
/// useful for understanding functional filtering patterns.
///
/// # Examples
///
/// ```
/// POST /api/functional/demo/filter
/// {
///   "data": [1, 2, 3, 4, 5, 6],
///   "condition": "even"
/// }
/// ```
#[post("/demo/filter")]
async fn demo_filter(req: web::Json<FilterDemoRequest>) -> Result<HttpResponse, ServiceError> {
    info!("Processing filter demonstration with data: {:?}", req.data);

    // Validate condition before processing
    match req.condition.as_str() {
        "even" | "odd" | "greater_than_5" | "less_than_10" => {},
        _ => return Err(ServiceError::bad_request(
            &format!("Invalid condition '{}'. Supported: even, odd, greater_than_5, less_than_10", req.condition)
        )),
    }

    let input_data = req.data.clone();
    let start = std::time::Instant::now();

    // Apply filter based on condition
    let filtered = ChainBuilder::from_vec(input_data.clone())
        .filter(|x| match req.condition.as_str() {
            "even" => x % 2 == 0,
            "odd" => x % 2 != 0,
            "greater_than_5" => *x > 5,
            "less_than_10" => *x < 10,
            _ => unreachable!(), // Safe due to validation above
        })
        .collect::<Vec<_>>();

    let duration = start.elapsed().as_millis() as u64;

    let step = TransformationStep {
        operation: "filter".to_string(),
        description: format!("Filter by condition: {}", req.condition),
        input: Some(input_data.clone()),
        output: Some(filtered.clone()),
        duration_ms: duration,
    };

    let visualization = PipelineVisualization {
        pipeline_id: uuid::Uuid::new_v4().to_string(),
        name: format!("Filter Demo ({})", req.condition),
        steps: vec![step],
        initial_data: input_data,
        final_result: filtered,
        total_duration_ms: duration,
        executed_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(HttpResponse::Ok().json(visualization))
}

/// Demonstrates a map transformation operation with step-by-step visualization
///
/// Shows how data transforms when applying a mapping function to each element.
///
/// # Examples
///
/// ```
/// POST /api/functional/demo/map
/// {
///   "data": [1, 2, 3, 4, 5],
///   "transformation": "double"
/// }
/// ```
#[post("/demo/map")]
async fn demo_map(req: web::Json<MapDemoRequest>) -> Result<HttpResponse, ServiceError> {
    info!("Processing map demonstration with data: {:?}", req.data);

    // Validate transformation before processing
    match req.transformation.as_str() {
        "double" | "square" | "increment" | "decrement" | "absolute" => {},
        _ => return Err(ServiceError::bad_request(
            &format!("Invalid transformation '{}'. Supported: double, square, increment, decrement, absolute", req.transformation)
        )),
    }

    let input_data = req.data.clone();
    let start = std::time::Instant::now();

    // Apply mapping based on transformation type
    let mapped = ChainBuilder::from_vec(input_data.clone())
        .map(|x| match req.transformation.as_str() {
            "double" => x.saturating_mul(2),
            "square" => x.saturating_mul(x),
            "increment" => x.saturating_add(1),
            "decrement" => x.saturating_sub(1),
            "absolute" => x.abs(),
            _ => unreachable!(), // Safe due to validation above
        })
        .collect::<Vec<_>>();

    let duration = start.elapsed().as_millis() as u64;

    let step = TransformationStep {
        operation: "map".to_string(),
        description: format!("Map transformation: {}", req.transformation),
        input: Some(input_data.clone()),
        output: Some(mapped.clone()),
        duration_ms: duration,
    };

    let visualization = PipelineVisualization {
        pipeline_id: uuid::Uuid::new_v4().to_string(),
        name: format!("Map Demo ({})", req.transformation),
        steps: vec![step],
        initial_data: input_data,
        final_result: mapped,
        total_duration_ms: duration,
        executed_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(HttpResponse::Ok().json(visualization))
}

/// Demonstrates a complex chain of operations with full step-by-step visualization
///
/// Shows how multiple operations compose together, with each step shown separately
/// for animation/flux visualization.
///
/// # Examples
///
/// ```
/// POST /api/functional/demo/chain
/// {
///   "data": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
///   "operations": [
///     {"op_type": "filter", "param": "even"},
///     {"op_type": "map", "param": "double"},
///     {"op_type": "take", "param": "3"}
///   ]
/// }
/// ```
#[post("/demo/chain")]
async fn demo_chain(req: web::Json<ChainDemoRequest>) -> Result<HttpResponse, ServiceError> {
    info!("Processing chain demonstration with data: {:?}", req.data);

    let input_data = req.data.clone();
    let start = std::time::Instant::now();
    let mut steps = Vec::new();
    let mut current_data = input_data.clone();

    // Process each operation in sequence
    for operation in &req.operations {
        let step_start = std::time::Instant::now();
        let output_data: Vec<i32>;

        match operation.op_type.as_str() {
            "filter" => {
                output_data = ChainBuilder::from_vec(current_data.clone())
                    .filter(|x| match operation.param.as_str() {
                        "even" => x % 2 == 0,
                        "odd" => x % 2 != 0,
                        _ => true,
                    })
                    .collect();

                steps.push(TransformationStep {
                    operation: "filter".to_string(),
                    description: format!("Filter by: {}", operation.param),
                    input: Some(current_data.clone()),
                    output: Some(output_data.clone()),
                    duration_ms: step_start.elapsed().as_millis() as u64,
                });
            }
            "map" => {
                output_data = ChainBuilder::from_vec(current_data.clone())
                    .map(|x| match operation.param.as_str() {
                        "double" => x * 2,
                        "square" => x * x,
                        "increment" => x + 1,
                        _ => x,
                    })
                    .collect();

                steps.push(TransformationStep {
                    operation: "map".to_string(),
                    description: format!("Map: {}", operation.param),
                    input: Some(current_data.clone()),
                    output: Some(output_data.clone()),
                    duration_ms: step_start.elapsed().as_millis() as u64,
                });
            }
            "take" => {
                let count: usize = operation.param.parse()
                    .map_err(|_| ServiceError::bad_request(
                        &format!("Invalid count for 'take' operation: '{}' must be a positive integer", operation.param)
                    ))?;
                output_data = ChainBuilder::from_vec(current_data.clone())
                    .take(count)
                    .collect();

                steps.push(TransformationStep {
                    operation: "take".to_string(),
                    description: format!("Take first {} elements", count),
                    input: Some(current_data.clone()),
                    output: Some(output_data.clone()),
                    duration_ms: step_start.elapsed().as_millis() as u64,
                });
            }
            "skip" => {
                let count: usize = operation.param.parse()
                    .map_err(|_| ServiceError::bad_request(
                        &format!("Invalid count for 'skip' operation: '{}' must be a positive integer", operation.param)
                    ))?;
                output_data = ChainBuilder::from_vec(current_data.clone())
                    .skip(count)
                    .collect();

                steps.push(TransformationStep {
                    operation: "skip".to_string(),
                    description: format!("Skip first {} elements", count),
                    input: Some(current_data.clone()),
                    output: Some(output_data.clone()),
                    duration_ms: step_start.elapsed().as_millis() as u64,
                });
            }
            _ => {
                return Err(ServiceError::bad_request("Unknown operation type"))
            }
        }

        current_data = output_data;
    }

    let total_duration = start.elapsed().as_millis() as u64;

    let visualization = PipelineVisualization {
        pipeline_id: uuid::Uuid::new_v4().to_string(),
        name: "Complex Chain Demonstration".to_string(),
        steps,
        initial_data: input_data,
        final_result: current_data,
        total_duration_ms: total_duration,
        executed_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(HttpResponse::Ok().json(visualization))
}

/// Demonstrates immutable state transitions with full history
///
/// Shows how state evolves through a series of immutable transitions,
/// useful for understanding functional state management.
///
/// # Examples
///
/// ```
/// POST /api/functional/demo/state-transitions
/// {
///   "initial_value": 10,
///   "transitions": [
///     {"mutation_type": "increment", "value": 5},
///     {"mutation_type": "multiply", "value": 2},
///     {"mutation_type": "decrement", "value": 3}
///   ]
/// }
/// ```
#[post("/demo/state-transitions")]
async fn demo_state_transitions(
    req: web::Json<StateTransitionDemoRequest>,
) -> Result<HttpResponse, ServiceError> {
    info!(
        "Processing state transition demonstration starting from: {}",
        req.initial_value
    );

    let start = std::time::Instant::now();
    let mut steps = Vec::new();
    let mut current_state = req.initial_value;
    let initial_value = req.initial_value;

    // Apply each transition and record the step
    for mutation in &req.transitions {
        let step_start = std::time::Instant::now();
        let previous_state = current_state;

        current_state = match mutation.mutation_type.as_str() {
            "increment" => current_state.saturating_add(mutation.value),
            "decrement" => current_state.saturating_sub(mutation.value),
            "multiply" => current_state.saturating_mul(mutation.value),
            "divide" => {
                if mutation.value == 0 {
                    return Err(ServiceError::bad_request("Division by zero is not allowed"));
                }
                current_state / mutation.value
            }
            "set" => mutation.value,
            _ => return Err(ServiceError::bad_request(
                &format!("Unknown mutation type: '{}'", mutation.mutation_type)
            )),
        };

        steps.push(TransformationStep {
            operation: mutation.mutation_type.clone(),
            description: format!(
                "{} by {} ({} → {})",
                mutation.mutation_type, mutation.value, previous_state, current_state
            ),
            input: Some(vec![previous_state]),
            output: Some(vec![current_state]),
            duration_ms: step_start.elapsed().as_millis() as u64,
        });
    }

    let total_duration = start.elapsed().as_millis() as u64;

    let visualization = PipelineVisualization {
        pipeline_id: uuid::Uuid::new_v4().to_string(),
        name: "State Transition Demonstration".to_string(),
        steps,
        initial_data: vec![initial_value],
        final_result: vec![current_state],
        total_duration_ms: total_duration,
        executed_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(HttpResponse::Ok().json(visualization))
}

/// Get available functional operations and their descriptions
///
/// Lists all available functional demonstration endpoints and their parameters.
#[get("/operations")]
async fn get_available_operations() -> Result<HttpResponse, ServiceError> {
    let operations = serde_json::json!({
        "operations": [
            {
                "name": "Filter",
                "endpoint": "POST /api/functional/demo/filter",
                "description": "Filter data based on a condition",
                "conditions": ["even", "odd", "greater_than_5", "less_than_10"]
            },
            {
                "name": "Map",
                "endpoint": "POST /api/functional/demo/map",
                "description": "Transform each element using a function",
                "transformations": ["double", "square", "increment", "decrement", "absolute"]
            },
            {
                "name": "Chain",
                "endpoint": "POST /api/functional/demo/chain",
                "description": "Apply a sequence of operations",
                "operations": ["filter", "map", "take", "skip"]
            },
            {
                "name": "State Transitions",
                "endpoint": "POST /api/functional/demo/state-transitions",
                "description": "Demonstrate immutable state transitions",
                "mutations": ["increment", "decrement", "multiply", "divide", "set"]
            }
        ]
    });

    Ok(HttpResponse::Ok().json(operations))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App, http::StatusCode};

    #[actix_web::test]
    async fn test_demo_filter_even_success() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api/functional")
                    .service(demo_filter)
                    .service(demo_map)
                    .service(demo_chain)
                    .service(demo_state_transitions)
                    .service(get_available_operations),
            ),
        )
        .await;

        let req_body = FilterDemoRequest {
            data: vec![1, 2, 3, 4, 5, 6],
            condition: "even".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/api/functional/demo/filter")
            .set_json(&req_body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify response body contains expected result
        let body = test::read_body(resp).await;
        let visualization: PipelineVisualization = serde_json::from_slice(&body)
            .expect("Failed to parse response as PipelineVisualization");
        
        assert_eq!(visualization.initial_data, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(visualization.final_result, vec![2, 4, 6]);
        assert_eq!(visualization.steps.len(), 1);
        assert_eq!(visualization.steps[0].operation, "filter");
        assert_eq!(visualization.steps[0].output, Some(vec![2, 4, 6]));
    }

    #[actix_web::test]
    async fn test_demo_filter_invalid_condition() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api/functional")
                    .service(demo_filter)
                    .service(demo_map)
                    .service(demo_chain)
                    .service(demo_state_transitions)
                    .service(get_available_operations),
            ),
        )
        .await;

        let req_body = FilterDemoRequest {
            data: vec![1, 2, 3, 4, 5, 6],
            condition: "invalid_condition".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/api/functional/demo/filter")
            .set_json(&req_body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_demo_map_double_success() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api/functional")
                    .service(demo_filter)
                    .service(demo_map)
                    .service(demo_chain)
                    .service(demo_state_transitions)
                    .service(get_available_operations),
            ),
        )
        .await;

        let req_body = MapDemoRequest {
            data: vec![1, 2, 3, 4, 5],
            transformation: "double".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/api/functional/demo/map")
            .set_json(&req_body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify response body
        let body = test::read_body(resp).await;
        let visualization: PipelineVisualization = serde_json::from_slice(&body)
            .expect("Failed to parse response as PipelineVisualization");
        
        assert_eq!(visualization.initial_data, vec![1, 2, 3, 4, 5]);
        assert_eq!(visualization.final_result, vec![2, 4, 6, 8, 10]);
        assert_eq!(visualization.steps.len(), 1);
        assert_eq!(visualization.steps[0].operation, "map");
    }

    #[actix_web::test]
    async fn test_demo_map_invalid_transformation() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api/functional")
                    .service(demo_filter)
                    .service(demo_map)
                    .service(demo_chain)
                    .service(demo_state_transitions)
                    .service(get_available_operations),
            ),
        )
        .await;

        let req_body = MapDemoRequest {
            data: vec![1, 2, 3],
            transformation: "invalid_transform".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/api/functional/demo/map")
            .set_json(&req_body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_demo_chain_success() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api/functional")
                    .service(demo_filter)
                    .service(demo_map)
                    .service(demo_chain)
                    .service(demo_state_transitions)
                    .service(get_available_operations),
            ),
        )
        .await;

        let req_body = ChainDemoRequest {
            data: vec![1, 2, 3, 4, 5, 6],
            operations: vec![
                ChainOperation {
                    op_type: "filter".to_string(),
                    param: "even".to_string(),
                },
                ChainOperation {
                    op_type: "map".to_string(),
                    param: "double".to_string(),
                },
            ],
        };

        let req = test::TestRequest::post()
            .uri("/api/functional/demo/chain")
            .set_json(&req_body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify response body
        let body = test::read_body(resp).await;
        let visualization: PipelineVisualization = serde_json::from_slice(&body)
            .expect("Failed to parse response as PipelineVisualization");
        
        assert_eq!(visualization.initial_data, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(visualization.final_result, vec![4, 8, 12]); // even then double
        assert_eq!(visualization.steps.len(), 2);
    }

    #[actix_web::test]
    async fn test_demo_chain_invalid_count() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api/functional")
                    .service(demo_filter)
                    .service(demo_map)
                    .service(demo_chain)
                    .service(demo_state_transitions)
                    .service(get_available_operations),
            ),
        )
        .await;

        let req_body = ChainDemoRequest {
            data: vec![1, 2, 3, 4, 5],
            operations: vec![
                ChainOperation {
                    op_type: "take".to_string(),
                    param: "not_a_number".to_string(),
                },
            ],
        };

        let req = test::TestRequest::post()
            .uri("/api/functional/demo/chain")
            .set_json(&req_body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_demo_state_transitions_success() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api/functional")
                    .service(demo_filter)
                    .service(demo_map)
                    .service(demo_chain)
                    .service(demo_state_transitions)
                    .service(get_available_operations),
            ),
        )
        .await;

        let req_body = StateTransitionDemoRequest {
            initial_value: 10,
            transitions: vec![
                StateMutation {
                    mutation_type: "increment".to_string(),
                    value: 5,
                },
                StateMutation {
                    mutation_type: "multiply".to_string(),
                    value: 2,
                },
            ],
        };

        let req = test::TestRequest::post()
            .uri("/api/functional/demo/state-transitions")
            .set_json(&req_body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify response body
        let body = test::read_body(resp).await;
        let visualization: PipelineVisualization = serde_json::from_slice(&body)
            .expect("Failed to parse response as PipelineVisualization");
        
        assert_eq!(visualization.initial_data, vec![10]);
        assert_eq!(visualization.final_result, vec![30]); // 10 + 5 = 15, then 15 * 2 = 30
        assert_eq!(visualization.steps.len(), 2);
    }

    #[actix_web::test]
    async fn test_demo_state_transitions_division_by_zero() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api/functional")
                    .service(demo_filter)
                    .service(demo_map)
                    .service(demo_chain)
                    .service(demo_state_transitions)
                    .service(get_available_operations),
            ),
        )
        .await;

        let req_body = StateTransitionDemoRequest {
            initial_value: 10,
            transitions: vec![
                StateMutation {
                    mutation_type: "divide".to_string(),
                    value: 0,
                },
            ],
        };

        let req = test::TestRequest::post()
            .uri("/api/functional/demo/state-transitions")
            .set_json(&req_body)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_web::test]
    async fn test_get_available_operations() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api/functional")
                    .service(demo_filter)
                    .service(demo_map)
                    .service(demo_chain)
                    .service(demo_state_transitions)
                    .service(get_available_operations),
            ),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/functional/operations")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Verify response contains operation metadata
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("\"Filter\""));
        assert!(body_str.contains("\"Map\""));
        assert!(body_str.contains("\"Chain\""));
        assert!(body_str.contains("\"State Transitions\""));
    }
}
