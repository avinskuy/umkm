# Backend

API merupakan layanan Rust/Axum dalam paket workspace `usahapintar-api`.

## Konfigurasi runtime

Server membaca alamat bind dari variabel lingkungan:

| Variabel | Nilai bawaan | Keterangan |
| --- | --- | --- |
| `HOST` | `127.0.0.1` | Alamat IP untuk bind. Mendukung literal IPv4 dan IPv6. |
| `PORT` | `5000` | Port TCP. Harus berupa bilangan bulat dari 1 sampai 65535. |

Jika salah satu variabel tidak diisi, server menggunakan nilai bawaannya. Nilai yang tidak valid akan menghentikan proses startup dengan kesalahan konfigurasi, bukan diam-diam diganti dengan nilai bawaan.

Alamat loopback bawaan menjaga API tetap dapat diakses dari host itu sendiri. Atur `HOST=0.0.0.0` hanya jika lingkungan deployment memang memerlukan antarmuka yang dapat dijangkau dari luar. Lindungi layanan menggunakan kontrol jaringan atau reverse proxy yang sesuai. Jangan membuka API secara langsung ke internet publik tanpa tinjauan keamanan deployment yang jelas.

Aplikasi tidak memuat file `.env` dengan sendirinya. Atur variabel lingkungan melalui shell, service manager, container runtime, atau platform deployment.

## PostgreSQL

Runtime HTTP memakai principal aplikasi dengan hak minimum dan hanya membaca konfigurasi koneksi dari:

| Variabel | Wajib | Nilai bawaan | Keterangan |
| --- | --- | --- | --- |
| `APP_DATABASE_URL` | Ya | — | URL koneksi PostgreSQL milik `usahapintar_app_login`; jangan gunakan kredensial DBA atau migrator. |
| `DATABASE_MAX_CONNECTIONS` | Tidak | `10` | Batas connection pool; rentang yang diterima 1–100. |

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
APP_DATABASE_URL='postgres://usahapintar_app_login:local_password@127.0.0.1:5432/usahapintar_dev' HOST=127.0.0.1 PORT=5000 cargo run --locked -p usahapintar-api
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
