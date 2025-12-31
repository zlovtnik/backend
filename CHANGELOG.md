# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-12-30

### Added
- New `functional` feature in the main crate to enable advanced functional programming capabilities.
- `rcs-functional` crate integrated as a workspace member.
- `Default` implementation for `Tenant` model.

### Changed
- **BREAKING**: `unified_pagination` is now feature-gated under the `functional` feature. It is enabled by default, but users disabling default features must explicitly enable `functional`.
- **BREAKING**: `LazyPipeline` refactored for homogeneous stages. The `O` generic has been removed from the struct; `map` now returns a new pipeline instance.
- **BREAKING**: `ParallelMetrics::efficiency` changed from `f64` to `Option<f64>` to better represent cases where a sequential baseline is missing.
- **BREAKING**: `state_transitions::apply_transition` now returns `Result<S, StateTransitionError>` instead of panicking on invalid transitions.
- Standardized memory estimation across `LazyPipeline` and `ParallelIteratorExt` using the formula `(input + output) * size`.
- Improved `MessagePack` fallback logic in `response_transformers`.

### Fixed
- Duplicated assertion in `immutable_state.rs`.
- Import shadowing in `query_composition.rs`.
- Scoping issues in `immutable_state.rs`.

## [0.1.0] - 2025-01-01
- Initial release with multi-tenant Actix Web and JWT support.
