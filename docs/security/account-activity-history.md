# Kebijakan Riwayat Aktivitas Akun

- **Status:** Disetujui sebagai baseline produk dan transparansi pengguna.
- **Cakupan:** Riwayat tindakan moderasi terhadap akun pengguna, status tindakan, banding, dan pemberitahuan terkait.
- **Hubungan:** Melengkapi Admin Enforcement, Moderation & Appeals, Security Notifications, dan Logging & Audit Trail.

## 1. Tujuan

USahapintar menyediakan halaman **Riwayat Aktivitas Akun** agar pengguna dapat memahami tindakan moderasi yang berdampak pada akunnya, status terkini, dan kesempatan banding yang tersedia.

Riwayat ini merupakan tampilan yang aman dan relevan bagi pengguna, bukan salinan seluruh audit log atau catatan internal Admin.

## 2. Informasi yang dapat dilihat pengguna

Untuk setiap tindakan moderasi pada akunnya sendiri, tampilkan informasi yang relevan:

- Tanggal dan waktu tindakan.
- Jenis tindakan, misalnya peringatan, pembatasan fitur, atau suspensi.
- Alasan dan aturan yang relevan dalam bahasa yang mudah dipahami.
- Konten atau fitur yang terdampak.
- Durasi atau kondisi berakhir jika ada.
- Status terkini tindakan.
- Status banding dan hasilnya jika diajukan.
- Penjelasan mengenai perubahan, pembatalan, atau pemulihan akses.

Riwayat harus tetap dapat dilihat setelah pembatasan berakhir atau akses dipulihkan, selama data tersebut masih dapat ditampilkan sesuai kebijakan retensi.

## 3. Informasi yang tidak boleh ditampilkan

Jangan tampilkan kepada pengguna:

- Identitas internal Admin atau petugas peninjau.
- Catatan investigasi internal.
- Sinyal keamanan, metode deteksi, atau aturan internal yang dapat disalahgunakan.
- Identitas atau data pribadi pelapor.
- Data pribadi pengguna lain.
- Bukti sensitif yang tidak berhak diterima pemilik akun.
- Password, OTP, token, kredensial, atau rahasia keamanan.

Jika pengguna mengajukan banding, tampilkan penjelasan dan status yang relevan tanpa membuka materi internal yang dilindungi.

## 4. Privasi dan otorisasi

- Pengguna hanya dapat membaca riwayat milik akunnya sendiri.
- Pemeriksaan dilakukan di backend pada setiap permintaan; menyembunyikan menu di frontend bukan kontrol akses.
- Admin hanya dapat mengakses riwayat sesuai tugas dan kewenangannya.
- Akses atau perubahan administratif harus dicatat sesuai Logging & Audit Trail.
- Data yang ditampilkan mengikuti klasifikasi dan minimisasi data.
- Riwayat tidak boleh tersedia secara publik atau dapat dicari pengguna lain.

## 5. Hubungan dengan notifikasi dan banding

- Tindakan baru dan perubahan status penting menghasilkan notifikasi sesuai Kebijakan Notifikasi Keamanan & Peringatan.
- Halaman riwayat menjadi tempat pengguna melihat rincian dan status yang lebih lengkap.
- Jika banding tersedia, sediakan akses yang jelas dari detail tindakan.
- Status banding dan hasilnya harus konsisten antara halaman riwayat, proses moderasi, dan notifikasi.
- Notifikasi tidak menggantikan riwayat dan riwayat tidak menggantikan pemberitahuan untuk tindakan penting.

## 6. Pengalaman pengguna

- Gunakan istilah yang jelas dan tidak menyalahkan pengguna.
- Tampilkan tindakan terbaru dengan urutan waktu yang mudah dipahami.
- Bedakan status aktif, berakhir, dibatalkan, dan sedang ditinjau tanpa mengandalkan warna saja.
- Berikan penjelasan tentang dampak tindakan dan langkah berikutnya.
- Sediakan state kosong yang menjelaskan bahwa belum ada tindakan moderasi pada akun.
- Pastikan halaman dapat digunakan pada perangkat mobile, keyboard, dan teknologi bantu.

Untuk tahap awal, cukup sediakan satu halaman riwayat yang sederhana. Jangan membuat dashboard analitik moderasi kompleks tanpa kebutuhan dan bukti manfaat.

## 7. Retensi dan konsistensi

- Riwayat tetap tersedia setelah tindakan selesai sesuai masa retensi yang ditetapkan.
- Data yang ditampilkan tidak boleh menghalangi proses penghapusan atau retensi yang berlaku.
- Jika sebagian bukti harus disimpan untuk alasan hukum atau keamanan, batasi akses dan jangan otomatis menampilkannya kepada pengguna.
- Perubahan tindakan atau hasil banding tidak boleh menghapus histori sebelumnya; tampilkan status terbaru dengan jejak perubahan yang sesuai.

Durasi retensi numerik mengikuti kebijakan Account Lifecycle & Data Retention dan tidak ditetapkan ulang di dokumen ini.

## 8. Pengujian minimum

- Pengguna hanya dapat melihat riwayat akunnya sendiri.
- Pengguna tidak dapat menebak atau mengakses riwayat akun lain melalui ID atau parameter yang diubah.
- Tindakan baru dan perubahan status muncul secara konsisten.
- Riwayat tetap ada setelah pembatasan berakhir.
- Banding dan hasilnya terhubung ke tindakan yang benar.
- Identitas Admin, pelapor, catatan investigasi, dan data sensitif tidak bocor.
- Halaman memiliki state kosong, loading, error, dan tampilan aksesibel.

## 9. Referensi

- NIST SP 800-53, Audit and Accountability serta Access Control.
- OWASP ASVS.
- WCAG 2.2 Level AA untuk aksesibilitas antarmuka.

Halaman ini merupakan fitur transparansi bagi pengguna. Detail implementasi, pagination, dan retensi teknis ditetapkan saat pengembangan.
