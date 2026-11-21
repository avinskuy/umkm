# Kebijakan Siklus Hidup Akun & Retensi Data

- **Status:** Disetujui sebagai baseline produk, keamanan, dan tata kelola data.
- **Cakupan:** Penonaktifan, penghapusan akun, retensi data, backup, pemulihan, dan pelepasan identitas utama.
- **Hubungan:** Melengkapi keputusan Identity Core, Database Security, serta Business Continuity & Disaster Recovery.

## 1. Prinsip

- Simpan data hanya selama diperlukan untuk tujuan yang jelas atau kewajiban yang sah.
- Bedakan penonaktifan sementara, permintaan penghapusan, proses penghapusan, dan retensi terbatas.
- Jangan menjanjikan seluruh salinan data hilang seketika ketika pengguna mengajukan penghapusan.
- Data yang dipertahankan untuk kewajiban hukum atau keamanan harus memiliki alasan, akses terbatas, tujuan, dan masa retensi.
- Data yang dihapus tidak boleh kembali aktif setelah backup dipulihkan.
- Jangan menggunakan data yang ditahan untuk marketing, personalisasi, atau tujuan baru yang tidak sesuai.

## 2. Penonaktifan sementara

- Penonaktifan dimulai oleh pengguna melalui alur yang aman.
- Profil publik disembunyikan dan akses normal ke akun dibatasi.
- Data tetap dipertahankan selama penonaktifan, sesuai kebutuhan layanan dan retensi.
- Role utama tidak berubah.
- Pengaktifan kembali memerlukan login dan verifikasi keamanan yang sesuai.
- Penonaktifan bukan penghapusan permanen dan bukan suspensi moderasi oleh Admin.

## 3. Penghapusan permanen oleh pengguna

Alur baseline:

1. Pengguna mengajukan penghapusan secara eksplisit dan melakukan autentikasi ulang.
2. Akun memasuki masa tunggu 30 hari untuk pembatalan.
3. Selama masa tunggu, profil disembunyikan dan akses normal dibatasi.
4. Pengguna dapat membatalkan permintaan melalui pemeriksaan keamanan ulang.
5. Setelah 30 hari, proses penghapusan dimulai. Penyelesaian penghapusan bergantung pada data, dependensi, kewajiban retensi, dan siklus backup.
6. Pengguna diberi penjelasan tentang status, data yang mungkin dipertahankan, dan keterlambatan penghapusan dari backup.

Masa 30 hari adalah masa tunggu sebelum proses penghapusan dimulai, bukan janji bahwa seluruh data sudah hilang pada hari ke-30.

## 4. Data yang dihapus atau dianonimkan

Setelah proses penghapusan dan pemeriksaan kewajiban retensi, hapus atau anonimisasi data pribadi yang tidak lagi diperlukan, sejauh diizinkan dan memungkinkan, termasuk:

- Data profil dan informasi pribadi.
- Media dan dokumen milik akun.
- Password hash, sesi, token, dan identitas autentikasi eksternal yang tidak lagi diperlukan.
- Relasi kontak pemulihan dan informasi kontak terkait.
- Data bisnis atau aktivitas yang dapat mengidentifikasi pengguna, jika tidak ada dasar retensi yang sah.
- Data lain yang tidak lagi diperlukan untuk layanan, keamanan, atau kewajiban yang berlaku.

Penghapusan harus mempertimbangkan relasi, kepemilikan bersama, percakapan dengan pengguna lain, catatan transaksi, moderasi, dan audit. Jangan menghapus data pihak lain atau bukti yang wajib dipertahankan secara tidak sengaja.

## 5. Data yang dapat dipertahankan secara terbatas

Data dapat dipertahankan jika ada kebutuhan yang sah, misalnya:

- Kewajiban hukum atau akuntansi.
- Pencegahan dan investigasi penyalahgunaan atau insiden keamanan.
- Penyelesaian sengketa dan penegakan hak.
- Catatan transaksi yang wajib dipertahankan.
- Audit dan akuntabilitas yang memiliki tujuan jelas.

Setiap pengecualian harus mencatat kategori data, alasan, dasar, pemilik keputusan, akses yang diizinkan, dan kriteria atau durasi retensi. Data yang ditahan harus dipisahkan atau dibatasi agar tidak digunakan untuk aktivitas produk biasa, marketing, atau personalisasi.

Durasi numerik untuk setiap kategori belum ditetapkan dan memerlukan tinjauan hukum, kebutuhan operasional, serta risiko sebelum implementasi.

## 6. Reservasi dan penggunaan kembali kontak utama

- Email dan nomor telepon utama tetap direservasi selama masa tunggu penghapusan 30 hari.
- Jika pengguna membatalkan penghapusan, pemulihan kontak dilakukan setelah pemeriksaan keamanan.
- Setelah proses penghapusan benar-benar selesai dan kewajiban retensi mengizinkan pelepasan, kontak utama dapat digunakan untuk pendaftaran akun baru sesuai aturan identitas.
- Kontak tidak otomatis dilepas hanya karena masa tunggu berakhir jika pekerjaan penghapusan atau kewajiban retensi belum selesai.
- Aturan ini berbeda dari kontak pemulihan opsional: email dan nomor kontak pemulihan boleh digunakan bersama pada beberapa akun dan tidak memiliki unique constraint global. Setiap akun tetap memiliki relasi kontaknya sendiri.

## 7. Backup dan pemulihan

- Backup mengikuti klasifikasi data, enkripsi, kontrol akses, dan jadwal retensi yang ditetapkan.
- Backup memiliki siklus penghapusan tersendiri; data yang telah dihapus dapat tetap berada dalam backup sampai salinan tersebut kedaluwarsa atau diproses sesuai prosedur.
- Catat permintaan dan hasil penghapusan dalam mekanisme yang tahan terhadap pemulihan backup, tanpa menyimpan data pribadi berlebihan.
- Sebelum hasil restore dibuka kembali untuk penggunaan normal, terapkan kembali catatan penghapusan dan perubahan retensi yang terjadi setelah backup dibuat.
- Validasi agar data akun yang sudah dihapus atau dinonaktifkan tidak kembali aktif secara tidak sengaja.
- Uji proses restore dan penerapan ulang penghapusan secara berkala.

## 8. Interaksi dengan moderasi dan Admin

- Suspensi atau pembatasan oleh Admin berbeda dari penonaktifan dan penghapusan yang diminta pengguna.
- Admin tidak dapat menghapus akun secara permanen secara sepihak.
- Riwayat moderasi dan audit dapat dipertahankan sesuai kebutuhan akuntabilitas, dengan akses terbatas.
- Penghapusan akun tidak boleh menghapus catatan yang wajib dipertahankan, tetapi data tersebut tidak boleh digunakan untuk tujuan lain.
- Proses banding dan kewajiban penanganan laporan harus diperhitungkan sebelum penghapusan data terkait.

## 9. Implementasi dan pengujian

Pengujian minimum:

- Penonaktifan menyembunyikan profil dan membatasi akses tanpa mengubah role.
- Pembatalan penghapusan memerlukan pemeriksaan keamanan.
- Kontak utama tidak dapat digunakan untuk akun baru selama masa tunggu.
- Proses penghapusan tidak menghapus data pengguna lain atau catatan yang wajib dipertahankan.
- Data yang dipertahankan tidak muncul pada fitur publik atau marketing.
- Backup restore tidak menghidupkan kembali akun/data yang sudah dihapus.
- Kontak pemulihan bersama tidak salah terhapus atau diperlakukan sebagai identitas utama unik global.
- Semua transisi penting dapat diaudit.

## 10. Keputusan yang ditunda

Jangan mengarang masa retensi numerik, jadwal backup, durasi penyelesaian penghapusan, atau aturan pelepasan setiap kategori data. Tetapkan melalui tinjauan hukum, arsitektur penyimpanan, dan pengujian restore sebelum produksi.

## Referensi

- NIST SP 800-53 (kontrol akses, audit, dan perlindungan informasi).
- NIST SP 800-34 Rev. 1 (contingency planning).
- NIST Privacy Framework.
- PostgreSQL backup and recovery documentation.
