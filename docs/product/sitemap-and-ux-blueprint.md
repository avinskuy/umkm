# USahapintar — Sitemap & UI/UX Blueprint

> Status: product design baseline  
> Scope: responsive web application and PWA  
> Design direction: Minimalist Editorial × Modern Marketplace × SaaS  
> Primary palette: soft white, sage green, forest green, warm neutral accents

This document defines the information architecture and page-level experience before feature implementation. It is a product blueprint, not a claim that the routes or features are already implemented.

## 1. Product model

USahapintar is a membership-based business promotion, intelligence, and networking platform. It helps business owners present opportunities and helps investors discover businesses and partnership opportunities.

It is **not** a goods marketplace or an investment transaction platform. The MVP facilitates discovery, introductions, approved connections, and communication. It does not hold funds, execute investments, or guarantee funding or returns.

### Client roles

- **Business Owner** — creates and manages business profiles, publishes business promotion/opportunity posts, uses Growth Advisor, and builds connections.
- **Investor** — discovers business opportunities, creates and manages partnership-seeking posts using the Investor post format, uses Investment Intelligence, and builds connections.
- **Admin** — internal operational role for verification, moderation, membership, reports, and platform governance. Admin accounts are provisioned internally and are not offered in public registration.

Each client account has exactly one immutable primary client role: `owner` or `investor`. The role is selected during registration and cannot be changed later through profile settings, support requests, account migration, OAuth, or any other self-service or administrative workflow. Anyone who wants to use the other client role must register a separate new account with eligible, unlinked login identities. Owner and Investor experiences use separate login entry points and role-aware authorization. The backend is authoritative for role and resource access. Admin is a separately provisioned internal role and is not a client registration choice.

### Membership model

USahapintar has Free and Premium membership tiers that apply independently of the account's immutable client role. Both tiers retain core discovery, connection requests, connection approval, and messaging after an accepted connection. Premium adds higher posting and connection-request quotas, priority visibility/featured promotion, richer analytics, advanced discovery filters, and enhanced profile features. Membership never grants another role's permissions or access to private information that has not been explicitly shared.

The detailed role-access baseline, entitlement keys, quota enforcement rules, downgrade behavior, and required tests are maintained in [Membership & Entitlements Policy](membership-and-entitlements.md). Exact numeric quotas and commercial terms remain open decisions and must not be invented in UI or implementation.

## 2. Global information architecture

### Public website (no login required)

| Route | Page | Purpose |
| --- | --- | --- |
| `/` | Landing | Explain the platform and direct visitors to register or sign in |
| `/cara-kerja` | How it works | Explain discovery, connection approval, and networking |
| `/fitur` | Features | Present Owner, Investor, and shared platform capabilities |
| `/membership` | Membership | Compare Free and Premium benefits and quotas |
| `/tentang` | About | Explain mission, scope, and platform principles |
| `/pusat-bantuan` | Help center | FAQs and basic guidance |
| `/kontak` | Contact | Official support and inquiry channels |
| `/kebijakan/privasi` | Privacy policy | Explain data collection, use, consent, and retention |
| `/ketentuan` | Terms | Explain platform rules, limitations, and user responsibilities |

Public visitors can understand the service and membership model. The internal discovery feed, private profiles, connections, chat, and AI workspaces require authentication.

### Authentication

| Route | Page | Purpose |
| --- | --- | --- |
| `/masuk/pemilik` | Owner login | Dedicated Owner sign-in entry |
| `/masuk/investor` | Investor login | Dedicated Investor sign-in entry |
| `/daftar` | Registration role choice | Choose Owner or Investor; no public Admin option |
| `/daftar/pemilik` | Owner registration | Create an Owner account and begin business onboarding |
| `/daftar/investor` | Investor registration | Create an Investor account and begin investor onboarding |
| `/verifikasi/email` | Email verification | Verify email address |
| `/verifikasi/telepon` | Phone OTP verification | Verify phone number |
| `/lupa-kata-sandi` | Password recovery | Request a password reset |
| `/atur-kata-sandi` | Set new password | Complete password recovery |

Initial registration requires choosing exactly one contact method: email verified by a one-time code delivered to email, or phone number verified by a one-time code delivered through WhatsApp. Both contact methods are not required at signup; the other may be added and verified later. Google OAuth behavior is pending an explicit product decision. Existing role-specific login entry points remain separate, and the backend rejects a role mismatch with a clear, non-revealing message. Include password visibility, loading/error states, resend cooldown, terms/privacy links, and accessible validation.

Admin uses a separate internal entry at `/admin/login`, with stronger session controls and MFA. It is not linked as a public registration role.

## 3. Authenticated application shell

After login, users enter the application shell for their primary role. The shell should share visual primitives while keeping navigation, dashboard content, and permissions role-specific.

### Shared authenticated routes

| Route | Page | Access |
| --- | --- | --- |
| `/app/notifikasi` | Notifications | Owner, Investor |
| `/app/koneksi` | Connections | Owner, Investor |
| `/app/pesan` | Messages | Owner, Investor; approved connections only |
| `/app/pesan/:conversationId` | Conversation | Conversation participants only |
| `/app/profil` | My profile | Owner, Investor |
| `/app/profil/:userId` | Public-facing member profile | Authenticated users, field-level visibility |
| `/app/pengaturan` | Account settings | Owner, Investor |
| `/app/keanggotaan` | Membership and usage | Owner, Investor |

A connection request must be approved before a conversation can be started. Direct messaging is not a Premium-only feature. Free and Premium users can connect and chat after approval, subject to fair-use and safety limits.

## 4. Business Owner workspace

Suggested route prefix: `/owner`.

| Route | Page | Main content and actions |
| --- | --- | --- |
| `/owner/dashboard` | Owner dashboard | Business summary, profile completeness, post performance, connection activity, membership usage, next actions |
| `/owner/bisnis` | My businesses | List and manage business profiles |
| `/owner/bisnis/baru` | Create business profile | Business identity, sector, location, stage, description, supporting information |
| `/owner/bisnis/:businessId` | Business overview | Business information, verification status, approved metrics, published posts |
| `/owner/postingan` | My posts | Promotion/opportunity posts, status, performance, edit/archive actions |
| `/owner/postingan/baru` | Create promotion post | Publish a business promotion or opportunity with clear evidence and visibility settings |
| `/owner/postingan/:postId` | Post detail and performance | Post content, moderation status, engagement and analytics |
| `/owner/jelajah` | Discover investors and opportunities | Search and filter eligible profiles and partnership posts |
| `/owner/koneksi` | Connection management | Incoming/outgoing requests and accepted connections |
| `/owner/pesan` | Messages | Chat with approved connections |
| `/owner/growth-advisor` | Growth Advisor home | Guided business analysis and saved reports |
| `/owner/growth-advisor/validasi-ide` | Idea validation | Evaluate an early-stage business idea |
| `/owner/growth-advisor/perencanaan` | Business planning | Build a structured business plan |
| `/owner/growth-advisor/keuangan` | Financial planning | HPP, pricing, break-even, and capital-planning worksheets |
| `/owner/growth-advisor/pasar` | Market research | Market, customer, competitor, and regional research |
| `/owner/growth-advisor/pertumbuhan` | Growth strategy | Practical growth priorities and scenarios |
| `/owner/growth-advisor/kesehatan-bisnis` | Business health check | Review business indicators and data completeness |
| `/owner/analitik` | Business analytics | Trends and post/profile performance based on available data |
| `/owner/keanggotaan` | Membership | Plan, usage quotas, and upgrade information |
| `/owner/pengaturan` | Settings | Account, security, privacy, and notification preferences |

Owner onboarding adapts to the business stage: idea only, preparing to start, currently operating, or growing. The UI should ask only for information relevant to the selected stage and allow users to complete their profile progressively.

## 5. Investor workspace

Suggested route prefix: `/investor`.

| Route | Page | Main content and actions |
| --- | --- | --- |
| `/investor/dashboard` | Investor dashboard | Saved opportunities, discovery highlights, connection activity, preference summary |
| `/investor/jelajah` | Opportunity discovery | Search and filter business opportunities |
| `/investor/jelajah/:postId` | Opportunity detail | Owner-approved business information, evidence labels, and connection request |
| `/investor/tersimpan` | Saved opportunities | Bookmarked opportunities and status changes |
| `/investor/postingan` | My partnership posts | Create and manage partnership-seeking posts using the Investor-specific format, subject to membership quotas and moderation |
| `/investor/postingan/baru` | Create partnership post | Create an Investor-format post describing partnership intent, preferred sector, region, and collaboration scope |
| `/investor/koneksi` | Connection management | Requests, accepted connections, and introductions |
| `/investor/pesan` | Messages | Chat with approved connections |
| `/investor/investment-intelligence` | Investment Intelligence home | Guided analysis and saved reports |
| `/investor/investment-intelligence/analisis` | Opportunity analysis | Financial health, profitability, growth, market, and risk review |
| `/investor/investment-intelligence/perbandingan` | Benchmarking | Compare opportunities using available, comparable data |
| `/investor/investment-intelligence/simulasi` | Scenario simulation | Explore assumptions and possible scenarios without promising outcomes |
| `/investor/analitik` | Discovery analytics | Saved-search and discovery activity insights |
| `/investor/keanggotaan` | Membership | Plan, usage quotas, and upgrade information |
| `/investor/pengaturan` | Settings | Account, security, privacy, and notification preferences |

Investor onboarding captures profile information and discovery preferences such as sector, target region, indicative range, horizon, and risk preference. These preferences support discovery and analysis; they must not be presented as a promise of investment suitability or returns.

Investor-facing AI may only use business data the Owner has explicitly approved for sharing. Customer-level transaction details are not exposed.

## 6. Admin workspace

Suggested route prefix: `/admin`. Access is internal and enforced server-side.

| Route | Page | Main content and actions |
| --- | --- | --- |
| `/admin/login` | Admin login | Internal sign-in, MFA, and security notices |
| `/admin/dashboard` | Operations overview | Platform health, pending reviews, reports, and operational metrics |
| `/admin/users` | User management | Search, status, role, verification, and account actions |
| `/admin/users/:userId` | User detail | Profile review, verification history, reports, and moderation actions |
| `/admin/businesses` | Business verification | Review business profiles and submitted evidence |
| `/admin/posts` | Post moderation | Review, approve, reject, hide, or request edits |
| `/admin/reports` | Reports and complaints | Triage reports, investigate, and record outcomes |
| `/admin/memberships` | Membership management | Plans, entitlements, quotas, and subscription status |
| `/admin/connections` | Connection oversight | Investigate abuse reports and relevant safety events; no routine private-chat browsing |
| `/admin/ai-governance` | AI and data governance | Review data-source status, consent, output incidents, and audit metadata |
| `/admin/admins` | Admin accounts | Super-admin provisioning and permission assignment |
| `/admin/audit-logs` | Audit logs | Search security-sensitive administrative actions |
| `/admin/settings` | Platform settings | Controlled configuration and operational settings |

Use least-privilege permissions, MFA, strict session expiry, rate limits, and append-only audit records for sensitive actions. Admin access to private user content must be purpose-limited, permission-checked, and auditable.

## 7. Primary user journeys

### Owner journey

1. Public landing → choose Owner registration.
2. Select Business Owner as the permanent primary role, choose either email or phone/WhatsApp as the registration contact, and verify the selected contact.
3. Complete stage-aware business onboarding.
4. Enter Owner dashboard and complete the business profile.
5. Publish a promotion/opportunity post within membership limits.
6. Discover relevant investors and receive connection requests.
7. Approve a connection → start chat.
8. Use Growth Advisor and analytics with clearly labeled data sources.

### Investor journey

1. Public landing → choose Investor registration.
2. Select Investor as the permanent primary role, choose either email or phone/WhatsApp as the registration contact, and verify the selected contact.
3. Complete investor profile and discovery preferences.
4. Enter Investor dashboard and explore opportunities.
5. Review an opportunity and its data-source/verification labels.
6. Send a connection request → wait for approval.
7. Chat after approval and use Investment Intelligence with Owner-approved data.

### Admin journey

1. Admin is provisioned internally.
2. Sign in through the separate admin portal and complete MFA.
3. Review pending user/business/post verification queues.
4. Moderate reports and manage membership entitlements.
5. Record sensitive actions in the audit log.

## 8. Navigation model

### Desktop

- Public header: brand, How it works, Features, Membership, Help, Sign in, primary Register CTA.
- Owner sidebar: Overview, My Business, Posts, Discover, Connections, Messages, Growth Advisor, Analytics, Membership, Settings.
- Investor sidebar: Overview, Discover, Saved, Partnership Posts, Connections, Messages, Investment Intelligence, Analytics, Membership, Settings.
- Admin sidebar: Overview, Users, Business Review, Post Moderation, Reports, Memberships, AI & Data Governance, Admin Accounts, Audit Logs, Settings.

### Mobile

Use a compact top bar and a role-aware bottom navigation for the most frequent destinations. Place secondary destinations in a clearly grouped drawer. Do not simply shrink the desktop sidebar. Keep labels visible for primary navigation, show active state clearly, and ensure touch targets are comfortable.

## 9. Page layout patterns

Use a small set of consistent patterns instead of inventing a new layout for every route:

1. **Public marketing page** — header, hero, proof/benefit sections, explanatory content, CTA, footer.
2. **Authenticated dashboard** — page heading, contextual action, summary cards, primary work area, recent activity.
3. **Discovery/list page** — heading, search/filter controls, result count, responsive cards/list, empty/loading/error states.
4. **Detail page** — identity/summary, verification and data-source labels, main content, contextual actions, related information.
5. **Form/onboarding page** — short step indicator, grouped fields, inline guidance, validation, save/continue action.
6. **AI workspace** — task selection, structured input, progress state, result sections, assumptions/evidence/limitations, save/export where supported.
7. **Admin review page** — queue filters, review context, evidence panel, decision controls, reason capture, audit metadata.

## 10. Visual and interaction system

### Visual direction

- Soft white is the dominant surface; sage green is the primary accent; forest green is reserved for strong actions and emphasis.
- Use warm beige and pale sage for restrained secondary surfaces.
- Typography should feel editorial and readable, with clear hierarchy and generous line-height.
- Prefer whitespace, clear alignment, and consistent spacing over dense dashboards.
- Use real product imagery or neutral illustrations only where they explain a business category or page purpose. Avoid decorative clutter and excessive gradients.

### Core design tokens (initial)

| Token | Value | Use |
| --- | --- | --- |
| Soft white | `#F8FAF7` | Main background |
| Pale sage | `#E8EDE5` | Soft surfaces |
| Sage green | `#A3B18A` | Accent |
| Forest green | `#344E41` | Primary emphasis |
| Deep charcoal | `#29332D` | Main text |
| Warm beige | `#D8CDBB` | Neutral accent |
| Muted green | `#6B9277` | Secondary accent |

These are starting tokens and can be refined after the first visual page review.

### Required states and accessibility

Every data-driven page must define loading, empty, error, and success states. Forms need visible labels, keyboard support, useful inline errors, and clear focus indicators. Status must never rely on color alone. Charts need text summaries or accessible data alternatives. Respect reduced-motion preferences.

### Trust and data transparency

Display the data status wherever business metrics or AI analysis appear:

- **Verified** — reviewed through an explicit verification process.
- **System-integrated** — received from a connected system.
- **Owner-reported** — entered or declared by the business owner.
- **AI estimate** — inferred or modeled; assumptions must be visible.

Show source, reporting period, last update, consent/visibility, and completeness where applicable. Never let an AI estimate appear indistinguishable from verified financial data.

## 11. Membership experience

Use a shared membership page with role-aware benefit details. The guiding principle is **Free to Connect, Premium to Grow**.

- Both Free and Premium users can discover eligible opportunities, request/accept connections, and chat after approval.
- Premium may offer higher posting and connection-request quotas, priority visibility, featured promotion, richer profiles, advanced discovery filters, and deeper analytics/AI history.
- Exact prices, quota amounts, and plan durations remain product decisions and are intentionally not fixed in this blueprint.
- Clearly show quota usage, reset period, and what happens when a quota is reached. Never block an already-approved conversation solely because a user is Free.

## 12. MVP implementation order after design review

1. Finalize sitemap, role access matrix, and primary user journeys.
2. Finalize shared visual foundations and responsive application shells.
3. Implement authentication and onboarding (Owner, Investor, separate Admin portal).
4. Implement profiles and business profiles with ownership/visibility controls.
5. Implement posts and discovery.
6. Implement connection requests and approved-connection messaging.
7. Implement membership entitlements and quota enforcement.
8. Implement Growth Advisor and Investment Intelligence on top of governed business data.
9. Implement Admin verification, moderation, membership operations, and audit tools.
10. Add end-to-end coverage, security review, accessibility review, and deployment readiness.

## 13. Explicitly out of scope for the MVP

- Goods checkout, cart, shipping, or inventory-first e-commerce workflows.
- Holding or transferring investment funds.
- Executing investment contracts or guaranteeing returns.
- Public Admin registration.
- Premium-only direct messaging.
- Investor access to private Owner data without explicit permission.
- AI-generated financial claims without evidence, assumptions, and limitations.

---

**Design review checkpoint:** validate the page hierarchy, navigation labels, and page-level content before moving into high-fidelity visual design or feature implementation.
