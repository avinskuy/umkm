# Authentication Product Rules

## Account model

- Public accounts have exactly one primary role: `business_owner` or `investor`.
- Public registration requires an explicit role selection.
- An account cannot self-switch its role.
- Admin accounts are provisioned internally and are never offered in public registration.
- The same email identity is globally unique; OAuth linking must not silently create a second role/account.

A person's client role is permanent for that account. If they want to use the other client role, they must register a separate account with eligible identities that are not already linked to another account. No profile, support, Admin, or OAuth flow may convert an existing account's role.

Membership is independent of role. Free and Premium plans do not change role permissions or grant access to resources without the required ownership, consent, and authorization checks. See [Membership & Entitlements Policy](membership-and-entitlements.md).

## Login surfaces

- Business Owner: `/login/owner`
- Investor: `/login/investor`
- Admin: `/admin/login`
- Public registration: `/register`

Separate client login pages are a user-experience distinction. They must use the same backend identity system and enforce the expected role server-side.

## Authentication requirements

- Email verification before enabling protected account capabilities.
- Password hashing with Argon2id.
- Secure, revocable sessions.
- Rate limits for login, password recovery, and verification-code requests.
- Password recovery through short-lived, single-use tokens.
- Google OAuth must bind to a verified identity and preserve the account's existing role.
- Admin MFA is mandatory before production use.

## Authorization

The backend must verify:
1. The session is valid.
2. The account is active.
3. The account role matches the requested client surface.
4. The account is permitted to access the requested resource.

Frontend route guards improve UX but are not a security boundary.

## Privacy

Investors may only see business information the owner has explicitly shared. Customer-level transaction data is not exposed to investors.