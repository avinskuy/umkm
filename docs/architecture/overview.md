# Architecture Overview

## Direction

USahapintar starts as a **modular monolith**. The frontend and backend are separate deployable applications, while the backend keeps domain modules inside one service. This minimizes operational complexity while preserving clear boundaries.

## Applications

### Frontend

- React and TypeScript.
- Feature-oriented organization.
- Shared UI primitives live separately from domain-specific components.
- Component styles are colocated and scoped (CSS Modules where appropriate).
- Global styles are limited to reset, tokens, and truly global rules.

### Backend

- Rust and Axum.
- Domain-oriented modules with explicit API, application, domain, and infrastructure responsibilities as complexity requires.
- PostgreSQL is the planned primary relational database.
- SQLx is the planned database access layer.
- Authorization is enforced server-side.

## Initial domain boundaries

- Identity and authentication
- Owner profiles and businesses
- Investor profiles
- Opportunities and partnership requests
- Connections and messaging
- Memberships
- Business data and verification
- AI advisory and intelligence
- Moderation and administration

These are logical boundaries, not a requirement to create empty modules in advance. Add a module when it has real behavior and a clear contract.

## Identity Core decisions

The approved and unresolved registration, contact-verification, and account-lifecycle decisions are tracked in [Identity Core — Registration Decisions](identity-core-registration-decisions.md). No Identity Core migration should be created until its implementation gate is satisfied.

## Database schema

The proposed relational model, integrity constraints, privacy boundaries, and migration sequence are documented in [PostgreSQL Database Schema Design](database-schema-design.md). It remains a proposal until the first migration review gate is completed.

## Data access and privacy

Business owners control which business information is shared. Investor-facing data must be explicitly permitted and must not include customer-level transaction details. Data should carry provenance, reporting period, update time, and verification status.

## Architecture constraints

- No microservices for the initial product.
- No generic `utils` or `common` dumping grounds.
- Avoid circular dependencies between features.
- Shared code must have multiple real consumers or a clear platform-level responsibility.
- Keep transport concerns out of domain rules.
- Do not trust frontend role or ownership claims.