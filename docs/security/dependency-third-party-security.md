# Kebijakan Keamanan Dependency & Komponen Pihak Ketiga

- **Status:** Disetujui sebagai baseline
- **Tanggal:** 21 November 2026
- **Cakupan:** Dependency langsung/transitif, package registry, SDK, API, cloud service, dan vendor

## 1. Referensi utama

- OWASP Software Component Verification Standard (SCVS).
- Google SLSA.
- OpenSSF Best Practices.
- NIST SSDF.

## 2. Inventaris dan keterlacakan

- Catat dependency langsung dan transitif melalui manifest dan lockfile.
- Commit lockfile yang digunakan untuk build reproducible.
- Dokumentasikan komponen penting, versi, pemilik, dan alasan penggunaannya.
- Buat Software Bill of Materials (SBOM) untuk release ketika alur build mendukungnya.
- Tinjau dependency yang memiliki akses sensitif atau memproses data L3/L4.

## 3. Evaluasi sebelum adopsi

Nilai dependency dan vendor berdasarkan:

- Reputasi dan sumber distribusi.
- Status pemeliharaan, frekuensi pembaruan, dan dukungan.
- Riwayat kerentanan dan respons maintainer.
- Kesesuaian lisensi dan kewajiban atribusi.
- Ukuran, kompleksitas, transitive dependencies, dan permukaan serangan.
- Permission, data yang diakses, serta kebutuhan integrasi.
- Alternatif dan risiko jika komponen tidak lagi tersedia.

Hindari dependency yang tidak diperlukan atau tidak memiliki pemeliharaan yang dapat dipercaya.

## 4. Supply chain dan registry

- Gunakan registry dan sumber package yang dipercaya.
- Periksa nama package untuk menghindari typosquatting atau paket tiruan.
- Verifikasi integritas package sesuai kemampuan ekosistem.
- Lindungi token registry, build credential, signing key, dan pipeline.
- Gunakan provenance dan signature artifact bila didukung dan layak.
- Jangan menjalankan script atau binary dari sumber tidak tepercaya tanpa evaluasi.

## 5. Pemantauan dan pembaruan

- Jalankan dependency vulnerability scanning secara berkala dan pada perubahan dependency.
- Pantau advisori keamanan upstream.
- Perbarui dependency secara terkontrol dengan compatibility review dan test.
- Prioritaskan kerentanan berdasarkan severity, exploitability, exposure, dan dampak pada USahapintar.
- Ganti komponen yang tidak dipelihara atau tidak lagi memenuhi kebutuhan keamanan.
- Catat alasan dan mitigasi jika pembaruan belum dapat dilakukan.

## 6. API, SDK, dan vendor eksternal

- Minta scope dan akses minimum.
- Minimalkan data yang dibagikan.
- Evaluasi keamanan vendor, subprocessor, lokasi/pemrosesan data, serta mekanisme insiden.
- Lindungi dan cabut credential integrasi saat tidak diperlukan atau diduga bocor.
- Tetapkan timeout, batas penggunaan, validasi respons, dan rencana ketika provider gagal.
- Jangan mengandalkan layanan eksternal sebagai satu-satunya sumber kebenaran untuk hak akses internal tanpa verifikasi.

## 7. Penanganan komponen yang terkompromi

Jika dependency atau vendor diduga terkompromi:

1. Identifikasi versi, layanan, artifact, dan data yang terdampak.
2. Batasi atau hentikan penggunaan jika risiko membenarkan.
3. Evaluasi jalur masuk, build, release, serta kemungkinan paparan secret/data.
4. Perbarui, ganti, atau isolasi komponen.
5. Jalankan pengujian keamanan dan regresi.
6. Catat dampak, keputusan, dan tindakan pencegahan.

Ikuti Incident Response Policy untuk kejadian yang memenuhi kriteria insiden.

## 8. Repository publik

Repository boleh memuat manifest, lockfile, SBOM, lisensi, dan catatan risiko yang aman. Jangan mempublikasikan private registry credential, token, secret, atau konfigurasi vendor yang sensitif.

## 9. Baseline penerapan awal

Prioritas awal:

- Lockfile terpelihara dan diperiksa.
- Dependency vulnerability scanning.
- Review lisensi dan dependency baru.
- SBOM untuk release jika alur build mendukung.
- Pembaruan terkontrol disertai test.
- Perlindungan token registry dan credential integrasi.

Provenance lanjutan, signing, dan kontrol supply chain tingkat lanjut diterapkan sesuai kebutuhan dan kemampuan tooling.

## Referensi

- OWASP SCVS.
- Google SLSA.
- OpenSSF Best Practices.
- NIST SSDF.
