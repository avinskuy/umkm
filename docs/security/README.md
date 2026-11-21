# Kebijakan Keamanan, Privasi & Perlindungan Data

Dokumen dalam direktori ini merangkum baseline keamanan, privasi, dan tata kelola data USahapintar yang telah disetujui pada 21 November 2026. Kebijakan mengadopsi praktik dari standar dan panduan industri yang relevan, lalu menerapkannya secara proporsional terhadap tahap proyek, arsitektur modular monolith, risiko, dan kapasitas operasional.

## Status dan cara membaca

- **Disetujui** berarti kebijakan menjadi acuan desain dan implementasi.
- Persetujuan kebijakan **tidak berarti kontrol sudah diterapkan, diuji, atau tersertifikasi**.
- Dokumen arsitektur, keputusan produk, dan kebijakan terkait harus dibaca bersama jika membahas alur yang sama.
- Detail seperti vendor, algoritma, konfigurasi, batas numerik, jadwal, dan tooling ditetapkan saat implementasi atau peninjauan risiko; jangan mengarang nilai yang belum diputuskan.
- Jika ada konflik, jangan menyelesaikannya dengan asumsi. Catat konflik dan lakukan peninjauan keputusan terkait.

## Daftar kebijakan

| Dokumen | Cakupan |
| --- | --- |
| [Access Control & Authorization](access-control-authorization.md) | RBAC + ABAC, least privilege, otorisasi per permintaan, akses Admin |
| [Data Classification & Sensitivity](data-classification.md) | Klasifikasi L1–L4 dan kontrol berdasarkan sensitivitas |
| [Encryption & Key Management](encryption-key-management.md) | Enkripsi, password hashing, secret, dan pengelolaan kunci |
| [Logging, Monitoring & Audit Trail](logging-monitoring-audit.md) | Pencatatan peristiwa, audit Admin, monitoring, dan perlindungan log |
| [API & Integration Security](api-integration-security.md) | Keamanan endpoint, validasi, OAuth, webhook, dan integrasi eksternal |
| [Database Security & Data Integrity](database-security-data-integrity.md) | PostgreSQL, SQLx, privilege, transaksi, migration, backup, dan integritas |
| [Infrastructure & Deployment Security](infrastructure-deployment-security.md) | Hardening server, jaringan, environment, CI/CD, release, dan recovery |
| [Dependency & Third-Party Component Security](dependency-third-party-security.md) | Inventaris dependency, supply chain, advisori, SDK, dan komponen vendor |
| [Privacy & Data Control](privacy-data-control.md) | Default privasi, visibilitas profil/postingan, persetujuan berbagi, dan kendali pengguna |
| [Consent & Data Processing](consent-data-processing.md) | Tujuan dan dasar pemrosesan, persetujuan opsional, perubahan tujuan, dan pencabutan |
| [Account Lifecycle & Data Retention](account-lifecycle-data-retention.md) | Penonaktifan, penghapusan 30 hari, retensi, backup, restore, dan pelepasan kontak |
| [Third-Party Data Sharing & Vendor Protection](third-party-data-sharing-vendor-protection.md) | Kategori penerima, pembagian data, kewajiban vendor, subprocessor, dan transfer data |
| [Security Notifications & Alerts](security-notifications-alerts.md) | Notifikasi peristiwa keamanan, kanal, isi minimum, dan perlindungan informasi |
| [Admin Enforcement](admin-enforcement-policy.md) | Kewenangan Admin, peringatan, pembatasan fitur, suspensi, dan pemulihan |
| [Moderation & Appeals](moderation-appeals-policy.md) | Peninjauan konten/laporan, banding, hasil, dan pencatatan keputusan |
| [Account Activity History](account-activity-history.md) | Riwayat tindakan moderasi yang dapat dilihat pemilik akun |

## Hubungan dengan dokumen lain

- [Standar Engineering](../engineering/standards.md)
- [Gambaran Arsitektur](../architecture/overview.md)
- [Rancangan Skema Database PostgreSQL](../architecture/database-schema-design.md)
- [Keputusan Pendaftaran Identity Core](../architecture/identity-core-registration-decisions.md)
- [Tinjauan Skema Identity Core — Verifikasi dan Pemulihan](../architecture/identity-core-shared-recovery-contacts.md)
- [Authentication & Identity Security Design](../architecture/authentication-and-identity-security-design.md)
- [Prinsip Pengalaman Pengguna yang Inklusif](../product/prinsip-pengalaman-pengguna.md)
- [Kebijakan Keamanan Repository](../../SECURITY.md)
- [Panduan Kontribusi](../../CONTRIBUTING.md)

## Referensi utama

Setiap kebijakan mencantumkan acuan yang relevan. Referensi digunakan sebagai dasar praktik, bukan pernyataan bahwa USahapintar telah diaudit atau memenuhi seluruh persyaratan suatu standar. Persyaratan hukum dan kontraktual harus ditinjau sesuai yurisdiksi serta layanan yang benar-benar digunakan.
