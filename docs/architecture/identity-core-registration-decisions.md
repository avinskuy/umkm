# Identity Core — Registration Decisions

> Status: partial product decision record; implementation is blocked until the open decisions below are approved.

## Approved registration policy

1. Public registration is available only for Business Owner and Investor.
2. The registrant chooses one permanent primary role, then exactly one initial contact method:
   - Email: a one-time code is delivered to the supplied email address.
   - Phone: a one-time code is delivered through WhatsApp.
3. Email and phone are alternatives. Both are not required during initial registration.
4. Email- or phone-based registration requires a password. Registration through Google OAuth does not require the user to create or enter a separate local password.
5. The account remains pending verification until the selected channel's code is successfully verified. Verification is an account-activation and anti-bot control, not a passwordless login method.
6. The selected contact is normalized and uniquely reserved. A contact already associated with another account cannot be reused to create or merge an account.
7. The non-selected contact may be added later, but must be verified independently before being treated as verified or used for security-sensitive recovery.
8. Phone numbers use E.164 normalization. WhatsApp delivery is a server-side provider integration; credentials must never enter source control or frontend configuration.
9. OTPs are never stored or logged in plaintext. Use keyed hashes, expiry, one-time use, bounded attempts, resend cooldown, and destination/account/network rate limits. Delivery failure never marks a contact verified.
10. Registration and recovery responses must not reveal whether a contact is already associated with an account.

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
5. User submits the code; the server validates expiry, attempt budget, challenge state, purpose, and destination binding. This verifies contact ownership and anti-bot eligibility; it does not authenticate a login session.
6. In one transaction, consume the challenge, mark the selected contact verified, and activate the account only when all account-creation conditions are satisfied.
7. User continues to role-specific onboarding.

Pending-registration expiry and cleanup must be specified before migration. Avoid leaving duplicate active pending registrations or reserving contacts indefinitely.

## Data-model implications (proposal only)

The eventual schema should distinguish normalized email and E.164 phone values, the initial registration contact method, independent verification timestamps, challenge delivery channel (email or WhatsApp), challenge purpose and destination binding, keyed code hash, attempts, expiry, consumption, and pending-registration expiry.

Do not infer that the user supplied both contact methods. Do not create a migration until the complete Identity Core decisions and schema review gate are approved.

## Decisions still required

### Google OAuth — retained

Google OAuth remains part of the product for sign-in and registration. A new user must explicitly choose Owner or Investor before starting Google registration; that choice is bound to server-side OAuth state. Google registration does not require a separate local password, but it does not replace mandatory email/WhatsApp contact verification.

- The user must choose Owner or Investor before starting a Google-based registration flow.
- Role context is bound to server-side OAuth state and cannot be changed by callback parameters or client input.
- For a new Google identity, the user must still choose exactly one registration contact: email with a code delivered to that email, OR phone with a code delivered through WhatsApp. A Google-asserted email does not waive this separate code-verification requirement.
- The new account is created only after the selected contact has been verified and all registration requirements are met.
- A previously linked Google identity may sign in only to its existing account. Its immutable primary role determines the correct portal; a role mismatch must never create a second account or change the existing role.
- Never automatically link an existing account merely because Google returns an email matching the account's email. Linking requires an authenticated session plus fresh proof of control of the new identity, or an explicitly approved secure recovery process.
- Admin identities are never created through public Google OAuth.
- Validate OAuth state, nonce, issuer, audience, signature, expiry, and exact redirect URI. Reject duplicate provider subjects and replayed callbacks.

### Password and login model — decided
Email- or phone-based registration requires a local password; normal password login uses the registered email or phone plus that password. Google OAuth registration does not require a separate local password and Google sign-in is used to authenticate that linked identity. A new Google registrant must choose Owner or Investor before OAuth begins; the selected role is bound to server-side state and cannot be supplied or changed by the callback. Email/WhatsApp codes are only for initial account activation/contact verification and anti-bot protection; they are not login credentials and remain required for new Google registrations. Registration OTPs cannot be reused for password recovery. A later local-password setup for a Google-only account is not yet specified.

### Account lifecycle
Before migration, define pending-registration expiry and cleanup; suspension, closure, and deletion states; retention of identity/contact records and audit history after closure; whether and under what verified process a closed account's email/phone can ever be reused; and recovery when a user loses access to the selected contact channel.

### Provider operations
Select email and WhatsApp Business/API providers. Confirm sender/template requirements, delivery monitoring, cost controls, retries, and operational ownership. Keep credentials server-side.

### Security parameters
Approve OTP lifetime, attempt cap, resend cooldown, rate-limit windows, pending-registration lifetime, session idle/absolute expiry, and password hashing parameters through security/configuration review. Do not invent production constants in code or UI.

## Implementation gate

Identity Core migrations and runtime endpoints may begin only after pending-registration/account-closure/identity-retention policy, provider integration boundaries, schema concurrency/expiry/cleanup/privacy review, and test plans are approved. Password is mandatory for email/phone registration; Google OAuth registration does not require a local password. Phone/WhatsApp OTP is not a login method. Google OAuth is retained under the constraints in this document.
