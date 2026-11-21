# Authentication & Identity — Architecture and Security Design

> Status: proposed technical design; review required before implementation  
> Product reference: `docs/product/public-and-auth-uiux-spec.md`  
> Wireframe reference: `docs/product/low-fidelity-public-auth-wireframes.md`  
> Backend direction: Rust + Axum; PostgreSQL + SQLx is the proposed persistence baseline.

This document defines the first authentication architecture for USahapintar. It is a design contract, not an implemented or security-audited system. Provider choices, operational secrets, retention periods, and final policies must be approved before production use.

## 1. Scope and invariants

### In scope

- Public registration for Business Owner and Investor only.
- Exactly one immutable primary client role (`owner` or `investor`) per account.
- Separate Owner and Investor login entry points.
- Email/password and phone OTP authentication.
- Google OAuth sign-in and carefully controlled account linking.
- Email and phone verification.
- Password recovery.
- Session lifecycle and logout.
- Server-side role and resource authorization.
- Internal Admin provisioning, MFA, and audit trail foundations.

### Out of scope for the first authentication slice

- Public Admin registration.
- Multiple client roles on one account.
- Social login providers other than Google.
- Passwordless email magic links unless separately approved.
- Investment transactions, payment custody, or financial account connections.
- A microservice split; authentication remains part of the modular monolith.

### Non-negotiable invariants

1. The backend is authoritative for identity, primary role, account status, and permissions.
2. Each client account has exactly one primary role, selected at registration; the role is immutable for the lifetime of that account.
3. A person who wants the other client role must create a separate new account using login identities not already linked to an existing account.
4. The frontend never grants access based only on route visibility or local state.
5. An Owner account cannot authenticate through the Investor portal and vice versa.
6. Admin accounts are provisioned internally; no public endpoint can create an Admin.
7. A conversation requires an accepted connection between its participants.
8. Free membership does not block messaging after connection approval.
9. OAuth does not silently change an account's role or merge identities.
10. Passwords, OTPs, access tokens, session secrets, and reset tokens are never written to application logs.
11. Every sensitive mutation is validated against the authenticated user and target resource.
12. The system must not reveal whether an email or phone number belongs to an account through unauthenticated recovery or registration responses.

## 2. Identity and role model

### Primary role

Use a single primary role for the MVP:

- `owner`
- `investor`
- `admin`

The role is assigned during registration for Owner/Investor and is immutable for the lifetime of the account. Admin is assigned only through an internal provisioning workflow. No profile endpoint, support process, administrative migration, or OAuth flow may change an existing account's primary role.

A person who wants to operate as the other client role must create a separate new account. Registration must reject login identities that are already linked to another account without disclosing which account owns them. Releasing an identity from a closed account, if ever supported, must follow a separately defined account-closure, verification, and retention policy; it must never silently transfer an identity or merge accounts.

### Authorization layers

Authorization is evaluated in this order:

1. Is the session valid and not revoked/expired?
2. Is the account active and not suspended/locked?
3. Does the authenticated account have the required primary role?
4. Does the account own or participate in the requested resource?
5. Is the requested action permitted for that resource's current state?
6. Does membership entitlement affect the requested quota/feature? Membership never bypasses ownership or role checks.

Do not implement authorization as scattered frontend conditions. Use shared backend extractors/middleware and domain-level policy functions, with tests.

## 3. Proposed data model

PostgreSQL is the proposed relational store. SQLx migrations should be versioned and reversible where practical. Use UUID primary keys and UTC timestamps. Exact SQL types and indexes are finalized in migrations.

### `users`

| Column | Purpose |
| --- | --- |
| `id` | UUID primary key |
| `full_name` | Display name |
| `email_normalized` | Normalized email, nullable until collected/verified according to registration policy |
| `phone_e164` | Normalized phone number, nullable |
| `password_hash` | Argon2id password hash; nullable for provider-only accounts |
| `primary_role` | Owner, Investor, or Admin |
| `account_status` | Pending verification, active, suspended, or closed |
| `email_verified_at` | Verification timestamp |
| `phone_verified_at` | Verification timestamp |
| `created_at`, `updated_at` | Audit timestamps |
| `closed_at` | Account closure timestamp, if applicable |

Constraints:
- Unique indexes on non-null normalized email and phone.
- Normalize before uniqueness checks.
- Never store plaintext passwords.
- Account status transitions must be explicit and audited.
- Do not expose raw database IDs as authorization secrets.

### `user_identities`

Stores linked authentication providers.

| Column | Purpose |
| --- | --- |
| `id` | UUID primary key |
| `user_id` | Owning user |
| `provider` | e.g. `google` |
| `provider_subject` | Stable provider subject identifier |
| `provider_email` | Optional provider email metadata |
| `created_at`, `last_used_at` | Lifecycle timestamps |

Constraints:
- Unique composite key on (`provider`, `provider_subject`).
- Provider subject, not email text, is the stable identity key.
- Linking requires proof of control of the existing account and the new provider identity.
- Do not auto-link solely because provider emails appear to match.

### `sessions`

Opaque, server-managed sessions.

| Column | Purpose |
| --- | --- |
| `id` | UUID primary key |
| `user_id` | Session owner |
| `token_hash` | Hash of a high-entropy opaque session token |
| `created_at`, `last_seen_at` | Activity timestamps |
| `expires_at` | Absolute expiry |
| `revoked_at` | Revocation timestamp |
| `user_agent_summary` | Optional minimized device hint |
| `ip_prefix_hash` | Optional privacy-preserving abuse signal, subject to retention policy |

Constraints:
- Store only a cryptographic hash of the session token.
- Rotate the token after authentication and privilege changes.
- Revoke sessions on password reset, account suspension, and relevant security events.
- Avoid collecting full IP/device fingerprints unless there is a documented security need and retention policy.

### `verification_challenges`

Supports email and phone verification without storing usable codes.

| Column | Purpose |
| --- | --- |
| `id` | UUID primary key |
| `user_id` | Related user, nullable only for pre-account registration flow if necessary |
| `channel` | Email or phone |
| `destination_hash` | Hash or protected reference to destination |
| `code_hash` | Hash of a high-entropy one-time code |
| `purpose` | Registration verification or contact change |
| `attempt_count` | Failed attempts |
| `expires_at` | Short expiry |
| `consumed_at` | One-time use marker |
| `created_at` | Creation timestamp |

Controls:
- Short expiry, one-time consumption, attempt caps, resend cooldown, and destination/account/IP rate limits.
- Use a keyed hash/HMAC for short numeric OTPs so database disclosure alone does not make offline guessing trivial.
- Never log code values or include them in analytics.
- Invalidate older active challenges when a new challenge is issued for the same purpose/destination.
- Provider delivery failures must not mark a destination as verified.

### `password_reset_tokens`

- Store a hash of a cryptographically random, single-use token.
- Include user ID, expiry, created timestamp, and consumed timestamp.
- Revoke existing sessions after a successful reset according to the security policy.
- Do not reveal account existence in request responses.

### `admin_audit_logs`

- Append-only records for sensitive administrative actions.
- Include actor ID, action, target type/ID, timestamp, outcome, and minimal reason/context.
- Do not store passwords, OTPs, raw session tokens, reset tokens, or unnecessary private content.
- Protect audit log read access separately from ordinary Admin permissions.

### Additional tables only when needed

- `admin_mfa_credentials` for TOTP/passkey credential metadata and recovery state.
- `account_link_requests` if explicit provider linking is included in the first release.
- `login_attempt_buckets` or a Redis-backed limiter if operational traffic requires distributed rate limiting.
- `user_consents` for versioned Terms/Privacy consent records.

Avoid creating unused tables before a concrete feature requires them.

## 4. API contract (proposed)

All routes are versioned under `/api/v1`. JSON request/response shapes should be defined in OpenAPI before implementation. The table below is the route inventory, not final wire-format documentation.

### Registration and verification

| Method | Route | Purpose |
| --- | --- | --- |
| POST | `/auth/register/owner` | Create Owner account in pending-verification state |
| POST | `/auth/register/investor` | Create Investor account in pending-verification state |
| POST | `/auth/verification/email/request` | Request/resend email verification |
| POST | `/auth/verification/email/confirm` | Confirm email challenge |
| POST | `/auth/verification/phone/request` | Request/resend phone OTP |
| POST | `/auth/verification/phone/confirm` | Confirm phone OTP |

There is intentionally no `/auth/register/admin` endpoint.

### Password and session authentication

| Method | Route | Purpose |
| --- | --- | --- |
| POST | `/auth/login/owner` | Authenticate only an Owner account |
| POST | `/auth/login/investor` | Authenticate only an Investor account |
| POST | `/auth/login/phone/request` | Request a phone OTP login challenge, if approved |
| POST | `/auth/login/phone/confirm` | Verify OTP and create a session |
| POST | `/auth/logout` | Revoke current session |
| POST | `/auth/logout-all` | Revoke all sessions for the authenticated account |
| GET | `/auth/session` | Return current authenticated user and safe role/session metadata |
| POST | `/auth/password/forgot` | Request a password reset |
| POST | `/auth/password/reset` | Consume reset token and set a new password |

The login endpoints must check the expected role on the server. A mismatch returns a generic role-portal guidance response without exposing additional account details.

### Google OAuth

| Method | Route | Purpose |
| --- | --- | --- |
| GET | `/auth/oauth/google/start?role=owner` | Begin Owner-context OAuth |
| GET | `/auth/oauth/google/start?role=investor` | Begin Investor-context OAuth |
| GET | `/auth/oauth/google/callback` | Validate provider response and complete or continue the flow |

OAuth state and nonce are generated and validated server-side. The role context is integrity-protected and cannot be changed by an untrusted callback parameter. Use an exact allowlist of redirect URIs.

### Admin authentication

Admin authentication uses a separate internal route group and policy. Proposed routes:
- `POST /admin/auth/login`
- `POST /admin/auth/mfa/verify`
- `POST /admin/auth/logout`
- `GET /admin/auth/session`

Admin account creation is a privileged internal operation, not a public authentication API. Admin authorization is permission-based, not just `primary_role=admin`.

## 5. Session and browser security

### Recommended browser session pattern

Prefer an opaque server-managed session delivered in a cookie:

- `HttpOnly`
- `Secure` in production
- `SameSite=Lax` or stricter where compatible with the OAuth callback design
- Narrow cookie path/domain
- Explicit idle and absolute expiration
- Session rotation after login and privilege-sensitive changes

Avoid storing long-lived bearer tokens in localStorage. If a cross-site architecture requires a different token model, document the trade-offs and add a separate security review.

### CSRF and CORS

- For cookie-authenticated state-changing requests, use CSRF protection appropriate to the deployment topology (token or robust origin/referer strategy).
- Restrict CORS to exact trusted frontend origins; never combine credentialed requests with wildcard origins.
- Validate content type and request origin for sensitive endpoints.
- Keep production origins and OAuth redirect URIs in environment configuration, not hard-coded source.

### Session lifecycle

- Login creates a new session and rotates any pre-auth session state.
- Logout revokes the current session.
- Logout-all revokes all sessions.
- Password reset revokes sessions.
- Account suspension blocks session use even if a session token has not expired.
- Role mismatch never creates a session.
- Sensitive admin actions may require recent re-authentication/MFA.

Exact idle/absolute timeouts must be chosen before production and documented in operations settings.

## 6. Password policy

- Use Argon2id with parameters benchmarked for the deployment environment.
- Use a well-maintained password hashing library; do not design a custom algorithm.
- Enforce a reasonable minimum length and allow long passphrases.
- Do not force arbitrary periodic password changes absent compromise.
- Block known-compromised/common passwords where a safe, privacy-preserving check is available.
- Do not silently truncate passwords.
- Rate-limit authentication attempts and use progressive abuse controls.
- Keep password reset tokens single-use and short-lived.

Exact Argon2id parameters and password policy values belong in configuration/security review, not guessed constants in UI code.

## 7. OTP and verification abuse controls

Apply layered limits:
- Per destination (email/phone).
- Per account.
- Per IP or network signal, with privacy-aware retention.
- Per challenge and purpose.
- Global provider spend/throughput guard.

Also require:
- Short code expiry.
- One-time use.
- Bounded attempts.
- Resend cooldown.
- Challenge invalidation on successful verification or superseding request.
- Uniform responses that do not confirm account existence.
- Monitoring for repeated delivery failures, enumeration patterns, and unusual OTP request volume.

Do not use a client-side countdown as the enforcement mechanism; the server owns expiry and rate limits.

## 8. Google OAuth validation and account linking

On callback, validate at minimum:
- State matches the server-issued one-time state.
- Nonce matches when used by the selected flow.
- Issuer, audience/client ID, signature, and token expiry.
- Exact redirect URI.
- Provider subject is present and stable.
- The requested role context is the same as the initiated flow.

If provider identity is already linked, authenticate the linked account only if its role matches the selected portal. If the identity is new, continue the same role-specific registration flow.

Account linking rules:
1. Require an authenticated session on the existing account.
2. Require fresh proof of control of the provider identity.
3. Show which provider is being linked and ask for confirmation.
4. Enforce uniqueness on provider + subject.
5. Audit link/unlink actions.
6. Never auto-merge two user records based only on matching email strings.
7. Do not allow linking that changes the account's primary role.

## 9. Authorization matrix

| Capability | Public | Owner | Investor | Admin |
| --- | --- | --- | --- | --- |
| View public landing/features | Yes | Yes | Yes | Yes |
| Public registration | Owner/Investor only | — | — | No |
| Owner login portal | No session | Owner only | Reject mismatch | Internal Admin portal only |
| Investor login portal | No session | Reject mismatch | Investor only | Internal Admin portal only |
| Manage own business profile | No | Own resources | No | Only with explicit review permission |
| Publish Owner promotion post | No | Yes, subject to quota/moderation | No | Moderation only |
| Publish partnership-seeking post | No | No | Yes, subject to Investor quota/moderation | Moderation only |
| Discover eligible opportunities | Public preview only | Authenticated | Authenticated | Operational review |
| Request connection | No | Yes | Yes | Not as an ordinary user |
| Accept/reject own connection request | No | Own requests | Own requests | No routine impersonation |
| Chat | No | Accepted connection only | Accepted connection only | No routine private-chat access |
| View Owner private business data | No | Own data | Only fields explicitly shared | Purpose-limited, permissioned review |
| Manage membership | No | Own plan/usage | Own plan/usage | Entitlement administration |
| Provision Admin | No | No | No | Super-admin only |

Admin permissions must be granular (e.g. user review, business verification, post moderation, membership operations, audit read). A broad Admin role alone is not sufficient.

## 10. Error response conventions

Use a consistent API error envelope, for example:

```json
{
  "error": {
    "code": "AUTH_INVALID_CREDENTIALS",
    "message": "Email/nomor atau kata sandi belum sesuai.",
    "request_id": "opaque-correlation-id"
  }
}
```

Requirements:
- Stable machine-readable error codes.
- User-safe messages.
- No stack traces, SQL errors, provider tokens, or secret values.
- Include a correlation/request ID for support.
- Do not return different unauthenticated recovery responses that reveal account existence.
- Validation errors identify safe field-level issues without echoing sensitive values.

## 11. Threat model and required mitigations

| Threat | Required controls |
| --- | --- |
| Credential stuffing | Rate limits, monitoring, safe lockout/progressive controls, compromised-password checks |
| Account enumeration | Uniform registration/recovery/login responses where applicable |
| OTP brute force | Short expiry, keyed hash, attempt cap, resend limits, destination/IP controls |
| Session theft | HttpOnly/Secure cookies, token hashing at rest, rotation, revocation, TLS |
| CSRF | SameSite policy plus CSRF/origin protections for cookie-authenticated mutations |
| OAuth login CSRF | One-time state, nonce, exact redirect URI, strict callback validation |
| OAuth account takeover/linking | Explicit linking proof; never merge on email match alone |
| Privilege escalation | Backend role checks, deny-by-default policies, ownership checks, Admin permission scopes |
| IDOR/resource access | Query resources through authorized ownership/participation scope; negative tests |
| Admin compromise | MFA, least privilege, short sessions, audit logs, re-auth for sensitive actions |
| Secret leakage | Secret manager/environment config, log redaction, no secrets in frontend bundle |
| Abuse of verification provider | Per-destination/global rate limits, spend caps, monitoring |
| Replay of reset/verification tokens | One-time consumption, expiry, hashed storage, atomic updates |

This is a baseline threat model, not a substitute for penetration testing or a formal security assessment.

## 12. Required test plan

### Unit tests

- Password hash/verify behavior.
- Email and phone normalization.
- Role parsing and expected-role comparison.
- Primary role is immutable after account creation; no profile, support, or admin workflow can mutate it.
- Separate registration is required to create an account for the other client role.
- Duplicate email, phone, or provider identities cannot be linked to a second account.
- Verification challenge expiry, attempt limits, and one-time consumption.
- Password reset token expiry and single-use behavior.
- Session expiry and revocation.
- Authorization policy decisions.

### API integration tests

- Owner can register and authenticate through Owner portal.
- Investor can register and authenticate through Investor portal.
- Owner login rejects Investor account and does not create a session.
- Investor login rejects Owner account and does not create a session.
- Public Admin registration does not exist and cannot be invoked.
- Unverified contact behavior matches the approved policy.
- OTP resend and confirmation limits are enforced server-side.
- Recovery response does not reveal account existence.
- Reset token cannot be reused.
- Logout and logout-all revoke sessions.
- Suspended accounts cannot use existing sessions.
- OAuth state mismatch/expired state/invalid audience/invalid issuer are rejected.
- Duplicate provider subject cannot be linked to a second user.
- Account linking requires fresh proof of both identities and cannot change the account's primary role.
- OAuth login with a role context that differs from the linked account is rejected; it must not silently create a second account using the same provider subject.
- Unauthorized user cannot read/update another user's business or profile.
- Non-participant cannot read a conversation.
- Chat access is granted only after a connection is accepted.
- Free membership can chat after an accepted connection.

### Security and operational tests

- Rate limiting is enforced across multiple application instances if horizontally scaled.
- CORS rejects untrusted origins.
- Cookie flags are correct in production configuration.
- Logs redact secrets and personal authentication values.
- Error responses do not expose internal details.
- Admin MFA is required before protected Admin routes.
- Audit logs are written for privileged actions and cannot be modified by ordinary Admin users.

### Frontend tests

- Separate Owner/Investor login pages render the correct copy and destinations.
- Password show/hide is accessible.
- Validation and error messages are associated with fields.
- Submit cannot be triggered repeatedly while pending.
- OTP supports paste and keyboard navigation.
- Registration progress and back/forward behavior are predictable.
- OAuth cancel/failure state is recoverable.
- Mobile layouts do not overflow horizontally.

## 13. Implementation sequence

1. **Dependency reproducibility:** commit `frontend/package-lock.json` and change CI to `npm ci`; keep the current npm compatibility configuration only if the lockfile and CI prove it is still required.
2. **Database baseline:** add PostgreSQL configuration, SQLx setup, migration runner, and test database strategy.
3. **Domain types:** define User, PrimaryRole, AccountStatus, Session, VerificationChallenge, and typed error codes.
4. **Registration:** implement Owner/Investor registration with validation and safe pending state.
5. **Verification:** implement email and phone challenge lifecycle behind provider interfaces.
6. **Password auth:** implement hashing, role-specific login, session cookie, logout, and recovery.
7. **Google OAuth:** add provider integration with state/nonce validation and explicit account linking rules.
8. **Authorization:** add backend role/resource policies and negative tests.
9. **Admin foundation:** provisioning, MFA, permissions, and audit trail.
10. **Hardening:** rate-limit review, security tests, logging review, accessibility, and deployment configuration.

Implement each step as a small pull request. Do not merge until CI, focused integration tests, and the relevant security checks pass.

## 14. Open decisions before implementation

- PostgreSQL hosting and backup/restore approach.
- Email delivery provider and sender/domain verification.
- SMS/OTP provider, cost limits, and Indonesia phone delivery reliability.
- Google OAuth client setup and production callback origins.
- Exact password and session timeout policy.
- Whether phone OTP is a login method at MVP launch or only a verification method.
- Admin MFA method and recovery procedure.
- Consent versioning and retention periods.
- Account closure, data export, and deletion workflow.
- Whether registration requires both email and phone verification before activating the account.

Do not invent provider credentials or put secrets in chat, repository files, screenshots, or frontend environment variables. Use server-side environment/secret management and test with non-production credentials.
