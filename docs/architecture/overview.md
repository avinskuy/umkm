# Gambaran Arsitektur

## Arsitektur sistem

USahapintar menggunakan **modular monolith**. Frontend dan backend merupakan aplikasi terpisah, sedangkan domain backend berada dalam satu layanan. Pemisahan layanan hanya dilakukan jika kebutuhan operasional dan batas domain membenarkannya.

## Teknologi

### Frontend

- React, TypeScript, dan Vite.
- Struktur kode berorientasi fitur.
- Komponen bersama dipisahkan dari komponen khusus domain.
- Style global dibatasi pada reset, design tokens, dan aturan global.

Frontend saat ini masih berupa shell awal. Fitur produk utama belum tersedia secara lengkap.

### Backend

- Rust dan Axum.
- Modul berorientasi domain dengan batas API, application, domain, dan infrastructure sesuai kebutuhan.
- PostgreSQL 16 sebagai database relasional.
- SQLx untuk connection pool dan migration.
- HTTP runtime menggunakan `APP_DATABASE_URL` dengan principal aplikasi berhak minimum.
- Migration dijalankan secara eksplisit menggunakan principal migrator. Server HTTP tidak menjalankan migration otomatis.
- `GET /api/v1/health` menyediakan liveness check; `GET /api/v1/readyz` memeriksa kesiapan database.

## Identity Core

Identity Core menyediakan fondasi akun Business Owner dan Investor, password authentication, OTP verification, session management, audit events, serta account recovery.

- Registrasi memerlukan password dan OTP melalui email atau WhatsApp.
- Login rutin menggunakan email/nomor telepon dan password tanpa OTP.
- Password menggunakan Argon2id.
- Sesi menggunakan token opaque yang disimpan sebagai hash, cookie aman, idle timeout tujuh hari, dan absolute expiry 30 hari.
- Session validation, refresh/rotation, logout, dan logout semua perangkat tersedia pada endpoint identitas.
- Recovery database operations, service core, HTTP router, active-session continuity signal, dan service review Admin tersedia dan memerlukan sesi Admin aktif. Alur login Admin dan endpoint review belum tersedia. Router recovery belum dipasang pada server default; password reset dan notifikasi keamanan belum tersedia.
- Router registrasi juga belum dipasang pada server default.
- Adapter OTP nyata, production key provider, approval recovery, password reset, dan integrasi middleware sesi ke endpoint fitur belum tersedia.

Lihat [Keputusan Pendaftaran Identity Core](identity-core-registration-decisions.md), [Kontrak HTTP Registrasi](identity-core-registration-http-api.md), dan [Kontrak Migration Recovery](identity-core-migration-0004-account-recovery-contract.md).

## Domain produk

Batas domain yang direncanakan:

- Identitas dan autentikasi.
- Profil Business Owner, bisnis, dan listing.
- Profil Investor dan peluang investasi.
- Discovery, koneksi, dan permintaan kemitraan.
- Pesan dan komunitas.
- Membership dan entitlement.
- Verifikasi bisnis.
- Asisten berbasis AI.
- Moderasi dan administrasi.

Daftar domain tidak mengharuskan pembuatan modul kosong. Implementasikan modul saat perilaku, kontrak, dan kebutuhan datanya jelas.

## Data dan otorisasi

- Backend menjadi sumber kebenaran untuk role, status akun, dan kepemilikan resource.
- Gunakan RBAC + ABAC, least privilege, dan deny-by-default.
- Setiap request yang mengakses resource terlindungi harus memeriksa izin pada tingkat objek.
- Data bisnis hanya dibagikan sesuai visibilitas dan persetujuan yang berlaku.
- Perubahan penting harus menggunakan transaksi dan menghasilkan audit event sesuai kebijakan.

## Batasan desain

- Jangan menambahkan microservices tanpa kebutuhan yang terukur.
- Hindari folder generik seperti `utils` atau `common` tanpa tanggung jawab spesifik.
- Hindari ketergantungan melingkar antarfitur.
- Jangan menduplikasi aturan domain atau kontrol keamanan pada frontend.
- Tambahkan abstraksi bersama hanya jika ada pemakai nyata atau tanggung jawab platform yang jelas.
