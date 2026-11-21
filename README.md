# USahapintar

**Platform Promosi Bisnis, Intelijen, dan Jejaring Investor**

USahapintar menghubungkan pemilik usaha dengan investor melalui penemuan bisnis, peluang kemitraan, dan informasi bisnis berbasis data.

> Status proyek: tahap fondasi. Kebijakan produk, arsitektur, keamanan, dan operasional telah didokumentasikan; implementasi fitur produk berjalan bertahap.

## Status implementasi saat ini

- Frontend React + TypeScript masih berupa shell awal proyek.
- Backend Rust + Axum menyediakan endpoint health check `/api/v1/health`.
- Fondasi koneksi PostgreSQL melalui SQLx, connection pool, dan migrator sudah tersedia.
- Server HTTP belum menginisialisasi koneksi database saat startup dan belum ada migrasi skema domain.
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
