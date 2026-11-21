# Authentication & Identity — Architecture and Security Design

> Status: skema akun, verifikasi, dan sesi tersedia; runtime autentikasi masih belum diimplementasikan  
> Product reference: `docs/product/public-and-auth-uiux-spec.md`  
> Wireframe reference: `docs/product/low-fidelity-public-auth-wireframes.md`  
> Backend direction: Rust + Axum; PostgreSQL 16 + SQLx 0.8 foundation is available. The first account/Google-identity migration is implemented; verification, sessions, recovery, and runtime flows remain unimplemented.

This document defines the authentication architecture for USahapintar. The account, contact-verification, and authentication-session database slices are implemented, but authentication endpoints, cookie handling, middleware, and runtime database integration are not. This is not a claim of a production-ready or security-audited system. Provider choices, operational secrets, retention periods, and final controls must be reviewed before production use.

## 1. Scope and invariants

### In scope

- Public registration for Business Owner and Investor only.
- Exactly one immutable primary client role (`business_owner` or `investor`) per account.
- Separate Owner and Investor login entry points.
- Email/phone registrations require password-based login. Google OAuth registrations do not require a separate local password and authenticate through the linked Google identity. Email/WhatsApp verification codes are only for account activation/contact verification and never replace a password or Google authentication at login.
- Email or phone/WhatsApp as mutually exclusive initial registration verification choices.
- Google OAuth is retained for sign-in and registration, but it never bypasses role selection or the separate email/WhatsApp registration-code verification.
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

## Membership and authorization boundary

Membership is a separate entitlement dimension and never changes the immutable primary role. A Free or Premium plan cannot grant Owner or Investor capabilities to the other role, bypass account status, ownership, consent, moderation, or Admin permission checks, or reveal private business information. The backend evaluates authentication, account status, role, resource permission, safety/moderation state, and only then membership entitlement and quota. See [Membership & Entitlements Policy](../product/membership-and-entitlements.md) for the plan capability baseline, quota rules, downgrade behavior, and test requirements.

## 2. Identity and role model

### Primary role

Use a single primary role for the MVP:

- `business_owner`
- `investor`
- `admin`

The role is assigned during registration for Owner/Investor and is immutable for the lifetime of the account. Admin is assigned only through an internal provisioning workflow. No profile endpoint, support process, administrative migration, or OAuth flow may change an existing account's primary role.

A person who wants to operate as the other client role must create a separate new account. Registration must reject login identities that are already linked to another account without disclosing which account owns them. Releasing an identity from a closed account, if ever supported, must follow a separately defined account-closure, verification, and retention policy; it must never silently transfer an identity or merge accounts.

### Authorization layers

Authorization is evaluated in this order:

1. Is the session valid and not revoked/expired?
2. Is the account lifecycle active, and are there no active moderation restrictions or security locks?
3. Does the authenticated account have the required primary role?
4. Does the account own or participate in the requested resource?
5. Is the requested action permitted for that resource's current state?
6. Does membership entitlement affect the requested quota/feature? Membership never bypasses ownership or role checks.

Do not implement authorization as scattered frontend conditions. Use shared backend extractors/middleware and domain-level policy functions, with tests.

## 3. Proposed data model

PostgreSQL 16 is the selected relational baseline. The first SQLx migration for account identity and Google provider identity is implemented and covered by PostgreSQL integration tests. The HTTP server still does not initialize the database at startup. Verification, session, recovery, audit, and product-domain migrations remain separate future slices. Migrations must be versioned and immutable after application to a shared database. Use UUID primary keys and `timestamptz`.

### `users`

The first migration implements the following account fields. Profile data and deletion-request history remain in separate future domain tables.

| Column | Purpose |
| --- | --- |
| `id` | UUID primary key |
| `primary_role` | Immutable role: `business_owner`, `investor`, or `admin` |
| `account_status` | Lifecycle: `pending_verification`, `active`, `deactivated`, `deletion_pending`, `deletion_processing`, `pending_expired`, or `closed` |
| `email_normalized` | Normalized primary email; nullable when phone is the only current primary contact |
| `phone_e164` | Normalized E.164 primary phone; nullable when email is the only current primary contact |
| `initial_contact_channel` | Immutable initial registration channel: `email` or `phone` |
| `email_verified_at`, `phone_verified_at` | Independent verification timestamps for the current contact values |
| `initial_contact_verified_at` | Immutable registration verification provenance, set once during activation and cleared only after completed deletion |
| `password_hash` | Nullable encoded local password hash; client accounts without it require a linked Google identity |
| `pending_expires_at` | Immutable 30-minute deadline while registration is pending |
| `created_at`, `updated_at`, `status_changed_at` | Database-managed timestamps |

Constraints and lifecycle guards:
- Partial unique indexes reserve non-null email and phone contacts while the account is pending, active, deactivated, deletion-pending, or deletion-processing.
- A pending account has exactly one initial contact, not yet verified. An active account has at least one current verified primary contact.
- The role and initial registration channel are immutable. Initial contact verification can be recorded only once during valid activation.
- Pending registration cannot activate after 30 minutes or expire before its deadline. The pending expiry cannot be extended.
- Deletion must wait 30 days in `deletion_pending` before entering `deletion_processing`; processing can only finish as `closed`. `pending_expired` and `closed` are terminal.
- Suspend and feature limits are moderation/enforcement records, not `account_status` values.
- Profile fields, deactivation/deletion request history, and retention records are not stored in this first slice.
- Never store plaintext passwords. Do not expose raw database IDs as authorization secrets.

### `user_identities`

The first migration implements linked Google provider identities.

| Column | Purpose |
| --- | --- |
| `id` | UUID primary key |
| `user_id` | Owning user; FK with ON DELETE RESTRICT |
| `provider` | Initially constrained to `google` |
| `provider_subject` | Stable provider subject identifier; not email |
| `created_at` | Creation timestamp |

Constraints:
- Unique composite key on (`provider`, `provider_subject`).
- Provider subject, not email text, is the stable identity key.
- Identity binding is immutable after insert; unlinking is a separate authenticated action.
- Client accounts without a local password cannot unlink their last Google identity.
- Terminal accounts cannot retain provider identities.
- No provider email or OAuth access/refresh/ID tokens are stored in this table.
- Linking requires proof of control of the existing account and the new provider identity.
- Do not auto-link solely because provider emails appear to match.

### `sessions`

Opaque, server-managed sessions.

| Column | Purpose |
| --- | --- |
| `id` | UUID primary key |
| `user_id` | Session owner |
| `token_hash` | SHA-256 hash 32-byte dari token sesi opaque berentropi tinggi; unique global |
| `created_at` | Waktu pembuatan sesi yang ditetapkan database |
| `last_seen_at` | Aktivitas terakhir yang diperbarui server untuk menegakkan batas idle 7 hari |
| `expires_at` | Batas absolut yang ditetapkan database, 30 hari sejak sesi dibuat |
| `token_rotated_at` | Waktu rotasi token terakhir yang ditetapkan database |
| `remember_device` | Penanda persistensi cookie browser; tidak mengubah batas idle maupun absolut |
| `revoked_at`, `revocation_reason` | Waktu dan alasan pencabutan server-side |
| `user_agent_summary` | Ringkasan perangkat opsional, diminimalkan dan dibatasi panjangnya |
| `ip_prefix_hash` | HMAC/hash prefix jaringan opsional; bukan alamat IP mentah |

Constraints:
- Simpan hanya hash kriptografis token sesi berentropi tinggi, bukan token asli. Skema mensyaratkan digest 32 byte dan keunikan global.
- Waktu pembuatan, aktivitas awal, rotasi token, dan batas absolut ditetapkan database. Batas absolut selalu 30 hari.
- Backend wajib menolak sesi yang tidak aktif lebih dari 7 hari, telah melewati `expires_at`, dicabut, atau dimiliki akun yang tidak aktif. Perangkat yang diingat tidak boleh melewati batas ini.
- Rotasi token dilakukan setelah autentikasi dan perubahan privilege; hash lama diganti secara atomik dan timestamp rotasi diperbarui database.
- Pencabutan sesi harus segera berlaku dari server. Logout, pencabutan satu perangkat, logout semua perangkat, reset password, perubahan status akun, tindakan Admin, dan peristiwa keamanan memiliki reason code yang eksplisit.
- Perubahan lifecycle akun dari `active` ke status lain otomatis mencabut seluruh sesi yang belum dicabut. Restriksi/suspensi moderasi dan peristiwa keamanan lain tetap harus memicu pencabutan melalui runtime.
- Jangan mengumpulkan alamat IP penuh atau fingerprint perangkat lengkap tanpa kebutuhan keamanan dan kebijakan retensi yang terdokumentasi.

### `verification_challenges`

Supports email and WhatsApp-delivered phone verification for account activation/contact verification without storing usable codes. These codes are not login credentials.

| Column | Purpose |
| --- | --- |
| `id` | UUID primary key |
| `user_id` | Related user, nullable only for pre-account registration flow if necessary |
| `channel` | email or whatsapp; phone is the normalized destination, WhatsApp is the delivery channel |
| `destination_hash` | Hash or protected reference to destination |
| `code_hash` | Hash of a high-entropy one-time code |
| `purpose` | Registration verification or contact change |
| `verification_session_id` | Alur verifikasi induk; jumlah kegagalan dan penguncian tetap berlaku setelah OTP dikirim ulang |
| `attempt_count` | Jumlah percobaan gagal untuk challenge ini |
| `expires_at` | Masa berlaku 10 menit |
| `superseded_at` | Diisi ketika OTP pengganti membatalkan kode ini |
| `consumed_at` | One-time use marker |
| `created_at` | Creation timestamp |

Controls:
- Catatan sesi verifikasi terpisah harus menyimpan akumulasi batas lima kegagalan dan penguncian satu jam. Pembuatan challenge baru tidak boleh mengatur ulang batas tersebut.
- Simpan atau tegakkan secara persisten jeda pengiriman ulang 60 detik dan batas lima pengiriman per jam untuk setiap alamat tujuan di seluruh instance aplikasi.
- Short expiry, one-time consumption, attempt caps, resend cooldown, and destination/account/IP rate limits.
- Use a keyed hash/HMAC for short numeric OTPs so database disclosure alone does not make offline guessing trivial.
- Never log code values or include them in analytics.
- Invalidate older active challenges when a new challenge is issued for the same purpose/destination.
- Provider delivery failures must not mark a destination as verified.

### Kontak pemulihan dan kasus pemulihan akun

Kontak pemulihan bersifat opsional dan ditambahkan setelah akun aktif. Simpan kontak ini terpisah dari kontak utama untuk login agar masing-masing dapat diverifikasi, diberi notifikasi, dan dicabut secara terpisah. Setiap kontak pemulihan perlu memiliki nilai yang sudah dinormalisasi, jenis kanal, waktu verifikasi, status, dan relasi ke pemilik akun. Terapkan aturan keunikan serta pencegahan penyalahgunaan tanpa membocorkan hubungan kontak dengan akun.

Pemulihan akun memerlukan alur kasus dan verifikasi yang tercatat, bukan sekadar penanda benar/salah. Catat tujuan (lupa kata sandi, lupa email, atau pemulihan alternatif), status, waktu pembuatan/kedaluwarsa/peninjauan, hasil penilaian risiko, dan referensi audit secukupnya. Jangan simpan rahasia yang dikirim pengguna atau bukti identitas berlebihan di log umum. Penugasan pemeriksaan manual harus mengacu pada petugas support dengan kewenangan khusus, dan setiap pemeriksaan dicatat dalam riwayat yang tidak dapat diubah sembarangan. Email utama lengkap hanya boleh ditampilkan setelah kontak pemulihan yang sudah terhubung ke akun berhasil diverifikasi.

Jika kontak utama dan seluruh kontak pemulihan tidak dapat diakses, alur pemulihan alternatif harus memeriksa perangkat tepercaya, riwayat sesi, dan bukti kepemilikan lainnya. Jika pemeriksaan otomatis belum cukup, kasus dapat diteruskan kepada petugas yang berwenang. Permintaan berisiko tinggi dapat ditunda atau ditolak. Proses pemulihan tidak boleh mengubah role utama akun.

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

## Kebijakan awal Identity Core yang telah disetujui

Nilai kebijakan produk berikut merupakan batas awal yang telah disepakati dan harus ditegakkan oleh backend. Nilai ini dapat ditinjau ulang melalui evaluasi keamanan dan bukan jaminan dari penyedia layanan.

### OTP dan pendaftaran tertunda

- OTP berupa kode angka dengan jumlah digit yang belum ditetapkan. Kode berlaku selama 10 menit, disimpan sebagai keyed hash, terikat pada tujuan dan alamat penerima, hanya dapat digunakan satu kali secara atomik, serta tidak boleh dicatat dalam log sebagai teks asli.
- Maksimal lima percobaan gagal untuk setiap sesi verifikasi. Kegagalan kelima mengunci sesi tersebut selama satu jam. Pengiriman kode pengganti tidak boleh menghapus atau melewati penguncian.
- Jeda sebelum pengiriman ulang adalah 60 detik. Maksimal lima pengiriman per jam untuk setiap alamat kontak. Terapkan pembatasan tambahan yang menyesuaikan IP/jaringan, akun, tujuan, dan pola aktivitas mencurigakan.
- Kode baru langsung membatalkan kode sebelumnya untuk tujuan dan alamat kontak yang sama. Kegagalan pengiriman tidak boleh dianggap sebagai keberhasilan verifikasi.
- Pendaftaran yang belum selesai kedaluwarsa setelah 30 menit, terpisah dari masa berlaku OTP. Pembersihan harus aman dijalankan berulang kali dan saat terjadi proses bersamaan, serta tidak boleh menghapus akun yang sudah aktif.

### Sesi

- Pengguna boleh memiliki beberapa sesi aktif sekaligus. Pengguna dapat melihat daftar sesi, mencabut satu sesi, atau mengakhiri semua sesi.
- Batas awal sesi adalah 7 hari tanpa aktivitas dan maksimal 30 hari sejak sesi dibuat.
- Fitur “Ingat perangkat ini” hanya membuat sesi browser bertahan. Fitur ini tidak boleh melewati kedua batas waktu atau pencabutan sesi dari server. Menutup browser tidak otomatis mengakhiri sesi yang memang dibuat persisten.
- Pencabutan sesi dari server berlaku segera. Gunakan cookie `Secure` dan `HttpOnly`, pengaturan `SameSite`, serta perlindungan CSRF dan pemeriksaan origin. Tindakan sensitif dapat meminta verifikasi ulang.

### Siklus hidup akun

Penonaktifan sementara berbeda dari penghapusan permanen. Saat dinonaktifkan, profil publik disembunyikan dan akses normal dibatasi, tetapi data serta role tetap disimpan. Untuk mengaktifkan kembali akun, pengguna harus login dan melewati pemeriksaan keamanan. Penghapusan permanen memerlukan konfirmasi dan autentikasi ulang, lalu masa tunggu 30 hari. Selama masa tunggu, akun disembunyikan atau dibatasi dan permintaan penghapusan dapat dibatalkan melalui pemeriksaan keamanan. Setelah itu, proses penghapusan mengikuti aturan retensi data dan penanganan cadangan yang telah didokumentasikan. Kontak tidak boleh otomatis digunakan kembali.

### Pemulihan akun

Kontak pemulihan tidak wajib saat pendaftaran. Kontak ini dapat ditambahkan setelah akun aktif dan harus diverifikasi secara terpisah. Alur lupa kata sandi dan lupa email menggunakan email atau nomor HP pemulihan yang sudah terverifikasi. Email utama lengkap hanya boleh ditampilkan setelah kontak pemulihan berhasil diverifikasi dan terbukti terhubung ke akun.

Jika kontak utama dan seluruh kontak pemulihan sama-sama tidak dapat diakses, gunakan proses pemulihan alternatif berlapis. Periksa perangkat tepercaya, sesi sebelumnya, dan sinyal kepemilikan lainnya. Jika bukti otomatis belum cukup, teruskan kasus kepada petugas support dengan kewenangan khusus. Permintaan berisiko tinggi dapat ditunda atau ditolak. Terapkan hak akses minimum, kriteria pemeriksaan yang terdokumentasi, audit log, dan notifikasi yang sesuai. Pemulihan tidak boleh mengubah role utama. Pencabutan sesi dan perlindungan tambahan setelah pemulihan ditentukan berdasarkan tingkat risiko.

## 4. API contract (proposed)

All routes are versioned under `/api/v1`. JSON request/response shapes should be defined in OpenAPI before implementation. The table below is the route inventory, not final wire-format documentation.

### Registration and verification

| Method | Route | Purpose |
| --- | --- | --- |
| POST | `/auth/register/owner` | Create Owner account in pending-verification state |
| POST | `/auth/register/investor` | Create Investor account in pending-verification state |
| POST | `/auth/verification/email/request` | Request/resend email verification |
| POST | `/auth/verification/email/confirm` | Confirm email challenge |
| POST | `/auth/verification/whatsapp/request` | Request/resend a phone verification code delivered through WhatsApp |
| POST | `/auth/verification/whatsapp/confirm` | Confirm the WhatsApp-delivered phone verification code |

There is intentionally no `/auth/register/admin` endpoint.

### Password and session authentication

Email- and phone-based registrations must establish a local password. Google OAuth registrations do not need to create or enter a separate local password; they authenticate through their linked Google identity. Email/WhatsApp verification codes only activate the account and prove control of the selected contact; they are never accepted as login credentials.

| Method | Route | Purpose |
| --- | --- | --- |
| POST | `/auth/login/owner` | Authenticate only an Owner account |
| POST | `/auth/login/investor` | Authenticate only an Investor account |
| POST | `/auth/logout` | Revoke current session |
| POST | `/auth/logout-all` | Revoke all sessions for the authenticated account |
| GET | `/auth/session` | Return current authenticated user and safe role/session metadata |
| POST | `/auth/password/forgot` | Request a password reset |
| POST | `/auth/password/reset` | Consume reset token and set a new password |

Login accepts the registered email or phone number together with the account password. Verification codes are not accepted by login endpoints. The login endpoints must check the expected role on the server. A mismatch returns a generic role-portal guidance response without exposing additional account details.

### Google OAuth — retained

Google OAuth is supported for sign-in and registration. It does not replace mandatory role selection or contact verification; it does remove the need to create or enter a separate local password for a Google-based registration.

| Method | Route | Purpose |
| --- | --- | --- |
| GET | `/auth/oauth/google/start?role=business_owner` | Begin Owner-context OAuth |
| GET | `/auth/oauth/google/start?role=investor` | Begin Investor-context OAuth |
| GET | `/auth/oauth/google/callback` | Validate provider response and continue the role-bound flow |

For a new Google identity, the user must first select Owner or Investor, then complete Google OAuth and select exactly one contact method (email code or WhatsApp-delivered phone code) for separate verification before account activation. No separate local password is required. A Google-asserted email does not waive this verification. The role context is bound to server-side state and cannot be changed by callback input. Existing linked identities sign in only to their existing immutable-role account. Never auto-link based on matching email alone; linking requires fresh proof of control of both identities. Admin accounts cannot be created through public OAuth. Validate state, nonce, issuer, audience, signature, expiry, and exact redirect URI.

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
- Email/WhatsApp verification codes cannot be used to authenticate a login session.
- Account suspension blocks session use even if a session token has not expired.
- Role mismatch never creates a session.
- Sensitive admin actions may require recent re-authentication/MFA.

Enforce the approved session limits: 7 days of inactivity and 30 days absolute. Persistent-device preference must not bypass either limit or server-side revocation. Sensitive actions may require fresh authentication.

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

For a new Google identity, continue only the role-bound registration flow and require separate email or WhatsApp contact-code verification before activation. A previously linked provider identity may authenticate only its existing account. Never change role or auto-link based on matching email.

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
- Duplicate provider subject cannot be linked to a second user and linking requires fresh proof of both identities.
- OAuth must never create a second account using a provider subject already linked to an account with another role.
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
2. **Database baseline:** PostgreSQL/SQLx configuration, migrator, and first account/Google-identity migration are implemented and covered by integration tests; next is the separate verification migration.
3. **Domain types:** define User, PrimaryRole, AccountStatus, Session, VerificationChallenge, and typed error codes.
4. **Registration:** implement Owner/Investor registration with validation and safe pending state.
5. **Verification:** implement email and phone challenge lifecycle behind provider interfaces.
6. **Password auth:** implement hashing, role-specific login, session cookie, logout, and recovery.
7. **Google OAuth:** implement role-bound sign-in/registration; new accounts must select a role before OAuth, need no separate local password, and must still complete selected email or WhatsApp contact verification.
8. **Authorization:** add backend role/resource policies and negative tests.
9. **Admin foundation:** provisioning, MFA, permissions, and audit trail.
10. **Hardening:** rate-limit review, security tests, logging review, accessibility, and deployment configuration.

Implement each step as a small pull request. Do not merge until CI, focused integration tests, and the relevant security checks pass.

## 14. Keputusan tersisa sebelum implementasi

Hal-hal berikut harus ditetapkan sebelum sistem siap digunakan di lingkungan produksi:

- Pemilihan penyedia email dan WhatsApp Business/API, persyaratan pengirim/template, keandalan pengiriman di Indonesia, percobaan ulang, pemantauan, batas biaya, dan penanggung jawab operasional.
- Konfigurasi klien Google OAuth, alamat callback produksi, dan pengelolaan rahasia.
- Parameter hashing kata sandi yang tepat dan perilaku pemulihan kredensial untuk akun yang hanya menggunakan Google jika pengaturan kata sandi lokal kelak disediakan.
- Masa retensi numerik berdasarkan kategori data, jadwal backup, dan durasi penyelesaian penghapusan. Kebijakan produk telah menetapkan masa tunggu pembatalan 30 hari; kontak utama baru dapat digunakan kembali setelah penghapusan selesai dan kontak dilepas secara sah. Kontak pemulihan bersama tetap diperbolehkan.
- Metode dan penyedia MFA Admin, serta pemetaan izin teknis untuk petugas khusus yang menangani pemeriksaan pemulihan manual. Kebijakan pemulihan berbasis risiko dan standar bukti telah disetujui.
- Penyimpanan pembatasan permintaan terdistribusi dan ambang operasional; penegakannya harus berlaku di seluruh instance aplikasi.
- Skema metadata bukti pemulihan, pembobotan sinyal, kontrol eskalasi, dan kebutuhan persetujuan dua pihak untuk kasus berisiko tinggi; dokumen identitas atau selfie bukan metode standar pemulihan.

Batas awal produk yang telah disetujui (OTP, masa berlaku pendaftaran tertunda, durasi sesi, masa tunggu penonaktifan/penghapusan, dan perilaku pemulihan) sudah didokumentasikan di atas dan bukan lagi keputusan yang belum ditetapkan.
