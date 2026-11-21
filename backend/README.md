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

Fondasi database menggunakan SQLx dengan PostgreSQL serta runtime Tokio/Rustls.

| Variabel | Wajib | Nilai bawaan | Keterangan |
| --- | --- | --- | --- |
| `DATABASE_URL` | Ya, ketika akses database diinisialisasi | — | URL koneksi PostgreSQL. Berikan melalui secret manager atau lingkungan runtime. |
| `DATABASE_MAX_CONNECTIONS` | Tidak | `10` | Batas connection pool; rentang yang diterima adalah 1–100. |

Lapisan database diinisialisasi secara eksplisit melalui `DatabaseConfig::from_env()` dan `database::connect()`. HTTP server belum terhubung ke database sampai proses startup aplikasi memang diatur untuk melakukannya. Dengan demikian, perilaku aplikasi saat ini yang hanya menyediakan health endpoint tetap tidak berubah.

Migrasi SQLx disertakan dari `backend/migrations` dan dijalankan secara eksplisit melalui `database::migrations::run(&pool)`. Migration pertama Identity Core membuat tabel akun dan identitas Google, disertai constraint dan trigger integritas. Migration tersebut belum dijalankan pada database produksi dan tidak mengaktifkan endpoint registrasi/login. Domain verifikasi, sesi, recovery, audit, dan fitur produk lain tetap menunggu migration terpisah setelah review. Jangan pernah mengubah migrasi yang sudah diterapkan; buat migrasi baru dengan versi berikutnya.

Jangan menyimpan kredensial database di source control, pengujian, log, atau dokumentasi. CI menggunakan layanan PostgreSQL sementara dengan kredensial khusus pengujian.

## Menjalankan secara lokal

Jalankan dari direktori utama repositori:

```sh
HOST=127.0.0.1 PORT=5000 cargo run --locked -p usahapintar-api
```

Untuk menjalankan pengujian integrasi PostgreSQL secara lokal, buat database pengembangan sementara dan atur `DATABASE_URL` sebelum menjalankan pengujian yang secara default diabaikan:

```sh
DATABASE_URL='postgres://app_user:local_password@127.0.0.1:5432/usahapintar_test' cargo test --locked -p usahapintar-api --test postgres -- --ignored
```

Gunakan kredensial khusus lingkungan lokal. Jangan pernah menggunakan ulang kredensial produksi untuk pengembangan atau CI.

Endpoint kesehatan:

```
http://127.0.0.1:5000/api/v1/health
```

## Pemeriksaan kualitas

Jalankan dari direktori utama repositori:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
```
