# Migrasi Database

Direktori ini berisi SQLx migrations berversi. Migration yang sudah diterapkan pada database bersama bersifat immutable; perubahan skema berikutnya harus menggunakan migration baru.

## Daftar migration Identity Core

| Urutan | File | Tanggung jawab |
| ---: | --- | --- |
| 0001 | `20261121170000_create_identity_accounts.sql` | Akun, identitas utama, role, lifecycle, reservasi kontak, dan fungsi pembuatan akun terbatas. |
| 0002 | `20261122100000_create_identity_verification.sql` | Sesi verifikasi, challenge OTP, event percobaan, dan event pengiriman. |
| 0003 | `20261122110000_create_identity_sessions.sql` | Sesi autentikasi, hash token, expiry, rotasi, dan pencabutan. |
| 0004 | `20261122120000_create_account_recovery.sql` | Kontak pemulihan, kasus recovery, dan token reset yang disimpan sebagai hash. |
| 0005 | `20261122130000_create_identity_audit_events.sql` | Audit event append-only dan perlindungan perubahan data. |
| 0006 | `20261122140000_remove_google_oauth_identity_support.sql` | Menghapus dukungan identitas Google OAuth dari skema aktif. |
| 0007 | `20261122160000_add_verification_database_operations.sql` | Operasi database terkontrol untuk sesi, challenge, dispatch, verifikasi, dan rate limit OTP. |
| 0008 | `20261122170000_bind_verification_ids_from_runtime.sql` | Mengikat identifier verifikasi yang dibuat runtime ke operasi kriptografi. |
| 0009 | `20261122180000_audit_verification_runtime_events.sql` | Audit event lifecycle verifikasi dan pengiriman OTP. |
| 0010 | `20261122190000_add_public_profile_and_policy_acceptance.sql` | Profil publik, penerimaan Terms/Privacy, dan consent marketing opsional. |
| 0011 | `20261122200000_allow_otp_resend_during_verification_lock.sql` | Mengizinkan resend saat lock tanpa membuka verifikasi sebelum lock berakhir. |
| 0012 | `20261122210000_add_password_authentication_runtime.sql` | Operasi login password, session creation/revocation, dan rate limit login. |
| 0013 | `20261122220000_add_session_runtime_operations.sql` | Validasi sesi, pembaruan aktivitas, rotasi token, dan logout semua sesi. |
| 0014 | `20261122230000_add_account_recovery_database_operations.sql` | Pembuatan sesi recovery dan pengaitan akun secara atomik. |
| 0015 | `20261122240000_add_account_recovery_account_candidates.sql` | Daftar kandidat akun setelah OTP diverifikasi dan pencocokan kontak pemulihan. |
| 0016 | `20261123100000_add_account_recovery_review_and_session_signals.sql` | Sinyal sesi aktif sebelumnya, keputusan review Admin append-only, dan audit event review. |
| 0017 | `20261123110000_add_password_reset_runtime.sql` | Penerbitan dan pencabutan token reset, konsumsi satu kali, penggantian hash password atomik, pencabutan seluruh sesi, penyelesaian kasus, dan audit event. |
| 0018 | `20261123120000_add_single_admin_totp_credentials.sql` | Membatasi akun Admin menjadi satu dan menyediakan penyimpanan kredensial TOTP terenkripsi dengan perlindungan replay-counter dan perubahan langsung. |

## Batas runtime

- HTTP runtime memakai `APP_DATABASE_URL` dan principal aplikasi berhak minimum.
- `DATABASE_URL` digunakan oleh proses migration dan pengujian integrasi yang dijalankan secara eksplisit.
- HTTP server tidak menjalankan migration otomatis saat startup.
- Provisioning principal PostgreSQL berada di `backend/database/bootstrap/`.
- Jalankan migration dengan principal migrator; jangan gunakan principal aplikasi atau DBA untuk runtime.
- Jangan menerapkan migration pada production tanpa prosedur deployment dan pemeriksaan terpisah.

## Status integrasi

Migration 0001–0018 tersedia pada repository. Migration 0018 hanya menyediakan fondasi skema akun Admin tunggal dan kredensial TOTP terenkripsi; login Admin, enrollment, verifikasi TOTP, provisioning internal, serta dashboard Admin belum tersedia. Pengujian PostgreSQL CI mencakup invariant token reset, privilege function, serta operasi reset dan pencabutan sesi. Runtime identity menyediakan operasi registrasi/verifikasi, login password, session management, dan account recovery.

Router registrasi dan recovery tersedia sebagai komponen terinjeksi, tetapi belum dipasang pada server default. Recovery mendukung sinyal sesi aktif sebelumnya dan keputusan review Admin melalui service internal. Password reset memiliki service penerbitan token setelah approval dan endpoint konsumsi token pada recovery router. Token berlaku 15 menit, disimpan sebagai hash, hanya dapat digunakan sekali, dan kegagalan pengiriman mencabut token. Konsumsi token memperbarui hash password, mencabut seluruh sesi aktif, menyelesaikan kasus recovery, dan menulis audit event dalam satu operasi PostgreSQL. Orkestrasi penerbitan token dari keputusan review Admin, endpoint review Admin, notifikasi keamanan pasca-reset, dan UI frontend reset password belum tersedia. Provider OTP nyata dan key provider produksi juga belum dikonfigurasi.

## Pengujian

Pengujian migration, invariant database, principal/privilege, dan operasi PostgreSQL berada di `backend/tests/postgres.rs`. Pengujian runtime HTTP berada di `backend/tests/runtime.rs`. CI menggunakan database disposable dan fake provider; pengujian tidak mengirim OTP nyata.

Perintah pemeriksaan backend tersedia di [Backend README](../README.md).

## Provisioning dan urutan operasi

1. Provision principal dan grant dasar melalui skrip pada `backend/database/bootstrap/`.
2. Jalankan migration secara eksplisit menggunakan principal migrator.
3. Terapkan grant runtime setelah ownership fungsi dan privilege diperiksa.
4. Jalankan pengujian PostgreSQL menggunakan principal yang sesuai.
5. Verifikasi konfigurasi runtime sebelum layanan diaktifkan.

Jangan mengubah file migration yang telah digunakan pada database bersama. Buat migration baru untuk setiap perubahan skema berikutnya.
