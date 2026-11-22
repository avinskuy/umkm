# USahapintar

**Temukan peluang. Bangun koneksi.**

Platform bisnis dan jejaring investor untuk menemukan peluang, membangun koneksi, dan mengembangkan kerja sama.

## Status proyek

USahapintar menggunakan arsitektur modular monolith dengan frontend React + TypeScript, backend Rust + Axum, dan PostgreSQL melalui SQLx.

Fondasi repository, standar engineering, kebijakan produk dan keamanan, serta Identity Core telah tersedia. Implementasi fitur produk masih bertahap.

### Implementasi yang tersedia

- Backend menyediakan liveness `GET /api/v1/health` dan readiness `GET /api/v1/readyz`.
- PostgreSQL runtime menggunakan principal aplikasi melalui `APP_DATABASE_URL`. Migration dijalankan secara eksplisit oleh principal migrator dan tidak dijalankan saat HTTP server startup.
- Lima belas migration Identity Core tersedia dan diuji melalui PostgreSQL CI.
- Login Business Owner dan Investor menggunakan email/nomor telepon serta password. Login rutin tidak meminta OTP.
- Registrasi menggunakan password dan verifikasi OTP melalui email atau WhatsApp. Service dan router HTTP tersedia, tetapi router registrasi belum dipasang pada server default.
- Session validation, idle timeout, absolute expiry, token rotation, logout, dan logout seluruh perangkat tersedia pada endpoint identitas.
- Account Recovery memiliki database operations, service core, dan router HTTP. Router belum dipasang pada server default; alur saat ini berhenti pada status `under_review`.
- Provider OTP nyata, key provider produksi, persetujuan recovery, password reset, dan integrasi fitur produk belum tersedia.
- Frontend masih berupa shell awal dan belum mengimplementasikan alur produk utama.

Ketersediaan kode atau dokumen tidak dengan sendirinya membuktikan bahwa fitur telah diaktifkan di production atau seluruh kontrol telah memenuhi sertifikasi tertentu.

## Struktur repository

- `frontend/` — aplikasi web React, TypeScript, dan Vite.
- `backend/` — API Rust/Axum, modul domain, PostgreSQL, dan SQLx migrations.
- `docs/` — dokumentasi produk, arsitektur, keamanan, engineering, dan operasional.

Panduan masing-masing aplikasi:
- [Frontend](frontend/README.md)
- [Backend](backend/README.md)
- [Engineering standards](docs/engineering/standards.md)
- [Contributing](CONTRIBUTING.md)

## Prinsip pengembangan

- Pertahankan modular monolith sampai kebutuhan nyata membenarkan pemisahan layanan.
- Kelompokkan kode berdasarkan fitur atau domain dan hindari abstraksi tanpa pengguna yang jelas.
- Audit struktur, tanggung jawab, dan implementasi yang sudah ada sebelum menambahkan kode.
- Setiap perubahan harus memiliki pengujian yang sesuai dan dapat ditinjau.
- Simpan secret hanya pada lingkungan server atau secret manager.
- Tegakkan autentikasi dan otorisasi pada backend, termasuk pemeriksaan kepemilikan resource.
- Selaraskan dokumentasi dengan perilaku kode yang benar-benar tersedia.
