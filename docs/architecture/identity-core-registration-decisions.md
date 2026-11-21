# Identity Core — Registration Decisions

> Status: approved product-policy baseline for Identity Core; detailed technical design, provider selection, and security review remain required before implementation.

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

Pending registration expires after **30 minutes** if required contact verification is not completed. Expiry and cleanup must be atomic and idempotent, safely release eligible pending contact reservations, and never delete an activated account. Prevent duplicate active pending registrations and reservation races.

## Data-model implications (proposal only)

The eventual schema should distinguish normalized email and E.164 phone values, the initial registration contact method, independent verification timestamps, challenge delivery channel (email or WhatsApp), challenge purpose and destination binding, keyed code hash, attempts, expiry, consumption, and pending-registration expiry.

Do not infer that the user supplied both contact methods. Do not create a migration until the complete Identity Core decisions and schema review gate are approved.

## Approved Identity Core policies

### Google OAuth and password model

Google OAuth is retained for sign-in and registration. A new registrant must choose exactly one immutable client role (Owner or Investor) before OAuth begins; the role is bound to server-side OAuth state. Google-only registration does not require a separate local password. A new Google registrant must still choose and verify one separate registration contact (email or phone through WhatsApp) before activation; a Google-asserted email does not waive this requirement.

Email- or phone-based registration requires a local password. Normal password login uses the registered email or phone plus that password. Registration OTPs are only for contact verification/account activation and anti-bot protection; they are not login credentials and cannot be reused for password recovery. A future local-password setup flow for Google-only accounts remains a separate product decision if needed.

Existing linked Google identities authenticate only their existing account. Never auto-link by matching email. Linking requires an authenticated session and fresh proof of control of the new identity, or an explicitly approved secure recovery process. Validate state, nonce, issuer, audience, signature, expiry, exact redirect URI, provider subject, and replay protection. Admin identities are never created through public OAuth.

### OTP and registration lifecycle

- OTP validity: **10 minutes**.
- Maximum verification attempts: **5 failed attempts per verification session**.
- After the fifth failure, lock that verification session for **1 hour**. Resending or requesting a replacement code must not reset or bypass this lock.
- Resend cooldown: **60 seconds**.
- Maximum delivery: **5 sends per hour per destination/contact**, with additional adaptive limits by IP/network, account, purpose, and suspicious activity.
- Issuing a new OTP immediately invalidates the prior code for the same purpose and destination.
- A new code does not clear the verification-session failure lock or delivery rate limits.
- All counters, expiry, invalidation, and cooldown enforcement are server-side and concurrency-safe. Client countdowns are presentation only.
- OTPs are six-digit numeric codes for the initial baseline, generated cryptographically, stored only as keyed hashes, never logged in plaintext, bound to a purpose and destination, and consumed atomically once.
- Delivery failure never marks a contact verified. Responses must not reveal whether a contact is already registered.
- Pending registration lifetime is **30 minutes**, independently of the 10-minute OTP lifetime. Expiry cleanup must be idempotent and must never delete an activated account.

### Session management

- Multiple concurrent devices are allowed.
- Provide an Active Sessions page in Security Settings, individual session revocation, and logout-all-devices.
- Session revocation is enforced server-side; suspicious sessions may be revoked based on risk signals.
- Initial duration baseline: **7-day idle timeout** and **30-day absolute timeout**.
- “Remember this device” may enable a persistent browser session but never bypasses either timeout or server-side revocation. Closing the browser alone does not terminate a deliberately persistent session.
- Use Secure, HttpOnly cookies and an appropriate SameSite/CSRF strategy. Sensitive actions may require fresh authentication. Manual logout revokes immediately.

### Account lifecycle

- Temporary deactivation and permanent deletion are separate user-initiated flows.
- Temporary deactivation hides the public profile and restricts normal access while retaining account data and immutable role. Reactivation requires login and security verification.
- Permanent deletion requires explicit confirmation and fresh authentication, followed by a **30-day waiting period**. During that period the account is hidden/restricted and the user may cancel with fresh security verification.
- After the waiting period, deletion processing begins. Legal/security-required records and backups follow a transparent, category-specific retention policy; do not promise immediate erasure from backups.
- Contact reuse after closure/deletion is not automatic; define a verified, abuse-resistant retention and release procedure before implementing it.

### Account recovery

Support distinct flows for **forgot password** and **forgot email**, using previously verified recovery email and/or recovery phone (WhatsApp-delivered verification) where available. Recovery contacts are optional during registration, can be added after activation, and must be independently verified before use. Encourage setup during onboarding without blocking activation.

- Forgot password: verify ownership through an eligible recovery method before allowing a password reset. Registration OTPs cannot be reused as reset credentials.
- Forgot email: after successful verification of a recovery contact linked to the account, the user may see the full primary email address. Never reveal it before verification.
- Use uniform unauthenticated responses, rate limits, notifications, and risk assessment to prevent account enumeration and abuse.
- Recovery is layered and risk-based. Consider trusted devices, prior sessions, previously verified recovery factors, suspicious activity, and recent recovery-factor changes. Use protective delays or additional checks when justified.
- If the user loses access to both primary and recovery contacts, accept an alternative recovery request and assess trusted devices, prior sessions, and other ownership signals. If automated evidence is insufficient, escalate to manual review by specially authorized support staff. Delay or reject recovery when risk is too high. Notify the user through available safe channels and audit each action.
- Manual review must use least-privilege access, documented evidence criteria, dual control for exceptional/high-risk overrides where appropriate, and immutable audit records. Support staff must never change the immutable primary role through recovery.
- Post-recovery controls are adaptive: revoke suspicious sessions, notify the user, and require additional verification for sensitive changes when risk warrants it.

### Provider operations and remaining technical decisions

Email and WhatsApp Business/API providers are not yet selected. Keep provider adapters replaceable, credentials server-side, and define delivery monitoring, retries, cost controls, sender/template requirements, operational ownership, and incident handling before production. Also finalize password hashing parameters, provider configuration, retention schedules, Admin MFA/recovery, and operational thresholds through security review. Do not invent production credentials or put secrets in source control.

## Implementation gate

Identity Core migrations and runtime endpoints may begin after this approved policy baseline is translated into reviewed schema constraints, provider boundaries are defined, concurrency/expiry/cleanup/privacy design is reviewed, and the test plan is approved. Provider selection, exact operational thresholds, retention schedules, and Google-only local-password setup remain open technical/product items. Password is mandatory for email/phone registration; Google OAuth registration does not require a local password. Phone/WhatsApp OTP is not a login method. All policies remain revisable through explicit product/security review.
