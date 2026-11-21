# Kebijakan Moderasi & Banding

- **Status:** Disetujui sebagai baseline produk dan perlindungan pengguna.
- **Cakupan:** Peninjauan konten/laporan, tindakan moderasi, pengajuan banding, keputusan banding, dan pencatatan hasil.
- **Hubungan:** Melengkapi Admin Enforcement, Access Control, Security Notifications, dan Account Activity History.

## 1. Prinsip moderasi

Moderasi menjaga platform tetap aman, relevan, dan dapat dipercaya tanpa menerapkan pembatasan secara sewenang-wenang.

Terapkan prinsip:

- Keputusan berdasarkan aturan, bukti, dan konteks yang relevan.
- Tindakan proporsional terhadap tingkat dan dampak pelanggaran.
- Penjelasan kepada pengguna dengan bahasa yang jelas.
- Perlindungan identitas pelapor dan informasi pihak lain.
- Kesempatan mengajukan banding.
- Pencatatan keputusan dan perubahan status.
- Pemisahan status publikasi dari status moderasi.

Konten yang telah diterbitkan dapat ditinjau setelah publikasi. Status moderasi tidak boleh disamakan dengan status publikasi.

## 2. Laporan dan peninjauan

Laporan dapat berasal dari pengguna, Admin, atau proses keamanan yang sah. Sebelum tindakan permanen pada konten atau pembatasan akun, tinjau laporan, aturan terkait, bukti, konteks, dan dampak yang mungkin terjadi.

Jika terdapat risiko mendesak terhadap pengguna atau platform, tindakan sementara yang proporsional dapat diterapkan terlebih dahulu. Tindakan tersebut tetap harus memiliki alasan, ruang lingkup, catatan, pemberitahuan yang sesuai, dan kesempatan peninjauan.

Jangan menganggap laporan sebagai bukti otomatis bahwa pelanggaran terjadi. Hindari tindakan hanya karena jumlah laporan tanpa menilai konteks dan bukti.

## 3. Jenis hasil moderasi

Tindakan moderasi dapat berupa:

- Peringatan.
- Pembatasan fitur tertentu.
- Pembatasan atau penghapusan konten sesuai aturan.
- Suspensi sementara akun.
- Pemulihan akses atau konten setelah peninjauan.

Penghapusan akun permanen bukan tindakan moderasi Admin. Penghapusan permanen mengikuti proses yang diminta pengguna dan kebijakan siklus hidup akun.

Setiap tindakan harus memiliki alasan, aturan yang relevan, cakupan, durasi atau kondisi evaluasi, serta status saat ini.

## 4. Hak dan batas banding

- Pengguna dapat mengajukan banding atas setiap tindakan moderasi berupa peringatan, pembatasan fitur, atau suspensi.
- Satu banding dapat diajukan untuk setiap tindakan moderasi.
- Pengguna dapat menyampaikan penjelasan, konteks, dan bukti pendukung yang relevan.
- Antarmuka harus menjelaskan cara dan status banding.
- Jangan meminta password, OTP, token, atau data rahasia yang tidak relevan sebagai bukti banding.
- Pengajuan banding tidak otomatis menghentikan tindakan yang sedang berlaku.

Jika risiko keselamatan mengharuskan pembatasan tetap berjalan selama peninjauan, jelaskan bahwa tindakan masih berlaku sementara. Admin dapat mengurangi atau mencabutnya lebih awal jika konteks dan bukti mendukung.

## 5. Proses peninjauan banding

1. Terima banding dan kaitkan dengan tindakan moderasi yang tepat.
2. Pastikan belum ada banding lain untuk tindakan yang sama.
3. Tinjau alasan awal, aturan, bukti, konteks tambahan, dan dampak.
4. Jika tersedia, utamakan Admin berbeda dari pembuat keputusan awal.
5. Jika kapasitas terbatas, Admin yang sama boleh meninjau; alasan dan prosesnya harus tercatat.
6. Ambil keputusan dan berikan penjelasan yang dapat dipahami pengguna.
7. Perbarui status tindakan, riwayat akun, dan notifikasi secara konsisten.

Peninjauan harus dilakukan oleh petugas yang memiliki kewenangan sesuai kebijakan akses. Jangan membuka bukti atau data privat di luar kebutuhan peninjauan.

## 6. Hasil banding

Hasil banding memiliki tiga kategori:

- **Diterima / dibatalkan:** tindakan awal dibatalkan dan dampaknya dipulihkan sejauh memungkinkan.
- **Diubah:** tindakan awal disesuaikan, misalnya cakupan atau durasinya dikurangi.
- **Ditolak / tetap berlaku:** tindakan awal dipertahankan.

Setiap hasil harus menyertakan alasan yang dapat dipahami, perubahan status, dan langkah berikutnya jika ada. Jangan menampilkan catatan investigasi internal, identitas Admin, identitas pelapor, sinyal deteksi, atau data pribadi pihak lain.

## 7. Pencatatan dan konsistensi status

Catat:

- Referensi tindakan moderasi.
- Waktu pengajuan banding.
- Penjelasan dan bukti yang diberikan, dengan akses terbatas.
- Petugas peninjau.
- Hasil, alasan, dan waktu keputusan.
- Perubahan cakupan/durasi tindakan.
- Waktu pemberitahuan kepada pengguna.

Keputusan banding harus memperbarui status tindakan dan riwayat akun secara konsisten. Perubahan harus dapat diaudit dan tidak boleh menghapus jejak keputusan sebelumnya.

## 8. Pemberitahuan

Kirim pemberitahuan ketika:

- Tindakan moderasi baru diterapkan.
- Banding diterima.
- Banding sedang ditinjau atau statusnya berubah secara material.
- Hasil banding ditetapkan.
- Tindakan dibatalkan, diubah, atau berakhir.
- Akses dipulihkan.

Notifikasi hanya menyampaikan informasi yang relevan kepada pemilik akun. Jangan mengungkap identitas pelapor atau informasi investigasi yang sensitif.

## 9. Pengujian minimum

- Satu tindakan moderasi hanya dapat memiliki satu banding.
- Banding terkait dengan tindakan dan akun yang benar.
- Pengguna tidak dapat melihat atau mengubah banding akun lain.
- Ketiga hasil banding memperbarui status tindakan secara konsisten.
- Pembatasan tetap berlaku selama peninjauan jika diperlukan, dan dapat dikurangi secara sah.
- Identitas pelapor, catatan internal, dan data pengguna lain tidak terlihat pada antarmuka banding.
- Setiap keputusan dan perubahan status menghasilkan audit trail serta notifikasi yang sesuai.
- Penghapusan permanen akun tidak tersedia sebagai tindakan moderasi Admin.

## 10. Referensi

- NIST SP 800-53, Access Control dan Audit and Accountability.
- OWASP ASVS, kontrol otorisasi dan perlindungan data.
- Prinsip due process, transparansi, dan proporsionalitas pada tata kelola platform.

Prosedur peninjauan rinci dan batas waktu layanan ditetapkan saat tahap operasional. Jangan mengarang SLA sebelum kapasitas dan kebutuhan produk dievaluasi.
