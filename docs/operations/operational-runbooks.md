# Operational Runbooks

- **Status:** Disetujui sebagai baseline.
- **Cakupan:** Prosedur berulang untuk operasi, diagnosis, respons, pemulihan, dan pemeliharaan USahapintar.

## 1. Tujuan

Runbook memberikan langkah kerja yang konsisten kepada petugas berwenang untuk menangani tugas operasional dan gangguan. Runbook harus cukup jelas untuk diikuti tanpa menebak-nebak, tetapi tidak boleh memuat secret atau membuka detail sistem yang tidak diperlukan.

Runbook adalah prosedur praktis, bukan pengganti kebijakan, penilaian risiko, atau kewenangan persetujuan.

## 2. Prinsip penulisan

Setiap runbook harus:

- Memiliki nama, tujuan, ruang lingkup, pemilik, dan tanggal review.
- Menjelaskan kapan prosedur digunakan dan kapan harus dihentikan.
- Menyebut prasyarat, izin, risiko, dan dampak.
- Menggunakan langkah bernomor dengan hasil yang diharapkan.
- Menyertakan verifikasi sebelum dan sesudah tindakan.
- Menjelaskan eskalasi, rollback, dan kondisi selesai.
- Menghindari kredensial, token, alamat internal sensitif, data pribadi, dan detail produksi yang tidak aman dipublikasikan.
- Diuji pada environment aman sebelum diandalkan untuk tindakan kritis.

## 3. Template runbook

Gunakan struktur berikut untuk runbook individual:

### Metadata
- Nama dan ID runbook.
- Pemilik dan reviewer.
- Versi/tanggal review.
- Severity atau tingkat risiko yang relevan.

### Tujuan dan pemicu
- Masalah atau tugas yang ditangani.
- Indikator/pemicu untuk memulai.
- Kondisi yang tidak termasuk cakupan.

### Prasyarat dan keselamatan
- Role/izin yang diperlukan.
- Environment dan alat yang disetujui.
- Backup atau snapshot yang diperlukan.
- Dampak yang mungkin terjadi.
- Larangan dan kondisi untuk menghentikan prosedur.

### Langkah kerja
Untuk setiap langkah, tuliskan tindakan, hasil yang diharapkan, dan cara verifikasi. Pisahkan tindakan baca-saja dari tindakan yang mengubah sistem.

### Pemulihan dan eskalasi
- Cara membatalkan atau memulihkan tindakan jika aman.
- Kondisi eskalasi ke Incident Response.
- Kontak/peran yang harus diberi tahu.

### Penutupan
- Pemeriksaan akhir.
- Catatan audit/tiket yang harus dibuat.
- Bukti dan tindak lanjut.
- Waktu review berikutnya.

## 4. Katalog runbook minimum

Runbook individual perlu dibuat dan diuji ketika komponen terkait tersedia. Katalog awal:

1. **Service health dan diagnosis:** memeriksa status frontend, API, database, dependency, dan health check.
2. **API/service tidak tersedia:** validasi dampak, periksa log/metric, eskalasi, dan pemulihan.
3. **Database connectivity atau capacity:** diagnosis koneksi, pool, storage, dan query tanpa mengekspos data.
4. **Backup gagal:** validasi status, hentikan asumsi backup sehat, perbaiki penyebab, dan verifikasi backup berikutnya.
5. **Database/file restore:** pemulihan terkontrol, integritas, migration, dan penerapan ulang deletion records.
6. **Credential/secret compromise:** revoke, rotate, validasi pemakaian, dan koordinasi Incident Response.
7. **Security incident:** klasifikasi, containment, bukti, eskalasi, dan komunikasi.
8. **Release gagal/rollback:** hentikan promosi, verifikasi dampak, jalankan prosedur rollback yang disetujui.
9. **Dependency/provider outage:** nilai dampak, degradasi aman, komunikasi, dan alternatif.
10. **Admin/security access review:** pemeriksaan akses, pencabutan privilege, dan audit.
11. **Account recovery escalation:** penanganan kasus sesuai Identity Core tanpa membuka bukti sensitif.
12. **Data deletion/restore reconciliation:** memastikan data yang telah dihapus tidak muncul kembali.

Katalog bukan klaim bahwa semua runbook individual telah dibuat atau diuji. Tandai status setiap runbook secara eksplisit.

## 5. Keamanan tindakan operasional

- Gunakan akun pribadi/teridentifikasi dan least privilege.
- Hindari berbagi akun administrator.
- Gunakan prosedur approval untuk tindakan berisiko tinggi.
- Jangan menyalin secret atau data L3/L4 ke tiket dan dokumentasi umum.
- Catat aktor, waktu, alasan, target, dan hasil tindakan sensitif.
- Jangan menjalankan perintah destruktif tanpa memastikan environment, target, backup, dan dampaknya.
- Perubahan production mengikuti Release Management & Change Control.

## 6. Hubungan dengan kebijakan lain

- [Incident Response](incident-response.md) untuk insiden dan eskalasi.
- [Business Continuity & Disaster Recovery](business-continuity-disaster-recovery.md) untuk kesinambungan dan pemulihan.
- [Observability & Monitoring](observability-monitoring.md) untuk diagnosis dan alert.
- [Release Management & Change Control](release-management-change-control.md) untuk perubahan dan deployment.
- [Logging, Monitoring & Audit Trail](../security/logging-monitoring-audit.md) untuk pencatatan dan perlindungan log.
- [Database Security & Data Integrity](../security/database-security-data-integrity.md) untuk operasi database.

## 7. Pemeliharaan dan pengujian

- Review runbook setelah insiden, perubahan arsitektur, atau perubahan penyedia.
- Uji prosedur pada environment non-production bila memungkinkan.
- Catat tanggal, peserta, hasil, kegagalan, dan perbaikan.
- Hapus atau tandai langkah usang ketika implementasi berubah.
- Jangan menganggap prosedur berhasil sebelum hasilnya diverifikasi.

Pemilik, jadwal review, dan prioritas katalog ditetapkan sesuai kapasitas operasional. Runbook yang belum diuji harus diberi status belum tervalidasi.
