# Kontrak Skema Migration 0003 — Sesi Autentikasi

> Status: implementasi skema dan tes berada pada PR; belum menjadi fitur autentikasi runtime.
> Cakupan: PostgreSQL 16 + SQLx 0.8.
> Migration: `20261122110000_create_identity_sessions.sql`.

## 1. Tujuan dan batas

Migration ini menyediakan penyimpanan sesi autentikasi opaque yang dikelola server. Satu akun boleh memiliki beberapa sesi aktif pada perangkat berbeda. Skema mendukung daftar riwayat sesi, pencabutan satu sesi, logout semua perangkat, rotasi token, serta pembatasan waktu sesi.

Migration ini **tidak** mengimplementasikan endpoint login/logout, validasi password/Google OAuth, cookie browser, CSRF, middleware autentikasi, runtime database initialization, audit event terpisah, atau provisioning/grant principal. Tidak ada secret atau token sesi plaintext yang dibuat/disimpan oleh migration.

## 2. Struktur `sessions`

| Kolom | Kontrak |
| --- | --- |
| `id` | UUID primary key |
| `user_id` | FK wajib ke `users.id`, ON DELETE RESTRICT |
| `token_hash` | Digest SHA-256 32-byte dari token opaque berentropi tinggi; unique global |
| `created_at` | Waktu pembuatan yang ditetapkan database |
| `last_seen_at` | Waktu aktivitas terakhir yang ditetapkan database |
| `expires_at` | Batas absolut yang ditetapkan database: 30 hari sejak `created_at` |
| `token_rotated_at` | Waktu rotasi token terakhir yang ditetapkan database |
| `remember_device` | Penanda persistensi cookie browser; tidak mengubah batas waktu |
| `revoked_at` | Waktu pencabutan server-side |
| `revocation_reason` | Reason code pencabutan |
| `user_agent_summary` | Ringkasan perangkat opsional, maksimal 256 karakter |
| `ip_prefix_hash`, `ip_prefix_hmac_key_version` | HMAC 32-byte atas prefix jaringan dan versi key; keduanya opsional dan harus hadir berpasangan, bukan alamat IP mentah |

## 3. Invariant database

1. Sesi hanya dapat dibuat untuk akun dengan `account_status = 'active'`. Pemeriksaan mengunci baris akun selama transaksi pembuatan agar perubahan status tidak berlomba dengan pembuatan sesi.
2. Token hash harus tepat 32 byte dan unik secara global. Token asli tidak disimpan.
3. Waktu pembuatan, aktivitas awal, rotasi awal, dan batas absolut diatur database. Absolute expiry selalu 30 hari; nilai dari caller tidak dapat memperpanjangnya.
4. `user_id`, `created_at`, `expires_at`, `remember_device`, dan metadata perangkat tidak dapat diubah setelah sesi dibuat.
5. Aktivitas/rotasi ditolak bila sesi telah melewati absolute expiry, idle timeout tujuh hari, atau akun tidak lagi aktif.
6. Rotasi token mengganti hash dan memperbarui `token_rotated_at` serta `last_seen_at` dalam satu operasi database.
7. Pencabutan hanya dapat dilakukan satu kali. Sesi yang telah dicabut tidak dapat diaktifkan kembali, dirotasi, atau diperbarui.
8. Waktu pencabutan ditetapkan database dan wajib memiliki reason code yang valid.
9. Saat status lifecycle akun berubah dari `active` ke status lain, seluruh sesi yang belum dicabut otomatis dicabut dengan reason `account_status_changed`.
10. Metadata perangkat dibatasi dan immutable. Hash prefix jaringan harus berupa HMAC berkunci dengan versi key yang dicatat. Jangan menambahkan alamat IP mentah, fingerprint perangkat penuh, atau data sensitif yang tidak dibutuhkan.

Reason code yang disediakan: `logout`, `user_revoked`, `logout_all`, `password_reset`, `account_status_changed`, `security_event`, dan `admin_action`.

## 4. Validasi wajib pada runtime

Pada **setiap** permintaan terautentikasi, backend harus memeriksa secara konsisten:

- hash token cocok dengan satu sesi;
- `revoked_at IS NULL`;
- `expires_at > clock_timestamp()`;
- `last_seen_at > clock_timestamp() - INTERVAL '7 days'`;
- akun terkait masih `active`;
- tidak ada restriksi/suspensi keamanan atau moderasi yang sedang berlaku;
- role, ownership, consent, dan izin resource tetap diperiksa terpisah.

Jangan hanya memeriksa sesi saat login. Pencabutan server-side harus berlaku pada permintaan berikutnya, termasuk sesi yang token/cookie-nya masih tersimpan di perangkat.

Aktivitas sesi harus diperbarui secara terkontrol. Database menolak pembaruan aktivitas atau rotasi setelah idle/absolute expiry, sehingga sesi lama tidak dapat dihidupkan kembali hanya dengan mengirim request baru.

## 5. Aturan produk yang tetap berlaku

- Multi-perangkat diperbolehkan.
- Idle timeout: tujuh hari sejak aktivitas terakhir.
- Absolute timeout: 30 hari sejak sesi dibuat.
- “Ingat perangkat ini” hanya memengaruhi persistensi cookie browser; tidak memperpanjang kedua batas waktu dan tidak menghindari pencabutan server.
- Menutup browser tidak otomatis logout untuk sesi persisten.
- Pengguna dapat melihat sesi, mencabut satu sesi, atau logout dari semua perangkat.
- Reset password, perubahan status akun, restriksi Admin, dan peristiwa keamanan relevan harus mencabut sesi sesuai reason code dan risiko.
- Cookie produksi harus `Secure`, `HttpOnly`, `SameSite` yang sesuai, serta dilindungi CSRF/origin checks. Detail konfigurasi cookie ditetapkan pada implementasi runtime.

## 6. Tes PostgreSQL

Tes integrasi harus membuktikan:

- sesi hanya dapat dibuat untuk akun aktif;
- multi-sesi per akun diperbolehkan;
- hash token 32-byte dan keunikan global;
- absolute expiry tepat 30 hari dan tidak dapat diubah caller;
- token dapat dirotasi dan timestamp rotasi dikelola database;
- metadata perangkat dan pilihan remember-device immutable;
- pencabutan individual one-way dan reason code tercatat;
- sesi tidak dapat diaktifkan kembali atau dirotasi setelah dicabut;
- perubahan lifecycle akun ke status nonaktif mencabut semua sesi yang belum dicabut.

Tes runtime terpisah tetap diperlukan untuk validasi setiap request, cookie/CSRF, logout-all yang atomik, pencabutan saat reset password/restriksi Admin, race antara validasi dan revocation, serta integrasi audit event.

## 7. Batas operasional

- Migration belum dijalankan pada database produksi.
- HTTP server belum menginisialisasi database pada startup.
- Tidak ada GRANT yang dibuka oleh migration ini. Runtime application principal, migration principal, dan database administrator tetap harus dipisahkan sesuai least privilege.
- Belum ada runtime authentication/session middleware, cookie, endpoint, audit table, cleanup/retention job, atau deployment.
- Migration yang telah diterapkan pada database bersama tidak boleh diedit; koreksi berikutnya harus memakai migration baru.
