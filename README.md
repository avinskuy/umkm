# USahapintar

**Temukan peluang. Bangun koneksi.**

Platform bisnis dan jejaring investor untuk menemukan peluang dan membangun koneksi.

> Status proyek: tahap fondasi. Kebijakan produk, arsitektur, keamanan, dan operasional telah didokumentasikan; implementasi fitur produk berjalan bertahap.

## Status implementasi saat ini

- Frontend React + TypeScript masih berupa shell awal proyek.
- Backend Rust + Axum menyediakan liveness `/api/v1/health` dan readiness `/api/v1/readyz`.
- Pool PostgreSQL melalui SQLx diinisialisasi saat startup menggunakan principal aplikasi dari `APP_DATABASE_URL`.
- Migration Identity Core 0001–0011 tersedia dan diuji melalui PostgreSQL CI. Migration mencakup akun, verifikasi, sesi, pemulihan, audit, penghapusan dukungan OAuth, operasi database verifikasi, pengikatan ID verifikasi, audit runtime verifikasi, serta profil dan penerimaan kebijakan. Server HTTP tidak menjalankan migration otomatis saat startup; penerapan ke database produksi merupakan proses terpisah.
- Backend menyediakan hashing password, `VerificationService` untuk registrasi email/WhatsApp dengan OTP, serta router HTTP registrasi/verifikasi yang diuji tetapi belum dipasang pada aplikasi HTTP default. Kebijakan autentikasi menetapkan OTP wajib saat registrasi, sedangkan login dan login kembali setelah logout cukup menggunakan password. Endpoint login/logout, cookie/session middleware, adapter provider nyata, runtime recovery, dan fitur produk belum tersedia.
- Dokumen kebijakan adalah baseline keputusan, bukan bukti bahwa seluruh fitur/kontrol sudah diimplementasikan atau tersertifikasi.

## Struktur proyek

- `frontend/` — aplikasi web React + TypeScript; lihat [panduan frontend](frontend/README.md).
- `backend/` — API Rust/Axum dan fondasi PostgreSQL/SQLx; lihat [panduan backend](backend/README.md).
- `docs/` — dokumentasi produk, arsitektur, keamanan, engineering, dan operasional.

## Prinsip pengembangan

- Menggunakan modular monolith; tidak memecah layanan menjadi microservices sebelum benar-benar diperlukan.
- Modul frontend disusun berdasarkan fitur dan modul backend berdasarkan domain.
- Setiap file dan tanggung jawab harus memiliki tempat yang jelas.
- Perubahan dibuat dalam ukuran kecil, mudah ditinjau, dan disertai pengujian.
- Tidak menyimpan rahasia di source control.
- Otorisasi role dan akses sumber daya ditegakkan oleh backend.
- Kebijakan dan prosedur operasional harus selaras dengan implementasi aktual serta bukti pengujian.

## Memulai pengembangan

Lihat [CONTRIBUTING.md](CONTRIBUTING.md) untuk alur kontribusi dan [Standar Engineering](docs/engineering/standards.md) untuk pedoman rekayasa.

- [Kebijakan produk dan UX](docs/product/)
- [Dokumentasi arsitektur](docs/architecture/)
- [Kebijakan keamanan dan privasi](docs/security/README.md)
- [Kesiapan operasional](docs/operations/README.md)
- [Audit repository 21 November 2026](docs/engineering/repository-audit-2026-11-21.md)

Petunjuk setup frontend dan backend tersedia pada README masing-masing direktori.
