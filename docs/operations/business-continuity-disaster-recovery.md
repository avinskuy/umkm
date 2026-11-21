# Kesinambungan Bisnis & Pemulihan Bencana (BCDR)

- **Status:** Disetujui sebagai baseline.
- **Acuan utama:** NIST SP 800-34 Rev. 1.
- **Acuan pelengkap:** AWS Well-Architected Framework, Google Cloud Architecture Framework, dan Microsoft Azure Well-Architected Framework.
- **Cakupan:** Keberlangsungan layanan, backup, pemulihan, gangguan penyedia, dan kesiapan operasional.

## 1. Tujuan

BCDR memastikan layanan penting dapat dipertahankan atau dipulihkan ketika terjadi gangguan, kehilangan data, kegagalan infrastruktur, kesalahan perubahan, atau insiden keamanan.

Rencana harus disesuaikan dengan dampak bisnis, risiko, biaya, dan kemampuan tim. Jangan mengadopsi arsitektur high availability yang kompleks tanpa kebutuhan yang terbukti.

## 2. Analisis dampak dan prioritas layanan

Identifikasi layanan dan dependency yang diperlukan untuk operasi, termasuk:

- Frontend dan API/backend.
- PostgreSQL dan mekanisme migrasi.
- File/document storage bila digunakan.
- Identitas, autentikasi, dan session management.
- DNS, domain, TLS, hosting, dan jaringan.
- Email/OTP dan penyedia notifikasi.
- Integrasi eksternal dan layanan pihak ketiga.
- Repository, CI/CD, artifact, dan konfigurasi deployment.

Untuk setiap layanan, catat fungsi, pemilik, dependency, dampak kegagalan, alternatif, serta urutan pemulihan.

Prioritaskan layanan inti dan integritas data. Tentukan minimum viable service jika sebagian kemampuan belum dapat dipulihkan.

## 3. Strategi kesinambungan dan pemulihan

Tahapan strategi yang disetujui:

1. **Backup & Restore** sebagai baseline awal.
2. **Pilot Light** sebagai opsi peningkatan jika kebutuhan pemulihan dan kapasitas mendukung.
3. **Warm Standby** atau **Active-Active** hanya jika analisis dampak, biaya, risiko, dan kemampuan operasional membenarkannya.

Pilih strategi per layanan, bukan menerapkan satu pola untuk semua komponen. Dokumentasikan asumsi, dependency, dan keterbatasan.

## 4. Backup dan perlindungan salinan

- Backup database, file penting, konfigurasi, dan aset kritis sesuai kebutuhan.
- Enkripsi backup dan batasi akses berdasarkan least privilege.
- Simpan salinan terpisah dari environment production agar kegagalan atau kompromi satu environment tidak otomatis menghapus seluruh salinan.
- Lindungi backup dari pengubahan dan penghapusan tanpa izin.
- Dokumentasikan cakupan, jadwal, retensi, pemilik, lokasi, dan hasil backup.
- Jangan memasukkan secret atau data production ke repository publik.
- Retensi backup harus selaras dengan klasifikasi data, permintaan penghapusan akun, serta kewajiban hukum.

Jadwal, jumlah salinan, media, dan masa retensi numerik ditetapkan setelah infrastruktur dan kebutuhan pemulihan dipastikan.

## 5. Restore dan integritas data

Prosedur restore harus:

- Menggunakan sumber backup yang tervalidasi.
- Dilakukan pada environment terkontrol dengan akses terbatas.
- Memverifikasi konsistensi schema, migration, konfigurasi, dan dependency.
- Memeriksa integritas data setelah pemulihan.
- Menerapkan kembali catatan penghapusan dan perubahan lifecycle yang terjadi setelah backup, agar data yang telah dihapus tidak aktif kembali.
- Memastikan secret dan credential yang perlu dicabut tidak diaktifkan ulang.
- Mencatat sumber backup, waktu, pelaksana, hasil, dan penyimpangan.

Jangan menganggap backup berhasil hanya karena file atau snapshot tersedia; keberhasilan restore harus diuji.

## 6. RTO dan RPO

- **RTO (Recovery Time Objective):** target waktu maksimum pemulihan layanan.
- **RPO (Recovery Point Objective):** batas kehilangan data berdasarkan titik pemulihan yang dapat diterima.

Nilai RTO/RPO belum ditetapkan. Tentukan per layanan melalui analisis dampak bisnis, toleransi kehilangan data, kapasitas, biaya, dan kemampuan teknis. Jangan mencantumkan angka sebagai komitmen sebelum diuji.

## 7. Gangguan penyedia dan dependency eksternal

Untuk setiap dependency kritis, catat:

- Penyedia dan layanan yang digunakan.
- Dampak jika layanan tidak tersedia atau akun penyedia terganggu.
- Status backup/export dan kemampuan pemindahan data.
- Alternatif atau prosedur manual jika tersedia.
- Kontak eskalasi dan informasi status penyedia.
- Batasan kontrak, keamanan, dan pemulihan penyedia.

Jangan menjanjikan failover otomatis jika arsitektur dan kontrak layanan belum mendukungnya.

## 8. Aktivasi dan komunikasi

BCDR dapat diaktifkan ketika gangguan diperkirakan melampaui kemampuan operasi normal atau berdampak signifikan pada layanan/data.

Keputusan aktivasi harus mencatat:

- Pihak yang mengaktifkan dan dasar keputusan.
- Layanan terdampak dan prioritas pemulihan.
- Pemilik setiap aktivitas.
- Risiko terhadap data dan pengguna.
- Komunikasi internal, pengguna, dan penyedia.
- Kriteria untuk kembali ke operasi normal.

Koordinasikan insiden keamanan dengan [Incident Response](incident-response.md).

## 9. Pengujian dan latihan

Uji secara terencana:

- Ketersediaan dan integritas backup.
- Restore database dan file.
- Pemulihan konfigurasi dan dependency.
- Kesesuaian schema/migration.
- Penerapan ulang catatan penghapusan setelah restore.
- Prosedur komunikasi dan pembagian tanggung jawab.
- Kemampuan memenuhi RTO/RPO setelah target ditetapkan.

Catat skenario, lingkungan, hasil, durasi aktual, masalah, dan tindak lanjut. Jadwal latihan ditetapkan berdasarkan risiko dan kapasitas.

## 10. Pengujian minimum

- Backup mencakup aset yang ditetapkan dan aksesnya terbatas.
- Restore diuji, bukan hanya keberadaan backup.
- Pemulihan memverifikasi integritas database dan file.
- Data yang sudah dihapus tidak kembali aktif setelah restore.
- Dependency kritis dan alternatifnya terdokumentasi.
- RTO/RPO ditetapkan berdasarkan analisis dan diuji sebelum menjadi komitmen.
- Aktivasi BCDR memiliki pemilik, catatan, dan jalur komunikasi.

## Referensi

- NIST SP 800-34 Rev. 1, Contingency Planning Guide for Federal Information Systems.
- AWS Well-Architected Framework.
- Google Cloud Architecture Framework.
- Microsoft Azure Well-Architected Framework.
