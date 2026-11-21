# Kebijakan Keamanan & Perlindungan Data

Dokumen dalam direktori ini merangkum baseline keamanan USahapintar yang telah disetujui pada 21 November 2026. Kebijakan mengadopsi praktik dari standar dan panduan industri yang relevan, lalu menerapkannya secara proporsional terhadap tahap proyek, arsitektur modular monolith, risiko, dan kapasitas operasional.

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
| [Dependency & Third-Party Component Security](dependency-third-party-security.md) | Inventaris dependency, supply chain, advisori, SDK, dan vendor |

## Hubungan dengan dokumen lain

- [Standar Engineering](../engineering/standards.md)
- [Gambaran Arsitektur](../architecture/overview.md)
- [Rancangan Skema Database PostgreSQL](../architecture/database-schema-design.md)
- [Authentication & Identity Security Design](../architecture/authentication-and-identity-security-design.md)
- [Kebijakan Keamanan Repository](../../SECURITY.md)
- [Panduan Kontribusi](../../CONTRIBUTING.md)

## Referensi utama

Setiap kebijakan mencantumkan acuan yang relevan. Referensi digunakan sebagai dasar praktik, bukan pernyataan bahwa USahapintar telah diaudit atau memenuhi seluruh persyaratan suatu standar.
