# Kebijakan Keamanan Infrastruktur & Deployment

- **Status:** Disetujui sebagai baseline
- **Tanggal:** 21 November 2026
- **Cakupan:** Server, jaringan, environment, repository, CI/CD, release, dan operasi production

## 1. Referensi utama

- CIS Ubuntu Linux Benchmark.
- Ubuntu Security Guide.
- OWASP CI/CD Security Cheat Sheet.
- OWASP GitHub Actions Security Cheat Sheet.
- NIST Secure Software Development Framework (SSDF).

Gunakan panduan yang sesuai dengan sistem operasi, platform, dan tooling yang benar-benar dipilih.

## 2. Inventaris dan kepemilikan

Catat aset infrastruktur, server, layanan, domain, database, penyimpanan, integrasi eksternal, environment, pemilik teknis, serta tujuan penggunaannya. Perbarui inventaris ketika arsitektur berubah.

## 3. Server dan network hardening

- Gunakan OS/runtime yang masih didukung dan terapkan patch keamanan.
- Hardening konfigurasi mengikuti benchmark yang sesuai, dengan pengecualian yang memiliki alasan dan catatan.
- Nonaktifkan layanan dan port yang tidak diperlukan.
- Batasi koneksi masuk dan keluar berdasarkan kebutuhan.
- Batasi akses administratif; gunakan autentikasi kuat dan mekanisme akses yang aman.
- Database dan layanan internal tidak boleh terbuka ke internet publik tanpa alasan, perlindungan, dan tinjauan yang memadai.
- Lindungi konfigurasi reverse proxy, TLS, file permission, dan proses service.

## 4. Pemisahan environment

Development, test, staging, dan production harus dipisahkan secara logis dan menggunakan kredensial/secret masing-masing. Data production tidak boleh disalin ke pengujian biasa tanpa dasar, minimisasi, dan perlindungan yang sesuai.

## 5. Akses dan secret

- Terapkan least privilege pada akun server, repository, cloud/platform, dan pipeline.
- Gunakan identitas terpisah untuk tugas berbeda bila memungkinkan.
- Secret tidak boleh di-commit, dicetak ke log, atau dimasukkan ke artefak publik.
- Sediakan prosedur rotasi dan pencabutan secret.
- Akses production harus dibatasi, dapat ditelusuri, dan ditinjau.

## 6. Keamanan CI/CD dan repository

- Berikan permission minimum pada workflow dan token, misalnya `GITHUB_TOKEN` hanya dengan scope yang diperlukan.
- Tinjau perubahan workflow karena workflow dapat mengakses source, artifact, dan secret.
- Perlakukan pull request eksternal dan input tidak tepercaya sebagai data berisiko.
- Jangan memberikan secret production kepada workflow yang menjalankan kode kontribusi tidak tepercaya.
- Hindari self-hosted runner untuk repository publik jika runner memiliki akses sensitif atau dapat dipengaruhi oleh kode tidak tepercaya.
- Pin GitHub Actions ke commit SHA yang diverifikasi jika praktis; tinjau pembaruan pin secara terkontrol.
- Gunakan dependency/secret scanning dan pemeriksaan build yang relevan.
- Jaga kredensial registry, signing key, dan token deployment.

## 7. Build, artifact, dan release

- Utamakan build yang dapat diulang dengan lockfile dan versi tooling yang terdokumentasi.
- Batasi akses dan perubahan terhadap artifact release.
- Periksa hasil build, dependency, dan konfigurasi sebelum distribusi.
- Catat versi, commit sumber, hasil CI, perubahan penting, dan prosedur rollback.
- Jangan menyertakan secret, data pengguna, backup production, atau konfigurasi privat dalam artifact publik.
- Ikuti [Release Management & Change Control](../operations/release-management-change-control.md) untuk penilaian, persetujuan, deployment, dan rollback.

## 8. Production deployment

- Deployment production memerlukan persetujuan eksplisit dari pihak yang berwenang.
- Kebijakan CI yang lulus tidak otomatis memberikan izin deployment.
- Gunakan proses release yang dapat ditelusuri, health check, dan verifikasi pascadeployment.
- Sediakan rollback atau prosedur pemulihan yang telah dipahami.
- Perubahan darurat harus dicatat dan ditinjau setelah kondisi terkendali.

## 9. Monitoring, patching, dan operasi

Pantau kesehatan layanan, kapasitas, error, aktivitas administratif, dan kejadian keamanan. Tetapkan pemilik respons serta prosedur pemeliharaan. Terapkan patch berdasarkan tingkat risiko, paparan, dan ketersediaan pembaruan yang aman. Gunakan [Observability & Monitoring](../operations/observability-monitoring.md) dan [Operational Runbooks](../operations/operational-runbooks.md) sebagai prosedur pendukung.

## 10. Pengujian dan pemeriksaan

- Tinjau konfigurasi server dan network exposure.
- Verifikasi pemisahan environment dan secret.
- Tinjau permission workflow, token, runner, dan artifact.
- Uji health check, rollback, dan pemulihan di environment aman.
- Periksa dependency dan secret scanning.
- Dokumentasikan pengecualian hardening dan risiko yang diterima.

## 11. Penerapan bertahap

Prioritas awal: inventaris aset, hardening OS dasar, pembatasan jaringan, environment dan secret terpisah, least privilege CI, pemeriksaan dependency/secret, serta persetujuan deployment production. Kontrol lanjutan diterapkan sesuai risiko dan kemampuan proyek.

Mengadopsi kebijakan ini tidak berarti USahapintar telah tersertifikasi atau memenuhi seluruh kontrol benchmark.

## Referensi

- CIS Ubuntu Linux Benchmark.
- Ubuntu Security Guide.
- OWASP CI/CD Security Cheat Sheet.
- OWASP GitHub Actions Security Cheat Sheet.
- NIST SSDF.
