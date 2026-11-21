# Membership & Entitlements Policy

> Status: product policy baseline  
> Scope: MVP authorization, Free/Premium entitlements, and quota enforcement  
> This document defines product rules. It does not claim that membership billing or enforcement is implemented.

## 1. Core principles

1. **Free to build connections, Premium to expand and optimize connections.**
2. Membership never changes a user's primary role or grants another role's capabilities.
3. Free and Premium members can discover eligible opportunities, request connections, and message accepted connections.
4. Premium improves capacity, visibility, discovery, and insight; it must not become a paywall around basic networking.
5. Role permissions, resource ownership, consent, moderation state, and account status are enforced independently of membership.
6. The backend is authoritative for every entitlement and quota decision. Frontend hiding or disabling controls is UX only.

## 2. Separate authorization dimensions

Authorization is the intersection of independent checks:

- **Account status:** whether the account is allowed to use the platform.
- **Primary role:** the account's one immutable client role, `business_owner` or `investor`.
- **Resource permission:** ownership, participant status, explicit sharing consent, and moderation state.
- **Membership entitlement:** whether the plan allows a capability or a higher quota.
- **Usage availability:** whether the account has remaining quota for the relevant period.

A Premium subscription cannot override a role mismatch, suspended account, missing consent, denied resource permission, or moderation restriction.

Each client account has exactly one immutable primary role. A person who wants the other client role must register a separate account with eligible, unlinked identities. Membership upgrades and downgrades never change the role.

## 3. Role capability baseline

| Capability | Public visitor | Business Owner | Investor | Admin |
| --- | --- | --- | --- | --- |
| View public landing, features, and membership information | Yes | Yes | Yes | Yes |
| Register a client account | Choose Owner or Investor | No role switching | No role switching | No public registration |
| Maintain own role-specific profile | No | Own profile/businesses | Own investor profile | Only with explicit operational permission |
| Create Owner promotion/opportunity posts | No | Yes, within plan quota and moderation rules | No | Moderation only |
| Create Investor partnership-seeking posts | No | No | Yes, within plan quota and moderation rules | Moderation only |
| Discover eligible profiles/opportunities | Public preview only | Authenticated fields | Authenticated fields | Purpose-limited operational access |
| Request a connection | No | Yes, within plan quota | Yes, within plan quota | Not as an ordinary member |
| Accept/reject own connection requests | No | Yes | Yes | No routine impersonation |
| Message an accepted connection | No | Yes | Yes | No routine private-chat access |
| View private business information | No | Own data | Only information explicitly shared by the Owner | Purpose-limited, permissioned, audited |
| Manage own membership and usage | No | Yes | Yes | Operational administration only |
| Change a user's primary role | No | No | No | No routine role mutation; role is immutable |

Admin access is permission-scoped and audited. Admin is not a Free/Premium membership tier and must not be treated as a normal client role for entitlement calculations.

## 4. Free and Premium membership

### Free

Free is a complete entry tier for participating in the network:

- Create and maintain the role-appropriate profile.
- Create role-appropriate posts within the Free posting quota.
- Discover eligible members and opportunities using standard discovery.
- Send and receive connection requests within the Free connection-request quota.
- Accept or reject incoming requests.
- Chat after a connection is accepted.
- Use core account, privacy, and safety controls.
- See basic activity and performance information where the product supports it.

### Premium

Premium retains all Free capabilities and adds higher capacity and optimization:

- Higher role-appropriate posting quota.
- Higher connection-request quota.
- Priority visibility or featured promotion, subject to transparent ranking rules and moderation eligibility.
- Richer profile presentation and enhanced profile features.
- Advanced discovery filters.
- More detailed analytics and performance insights.

Premium does not guarantee views, connections, investment, funding, revenue, or business outcomes. Featured placement must be labeled where appropriate and must not bypass moderation or safety ranking.

### Explicitly not Premium-gated

The following remain available to both Free and Premium members, subject to role, consent, safety, and account status:

- Creating an account and completing a role-appropriate profile.
- Core opportunity discovery.
- Sending and receiving connection requests (with different quotas).
- Accepting or rejecting a connection request.
- Messaging after a connection is accepted.
- Blocking, reporting, privacy controls, and account security features.
- Accessing information that another member has explicitly shared with them.

Do not sell direct messaging as a Premium-only feature. Do not allow Premium status to reveal private data that the owner has not shared.

## 5. Role-specific posting

- Business Owners create business promotion/opportunity posts.
- Investors create partnership-seeking / investment-interest posts using the Investor-specific format.
- Both post types are first-class platform capabilities, not conditional role experiments.
- Posting quota is evaluated against the creator's membership entitlements and the applicable post type.
- Membership does not bypass content validation, publication rules, moderation, or suspension.
- Publication state and moderation state remain separate. A published post may subsequently be flagged, reviewed, restricted, or removed under platform policy.

## 6. Connection requests and messaging

- Both Free and Premium members may request connections.
- Membership controls the number of eligible requests a member may initiate within the configured quota period; it does not determine whether a member may connect at all.
- A request must be accepted before a conversation can be created or messaging can begin.
- Once a connection is accepted, messaging is available to both Free and Premium participants.
- Accepted connections and existing conversations are not automatically revoked solely because a member downgrades.
- Blocking, reporting, abuse prevention, account suspension, and other safety controls may restrict connection or messaging regardless of plan.
- Do not charge a separate message quota or require Premium to continue an otherwise permitted conversation in the MVP.

## 7. Entitlement and quota model

Represent entitlements as backend-managed plan data rather than scattered plan-name conditionals.

Suggested entitlement keys:

| Entitlement key | Meaning | Free | Premium |
| --- | --- | --- | --- |
| `posts.owner.monthly_limit` | Owner posts allowed in the configured period | Baseline quota | Higher quota |
| `posts.investor.monthly_limit` | Investor posts allowed in the configured period | Baseline quota | Higher quota |
| `connections.requests.monthly_limit` | New connection requests initiated in the configured period | Baseline quota | Higher quota |
| `discovery.advanced_filters` | Advanced discovery filters | No | Yes |
| `visibility.priority_eligible` | Eligibility for priority/featured visibility | No | Yes |
| `profile.enhanced_features` | Premium profile presentation features | No | Yes |
| `analytics.advanced` | Detailed analytics | No | Yes |

Exact numeric quotas, currency/pricing, billing cycle, and quota reset policy are intentionally not fixed in this document. They require a separate product and operational decision before implementation. Do not infer numeric limits from UI copy or hard-code them in application logic.

### Enforcement requirements

- Resolve the active plan and entitlement values on the server for every quota-sensitive action.
- Count usage against the authenticated account, not a client-supplied user ID.
- Make quota consumption atomic to prevent concurrent requests from exceeding limits.
- Define whether a failed, rejected, deleted, or moderated action consumes quota before implementation; apply one consistent policy and document it.
- Return a stable error code and safe message when a quota is exhausted, with the reset/renewal information only when known.
- Do not reset usage based on the device clock.
- Keep an auditable usage ledger or equivalent reliable counter history for support and dispute investigation.
- Avoid silently deleting or hiding existing content when a plan is downgraded.

## 8. Subscription lifecycle and downgrade behavior

The following behavior is required at the product-policy level:

- A member's current effective plan and entitlement snapshot must be resolved server-side.
- If Premium expires, is cancelled, or becomes inactive, new actions use Free entitlements from the effective downgrade time.
- Existing posts, profiles, accepted connections, and conversations remain available unless separately restricted by moderation, account status, or another platform rule.
- If a member exceeds the Free quota after downgrade, preserve existing content; block only additional quota-consuming actions until capacity becomes available.
- Premium-only presentation and discovery controls may become inactive after downgrade, but must not erase member data.
- Billing failures, grace periods, refunds, proration, and cancellation timing need a separate billing policy before payment implementation.

## 9. Authorization decision order

For a protected action, evaluate in this order:

1. Authenticate the session.
2. Confirm the account is active and eligible for the action.
3. Confirm the immutable primary role permits the action.
4. Confirm ownership, participant status, consent, and resource-level permission.
5. Confirm moderation and safety state permits the action.
6. Resolve the effective membership plan and entitlement.
7. Check remaining quota and atomically reserve/consume usage.
8. Execute the action and record the outcome.

A denial at any earlier step cannot be overridden by a later Premium entitlement.

## 10. Required tests

### Role and resource authorization

- Owner cannot create Investor partnership-seeking posts.
- Investor cannot create Owner promotion posts.
- Owner and Investor cannot access each other's role-specific write actions by changing request payloads or routes.
- Public visitors receive only approved public preview fields.
- Investor cannot access private business fields without explicit Owner sharing consent.
- Admin actions require the relevant permission and are audited.

### Membership and quotas

- Free and Premium can both request connections.
- Free and Premium can both message after an accepted connection.
- Premium has higher configured posting and connection-request limits than Free.
- Free cannot use Premium-only advanced filters or analytics.
- Premium cannot bypass role, ownership, consent, moderation, or account-status checks.
- Concurrent quota-consuming requests cannot exceed the configured limit.
- Quota exhaustion returns a stable, user-safe response.
- Downgrade preserves existing posts, connections, and conversations.
- Downgrade prevents additional actions above the Free quota without deleting existing content.
- Admin membership operations cannot silently alter a member's primary role.

## 11. Decisions required before implementation

These values and policies remain open and must not be guessed in code:

- Numeric Free and Premium post quotas for each role.
- Numeric Free and Premium connection-request quotas.
- Quota period and reset semantics (calendar month versus subscription/billing period).
- Whether unused quota carries over (recommended default: no, but requires approval).
- Exact Premium price, billing provider, tax/refund, grace-period, and cancellation behavior.
- Which profile fields and analytics metrics are Premium-only.
- Ranking formula and labeling for priority/featured visibility.
- Whether a post that is removed for policy violations consumes quota, and how appeals affect usage.
- Whether quota is refunded for platform-side failures or duplicate submissions.

Until these decisions are approved, use configurable placeholders in design work and do not expose invented numbers as product commitments.
