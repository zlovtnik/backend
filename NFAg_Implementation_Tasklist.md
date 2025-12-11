# NFAg Database and CRUD API Implementation Task List

## Overview

This task list outlines the steps to integrate NFAg (Nota Fiscal Eletrônica da Água) schemas into the existing Actix Web REST API project. The project uses Diesel ORM with PostgreSQL and supports multi-tenancy. We'll create database tables based on the provided XSD schemas and implement CRUD APIs for each entity.

## Phase 1: Schema Analysis and Database Design

### 1.1 Analyze XSD Schemas

- [x] Review all XSD files in `PL_NFAg_1.00d/` directory
- [x] Identify main entities: NFAg, consSitNFAg, consStatServNFAg, eventoNFAg, retNFAg, etc.
- [x] Map complex types to database tables (TNFAg, TConsSitNFAg, TEvento, etc.)
- [x] Identify relationships between entities (one-to-many, many-to-many)
- [x] Document data types, constraints, and validations from XSD restrictions
- [x] Consider multi-tenancy: add tenant_id to all tables

### 1.2 Design Database Schema

- [x] Create ER diagram showing table relationships
- [x] Define primary keys, foreign keys, and indexes
- [x] Map XSD data types to PostgreSQL types (e.g., xs:string → VARCHAR, xs:decimal → DECIMAL)
- [x] Plan for XML storage (store full XML or normalized data?)
- [x] Design audit fields (created_at, updated_at, created_by, updated_by)

## Phase 2: Database Implementation

### 2.1 Create Diesel Migrations

- [x] Generate migration files for each new table
- [x] Implement up/down migrations with proper constraints
- [x] Add tenant_id foreign key to all tables
- [x] Create indexes for performance (especially on tenant_id and commonly queried fields)
- [x] Run migrations and verify schema

### 2.2 Update Diesel Models

- [x] Regenerate models using `diesel print-schema`
- [x] Add custom derives and relationships
- [x] Implement validation structs using serde/validator
- [x] Add tenant isolation logic to models
- [x] Add XML content validation to prevent empty XML strings

## Phase 3: CRUD API Implementation

### 3.1 Core NFAg Entity

- [ ] Create NFAg model and schema structs
- [ ] Implement CRUD handlers: create, read, update, delete
- [ ] Add validation middleware for XSD compliance
- [ ] Implement tenant filtering in all queries

### 3.2 Consultation Entities

- [ ] Implement consSitNFAg (situation consultation) CRUD
- [ ] Implement consStatServNFAg (service status consultation) CRUD
- [ ] Add proper response formatting

### 3.3 Event Management

- [ ] Implement eventoNFAg (events) CRUD
- [ ] Handle different event types (cancelamento, encerramento, etc.)
- [ ] Implement event sequencing and validation

### 3.4 Response Entities

- [ ] Implement retNFAg (return responses) CRUD
- [ ] Implement retConsSitNFAg and retConsStatServNFAg CRUD
- [ ] Add status tracking and history

## Phase 4: API Routes and Middleware

### 4.1 Route Configuration

- [ ] Add routes for all CRUD operations
- [ ] Group routes by entity (e.g., `/api/nfag`, `/api/consSitNFAg`)
- [ ] Implement RESTful URL patterns
- [ ] Add OpenAPI/Swagger documentation

### 4.2 Authentication and Authorization

- [ ] Integrate with existing JWT authentication
- [ ] Add role-based permissions for NFAg operations
- [ ] Implement tenant context middleware
- [ ] Add audit logging for all operations

### 4.3 Validation and Error Handling

- [ ] Implement XSD-based validation for incoming data
- [ ] Add comprehensive error responses
- [ ] Handle XML parsing and generation
- [ ] Implement proper HTTP status codes

## Phase 5: Testing and Validation

### 5.1 Unit Tests

- [ ] Test all CRUD operations
- [ ] Test validation logic
- [ ] Test tenant isolation
- [ ] Test error scenarios

### 5.2 Integration Tests

- [ ] Test full API workflows
- [ ] Test XML serialization/deserialization
- [ ] Test multi-tenant scenarios
- [ ] Performance testing with large datasets

### 5.3 Documentation

- [ ] Update API documentation
- [ ] Add examples for each endpoint
- [ ] Document XSD compliance
- [ ] Create migration guide

## Phase 6: Deployment and Monitoring

### 6.1 Database Migration

- [ ] Plan production database migration strategy
- [ ] Backup existing data
- [ ] Test migration in staging environment
- [ ] Monitor migration performance

### 6.2 Monitoring and Logging

- [ ] Add application metrics
- [ ] Implement structured logging
- [ ] Set up alerts for critical operations
- [ ] Add health checks

## Blockers/Next Steps

### Critical Unfinished Tasks

#### Phase 3 CRUD API Implementation

- **Owner**: Backend Developer
- **ETA**: 1-2 weeks
- **Description**: Implement CRUD handlers for all NFAg entities with proper tenant isolation and validation
- **Blocker for**: Production deployment (core functionality not available)

**Phase 3 (CRUD API Implementation) is now unblocked** and can proceed immediately. Validation structs, tenant isolation, and XML content validation have been completed.

### Recently Completed

- [x] Added XML content validation to all NFAg Create*Request and Update*Request structs
- [x] Implemented #[validate(length(min = 1))] on xml_content fields to prevent empty XML
- [x] Added TODO comments for future XML parsing validation
- [x] Verified compilation with cargo check (no errors)

## Key Considerations

- **Multi-tenancy**: All tables must include tenant_id and queries must filter by tenant
- **XML Handling**: Decide whether to store normalized data or full XML documents
- **Performance**: Add proper indexing, especially for tenant-scoped queries
- **Compliance**: Ensure implementation matches XSD specifications exactly
- **Security**: Implement proper authentication and authorization
- **Scalability**: Design for high-volume NFAg processing

## Dependencies to Add

- [ ] XML parsing library (e.g., quick-xml or serde-xml-rs) - **READY**: Validation structs prepared for XML parsing
- [ ] XSD validation library if needed
- [ ] Additional Diesel features for complex queries
- [ ] Testing libraries for XML validation

## Estimated Timeline

- Phase 1: 1-2 weeks (analysis and design) ✅ **COMPLETED**
- Phase 2: 1 week (database implementation) ✅ **COMPLETED**
- Phase 3: 2-3 weeks (CRUD APIs) 🔄 **IN PROGRESS**
- Phase 4: 1 week (routes and middleware)
- Phase 5: 1-2 weeks (testing)
- Phase 6: 1 week (deployment)

Total estimated time: 7-12 weeks depending on complexity and team size.
