# Public Website & Authentication — UI/UX Specification

> Status: design specification  
> Parent blueprint: [Sitemap & UX Blueprint](./sitemap-and-ux-blueprint.md)  
> Scope: public landing experience, role selection, Owner/Investor authentication, onboarding entry, and Admin sign-in  
> Implementation status: not implemented yet

This document turns the sitemap into page-level interaction requirements. It is intentionally implementation-neutral so visual design and frontend implementation can be reviewed separately.

## 1. Experience goals

1. A first-time visitor understands within a few seconds that USahapintar connects business owners and investors.
2. Visitors can distinguish the Owner and Investor journeys without reading dense explanations.
3. Registration is guided and asks for only the information needed at the current step.
4. Owner and Investor login entry points are distinct, but use consistent shared interaction patterns.
5. Authentication errors are clear without exposing whether an account exists.
6. The mobile experience is designed as a first-class layout, not a compressed desktop page.
7. Trust claims, business metrics, and AI capabilities are described carefully; no funding, profit, or investment outcome is promised.

## 2. Global visual foundation

### Layout and surfaces

- Main page background: Soft White `#F8FAF7`.
- Main content surfaces: white with restrained borders and subtle shadows only where separation is needed.
- Accent surfaces: Pale Sage `#E8EDE5`.
- Primary action: Forest Green `#344E41`.
- Secondary accent: Sage Green `#A3B18A`.
- Main text: Deep Charcoal `#29332D`.
- Warm neutral accent: `#D8CDBB`.
- Keep the content width readable. Marketing sections may use a wider container; form screens should remain narrow enough to scan.
- Use consistent spacing tokens and a clear type scale. Avoid dense card grids and excessive rounded containers.

### Shared components

The first design system pass should define:

- `PublicHeader` and `PublicFooter`
- `PrimaryButton`, `SecondaryButton`, `TextButton`
- `TextField`, `PasswordField`, `PhoneField`, `SelectField`
- `CheckboxField`, `RadioCard`
- `FormFieldError`, `InlineNotice`, `Toast`
- `PageContainer`, `SectionHeading`, `AuthLayout`
- `LoadingButton`, `PasswordStrengthHint`, `OtpCodeInput`
- `StepIndicator`, `ProgressSummary`

Components must expose accessible labels and states rather than relying on placeholder text alone. Use semantic HTML, visible keyboard focus, and consistent error placement.

## 3. Public landing page — `/`

### Page objective

Communicate the platform's value proposition and give two clear next actions: explore the platform or register as an Owner/Investor.

### Desktop structure

1. **Header**
   - Left: USahapintar brand mark and wordmark.
   - Center: Cara Kerja, Fitur, Membership, Tentang.
   - Right: Masuk and a prominent Daftar CTA.
   - Header remains visually calm; do not make every navigation item a button.

2. **Hero**
   - Two-column editorial composition.
   - Left: short eyebrow, clear headline, concise description, two role-aware CTAs.
   - Right: one strong image/illustration representing business collaboration and opportunity discovery.
   - Suggested headline: **Bangun koneksi. Kembangkan peluang usaha.**
   - Suggested description: **USahapintar mempertemukan pemilik usaha dan investor untuk menemukan peluang, membangun kemitraan, dan mengambil keputusan dengan informasi yang lebih baik.**
   - Primary CTA: **Daftar sebagai Pemilik Usaha**
   - Secondary CTA: **Daftar sebagai Investor**
   - Supporting link: **Pelajari cara kerja USahapintar**

3. **Role explanation**
   - Two balanced sections/cards: Pemilik Usaha and Investor.
   - Owner: promote a business, organize business information, find relevant partners, use Growth Advisor.
   - Investor: discover business opportunities, review approved business information, connect with owners, use Investment Intelligence.
   - Each section has a specific CTA to the matching registration path.

4. **How it works**
   - Three or four steps with short labels:
     - Buat profil
     - Temukan peluang
     - Ajukan koneksi
     - Bangun komunikasi
   - Explain that chat opens after a connection request is accepted.

5. **Platform capabilities**
   - Business promotion and opportunity discovery.
   - Connection requests and approved-connection messaging.
   - Growth Advisor and Investment Intelligence.
   - Verification and transparent data labels.
   - Avoid implying all capabilities are available in the first release.

6. **Membership preview**
   - Explain the principle **Free to Connect, Premium to Grow**.
   - Both plans can discover opportunities, request/accept connections, and chat after approval.
   - Premium is positioned around growth tools, higher quotas, visibility, and richer insights.
   - CTA: **Lihat Membership**.
   - Do not show final prices or numeric quotas until product decisions are approved.

7. **Trust and responsibility**
   - Explain that USahapintar facilitates introductions and business networking.
   - State that the platform does not guarantee funding or returns and does not execute investment transactions in the MVP.
   - Explain that investors see only information the business owner has approved for sharing.

8. **Final CTA and footer**
   - Repeat the two role choices.
   - Footer groups: Platform, Bantuan, Legal, Kontak.
   - Include privacy policy and terms links.

### Mobile structure

- Compact header with logo and a menu trigger.
- Hero headline and description first, then role CTAs, then image/illustration.
- Role CTAs are full-width or stacked when space is limited.
- How-it-works steps become a vertical sequence.
- Feature sections stack; avoid horizontally scrolling text cards.
- Footer links are grouped into collapsible sections only if the interaction remains accessible.

### Landing page states

- Normal content is statically renderable and should not depend on authenticated API calls.
- If hero media fails, retain a designed neutral fallback with the headline and CTAs fully visible.
- Navigation menu opens/closes with keyboard support and exposes its expanded state.
- CTA navigation must preserve the selected role path.

## 4. Public navigation and registration entry

### Header actions

- **Masuk** opens a small role choice: Pemilik Usaha or Investor. It must not route both roles to an ambiguous generic client login.
- **Daftar** opens role selection at `/daftar`.
- Direct links to `/masuk/pemilik` and `/masuk/investor` remain available.
- Admin sign-in is not shown in the public header or registration role choice.

### Registration role choice — `/daftar`

Purpose: let a new user select their primary account role before entering a role-specific form.

Content:
- Heading: **Mulai bersama USahapintar**
- Supporting text: **Pilih bagaimana kamu akan menggunakan platform ini.**
- Two selectable role panels:
  - **Saya Pemilik Usaha** — promote a business, share opportunities, and find partners.
  - **Saya Investor** — discover business opportunities and connect with business owners.
- CTA on each panel routes to the matching registration path.
- A small existing-account link routes to the role-specific login choice.
- No Admin option.
- Role choice must be stored as part of the registration flow and validated by the backend; the user cannot self-select Admin.
- The selected client role is permanent for that account. Explain before submission that an account cannot switch between Business Owner and Investor later.
- If a person wants to use the other role, they must register a separate new account using login identities that are not already linked to another account. Do not offer a role-switch control or a support-based role migration path.

## 5. Owner login — `/masuk/pemilik`

### Content hierarchy

1. Brand and a short return link to the public site.
2. Heading: **Masuk sebagai Pemilik Usaha**
3. Supporting text: **Kelola profil usaha, bagikan peluang, dan kembangkan koneksi.**
4. Authentication form using the approved login identifier/method.
5. Google sign-in action for existing linked identities and role-bound new registrations.
6. Registration link for new Owners.
7. Privacy/terms links and support link.

### Form fields and behavior

- Login accepts the registered email or phone number plus the account password. Email/WhatsApp codes only activate the account and are never login credentials.
- Password field with show/hide toggle.
- Forgot password link.
- Remember-device/session option only if the backend session policy supports it safely; do not imply permanent sessions.
- Submit button: **Masuk**.
- Client-side validation catches empty or malformed input, but server validation remains authoritative.
- Disable duplicate submissions while a request is in progress.
- Preserve safe, non-sensitive field values after recoverable errors; never echo passwords.

### Role mismatch behavior

The account role is fixed at registration. If an Investor attempts to sign in through the Owner portal, show a neutral message such as:

**Akun ini tidak dapat masuk melalui halaman Pemilik Usaha. Coba halaman masuk Investor.**

Provide a link to the Investor login page. The server must enforce the role match; hiding routes in the frontend is not authorization. Do not offer to convert the account to Owner. A person who wants an Owner account must complete a separate registration with eligible, unlinked login identities.

### Error and recovery states

- Invalid credentials: **Email/nomor atau kata sandi belum sesuai. Periksa kembali lalu coba lagi.**
- Rate limit: explain that sign-in attempts are temporarily limited and when retry is available, if known.
- Network/server issue: **Belum bisa terhubung. Periksa koneksi lalu coba lagi.**
- Unverified contact: direct the user to the appropriate verification flow.
- Do not reveal whether an email or phone number is registered.
- Provide a support route for repeated account-access problems.

## 6. Investor login — `/masuk/investor`

Use the same layout and components as Owner login, with role-specific copy:

- Heading: **Masuk sebagai Investor**
- Supporting text: **Jelajahi peluang usaha dan bangun koneksi dengan pemilik bisnis.**
- Role mismatch link points to Owner login.
- Backend role validation is mandatory.

Keep the visual layout consistent so the two pages feel like one product. Distinguish them through copy and a restrained role indicator, not completely different designs.

## 7. Owner registration — `/daftar/pemilik`

### Registration principles

- Separate account creation from the longer business profile.
- Use progressive onboarding; do not ask for full business financials before the user has created an account.
- Clearly explain why each required field is needed.
- Save progress safely when supported; do not silently discard entered information.
- Collect only necessary personal data.

### Suggested steps

**Step 1 — Account**
- Full name.
- Choose one contact method: email address OR phone number.
- If email is chosen, send the verification code to that email address.
- If phone is chosen, send the verification code through WhatsApp.
- Required password and confirmation for email/phone registration. Google registration does not ask for a separate local password.
- Required acceptance of Terms and Privacy Policy.
- Optional product updates consent, unchecked by default.

**Step 2 — Contact verification**
- If Email was selected: enter the one-time code delivered to that email address.
- If Phone was selected: enter the one-time code delivered through WhatsApp.
- Only the selected contact is required for initial activation. The other contact may be added and verified later.
- Show destination in masked form.
- Resend action with visible cooldown and attempt limits.
- OTP expiry and retry states must be explicit.
- Never log or expose OTP values in client-visible error messages.

**Step 3 — Business stage**
- Idea only.
- Preparing to start.
- Already operating.
- Growing an existing business.

**Step 4 — Initial business profile**
Ask for only the fields relevant to the selected stage:
- Business or idea name.
- Sector/category.
- Main operating or target region.
- Short description.
- Current stage confirmation.
- Additional stage-specific questions can be deferred to the Owner workspace.

**Completion**
- Confirmation page with next steps.
- CTA to Owner dashboard.
- A visible profile-completion checklist that can be completed later.

## 8. Investor registration — `/daftar/investor`

### Suggested steps

**Step 1 — Account**
- Full name.
- Choose one contact method: email address OR phone number.
- If email is chosen, send the verification code to that email address.
- If phone is chosen, send the verification code through WhatsApp.
- Required password and confirmation fields for email/phone registration.
- Required Terms and Privacy Policy acceptance.
- Optional product updates consent, unchecked by default.

**Step 2 — Contact verification**
- Same mutually exclusive Email or Phone/WhatsApp choice and verification pattern as Owner registration. A password and confirmation are mandatory for this email/phone registration path.

**Step 3 — Investor profile**
- Investor type/profile description, using approved non-sensitive categories.
- Preferred sectors.
- Target regions.
- Indicative investment range, optional and clearly described as a discovery preference.
- Preferred time horizon, optional.
- Risk preference, optional and framed as a self-described preference, not a suitability assessment.

**Completion**
- Confirmation page.
- CTA to Investor dashboard and opportunity discovery.
- Preferences can be edited later.

## 9. Google OAuth behavior

- Google OAuth remains available for sign-in and registration.
- Before starting Google registration, the user must explicitly choose Owner or Investor; the choice is bound to server-side OAuth state. Google registration does not ask for a separate local password.
- A new Google identity must still complete the selected contact verification step: email code OR WhatsApp-delivered phone code. Google-asserted email alone does not satisfy this requirement.
- Existing linked Google identities sign in only to their existing account. The account's primary role remains immutable and determines the correct portal.
- Never auto-link based only on matching email. Linking requires fresh proof of control of both identities.
- Admin registration is never available through public Google OAuth.
- Validate state, nonce, issuer, audience, signature, expiry, redirect URI, and replay protection.

## 10. Password recovery

### Request page — `/lupa-kata-sandi`

- Accept the account's verified email or phone as the recovery destination, with a separate recovery challenge. Registration activation codes cannot be reused.
- Use a neutral confirmation message regardless of whether the account exists.
- Apply rate limits and abuse monitoring.
- Do not reveal account existence.

### Set-password page — `/atur-kata-sandi`

- Token is validated server-side and expires.
- New password and confirmation fields.
- Password policy guidance shown before submit.
- On success, revoke existing sessions according to the security policy and direct user to the appropriate role login.
- Invalid/expired token state offers a safe way to request a new link/code.

## 11. Admin login — `/admin/login`

Admin authentication is a separate internal experience.

- No public Admin registration.
- Admin identity is provisioned by an authorized administrator.
- Require MFA; prefer phishing-resistant options where supported.
- Use strict session expiry and re-authentication for sensitive actions.
- Apply stronger rate limits and security monitoring than ordinary client login.
- Do not expose detailed account or permission state in unauthenticated errors.
- Keep admin routes and APIs protected server-side with explicit permissions.
- Record successful/failed authentication and sensitive administrative actions in audit logs without storing passwords, OTPs, or session secrets.

## 12. Responsive behavior

### Breakpoint behavior

Use the existing project breakpoint conventions once confirmed in frontend tokens/config. Do not introduce arbitrary one-off breakpoints in individual pages.

- Wide desktop: full public navigation and two-column hero/form compositions where appropriate.
- Tablet: reduce navigation density; stack hero content when columns become cramped.
- Mobile: one-column content, stacked CTAs, form controls full width, no horizontal overflow.
- Authentication forms should remain usable on narrow screens and when the on-screen keyboard is open.
- Use correct input types and autocomplete attributes for email, telephone, current-password, and new-password fields.

### Accessibility

- All controls are keyboard accessible.
- Focus moves to the page heading or first invalid field after navigation/submission as appropriate.
- Validation messages are programmatically associated with their fields.
- Password visibility control has a meaningful accessible name and state.
- OTP input supports paste and assistive technology; avoid forcing users to navigate six unrelated inputs if a single semantic input can work.
- Color contrast meets WCAG AA targets.
- Respect reduced motion and browser zoom.

## 13. Analytics and privacy events

Collect only events needed to understand product usability and operational reliability, for example:
- Public CTA selected (role choice).
- Registration flow started/completed/abandoned at step level.
- Verification success/failure category.
- Login success/failure category.
- Password recovery requested/completed.

Do not send passwords, OTP values, raw phone/email, tokens, or sensitive form contents to analytics. Avoid third-party tracking before the consent/legal policy is defined.

## 14. Acceptance checklist before implementation

### Public landing
- [ ] Value proposition is clear without login.
- [ ] Owner and Investor paths are visually distinct and equally understandable.
- [ ] All public navigation and CTAs have defined destinations.
- [ ] Membership messaging does not imply Premium-only chat.
- [ ] Platform limitations and no-guarantee statement are clear.
- [ ] Hero fallback preserves core content if media fails.

### Login
- [ ] Owner and Investor have separate entry pages.
- [ ] Shared form behavior is consistent.
- [ ] Role mismatch is handled by the backend and gives a safe next step.
- [ ] Password visibility, forgot password, loading, error, and rate-limit states are designed.
- [ ] Google OAuth preserves mandatory role selection before new registration, separate contact verification, no separate local-password requirement, and safe identity linking.

### Registration and verification
- [ ] Only Owner and Investor can register publicly.
- [ ] Account creation is separate from longer onboarding.
- [ ] Email OR phone/WhatsApp registration choice is explicit; selected-channel verification, resend cooldown, expiry, and attempt limits are defined.
- [ ] Optional consent is unchecked by default.
- [ ] Progress and recovery behavior are defined.

### Admin
- [ ] Admin is not shown in public registration.
- [ ] Admin portal is separate and requires MFA.
- [ ] Server-side permission checks and audit requirements are documented.

### Engineering readiness
- [ ] Shared UI components have defined props and states before implementation.
- [ ] Form validation rules are documented and tested.
- [ ] Keyboard, mobile, and accessibility checks are included.
- [ ] Frontend dependency lockfile is committed before substantial frontend feature work.
- [ ] Authentication API contract, database model, and threat model are reviewed before implementing real authentication.

## 15. Next design checkpoint

After this specification is reviewed, create low-fidelity wireframes for:
1. Landing page desktop and mobile.
2. Registration role choice.
3. Owner and Investor login.
4. Owner and Investor registration steps.
5. Email/phone verification and recovery states.
6. Admin login.

Only after the wireframes and authentication contract are agreed should implementation begin.
