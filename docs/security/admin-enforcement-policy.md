# Kebijakan Penegakan Admin & Penanganan Akun

- **Status:** Disetujui sebagai baseline produk dan operasional.
- **Cakupan:** Peringatan, pembatasan fitur, suspensi sementara, pemulihan akses, dan tindakan operasional Admin terhadap akun atau konten.
- **Hubungan:** Melengkapi Access Control, Privacy & Data Control, Moderation & Appeals, dan Account Activity History.

## 1. Tujuan dan batas kewenangan

Admin menjaga keamanan, kualitas, dan kepatuhan penggunaan USahapintar melalui tindakan yang proporsional dan dapat ditinjau.

Akses operasional Admin yang luas pada tahap prototype/kompetisi tidak berarti kewenangan tanpa batas. Setiap tindakan harus mengikuti role dan izin Admin, tujuan kerja, ruang lingkup minimum, serta audit trail.

Admin dapat:

- Memberikan peringatan kepada pengguna.
- Membatasi fitur tertentu untuk jangka waktu atau kondisi yang ditentukan.
- Menangguhkan akun sementara.
- Meninjau ulang dan memulihkan akses setelah alasan pembatasan tidak lagi berlaku.
- Menangani laporan dan konten sesuai kewenangan moderasi.

Admin tidak boleh:

- Mengubah role utama Business Owner atau Investor.
- Menghapus akun secara permanen secara sepihak.
- Mengubah password, email utama, atau nomor telepon utama secara langsung di luar proses keamanan resmi.
- Melihat password atau OTP plaintext.
- Mengakses percakapan privat secara bebas tanpa laporan, permintaan dukungan, atau investigasi keamanan yang sah dan tercatat.
- Menggunakan tindakan moderasi untuk tujuan pribadi, diskriminatif, atau di luar kepentingan platform.

## 2. Jenis tindakan

### 2.1 Peringatan

Peringatan memberi tahu pengguna tentang perilaku atau konten yang perlu diperbaiki. Peringatan harus menjelaskan aturan yang relevan, tindakan yang diharapkan, dan konsekuensi jika pelanggaran berulang bila dapat dijelaskan.

### 2.2 Pembatasan fitur

Pembatasan hanya boleh menonaktifkan kemampuan yang relevan dengan risiko atau pelanggaran. Jangan memblokir fungsi yang tidak terkait jika tidak diperlukan. Tentukan cakupan, alasan, kondisi berakhir, dan cara pengguna meminta peninjauan.

### 2.3 Suspensi sementara

Suspensi membatasi akses akun untuk periode atau kondisi tertentu. Suspensi berbeda dari penonaktifan sukarela pengguna dan penghapusan permanen.

Suspensi harus memiliki alasan, cakupan, waktu mulai, durasi atau kondisi evaluasi, serta proses pemulihan. Jika risiko mengharuskan tindakan segera, lakukan pembatasan yang diperlukan dan lengkapi catatan serta pemberitahuan secepatnya.

### 2.4 Pemulihan akses

Admin dapat memulihkan akses setelah peninjauan menunjukkan bahwa pembatasan tidak lagi diperlukan, masa tindakan berakhir, atau banding diterima/diubah. Pemulihan harus tercatat dan diberitahukan kepada pengguna.

## 3. Dasar dan proporsionalitas

Sebelum melakukan tindakan, Admin harus mempertimbangkan:

- Aturan atau ketentuan yang relevan.
- Bukti dan konteks yang tersedia.
- Dampak terhadap pengguna dan pihak lain.
- Tingkat risiko serta urgensi.
- Alternatif yang lebih ringan.
- Apakah tindakan perlu dibatasi pada konten, fitur, atau akun tertentu.

Jangan membuat keputusan hanya berdasarkan dugaan yang belum ditinjau. Untuk keadaan mendesak, pembatasan sementara dapat diterapkan untuk mencegah dampak lebih lanjut, kemudian ditinjau sesuai prosedur.

## 4. Catatan wajib

Setiap tindakan harus mencatat sekurang-kurangnya:

- Identitas Admin atau aktor sistem yang melakukan tindakan.
- Akun, konten, atau resource yang menjadi target.
- Jenis dan ruang lingkup tindakan.
- Alasan atau aturan yang diterapkan.
- Waktu mulai dan waktu berakhir atau kondisi evaluasi.
- Status dan hasil tindakan.
- Referensi laporan atau bukti yang relevan dengan akses terbatas.
- Riwayat perubahan, pembatalan, dan pemulihan.

Catatan harus dilindungi dari perubahan tanpa izin. Jangan memasukkan password, OTP, token, isi dokumen rahasia, atau informasi investigasi yang tidak diperlukan ke dalam riwayat yang ditampilkan kepada pengguna.

## 5. Pemberitahuan dan transparansi pengguna

Pengguna harus diberi tahu ketika peringatan, pembatasan fitur, suspensi, perubahan status, atau pemulihan akses berdampak pada akunnya.

Pemberitahuan menjelaskan:

- Apa yang terjadi.
- Fitur atau konten yang terdampak.
- Alasan atau aturan dalam bahasa yang dapat dipahami.
- Durasi atau kondisi peninjauan bila ada.
- Cara mengajukan banding atau meminta bantuan.

Jangan mengungkap identitas pelapor, catatan investigasi internal, sinyal deteksi keamanan, atau data pribadi pihak lain.

## 6. Banding dan peninjauan

Pengguna dapat mengajukan banding atas peringatan, pembatasan fitur, atau suspensi. Proses, batas satu banding per tindakan, dan hasilnya diatur dalam [Kebijakan Moderasi & Banding](moderation-appeals-policy.md).

Selama banding, pembatasan dapat tetap berlaku jika diperlukan untuk keselamatan atau pencegahan dampak. Admin dapat mengurangi atau mencabut pembatasan lebih awal jika bukti dan konteks membenarkannya.

## 7. Pemisahan tugas dan pengawasan

- Berikan izin penegakan berdasarkan kebutuhan tugas.
- Jika tersedia, utamakan Admin berbeda untuk meninjau banding atau tindakan berisiko tinggi.
- Jika jumlah petugas terbatas, Admin yang sama boleh meninjau dengan alasan dan jejak audit yang jelas.
- Tindakan sensitif dan perubahan status harus dapat ditelusuri.
- Tinjau pola tindakan Admin untuk mendeteksi penyalahgunaan, inkonsistensi, atau tindakan yang tidak proporsional.

## 8. Pengujian minimum

- Admin tidak dapat mengubah role utama atau menghapus akun permanen secara sepihak.
- Peringatan, pembatasan, dan suspensi memiliki alasan dan cakupan yang jelas.
- Pembatasan fitur tidak memblokir fitur yang tidak terkait tanpa alasan.
- Setiap tindakan dan pemulihan tercatat serta memicu pemberitahuan yang tepat.
- Pengguna hanya dapat melihat riwayat miliknya sendiri.
- Admin tidak dapat membaca percakapan privat secara bebas.
- Akses istimewa dan perubahan status diuji berdasarkan RBAC + ABAC.

## 9. Referensi

- NIST SP 800-53, Access Control dan Audit and Accountability.
- NIST SP 800-61 Rev. 3 untuk koordinasi dengan penanganan insiden bila tindakan dipicu insiden keamanan.
- OWASP ASVS untuk kontrol otorisasi dan audit aplikasi.

Dokumen ini merupakan baseline kewenangan produk dan Admin. Prosedur operasional rinci, penugasan petugas, dan kebutuhan hukum ditetapkan sebelum peluncuran yang relevan.
