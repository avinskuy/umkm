# Migrasi Database

Direktori ini berisi migration SQLx berversi. Migration Identity Core yang tersedia:

- `20261121170000_create_identity_accounts.sql` — membuat `users` dan `user_identities`, constraint lifecycle/role/kontak, unique partial index reservasi kontak, trigger immutability, transisi lifecycle 30 menit/30 hari, pemeriksaan metode autentikasi, serta fungsi `create_client_user` yang hanya menerima role Business Owner atau Investor.
- `20261122100000_create_identity_verification.sql` — membuat `verification_sessions`, `verification_challenges`, `otp_verification_attempt_events`, dan `otp_delivery_events`, termasuk state constraints, batas umur challenge 10 menit, satu challenge pending per sesi, binding recovery pra-pemilihan akun, serta indeks event lintas sesi.
- `20261122110000_create_identity_sessions.sql` — membuat `sessions` dengan hash token SHA-256 unik, batas absolut 30 hari, batas idle 7 hari, rotasi token, pencabutan individual, metadata perangkat minimum, serta pencabutan otomatis saat status lifecycle akun tidak lagi aktif.
- `20261122120000_create_account_recovery.sql` — membuat `recovery_contacts` dengan relasi lintas akun yang diizinkan dan deduplikasi per akun, `account_recovery_cases` dengan pemilihan target setelah verifikasi, serta `password_reset_tokens` yang hanya menyimpan hash dan terikat ke kasus/akun.
- `20261122130000_create_identity_audit_events.sql` — membuat `identity_audit_events` append-only dengan actor/subject, action/outcome/reason code, korelasi request, metadata JSONB terbatas, serta perlindungan terhadap UPDATE, DELETE, dan TRUNCATE.
- `20261122160000_add_verification_database_operations.sql` — menambahkan fungsi terkontrol untuk pembuatan/pembacaan sesi, reservasi challenge, pencatatan hasil dispatch, dan verifikasi OTP registrasi dengan rate limit lintas sesi.
- `20261122170000_bind_verification_ids_from_runtime.sql` — mengubah fungsi pembuatan sesi dan reservasi challenge agar UUID dibuat runtime sebelum AEAD/AAD dan keyed OTP digest dihitung.
- `20261122180000_audit_verification_runtime_events.sql` — menambahkan audit event untuk permintaan/hasil dispatch OTP dan percobaan verifikasi gagal, serta mengikat event registrasi/aktivasi ke akun terkait.\n- `20261122190000_add_public_profile_and_policy_acceptance.sql` — menambahkan profil publik satu-ke-satu, nama tampilan publik, catatan penerimaan Terms of Service dan Privacy Policy per versi, serta event consent marketing opsional yang append-only. Fungsi registrasi terkontrol menyimpan akun, profil, penerimaan kebijakan, dan pilihan marketing secara atomik.

## Status runtime

Modul Rust `identity` menyediakan pembuatan akun pending, profil publik, penerimaan kebijakan wajib per versi, consent marketing opsional, dan `VerificationService` untuk registrasi email/WhatsApp, issue/resend OTP, pencatatan hasil dispatch, serta verifikasi dan aktivasi. Provider pengiriman masih berupa abstraksi; CI memakai fake provider dan tidak mengirim pesan nyata. Endpoint HTTP verifikasi, adapter provider nyata, dan konfigurasi key produksi belum tersedia.

Rate limit pengiriman dan percobaan verifikasi ditegakkan melalui fungsi PostgreSQL terkontrol. Batas pengiriman adalah lima kali per kontak dalam 60 menit dengan cooldown 60 detik; batas kegagalan verifikasi lintas sesi adalah lima kali per kontak dalam 60 menit, dengan lock sesi satu jam setelah lima kegagalan. Challenge berlaku maksimal 10 menit. Akun pending memiliki masa berlaku 30 menit.

Migration 0003 menyediakan penyimpanan sesi autentikasi, tetapi endpoint login/logout, validasi cookie, dan middleware autentikasi belum tersedia. Migration 0004 menyediakan skema recovery, tetapi runtime recovery belum tersedia. Batas idle 7 hari dan absolut 30 hari tetap harus diperiksa backend pada setiap permintaan. Pembacaan audit, retensi, forwarding eksternal, cleanup job, dan proses operasional lainnya belum termasuk.

## Pemisahan runtime dan migration

HTTP runtime membuat pool PostgreSQL melalui `DatabaseConfig::from_app_env()` dan hanya menggunakan `APP_DATABASE_URL` milik `usahapintar_app_login`. Pool dibatasi oleh `DATABASE_MAX_CONNECTIONS` (default 10, maksimum 100), acquire timeout 5 detik, idle timeout 10 menit, dan masa hidup koneksi 30 menit. Startup gagal tertutup jika konfigurasi atau koneksi database aplikasi tidak tersedia.

`DATABASE_URL` digunakan untuk proses migration dan pengujian integrasi yang berjalan eksplisit dengan principal `usahapintar_migrator`. Fungsi `database::migrations::run(&pool)` tetap tersedia untuk tooling tersebut, tetapi tidak dipanggil oleh startup HTTP. Jangan menjalankan migrasi otomatis saat server aplikasi mulai.

Endpoint `GET /api/v1/health` merupakan liveness check yang tidak bergantung pada database. Endpoint `GET /api/v1/readyz` menjalankan `SELECT 1` melalui pool runtime dan mengembalikan HTTP 503 jika database tidak siap.

Provisioning principal PostgreSQL berada terpisah dari SQLx migration di `backend/database/bootstrap/`. Jalankan `001_create_principals.sql` sebagai DBA pada database khusus sebelum migration, lalu jalankan seluruh migration sebagai `usahapintar_migrator`. Setelah migration 0001–0010 selesai, jalankan `002_identity_core_grants.sql` sebagai DBA; skrip memverifikasi ownership non-superuser sebelum memberi EXECUTE pada fungsi SECURITY DEFINER. CI menguji bootstrap dengan DBA sementara, migrator non-superuser, dan login aplikasi terpisah.

Pengujian PostgreSQL untuk migration, invariant identitas, privilege principal, dan orchestration OTP berada di `backend/tests/postgres.rs`. Pengujian readiness runtime berada di `backend/tests/runtime.rs` dan memakai `APP_DATABASE_URL` melalui principal aplikasi. Keduanya dijalankan terhadap database disposable di CI.

Jangan pernah mengubah migration setelah diterapkan pada database bersama. Perubahan skema berikutnya harus menggunakan migration baru dengan versi SQLx berikutnya. Migration, provisioning principal, dan deployment produksi tetap merupakan operasi terpisah dengan prosedur serta persetujuan masing-masing.
