# Secure Software Development Lifecycle (SSDLC)

- **Status:** Disetujui sebagai baseline
- **Tanggal:** 21 November 2026
- **Cakupan:** Perencanaan, desain, implementasi, verifikasi, build, release, dan pemeliharaan

## 1. Referensi utama

- NIST Secure Software Development Framework (SSDF) v1.1.
- OWASP Software Assurance Maturity Model (SAMM) v2.
- Microsoft Security Development Lifecycle (SDL).

Acuan digunakan sebagai praktik dasar dan kerangka peningkatan bertahap. Penerapan disesuaikan dengan risiko dan kematangan proyek.

## 2. Prinsip

- **Security by Design:** keamanan dipertimbangkan sejak perencanaan.
- **Shift Left:** temukan masalah sedini mungkin.
- **Defense in Depth:** gunakan beberapa lapis kontrol.
- **Least Privilege:** batasi akses dan kemampuan.
- **Risk-Based Security:** prioritaskan ancaman dengan dampak terbesar.
- **Continuous Improvement:** gunakan hasil review, test, dan insiden untuk meningkatkan proses.

## 3. Tahapan SSDLC

### Perencanaan dan kebutuhan

- Tetapkan kebutuhan keamanan dan privasi untuk setiap fitur.
- Tentukan data yang diproses dan klasifikasi L1–L4.
- Identifikasi role, pemilik data, tujuan, serta batas akses.
- Tentukan kriteria penerimaan dan pengujian keamanan yang relevan.

### Desain dan threat modeling

- Identifikasi aset, trust boundaries, entry points, aktor, dan dependency.
- Tinjau penyalahgunaan yang mungkin terjadi, termasuk akses lintas akun dan eskalasi privilege.
- Rancang mitigasi sebelum implementasi.
- Catat keputusan arsitektur dan risiko yang belum terselesaikan.

### Implementasi

- Ikuti standar coding frontend dan backend.
- Validasi input tidak tepercaya di backend.
- Tegakkan otorisasi pada setiap request dan tingkat objek.
- Gunakan parameterized queries, error handling aman, serta pengelolaan secret yang benar.
- Hindari dependency yang tidak perlu dan periksa dependency yang ditambahkan.
- Jangan masukkan data produksi atau secret ke source control.

### Review dan verifikasi

- Tinjau perubahan kode dan workflow.
- Jalankan unit, integration, security, regression, dan test lain sesuai risiko.
- Jalankan dependency, secret, konfigurasi, dan source scanning yang relevan.
- Periksa migration, API contract, authorization, dan data lifecycle.
- Catat temuan, perbaikan, pengecualian, serta hasil verifikasi ulang.

### Build dan release

- Gunakan lockfile dan proses build yang dapat diulang sejauh memungkinkan.
- Lindungi pipeline, kredensial, artifact, dan izin deployment.
- Verifikasi hasil CI dan artifact sebelum release.
- Production deployment membutuhkan persetujuan eksplisit terpisah.
- Sediakan catatan release, health check, dan prosedur rollback/pemulihan.

### Pemeliharaan dan respons kerentanan

- Pantau advisori dependency dan kerentanan yang relevan.
- Prioritaskan patch berdasarkan risiko dan paparan.
- Tinjau perubahan konfigurasi serta akses.
- Ikuti proses pelaporan kerentanan dan incident response.
- Perbarui dokumentasi dan test setelah perbaikan.

## 4. Repository publik dan open-source readiness

Repository publik boleh memuat source code, dokumentasi arsitektur yang disanitasi, schema/migration aman, API spec tanpa kredensial, synthetic test data, dan workflow yang aman.

Jangan publikasikan API key, password, token, private key, sertifikat privat, production environment, database production, PII, OTP, session data, bukti recovery, backup production, konfigurasi sensitif, atau detail investigasi internal.

Perubahan workflow dan file konfigurasi keamanan harus ditinjau. Gunakan secret scanning, dependency checks, SAST, DAST jika sesuai, dan proses responsible vulnerability disclosure. Visibilitas repository publik bukan izin untuk membuka sistem production.

## 5. Proporsionalitas dan bukti

Terapkan kontrol berdasarkan risiko dan tahap proyek. Tidak semua proses enterprise diperlukan sejak hari pertama. Namun, kontrol dasar autentikasi, otorisasi, validasi, secret, dependency, test, dan review tidak boleh diabaikan.

Catat bukti yang relevan: requirement keamanan, threat model, review, hasil tes, scan, keputusan risiko, dan release verification.

## Referensi

- NIST SSDF v1.1.
- OWASP SAMM v2.
- Microsoft Security Development Lifecycle.
