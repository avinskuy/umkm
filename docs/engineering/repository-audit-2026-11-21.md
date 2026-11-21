# Audit Repository — 21 November 2026

- **Jenis:** Audit dokumentasi, struktur, konfigurasi, dan status implementasi.
- **Baseline commit:** `777e01e2aadc0876ee2df8d401b1c485818abf49` (setelah Tahap 5).
- **Status:** Audit snapshot; bukan sertifikasi keamanan atau bukti kesiapan production.

## 1. Ruang lingkup pemeriksaan

Pemeriksaan mencakup:

- Struktur repository dan direktori frontend/backend.
- README, panduan kontribusi, kebijakan keamanan, dan konfigurasi ignore.
- Frontend manifest, entry point, komponen awal, test, dan konfigurasi Vite.
- Backend manifest, runtime config, HTTP router, database config/pool/migrator, serta PostgreSQL integration test.
- Workflow CI dan hasil pemeriksaan pada baseline commit.
- Dokumen produk, arsitektur, Identity Core, security/privacy, engineering, dan operational readiness.
- Tautan Markdown relatif pada seluruh 46 file Markdown yang tersedia pada snapshot audit.

## 2. Kondisi implementasi yang terverifikasi

### Frontend

- Stack: React 19, TypeScript, dan Vite.
- Script kualitas tersedia: format check, typecheck, lint, test, build, dan preview.
- Aplikasi saat ini masih berupa shell awal yang menampilkan identitas USahapintar.
- Alur autentikasi, profil, discovery, posting, koneksi, pesan, membership, Admin, notifikasi, dan riwayat aktivitas belum diimplementasikan.

### Backend

- Stack: Rust, Axum, Tokio, dan SQLx 0.8.
- HTTP API saat ini menyediakan `GET /api/v1/health`.
- Konfigurasi PostgreSQL melalui `DATABASE_URL`, connection pool, dan SQLx migrator tersedia.
- PostgreSQL 16 digunakan sebagai layanan integration test di CI.
- HTTP server belum menginisialisasi koneksi database pada startup.
- Belum ada migrasi skema domain; direktori migration memang sengaja belum berisi skema produk.

### CI dan pengujian

Workflow CI yang ada menjalankan:

- **Frontend:** `npm ci`, dependency audit, format check, typecheck, lint, test, dan build.
- **Backend:** `cargo fmt`, Clippy dengan warning sebagai error, workspace tests, PostgreSQL integration test, dan build.

Pada baseline commit di atas, workflow pasca-merge run `44920954366` selesai dengan hasil **Frontend quality: success** dan **Backend quality: success**.

Pemeriksaan berikut belum menjadi langkah otomatis pada workflow saat audit: secret scanning khusus, SAST, DAST, license compliance scan, SBOM/provenance/signing, E2E browser tests, accessibility automation, serta load/performance tests. Sebagian sudah menjadi arah kebijakan, tetapi tidak boleh diklaim telah berjalan sebelum workflow dan buktinya tersedia.

### Repository hygiene

- `.gitignore` mencakup file environment, PEM/key, Node modules, build output, Rust target, database lokal, dan log.
- Lockfile frontend dan Rust tersedia.
- CI menggunakan PostgreSQL sementara dengan kredensial khusus pengujian.
- Tidak ditemukan direktori `infra/` atau `scripts/` pada snapshot; README telah diselaraskan agar tidak menyajikannya sebagai direktori yang sudah tersedia.
- Workflow saat ini adalah quality CI; belum ada pipeline release/deployment production yang terdokumentasi sebagai workflow aktif.

## 3. Ketertinggalan dokumentasi yang diperbaiki

Audit menemukan dan memperbarui:

1. **Status repository:** README utama kini membedakan kebijakan yang sudah disetujui dari fitur yang benar-benar telah diimplementasikan. Panduan frontend ditambahkan dan README backend ditautkan.
2. **Security disclosure:** SECURITY.md kini merujuk indeks kebijakan keamanan dan kesiapan operasional, serta menjelaskan bahwa persetujuan kebijakan bukan bukti penerapan.
3. **Role dan rute autentikasi:** nilai role kanonis `business_owner` diselaraskan pada sitemap; dokumen autentikasi menggunakan rute produk yang konsisten; kemungkinan role migration melalui support/Admin dihapus.
4. **UX dan persetujuan:** ketentuan analytics diperbarui agar selaras dengan Consent & Data Processing Policy.
5. **Arsitektur database:** status PostgreSQL/SQLx diperbarui dari “direncanakan” menjadi fondasi runtime yang tersedia, dengan penegasan bahwa HTTP startup belum terhubung dan migrasi domain belum ada.
6. **Rancangan skema:** ditambahkan cakupan konseptual untuk consent records, account deletion requests, data retention records, data export requests, security notifications, delivery attempts, moderation appeals, serta account activity history sebagai read model.
7. **Identity Core:** batas sesi 7 hari idle/30 hari absolut dan role kanonis diselaraskan dengan keputusan yang sudah disetujui.
8. **Transparansi pengguna:** sitemap kini mencakup Riwayat Aktivitas Akun, alur banding, dan antrean banding Admin.
9. **Indeks dan tautan:** indeks keamanan, standar engineering, dan referensi operasional saling terhubung.

Pemeriksaan tautan Markdown relatif pada seluruh 46 file Markdown dalam snapshot audit tidak menemukan tautan rusak.

## 4. Kesenjangan implementasi yang masih terbuka

Kesenjangan berikut bukan kesalahan dokumentasi; ini adalah pekerjaan implementasi yang memang belum dilakukan dan harus dikelola sebagai roadmap:

### Produk dan Identity Core

- Implementasi registrasi, verifikasi email/WhatsApp, login, OAuth, sesi, recovery, dan Admin MFA.
- Pemilihan penyedia email/WhatsApp, konfigurasi pengiriman, monitoring, retry, serta pembatasan biaya.
- Implementasi rate limit OTP lintas sesi/instance dan state machine secara atomik.
- Migrasi database Identity Core dan integration tests untuk constraint, concurrency, expiry, recovery, dan role immutability.
- Implementasi profil, bisnis, posting, discovery, koneksi, messaging, membership, moderasi, dan Admin workflows.

### Privasi dan tata kelola

- Implementasi consent capture/withdrawal, data export, deactivation, account deletion, retention enforcement, dan backup deletion reconciliation.
- Implementasi notification service dan delivery tracking.
- Implementasi moderation appeal workflow dan halaman Riwayat Aktivitas Akun.
- Peninjauan hukum/kontraktual atas privasi, pemrosesan data, vendor, transfer data, dan masa retensi sebelum peluncuran publik.

### Operasional dan keamanan

- Instrumentasi logs/metrics/traces, dashboard, alert, serta pemilik eskalasi.
- Backup terjadwal, pengujian restore, dan target RTO/RPO berbasis analisis dampak.
- Runbook individual untuk komponen aktual; katalog runbook bukan bukti bahwa prosedurnya sudah dibuat atau diuji.
- Release/deployment approval, artifact traceability, health verification, dan rollback yang diuji.
- Otomatisasi secret scanning, SAST/DAST, license checks, SBOM/provenance, dan security regression sesuai prioritas risiko.
- E2E, accessibility/usability, dan performance tests untuk alur produk ketika tersedia.
- Kanal pelaporan kerentanan privat yang mudah ditemukan dan benar-benar dapat digunakan perlu dipastikan sebelum repository/produk dipromosikan secara luas.

## 5. Hal yang sengaja tidak diubah

- Tidak membuat migrasi skema domain sebelum review desain dan gerbang implementasi Identity Core terpenuhi.
- Tidak menambahkan direktori placeholder atau modul produk kosong.
- Tidak menetapkan angka kuota, harga Premium, SLO/SLA, RTO/RPO, retensi numerik, atau jadwal operasional tanpa keputusan dan pengukuran yang mendukung.
- Tidak mengubah kebijakan role immutable, membership, recovery, privasi, moderasi, atau kewenangan Admin yang telah disetujui.
- Tidak menambahkan dependency atau mengubah workflow CI pada audit dokumentasi ini.
- Tidak melakukan deployment production.

## 6. Catatan pemeliharaan

Beberapa dokumen teknis/produk lama masih menggunakan Bahasa Inggris, termasuk rancangan skema database, desain arsitektur autentikasi, membership, sitemap, dan spesifikasi UI/UX. Dokumen tersebut diperbarui pada bagian yang terdampak keputusan terbaru, tetapi tidak diterjemahkan massal dalam audit ini agar perubahan bahasa tidak menggeser makna kontrak teknis. Lokalisasi dapat dilakukan bertahap pada review berikutnya.

Audit ini merupakan snapshot kondisi repository pada 21 November 2026. Perubahan kode, dependency, workflow, arsitektur, atau kebijakan setelah tanggal/commit baseline harus memicu pembaruan audit yang relevan.
