# Strategi Pengujian & Quality Assurance

- **Status:** Disetujui sebagai baseline
- **Tanggal:** 21 November 2026
- **Cakupan:** Frontend, backend, API, database, keamanan, pengalaman pengguna, dan release

## 1. Tujuan dan referensi

Pengujian harus membuktikan bahwa fitur bekerja sesuai kebutuhan, aman, tidak merusak perilaku yang sudah ada, dan dapat dipelihara.

Acuan utama:

- NIST IR 8397 — Guidelines on Minimum Standards for Developer Verification of Software.
- OWASP ASVS 5.0.0 — baseline verifikasi keamanan; Level 2 menjadi referensi target kontrol yang relevan terhadap risiko, bukan klaim sertifikasi.
- OWASP SAMM — kerangka peningkatan kematangan keamanan.
- NIST SP 800-53 — kontrol keamanan dan evaluasi.
- NCSC Software Security Code of Practice.

## 2. Tingkatan pengujian

Gunakan tingkat pengujian yang sesuai dengan risiko dan ketersediaan komponen:

- **Unit test:** logika domain, validasi, utilitas terarah, dan komponen UI.
- **Integration test:** interaksi antar-modul, API, persistence, dan dependency.
- **API test:** kontrak endpoint, autentikasi, otorisasi, validasi, dan respons.
- **Database test:** constraint, transaksi, migration, concurrency, dan integritas.
- **End-to-end test:** alur pengguna penting dari awal hingga selesai.
- **Security test:** kontrol akses, input, konfigurasi, dependency, dan skenario penyalahgunaan.
- **Regression test:** memastikan perbaikan dan perubahan tidak merusak perilaku sebelumnya.
- **Performance test:** latency, throughput, resource use, dan stabilitas.
- **Accessibility dan usability test:** aksesibilitas serta kemudahan penggunaan pada kondisi pengguna yang beragam.

Tidak semua perubahan membutuhkan seluruh jenis tes. Pilih berdasarkan dampak dan risiko, lalu dokumentasikan pengecualian yang relevan.

## 3. Cakupan fitur produk

Ketika modul terkait tersedia, siapkan pengujian untuk:

- Identity Core: registrasi, verifikasi, login, recovery, sesi, dan lifecycle akun.
- Role dan authorization: Business Owner, Investor, Admin, serta akses resource.
- Profil: kepemilikan, privasi, dan perubahan data.
- Posting dan moderasi: publikasi, edit, penghapusan, laporan, dan banding.
- Connections: permintaan, persetujuan, penolakan, pembatalan, dan pembatasan.
- Messaging: percakapan hanya setelah koneksi diterima dan akses sesuai peserta.
- Membership: Free/Premium, entitlement, kuota, downgrade, dan konsistensi transaksi.
- Dokumen: klasifikasi, izin, persetujuan pemilik, dan pencabutan akses.
- Admin: kewenangan, audit, pembatasan, dan moderation appeal.
- Privacy: pengunduhan data, koreksi, deactivation, dan penghapusan akun.

Pengujian harus mengikuti keputusan produk yang sudah disetujui. Jangan menambahkan perilaku baru melalui asumsi test.

## 4. Pengujian keamanan

Prioritaskan:

- BOLA/IDOR, BFLA, dan BOPLA.
- Authentication bypass dan kesalahan session lifecycle.
- SQL injection dan validasi input.
- Rate limit, abuse prevention, serta resource exhaustion.
- Kebocoran data lintas akun dan field L3/L4.
- Privilege escalation dan penyalahgunaan Admin.
- Business logic abuse dan race condition.
- SSRF, file upload, integrasi eksternal, serta webhook replay bila fitur terkait tersedia.
- Kesalahan konfigurasi dan secret leakage.

Temuan harus dicatat, dinilai tingkat risikonya, diperbaiki, dan diuji ulang. Jangan mempublikasikan detail kerentanan yang dapat dieksploitasi.

## 5. Test environment dan test data

- Pisahkan test dari production.
- Gunakan synthetic atau anonymized data.
- Jangan memakai data pribadi production dalam pengujian biasa.
- Secret test harus berbeda dari secret production.
- Pengujian destruktif hanya dilakukan pada environment yang aman.
- Pastikan test tidak menghubungi layanan berbayar atau pengguna nyata tanpa kebutuhan dan izin.
- Jaga hasil pengujian agar tidak membocorkan data sensitif.

## 6. Otomatisasi dan CI

Gerbang otomatis diterapkan secara bertahap sesuai stack dan kebutuhan:

- Formatting, lint, dan type checking.
- Unit dan integration tests.
- API dan database tests.
- Migration checks pada database uji.
- Frontend/backend build.
- Dependency vulnerability dan license checks.
- Secret scanning.
- SAST dan DAST jika relevan serta tersedia.
- Regression checks untuk bug dan alur kritis.

Perubahan yang gagal pada pemeriksaan wajib tidak boleh dianggap siap dirilis. Workflow CI sendiri juga harus mengikuti kebijakan Infrastructure & Deployment Security.

## 7. Regression dan compatibility

- Bug penting yang diperbaiki harus memiliki regression test jika dapat diuji otomatis.
- Perubahan API harus menilai kompatibilitas client.
- Migration diuji terhadap schema/data yang relevan.
- Perubahan satu modul harus memeriksa dampak terhadap modul yang bergantung padanya.
- Fitur yang sudah bekerja harus tetap diuji pada perubahan terkait.

## 8. Performance, accessibility, dan usability

- Uji endpoint dan query penting menggunakan workload yang representatif.
- Lakukan load/stress/soak test di environment aman ketika kapasitas atau perubahan membenarkannya.
- Uji keyboard navigation, fokus, label, screen reader, kontras, dan pembesaran teks pada alur penting.
- Uji instruksi, validasi, error, dan pemulihan alur dengan pengguna yang memiliki tingkat literasi digital beragam.
- Catat keterbatasan jika usability test belum dapat dilakukan.

## 9. Severity dan release quality gate

| Severity | Kebijakan release |
| --- | --- |
| Critical | Release ditahan sampai diperbaiki dan diverifikasi |
| High | Release ditahan kecuali ada keputusan risiko yang eksplisit, terdokumentasi, dan disetujui pihak berwenang |
| Medium | Dinilai berdasarkan dampak, mitigasi, dan rencana perbaikan |
| Low | Dicatat dan dijadwalkan sesuai prioritas |

Bug management mencatat identitas temuan, langkah reproduksi, dampak, severity, pemilik, status, perbaikan, dan hasil verifikasi ulang.

## 10. Metrik dan bukti QA

Metrik dapat mencakup test pass/fail, cakupan pengujian berdasarkan risiko, bug menurut severity, regression result, build success, kerentanan dependency, temuan aksesibilitas, serta hasil performa.

Jangan menetapkan target coverage tunggal secara arbitrer. Prioritaskan autentikasi, otorisasi, recovery, transaksi, privasi, dan business logic.

Dokumentasi minimum: test strategy, test plan/cases, test results, bug reports, security verification, dan release quality checklist. Bukti harus dapat ditelusuri ke fitur, perubahan kode, dan versi yang diuji.

## Referensi

- NIST IR 8397.
- OWASP ASVS 5.0.0.
- OWASP SAMM.
- NIST SP 800-53.
- NCSC Software Security Code of Practice.
