# Identity Core — Registration Decisions

> Status: partial product decision record; implementation is blocked until the open decisions below are approved.

## Approved registration policy

1. Public registration is available only for Business Owner and Investor.
2. The registrant chooses one permanent primary role, then exactly one initial contact method:
   - Email: a one-time code is delivered to the supplied email address.
   - Phone: a one-time code is delivered through WhatsApp.
3. Email and phone are alternatives. Both are not required during initial registration.
4. The account remains pending verification until the selected channel's code is successfully verified.
5. The selected contact is normalized and uniquely reserved. A contact already associated with another account cannot be reused to create or merge an account.
6. The non-selected contact may be added later, but must be verified independently before being treated as verified or used for security-sensitive recovery.
7. Phone numbers use E.164 normalization. WhatsApp delivery is a server-side provider integration; credentials must never enter source control or frontend configuration.
8. OTPs are never stored or logged in plaintext. Use keyed hashes, expiry, one-time use, bounded attempts, resend cooldown, and destination/account/network rate limits. Delivery failure never marks a contact verified.
9. Registration and recovery responses must not reveal whether a contact is already associated with an account.

## Role and identity invariants

- Each client account has exactly one immutable primary role: owner or investor.
- A person who wants the other role must register a separate account using a different, eligible, unlinked contact identity.
- Admin accounts are provisioned internally and cannot use public registration.
- Identity linking must prove control of both the existing account and the new identity. Matching email text alone is never sufficient.
- No account merge or role mutation is permitted through registration, OAuth, support, or routine Admin operations.

## Proposed registration lifecycle

1. User selects Owner or Investor.
2. User selects Email or Phone/WhatsApp.
3. Server validates and normalizes the contact, applies abuse limits, and creates a pending registration/challenge.
4. Server sends a one-time code through the selected delivery channel.
5. User submits the code; the server validates expiry, attempt budget, challenge state, purpose, and destination binding.
6. In one transaction, consume the challenge, mark the selected contact verified, and activate the account only when all account-creation conditions are satisfied.
7. User continues to role-specific onboarding.

Pending-registration expiry and cleanup must be specified before migration. Avoid leaving duplicate active pending registrations or reserving contacts indefinitely.

## Data-model implications (proposal only)

The eventual schema should distinguish normalized email and E.164 phone values, the initial registration contact method, independent verification timestamps, challenge delivery channel (email or WhatsApp), challenge purpose and destination binding, keyed code hash, attempts, expiry, consumption, and pending-registration expiry.

Do not infer that the user supplied both contact methods. Do not create a migration until the complete Identity Core decisions and schema review gate are approved.

## Decisions still required

### Google OAuth
Existing product documents mention Google sign-in and OAuth registration. It is not decided whether Google may create a new account, only link after an email- or WhatsApp-verified account exists, or be removed from the MVP. Until approved, do not implement Google-based account creation or linking.

### Password and login model
The existing design mentions email/password and phone OTP login. Confirm whether phone/WhatsApp is only a registration verification channel or also a passwordless login method. Password requirements and recovery behavior must match the chosen model.

### Account lifecycle
Before migration, define pending-registration expiry and cleanup; suspension, closure, and deletion states; retention of identity/contact records and audit history after closure; whether and under what verified process a closed account's email/phone can ever be reused; and recovery when a user loses access to the selected contact channel.

### Provider operations
Select email and WhatsApp Business/API providers. Confirm sender/template requirements, delivery monitoring, cost controls, retries, and operational ownership. Keep credentials server-side.

### Security parameters
Approve OTP lifetime, attempt cap, resend cooldown, rate-limit windows, pending-registration lifetime, session idle/absolute expiry, and password hashing parameters through security/configuration review. Do not invent production constants in code or UI.

## Implementation gate

Identity Core migrations and runtime endpoints may begin only after Google OAuth behavior, phone OTP login behavior, pending-registration/account-closure/identity-retention policy, provider integration boundaries, schema concurrency/expiry/cleanup/privacy review, and test plans are approved.
