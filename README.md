# USahapintar

**Platform Promosi Bisnis, Intelijen, dan Jejaring Investor**

USahapintar menghubungkan pemilik usaha dengan investor melalui penemuan bisnis, peluang kemitraan, dan informasi bisnis berbasis data.

> Status proyek: tahap fondasi. Cakupan produk, arsitektur, dan implementasi dikembangkan secara bertahap.

## Struktur proyek

- `frontend/` — aplikasi web React + TypeScript.
- `backend/` — API Rust yang dibangun dengan Axum.
- `docs/` — dokumentasi produk, arsitektur, keamanan, dan rekayasa perangkat lunak.
- `infra/` — aset deployment dan infrastruktur, ditambahkan saat dibutuhkan.
- `scripts/` — tugas pengembangan dan pemeliharaan yang dapat dijalankan ulang.

## Prinsip pengembangan

- Menggunakan modular monolith; tidak memecah layanan menjadi microservices sebelum benar-benar diperlukan.
- Modul frontend disusun berdasarkan fitur dan modul backend berdasarkan domain.
- Setiap file dan tanggung jawab harus memiliki tempat yang jelas.
- Perubahan dibuat dalam ukuran kecil, mudah ditinjau, dan disertai pengujian.
- Tidak menyimpan rahasia di source control.
- Otorisasi role dan akses sumber daya ditegakkan oleh backend.

## Memulai pengembangan

Lihat [CONTRIBUTING.md](CONTRIBUTING.md) untuk alur kerja pengembangan dan dokumentasi di [docs/](docs/).

Petunjuk penyiapan frontend dan backend akan dilengkapi seiring hadirnya modul fungsional pertama.
