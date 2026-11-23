# Admin Authorization & Permission System

## Model

USahapintar uses immutable RBAC grants for the canonical roles `business_owner`, `investor`, and the single `admin` account. ABAC is evaluated for each protected request using server-derived resource ownership, active state, visibility, connection, explicit consent, data classification, purpose, and Admin case scope. Frontend visibility and client-supplied role/ownership claims are never authorization inputs.

## Enforcement contract

- Authentication middleware must run before permission middleware.
- Resource repositories must attach `AuthorizationResource` only after loading the target from PostgreSQL and verifying its current state. Never construct it from request JSON, URL parameters, or client claims.
- Missing resource context, inactive resources, unsupported roles, missing consent, or unavailable audit persistence fail closed.
- Private confidential documents require both a valid connection and explicit sharing consent unless the requester is the owner.
- Highly confidential data is not exposed through ordinary application permissions, including Admin operations.
- Admin permissions are fixed backend capabilities, not sub-roles. Each privileged operation requires an approved purpose and backend-confirmed case/report scope. Admin cannot change primary roles or permanently delete accounts.

## Permission catalog

The immutable catalog is defined in `backend/src/identity/authorization.rs`. It intentionally contains no role-change or permanent-delete capability. Protected routers apply `require_permission` with an `AuthorizationApiState` after the relevant session middleware and attach server-derived resource context for object-scoped permissions.

## Audit

Denied authorization decisions are appended through the existing append-only Identity Core audit writer. Successful Admin privilege decisions are also recorded before the handler executes; audit unavailability blocks the operation. Audit metadata uses the existing database allowlist and must not contain credentials, tokens, document contents, or L3/L4 payloads. The application principal has no audit read/update/delete capability.

## Security tests

Policy unit tests cover cross-owner BOLA/IDOR denial, BFLA role/function restrictions, private-resource connection and consent requirements, classification limits, Admin purpose/scope checks, and missing/inactive resource context. Route-level and PostgreSQL integration tests must accompany each product resource endpoint before it is exposed.
