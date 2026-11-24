# Backend

API merupakan layanan Rust/Axum dalam paket workspace `usahapintar-api`.

## Konfigurasi runtime

Server membaca alamat bind dari variabel lingkungan:

| Variabel | Nilai bawaan | Keterangan |
| --- | --- | --- |
| `HOST` | `127.0.0.1` | Alamat IP untuk bind. Mendukung literal IPv4 dan IPv6. |
| `PORT` | `5000` | Port TCP. Harus berupa bilangan bulat dari 1 sampai 65535. |
| `AUTH_RATE_LIMIT_HMAC_KEY` | — | Wajib. Secret 32 byte dalam 64 karakter hex untuk fingerprint rate limit login. Gunakan nilai acak dari secret manager; jangan commit ke repository atau log. |
| `IDENTITY_DATA_ENCRYPTION_KEY_ACTIVE_VERSION` | — | Wajib. Versi aktif key enkripsi tujuan kontak, misalnya `v1`. |
| `IDENTITY_DATA_ENCRYPTION_KEYS` | — | Wajib. Daftar key hex 32 byte dengan format `v1:<64-hex>`; beberapa versi dipisahkan titik koma. |
| `IDENTITY_CONTACT_FINGERPRINT_KEY_ACTIVE_VERSION` | — | Wajib. Versi aktif key fingerprint kontak. |
| `IDENTITY_CONTACT_FINGERPRINT_KEYS` | — | Wajib. Daftar key fingerprint dengan format versi yang sama. |
| `IDENTITY_OTP_VERIFICATION_KEY_ACTIVE_VERSION` | — | Wajib. Versi aktif key HMAC OTP. |
| `IDENTITY_OTP_VERIFICATION_KEYS` | — | Wajib. Daftar key OTP dengan format versi yang sama. |
| `RESEND_API_KEY` | — | Wajib. API key Resend untuk pengiriman OTP email. |
| `RESEND_FROM_EMAIL` | — | Wajib. Sender email dari domain yang sudah diverifikasi di Resend. |
| `FLOWKIRIM_API_KEY` | — | Wajib. API key FlowKirim Verify untuk OTP WhatsApp. |
| `REGISTRATION_TERMS_VERSION` | — | Wajib. Label versi Terms yang dicatat saat pendaftaran. |
| `REGISTRATION_PRIVACY_VERSION` | — | Wajib. Label versi Privacy yang dicatat saat pendaftaran. |
| `REGISTRATION_MARKETING_NOTICE_VERSION` | Tidak | Versi pemberitahuan marketing; wajib tersedia jika consent marketing diaktifkan. |

Variabel wajib yang tercantum tanpa nilai bawaan harus disediakan sebelum server dijalankan. Konfigurasi key Identity, Resend, FlowKirim, dan versi kebijakan divalidasi saat startup; konfigurasi yang hilang atau tidak valid menghentikan startup. Setiap purpose Identity harus memakai material key yang berbeda. Pertahankan versi key lama selama data yang dienkripsi atau fingerprint yang dibuat dengan versi tersebut masih perlu dibaca. Jangan menaruh key produksi di source control, dokumentasi, atau log.

Jika salah satu variabel tidak diisi, server menggunakan nilai bawaannya. Nilai yang tidak valid akan menghentikan proses startup dengan kesalahan konfigurasi, bukan diam-diam diganti dengan nilai bawaan.

Alamat loopback bawaan menjaga API tetap dapat diakses dari host itu sendiri. Atur `HOST=0.0.0.0` hanya jika lingkungan deployment memang memerlukan antarmuka yang dapat dijangkau dari luar. Lindungi layanan menggunakan kontrol jaringan atau reverse proxy yang sesuai. Jangan membuka API secara langsung ke internet publik tanpa tinjauan keamanan deployment yang jelas.

Aplikasi tidak memuat file `.env` dengan sendirinya. Atur variabel lingkungan melalui shell, service manager, container runtime, atau platform deployment. Generate key dengan `openssl rand -hex 32`, simpan di secret manager, dan gunakan nilai yang sama pada seluruh instance. Rotasi key harus dikoordinasikan agar tidak menghapus bucket rate limit yang masih aktif.

## PostgreSQL

Runtime HTTP memakai principal aplikasi dengan hak minimum dan hanya membaca konfigurasi koneksi dari:

| Variabel | Wajib | Nilai bawaan | Keterangan |
| --- | --- | --- | --- |
| `APP_DATABASE_URL` | Ya | — | URL koneksi PostgreSQL milik `usahapintar_app_login`; jangan gunakan kredensial DBA atau migrator. |
| `DATABASE_MAX_CONNECTIONS` | Tidak | `10` | Batas connection pool; rentang yang diterima 1–100. |

Login Owner/Investor tersedia melalui `POST /api/v1/identity/login/owner` dan `POST /api/v1/identity/login/investor`; logout perangkat saat ini melalui `POST /api/v1/identity/logout`; informasi sesi aktif melalui `GET /api/v1/identity/me`; rotasi token melalui `POST /api/v1/identity/session/refresh`; dan logout semua perangkat melalui `POST /api/v1/identity/logout-all`. Login menggunakan email/nomor telepon + password dan tidak meminta OTP rutin. Cookie sesi menggunakan prefix `__Host-`, `HttpOnly`, `Secure`, `SameSite=Lax`, expiry absolut 30 hari, dan idle timeout 7 hari. Middleware memvalidasi sesi serta memperbarui `last_seen_at` pada endpoint identitas terproteksi. Refresh merotasi token secara server-side tanpa memperpanjang expiry absolut; token lama langsung tidak berlaku. Rate limit database: maksimal 10 percobaan per identifier dan 30 per IP dalam 15 menit; fingerprint disimpan sebagai HMAC. Jika API berada di belakang reverse proxy lokal, proxy wajib menimpa header `X-Real-IP` dengan alamat klien (`$remote_addr` pada Nginx); backend hanya mempercayai header tersebut ketika koneksi TCP berasal dari loopback. Jangan meneruskan nilai `X-Real-IP` dari input klien tanpa ditimpa.

Pool runtime membuka minimal satu koneksi saat startup untuk memverifikasi database tersedia, dengan batas maksimum sesuai konfigurasi, waktu tunggu akuisisi 5 detik, idle timeout 10 menit, dan masa hidup koneksi maksimum 30 menit. Kegagalan koneksi awal menghentikan startup agar layanan tidak berjalan dalam keadaan database yang dibutuhkan tidak tersedia. Kredensial dan URL database tidak ditulis ke log.

`DATABASE_URL` tetap disediakan untuk tooling migration dan pengujian integrasi yang dijalankan secara eksplisit. Variabel ini tidak digunakan untuk membuat pool HTTP runtime. Pemisahan ini mencegah aplikasi secara tidak sengaja berjalan dengan hak migrator.

SQLx migrations berada di `backend/migrations` dan dijalankan secara eksplisit melalui `database::migrations::run(&pool)` oleh proses/alat migration yang menggunakan principal `usahapintar_migrator`. Server HTTP tidak menjalankan migrasi otomatis saat startup. Jangan pernah menjalankan migrasi produksi sebagai bagian dari startup aplikasi.

Principal dan grant PostgreSQL disiapkan terpisah melalui `backend/database/bootstrap/`. Lihat [panduan provisioning principal](../docs/security/database-principal-bootstrap.md) untuk urutan dan batas privilege. Jangan menyimpan kredensial database di source control, pengujian, log, atau dokumentasi.

## Health dan readiness

- `GET /api/v1/health` adalah pemeriksaan liveness. Endpoint ini memastikan proses HTTP merespons dan tidak bergantung pada koneksi database.
- `GET /api/v1/readyz` adalah pemeriksaan readiness. Endpoint menjalankan `SELECT 1` melalui pool aplikasi dan mengembalikan HTTP 200 saat database siap atau HTTP 503 saat pemeriksaan gagal.

Gunakan liveness untuk mengetahui apakah proses perlu dipulihkan dan readiness untuk menentukan apakah instance layak menerima traffic. Jangan gunakan endpoint liveness sebagai bukti bahwa database siap.

## Menjalankan secara lokal

Siapkan database lokal dan principal aplikasi terlebih dahulu, kemudian jalankan:

```sh
# Buat setiap key Identity secara terpisah; jangan gunakan satu key untuk beberapa purpose.
export IDENTITY_DATA_ENCRYPTION_KEY_ACTIVE_VERSION=v1
export IDENTITY_DATA_ENCRYPTION_KEYS="v1:$(openssl rand -hex 32)"
export IDENTITY_CONTACT_FINGERPRINT_KEY_ACTIVE_VERSION=v1
export IDENTITY_CONTACT_FINGERPRINT_KEYS="v1:$(openssl rand -hex 32)"
export IDENTITY_OTP_VERIFICATION_KEY_ACTIVE_VERSION=v1
export IDENTITY_OTP_VERIFICATION_KEYS="v1:$(openssl rand -hex 32)"

# Isi dengan kredensial akun/provider pengembangan yang sesuai.
export RESEND_API_KEY='re_your_development_key'
export RESEND_FROM_EMAIL='USahapintar <noreply@your-verified-domain.example>'
export FLOWKIRIM_API_KEY='your_flowkirim_development_key'
export REGISTRATION_TERMS_VERSION='terms-v1'
export REGISTRATION_PRIVACY_VERSION='privacy-v1'

AUTH_RATE_LIMIT_HMAC_KEY="$(openssl rand -hex 32)" APP_DATABASE_URL='postgres://usahapintar_app_login:local_password@127.0.0.1:5432/usahapintar_dev' HOST=127.0.0.1 PORT=5000 cargo run --locked -p usahapintar-api
```

Jangan menggunakan kredensial produksi untuk pengembangan atau CI. Untuk migration lokal yang eksplisit, gunakan URL khusus migrator:

```sh
DATABASE_URL='postgres://usahapintar_migrator:local_password@127.0.0.1:5432/usahapintar_dev' cargo test --locked -p usahapintar-api --test postgres -- --ignored
```

Endpoint:
- Liveness: `http://127.0.0.1:5000/api/v1/health`
- Readiness: `http://127.0.0.1:5000/api/v1/readyz`

## Pemeriksaan kualitas

Jalankan dari direktori utama repositori:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
```

## Akses runtime Identity Core

Lapisan `usahapintar_api::identity` menyediakan operasi terbatas untuk registrasi dan verifikasi melalui principal aplikasi:

- `create_client_user` memanggil fungsi database `SECURITY DEFINER` untuk membuat akun pending dengan role Business Owner atau Investor. Aplikasi tidak memperoleh hak INSERT langsung ke tabel identitas.
- `append_audit_event` menambahkan event ke tabel audit append-only. Principal aplikasi tidak memperoleh hak membaca, mengubah, atau menghapus event audit.
- `VerificationService` mengorkestrasi registrasi, penerbitan/resend OTP, pencatatan hasil pengiriman, verifikasi, dan aktivasi akun. Password diproses menggunakan komponen hashing backend; password plaintext tidak disimpan atau diteruskan ke provider.
- `RecoveryService` memakai mekanisme OTP bersama dengan purpose separation untuk membuat sesi recovery tanpa akun terikat, verifikasi kontak, daftar kandidat akun setelah verifikasi, dan pemilihan akun dengan validasi kepemilikan kontak di PostgreSQL.
- `registration_router` dan `recovery_router` menyediakan endpoint HTTP terinjeksi dengan key provider, delivery provider, dan abuse guard. Runtime `main()` memasang router registrasi melalui `app_with_authentication_admin_and_registration`; builder `app()` tetap menjadi aplikasi dasar tanpa route identitas. Recovery belum dipasang sampai dukungan FlowKirim Verify pada lifecycle recovery selesai.

Registrasi publik menggunakan email + password dengan OTP email melalui Resend atau nomor telepon/WhatsApp + password dengan OTP WhatsApp melalui FlowKirim Verify. Router registrasi sudah dipasang pada runtime utama; CI menguji komposisi route menggunakan provider lokal/fake dan tidak mengirim pesan sungguhan. Login Owner/Investor menggunakan email/nomor telepon + password tanpa OTP rutin dan sudah tersedia pada runtime. Recovery HTTP tersedia sebagai router terinjeksi, tetapi belum dipasang pada runtime sampai FlowKirim Verify mendukung lifecycle recovery. Recovery mendukung pencatatan kecocokan sesi aktif yang sudah ada sebelum kasus dibuat serta keputusan approval/rejection oleh Admin melalui `RecoveryReviewService`. Reviewer ID dan token sesi wajib berasal dari konteks Admin aktif yang divalidasi server. Alur login Admin dan endpoint Admin untuk review belum tersedia. `PasswordResetIssuanceService` dapat menerbitkan token acak sekali pakai setelah kasus disetujui; token disimpan sebagai hash, berlaku 15 menit, dikirim melalui provider terpisah, dan dicabut jika pengiriman dinyatakan gagal. `POST /api/v1/identity/recovery/password-reset` memvalidasi token, menerapkan kebijakan password, mengganti hash secara atomik, mencabut seluruh sesi aktif, menyelesaikan kasus recovery, dan menulis audit event. Penerbitan token belum diorkestrasi otomatis dari alur review Admin; pemanggilan service tetap harus dilakukan oleh komponen server tepercaya setelah approval. Notifikasi keamanan pasca-reset dan UI frontend reset password belum terintegrasi. Sinyal sesi hanya bukti tambahan dan tidak cukup untuk menyetujui recovery dengan sendirinya. Penerapan middleware sesi pada endpoint fitur dan UI daftar/pencabutan sesi perangkat juga belum tersedia.

Adapter HTTP Resend (email) dan FlowKirim Verify (WhatsApp) sudah tersedia dan dikonfigurasi dari environment runtime. Pengiriman langsung ke provider eksternal belum menjadi bagian dari CI; CI menggunakan provider lokal/fake dan tidak mengirim pesan. Pastikan domain sender Resend sudah diverifikasi sebelum mengaktifkan pendaftaran email.

Endpoint recovery pada router terinjeksi:
- `POST /api/v1/identity/recovery` membuat sesi dan kasus recovery tanpa mengungkap apakah kontak terdaftar.
- `POST /api/v1/identity/recovery/{verification_id}/resend` dan `POST /api/v1/identity/recovery/{verification_id}/verify` mengelola OTP.
- `GET /api/v1/identity/recovery/cases/{case_id}/accounts` hanya menampilkan kandidat setelah kontak berhasil diverifikasi.
- `POST /api/v1/identity/recovery/cases/{case_id}/select` mengikat akun yang dipilih setelah PostgreSQL memastikan akun aktif memiliki kontak pemulihan terverifikasi yang cocok.
- `POST /api/v1/identity/recovery/password-reset` mengonsumsi token reset sekali pakai dan memperbarui password serta mencabut sesi secara atomik.

Untuk menjaga konsistensi, operasi pembuatan akun, profil, penerimaan Terms/Privacy, consent opsional, dan event audit yang terkait harus menggunakan transaksi sesuai kontrak database dan hanya commit setelah seluruh operasi berhasil. Untuk audit, `actor_type` dan `actor_id` wajib berasal dari konteks server tepercaya, bukan payload klien. Metadata hanya boleh memuat key allowlist non-sensitif; jangan memasukkan password, OTP, token, kontak plaintext, isi chat, atau payload privat.

Login password Owner/Investor, middleware sesi untuk endpoint identitas, refresh/rotasi token, logout perangkat saat ini, logout semua perangkat, router registrasi runtime, adapter HTTP Resend/FlowKirim Verify, keyring Identity berbasis environment, dan endpoint konsumsi token reset telah tersedia. Router recovery belum dipasang pada runtime. HTTP endpoint untuk review Admin, dukungan FlowKirim pada lifecycle recovery, orkestrasi penerbitan token dari keputusan review, notifikasi keamanan pasca-reset, UI frontend reset password, pengujian pengiriman langsung provider, middleware sesi pada endpoint fitur, dan pembacaan audit belum tersedia. Tambahkan operasi baru hanya setelah kontrak akses, kebutuhan data minimum, pemeriksaan otorisasi, grant database, dan tes principal aplikasi ditinjau bersama. Jangan memberikan SELECT langsung pada tabel identitas sebagai jalan pintas.
