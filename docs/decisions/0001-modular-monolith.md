# ADR 0001: Start with a Modular Monolith

- Status: Accepted
- Date: 2026-11-21

## Context

USahapintar is at the beginning of implementation. The product has several domains, but there is no operational need to deploy each domain independently.

## Decision

Build one Rust backend service organized into explicit domain modules, plus a separate React frontend. Use PostgreSQL as the planned primary database. Keep domain boundaries clear without introducing microservices.

## Consequences

### Positive

- Lower deployment and operational complexity.
- Easier local development and integration testing.
- Domain boundaries can be extracted later if real scaling or ownership needs justify it.

### Trade-offs

- Module boundaries must be maintained through code review and tests.
- A single backend deployment scales as one unit initially.

## Revisit when

Independent scaling, deployment cadence, team ownership, or isolation requirements provide measurable reasons to split a domain.