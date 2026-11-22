# Backend

API merupakan layanan Rust/Axum dalam paket workspace `usahapintar-api`.

## Konfigurasi runtime

Server membaca alamat bind dari variabel lingkungan:

| Variabel | Nilai bawaan | Keterangan |
| --- | --- | --- |
| `HOST` | `127.0.0.1` | Alamat IP untuk bind. Mendukung literal IPv4 dan IPv6. |
| `PORT` | `5000` | Port TCP. Harus berupa bilangan bulat dari 1 sampai 65535. |
| `AUTH_RATE_LIMIT_HMAC_KEY` | — | Wajib. Secret 32 byte dalam 64 karakter hex untuk fingerprint rate limit login. Gunakan nilai acak dari secret manager; jangan commit ke repository atau log. |

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
- `registration_router` dan `recovery_router` menyediakan endpoint HTTP terinjeksi dengan key provider, delivery provider, dan abuse guard. Keduanya belum dipasang pada `app()`/`main()` default. `app_with_authentication_and_recovery` menyediakan builder eksplisit untuk pemasangan terkontrol.

Registrasi publik menggunakan email + password dengan OTP email atau nomor telepon/WhatsApp + password dengan OTP WhatsApp. Router registrasi tersedia tetapi belum dipasang pada server default. Login Owner/Investor menggunakan email/nomor telepon + password tanpa OTP rutin dan sudah tersedia pada runtime. Recovery HTTP tersedia sebagai router terinjeksi, tetapi belum dipasang pada server default karena key provider, delivery provider, dan abuse guard produksi belum dikonfigurasi. Recovery saat ini berhenti pada status `under_review`; approval/rejection, pemeriksaan perangkat tepercaya yang terintegrasi, penerbitan token reset, reset password, notifikasi keamanan, dan pencabutan sesi pasca-reset belum tersedia. Penerapan middleware sesi pada endpoint fitur dan UI daftar/pencabutan sesi perangkat juga belum tersedia.

Provider pengiriman masih berupa abstraksi. Resend (email) dan FlowKirim (WhatsApp) merupakan target adapter, bukan integrasi nyata yang telah dikonfigurasi atau diuji dengan pengiriman langsung. CI menggunakan fake provider dan tidak mengirim pesan.

Endpoint recovery pada router terinjeksi:
- `POST /api/v1/identity/recovery` membuat sesi dan kasus recovery tanpa mengungkap apakah kontak terdaftar.
- `POST /api/v1/identity/recovery/{verification_id}/resend` dan `POST /api/v1/identity/recovery/{verification_id}/verify` mengelola OTP.
- `GET /api/v1/identity/recovery/cases/{case_id}/accounts` hanya menampilkan kandidat setelah kontak berhasil diverifikasi.
- `POST /api/v1/identity/recovery/cases/{case_id}/select` mengikat akun yang dipilih setelah PostgreSQL memastikan akun aktif memiliki kontak pemulihan terverifikasi yang cocok.

Untuk menjaga konsistensi, operasi pembuatan akun, profil, penerimaan Terms/Privacy, consent opsional, dan event audit yang terkait harus menggunakan transaksi sesuai kontrak database dan hanya commit setelah seluruh operasi berhasil. Untuk audit, `actor_type` dan `actor_id` wajib berasal dari konteks server tepercaya, bukan payload klien. Metadata hanya boleh memuat key allowlist non-sensitif; jangan memasukkan password, OTP, token, kontak plaintext, isi chat, atau payload privat.

Login password Owner/Investor, middleware sesi untuk endpoint identitas, refresh/rotasi token, logout perangkat saat ini, logout semua perangkat, serta komponen HTTP recovery telah tersedia. Router registrasi dan recovery belum dipasang pada server default. Approval recovery, password reset, adapter provider OTP nyata, production key provider, middleware sesi pada endpoint fitur, dan pembacaan audit belum tersedia. Tambahkan operasi baru hanya setelah kontrak akses, kebutuhan data minimum, pemeriksaan otorisasi, grant database, dan tes principal aplikasi ditinjau bersama. Jangan memberikan SELECT langsung pada tabel identitas sebagai jalan pintas.
