# Low-Fidelity Wireframes — Public & Authentication

> Status: structural wireframe specification  
> Related documents:
> - [Sitemap & UX Blueprint](./sitemap-and-ux-blueprint.md)
> - [Public & Authentication UI/UX Specification](./public-and-auth-uiux-spec.md)
>
> These wireframes define hierarchy, placement, and interaction—not final colors, imagery, typography, or pixel-perfect dimensions.

## 1. Wireframe conventions

- `[ ... ]` represents a control, card, or content region.
- `( ... )` represents supporting text or a secondary action.
- `{ ... }` represents a state or conditional content.
- Desktop layouts use a centered content container with a wider marketing grid.
- Mobile layouts use a single column and vertically stacked actions.
- All primary actions must remain visible without requiring horizontal scrolling.

## 2. Landing page

### Desktop

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [USahapintar]     Cara Kerja   Fitur   Membership   Tentang   [Masuk] [Daftar]│
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  [PLATFORM INTRO]                           ┌───────────────────────────┐  │
│  Bangun koneksi.                            │                           │  │
│  Kembangkan peluang usaha.                  │     HERO VISUAL           │  │
│                                             │  Business collaboration  │  │
│  Short value proposition                    │  / opportunity discovery │  │
│  in 2–3 readable lines.                     │                           │  │
│                                             └───────────────────────────┘  │
│  [Daftar sebagai Pemilik Usaha]                                           │
│  [Daftar sebagai Investor]                                                │
│  (Pelajari cara kerja)                                                    │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│                       DUA JALUR UTAMA                                      │
│  ┌────────────────────────────┐      ┌────────────────────────────┐       │
│  │ PEMILIK USAHA              │      │ INVESTOR                   │       │
│  │ Promote business           │      │ Discover opportunities     │       │
│  │ Build business profile     │      │ Review shared information  │       │
│  │ Find relevant partners     │      │ Connect with owners        │       │
│  │ [Mulai sebagai Owner]      │      │ [Mulai sebagai Investor]   │       │
│  └────────────────────────────┘      └────────────────────────────┘       │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│  CARA KERJA                                                                │
│  [1 Buat profil] → [2 Temukan peluang] → [3 Ajukan koneksi] → [4 Terhubung]│
├────────────────────────────────────────────────────────────────────────────┤
│  FITUR UTAMA                                                               │
│  [Promosi usaha] [Discovery] [Koneksi & chat] [AI Intelligence]            │
├────────────────────────────────────────────────────────────────────────────┤
│  MEMBERSHIP                                                                │
│  Free to Connect, Premium to Grow                                          │
│  [Ringkasan Free]                    [Ringkasan Premium]                   │
│  (Lihat detail membership)                                                 │
├────────────────────────────────────────────────────────────────────────────┤
│  TRUST & RESPONSIBILITY                                                    │
│  Introductions, owner-approved data, no guaranteed funding or returns.    │
├────────────────────────────────────────────────────────────────────────────┤
│  FINAL CTA: [Daftar sebagai Pemilik Usaha] [Daftar sebagai Investor]      │
├────────────────────────────────────────────────────────────────────────────┤
│  Footer: Platform | Bantuan | Legal | Kontak                              │
└────────────────────────────────────────────────────────────────────────────┘
```

### Mobile

```text
┌─────────────────────────────┐
│ [USahapintar]          [≡]  │
├─────────────────────────────┤
│ PLATFORM INTRO              │
│                             │
│ Bangun koneksi.             │
│ Kembangkan peluang usaha.   │
│                             │
│ Short value proposition     │
│                             │
│ [Daftar sebagai Pemilik]    │
│ [Daftar sebagai Investor]   │
│ (Pelajari cara kerja)       │
│                             │
│ ┌─────────────────────────┐ │
│ │       HERO VISUAL       │ │
│ └─────────────────────────┘ │
├─────────────────────────────┤
│ PEMILIK USAHA               │
│ Benefits in short bullets   │
│ [Mulai sebagai Owner]       │
├─────────────────────────────┤
│ INVESTOR                    │
│ Benefits in short bullets   │
│ [Mulai sebagai Investor]    │
├─────────────────────────────┤
│ CARA KERJA                  │
│ 1. Buat profil              │
│ 2. Temukan peluang          │
│ 3. Ajukan koneksi           │
│ 4. Mulai terhubung          │
├─────────────────────────────┤
│ FITUR UTAMA                 │
│ [Feature row]               │
│ [Feature row]               │
│ [Feature row]               │
├─────────────────────────────┤
│ MEMBERSHIP                  │
│ Free to Connect...          │
│ [Lihat Membership]          │
├─────────────────────────────┤
│ TRUST & RESPONSIBILITY      │
├─────────────────────────────┤
│ FINAL CTA                   │
│ [Daftar sebagai Pemilik]    │
│ [Daftar sebagai Investor]   │
├─────────────────────────────┤
│ Footer groups               │
└─────────────────────────────┘
```

### Landing interactions

- Header Daftar opens the role-choice page.
- Header Masuk opens a compact Owner/Investor choice.
- Each role CTA routes directly to its matching registration page.
- Feature and membership links scroll or navigate to their corresponding public sections/pages.
- Mobile menu traps focus while open, closes with Escape, and restores focus to its trigger.
- Hero media has a neutral fallback; text and CTAs do not depend on media loading.

## 3. Registration role choice — `/daftar`

### Desktop

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [USahapintar]                                      (Kembali ke Beranda)     │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│                       Mulai bersama USahapintar                            │
│             Pilih bagaimana kamu akan menggunakan platform ini.           │
│                                                                            │
│      ┌────────────────────────────────┐  ┌────────────────────────────────┐│
│      │ PEMILIK USAHA                  │  │ INVESTOR                       ││
│      │                                │  │                                ││
│      │ Promote your business, share   │  │ Discover opportunities and     ││
│      │ opportunities, find partners. │  │ connect with business owners. ││
│      │                                │  │                                ││
│      │ [Daftar sebagai Pemilik]       │  │ [Daftar sebagai Investor]      ││
│      └────────────────────────────────┘  └────────────────────────────────┘│
│                                                                            │
│                Sudah punya akun? (Masuk sebagai Owner / Investor)           │
└────────────────────────────────────────────────────────────────────────────┘
```

### Mobile

```text
┌─────────────────────────────┐
│ [USahapintar]               │
├─────────────────────────────┤
│ Mulai bersama USahapintar   │
│ Choose your primary role.   │
│                             │
│ ┌─────────────────────────┐ │
│ │ PEMILIK USAHA           │ │
│ │ Short role description  │ │
│ │ [Daftar sebagai Pemilik]│ │
│ └─────────────────────────┘ │
│                             │
│ ┌─────────────────────────┐ │
│ │ INVESTOR                │ │
│ │ Short role description  │ │
│ │ [Daftar sebagai Investor]││
│ └─────────────────────────┘ │
│                             │
│ Sudah punya akun?           │
│ [Masuk sebagai Owner]       │
│ [Masuk sebagai Investor]    │
└─────────────────────────────┘
```

Interaction: selecting a role navigates to its dedicated registration path. The role is not editable later through a casual profile setting; role changes require an explicit account migration/support policy.

## 4. Owner and Investor login

The two login pages share the same layout. Only role-specific heading, helper copy, role indicator, and mismatch destination differ.

### Desktop

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [USahapintar]                                      (Kembali ke Beranda)     │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   ┌───────────────────────────────┐    ┌────────────────────────────────┐  │
│   │ AUTH CONTEXT                  │    │ MASUK SEBAGAI                  │  │
│   │                               │    │ [Pemilik Usaha / Investor]     │  │
│   │ Role-specific short message   │    │                                │  │
│   │ and a restrained illustration │    │ [Email atau nomor telepon]     │  │
│   │                               │    │ [Kata sandi             (eye)]  │  │
│   │                               │    │ (Lupa kata sandi?)             │  │
│   │                               │    │ [Masuk]                        │  │
│   │                               │    │ ───────── atau ─────────        │  │
│   │                               │    │ [Lanjutkan dengan Google]      │  │
│   │                               │    │ Belum punya akun? [Daftar]     │  │
│   └───────────────────────────────┘    └────────────────────────────────┘  │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### Mobile

```text
┌─────────────────────────────┐
│ [USahapintar]               │
├─────────────────────────────┤
│ (Kembali ke Beranda)        │
│                             │
│ MASUK SEBAGAI               │
│ Pemilik Usaha / Investor    │
│ Short role-specific copy    │
│                             │
│ [Email atau nomor telepon]  │
│ [Kata sandi           (eye)]│
│ (Lupa kata sandi?)          │
│                             │
│ [Masuk]                     │
│                             │
│ ───────── atau ─────────    │
│ [Lanjutkan dengan Google]   │
│                             │
│ Belum punya akun? [Daftar]  │
│                             │
│ [Privacy] [Terms] [Bantuan] │
└─────────────────────────────┘
```

### Login states

```text
DEFAULT
  Fields enabled; submit available when input is valid.

SUBMITTING
  Submit disabled; progress indicator shown; duplicate request prevented.

INVALID CREDENTIALS
  Neutral inline notice; do not identify whether the email/phone exists.

ROLE MISMATCH
  Explain that this account uses the other role portal; link to the correct portal.

RATE LIMITED
  Show a safe retry message and remaining wait time only if provided by server.

NETWORK ERROR
  Keep non-sensitive fields; offer retry.

UNVERIFIED CONTACT
  Provide the relevant verification continuation without exposing account details.
```

## 5. Owner registration

Registration is a staged flow. The account is created before the user is asked for extended business information.

### Step 1 — Account

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                               Langkah 1 dari 4 │
│                                                              │
│ Buat akun Pemilik Usaha                                      │
│                                                              │
│ Nama lengkap          [________________________________]     │
│ Email                 [________________________________]     │
│ Nomor telepon         [________________________________]     │
│ Kata sandi            [________________________________]     │
│ Konfirmasi sandi      [________________________________]     │
│                                                              │
│ [ ] Saya menyetujui Ketentuan dan Kebijakan Privasi           │
│ [ ] Saya ingin menerima kabar produk (opsional)               │
│                                                              │
│ [Lanjutkan]                                                   │
│ Sudah punya akun? [Masuk]                                     │
└──────────────────────────────────────────────────────────────┘
```

### Step 2 — Verification

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                               Langkah 2 dari 4 │
│                                                              │
│ Verifikasi kontak                                            │
│ Kode/link dikirim ke email dan nomor yang ditampilkan aman.  │
│                                                              │
│ Email: m••••@example.com                                     │
│ [Buka instruksi email] [Kirim ulang] (cooldown)               │
│                                                              │
│ Nomor: +62 •••• •••• 1234                                    │
│ [ _ ] [ _ ] [ _ ] [ _ ] [ _ ] [ _ ]                           │
│ [Verifikasi nomor]                                            │
│                                                              │
│ (Ubah kontak)                                                 │
└──────────────────────────────────────────────────────────────┘
```

The visual OTP control may use grouped boxes, but the underlying input must support paste, keyboard navigation, and assistive technology. Do not expose the OTP in logs or analytics.

### Step 3 — Business stage

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                               Langkah 3 dari 4 │
│                                                              │
│ Sejauh mana perjalanan usahamu?                              │
│                                                              │
│ ( ) Saya masih punya ide usaha                               │
│ ( ) Saya sedang mempersiapkan usaha                           │
│ ( ) Usaha saya sudah berjalan                                │
│ ( ) Saya ingin mengembangkan usaha                           │
│                                                              │
│ [Kembali]                                      [Lanjutkan]    │
└──────────────────────────────────────────────────────────────┘
```

### Step 4 — Initial business profile

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                               Langkah 4 dari 4 │
│                                                              │
│ Ceritakan sedikit tentang usahamu                            │
│                                                              │
│ Nama usaha/ide         [______________________________]      │
│ Bidang usaha           [Pilih kategori               v]      │
│ Wilayah                [Pilih wilayah                 v]      │
│ Deskripsi singkat      [______________________________]      │
│                        [______________________________]      │
│                                                              │
│ (Kamu bisa melengkapi profil setelah masuk.)                 │
│ [Kembali]                                      [Selesaikan]   │
└──────────────────────────────────────────────────────────────┘
```

If the stage is idea-only, label the business name as an idea/project name and avoid requiring operating metrics.

## 6. Investor registration

### Step 1 and 2

Reuse the account and contact verification layouts from Owner registration, changing the role copy only.

### Step 3 — Investor preferences

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                               Langkah 3 dari 3 │
│                                                              │
│ Atur preferensi jelajah                                      │
│ (Informasi ini membantu menyaring peluang; bisa diubah.)     │
│                                                              │
│ Bidang yang diminati   [Pilih satu atau beberapa       v]    │
│ Wilayah target         [Pilih wilayah                 v]     │
│ Rentang indikatif      [Opsional                      v]     │
│ Horizon waktu          [Opsional                      v]     │
│ Preferensi risiko      [Opsional                      v]     │
│                                                              │
│ [ ] Saya memahami bahwa preferensi bukan rekomendasi         │
│     investasi atau jaminan hasil.                            │
│                                                              │
│ [Kembali]                                      [Selesaikan]   │
└──────────────────────────────────────────────────────────────┘
```

Keep financial range and risk preference optional in the initial onboarding unless product/legal review later establishes a justified requirement.

## 7. Password recovery and reset

### Request reset

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                                                 │
│                                                              │
│ Lupa kata sandi?                                             │
│ Masukkan email atau nomor telepon yang terkait dengan akun. │
│                                                              │
│ [Email atau nomor telepon____________________________]       │
│ [Kirim instruksi]                                            │
│                                                              │
│ {Neutral confirmation message shown regardless of account}  │
│ (Kembali ke halaman masuk)                                   │
└──────────────────────────────────────────────────────────────┘
```

### Set new password

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                                                 │
│                                                              │
│ Buat kata sandi baru                                         │
│ [Kata sandi baru______________________________ (eye)]        │
│ [Ulangi kata sandi____________________________ (eye)]        │
│ [Simpan kata sandi]                                          │
│                                                              │
│ {Expired token: offer request-new-link action}                │
└──────────────────────────────────────────────────────────────┘
```

## 8. Admin login

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar Admin]                         Internal access  │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│ Akses Administrator                                         │
│                                                              │
│ Email admin             [____________________________]       │
│ Kata sandi              [____________________________]       │
│ [Masuk]                                                     │
│                                                              │
│ {MFA challenge after primary authentication}                 │
│ Authenticator / passkey challenge                            │
│                                                              │
│ (Hubungi super-admin jika akses bermasalah.)                 │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

No public Admin registration or role selector is present. The exact MFA method and recovery policy must be decided in the security design before implementation.

## 9. Shared error, loading, and empty states

| State | Visual placement | Behavior |
| --- | --- | --- |
| Field validation | Directly below field | Explain the correction; associate with input |
| Form-level error | Above submit button | Neutral, non-sensitive explanation |
| Loading | Submit button | Prevent duplicate submit and preserve layout |
| Success | Inline confirmation or next page | Explain next action |
| Network failure | Form-level notice | Retry without losing safe input |
| Rate limit | Form-level notice | Explain wait period if known |
| Expired verification | Verification panel | Offer safe resend/restart |
| OAuth cancelled | Inline notice | Allow retry or use another method |

Do not use color alone to convey state. Keep error messages concise and actionable.

## 10. Component inventory for the first implementation slice

| Component | Responsibility | Initial states |
| --- | --- | --- |
| PublicHeader | Public navigation and role entry | Desktop, mobile menu open/closed |
| PublicFooter | Public/legal/help links | Responsive groups |
| RoleChoiceCard | Owner/Investor registration entry | Default, focus, selected |
| AuthLayout | Shared role login composition | Desktop, mobile |
| TextField | Labeled text input | Default, focus, error, disabled |
| PasswordField | Password input with visibility toggle | Hidden, visible, error |
| PhoneField | International phone input | Default, invalid |
| OtpCodeInput | Contact verification code | Empty, partial, complete, expired |
| StepIndicator | Multi-step progress | Current, complete, upcoming |
| FormNotice | Form-level status message | Info, error, success |
| LoadingButton | Async submit control | Idle, loading, disabled |

Keep component styles feature-local or in CSS modules. Do not centralize every page's styling in one large stylesheet.

## 11. Review checklist

- [ ] Desktop and mobile hierarchy is clear for every screen.
- [ ] Owner and Investor are distinguished without creating two unrelated design systems.
- [ ] Admin is clearly separated from public user journeys.
- [ ] No role selector appears on a signed-in profile as a casual role switch.
- [ ] All forms have visible labels, inline validation, and server-error states.
- [ ] OTP supports paste, keyboard, and assistive technology.
- [ ] Google OAuth preserves role context and handles cancelled/error states.
- [ ] Mobile layouts avoid horizontal overflow and remain usable with the on-screen keyboard.
- [ ] Membership copy preserves free approved-connection chat.
- [ ] No UI implies guaranteed funding, investment returns, or verified metrics without evidence.
- [ ] Every action has a destination or a documented state transition.

## 12. Handoff to implementation

Once the wireframes are accepted:

1. Confirm exact copy, required/optional fields, and Owner/Investor role mismatch behavior.
2. Commit the frontend lockfile and stabilize dependency installation.
3. Define authentication API contracts and database migrations.
4. Review the auth threat model (OTP abuse, session fixation, OAuth state/nonce, password reset, account linking, enumeration, CSRF/CORS, rate limiting).
5. Implement shared form primitives and the public/auth shell.
6. Add focused component, integration, and browser tests before connecting real authentication providers.

No authentication implementation should begin from these wireframes alone; the API contract and security design are required first.
