# Prinsip Pengalaman Pengguna yang Inklusif

> Status: baseline produk yang disetujui untuk pengembangan dan dapat ditinjau kembali.
> Cakupan: seluruh pengalaman USahapintar, termasuk situs publik, autentikasi, onboarding, dashboard Owner dan Investor, fitur bisnis, membership, bantuan, serta administrasi.

## 1. Tujuan

USahapintar ditujukan untuk pengguna dengan latar belakang, usia, pengalaman, perangkat, dan tingkat literasi digital yang beragam. Tidak semua pengguna terbiasa dengan aplikasi modern atau memahami istilah teknologi.

Karena itu, pengalaman pengguna (UX), aksesibilitas, dan kemudahan memahami alur merupakan persyaratan produk, bukan sekadar penyempurnaan visual menjelang peluncuran.

Prinsip utama:

**Mudah dipahami. Mudah digunakan. Aman untuk dipercaya.**

Desain tidak boleh menganggap pengguna sudah memahami istilah teknis, pola antarmuka, atau proses digital tertentu. Pengalaman harus tetap jelas bagi orang yang baru pertama kali menggunakan platform.

## 2. Prinsip dasar

### 2.1 Gunakan bahasa sehari-hari yang jelas

- Gunakan Bahasa Indonesia yang alami, langsung, sopan, dan mudah dipahami.
- Hindari istilah teknis jika ada padanan yang lebih sederhana.
- Jika istilah teknis memang diperlukan, jelaskan artinya saat pertama kali muncul.
- Gunakan satu istilah yang konsisten untuk tindakan dan konsep yang sama di seluruh platform.
- Hindari pesan yang menyalahkan, merendahkan, menakut-nakuti, atau membuat pengguna merasa bersalah.
- Jangan menganggap semua pengguna lanjut usia memiliki kemampuan digital yang sama; uji kebutuhan berdasarkan pengalaman nyata.

Contoh:

| Hindari | Gunakan |
| --- | --- |
| Invalid credentials | Email/nomor HP atau kata sandi belum sesuai. Periksa kembali lalu coba lagi. |
| Session expired | Sesi masuk sudah berakhir. Silakan masuk kembali. |
| OTP expired | Kode sudah kedaluwarsa. Minta kode baru untuk melanjutkan. |
| Unauthorized | Kamu belum memiliki izin untuk membuka halaman ini. |
| Request failed | Permintaan belum berhasil. Coba lagi beberapa saat. |

Contoh kalimat harus disesuaikan dengan konteks dan tidak boleh membocorkan apakah suatu akun terdaftar atau tidak.

### 2.2 Buat tindakan dan pilihan mudah dikenali

- Setiap halaman atau langkah harus memiliki tujuan utama yang jelas.
- Gunakan label tombol yang menjelaskan tindakan, misalnya **Kirim kode**, **Simpan perubahan**, atau **Lanjutkan**.
- Hindari label yang tidak menjelaskan hasil tindakan, seperti **Klik di sini** atau **Submit**, jika ada pilihan yang lebih jelas.
- Tampilkan pilihan yang relevan pada saat dibutuhkan; jangan membanjiri pengguna dengan semua pengaturan sekaligus.
- Berikan penjelasan singkat ketika pilihan memiliki konsekuensi penting.
- Bedakan tindakan utama, tindakan sekunder, dan tindakan berisiko secara visual dan semantik.

### 2.3 Kurangi beban berpikir dan langkah yang tidak perlu

- Minta hanya informasi yang diperlukan untuk menyelesaikan langkah saat ini.
- Gunakan progressive disclosure: tampilkan detail lanjutan ketika memang dibutuhkan.
- Jangan meminta data yang sama berulang kali jika sudah tersedia dan masih sah digunakan.
- Jangan memaksa pengguna mengisi profil lengkap sebelum dapat menggunakan fungsi dasar yang sudah boleh diakses.
- Berikan nilai awal yang aman jika memang ada pilihan yang wajar, tetapi jangan memilih persetujuan atau tindakan penting atas nama pengguna.
- Jelaskan alasan pengumpulan informasi sensitif atau pribadi.

### 2.4 Buat alur yang toleran terhadap kesalahan

- Validasi input sedekat mungkin dengan kolom yang bermasalah.
- Jelaskan apa yang salah dan cara memperbaikinya; jangan hanya menampilkan kode error.
- Pertahankan input non-sensitif yang sudah benar ketika terjadi kesalahan yang dapat dipulihkan.
- Jangan pernah mengisi ulang atau menampilkan kata sandi, OTP, token, atau rahasia lain setelah kegagalan.
- Cegah pengiriman formulir berulang yang tidak disengaja.
- Berikan kesempatan untuk meninjau perubahan penting sebelum disimpan.
- Untuk tindakan destruktif, jelaskan dampak dan sediakan konfirmasi yang proporsional.
- Jangan membuat pengguna mengulang seluruh proses jika hanya satu langkah yang gagal.

### 2.5 Selalu berikan umpan balik dan arah berikutnya

Setiap tindakan yang membutuhkan waktu atau mengubah data harus memberikan umpan balik yang sesuai:

- Loading: jelaskan tindakan yang sedang diproses.
- Berhasil: jelaskan hasil dan langkah berikutnya.
- Gagal: jelaskan masalah dalam bahasa yang mudah dipahami dan berikan tindakan pemulihan.
- Kosong: jelaskan mengapa belum ada data dan apa yang bisa dilakukan.
- Offline atau gangguan jaringan: jelaskan apakah data sudah tersimpan, belum tersimpan, atau aman untuk dicoba kembali.

Jika proses dapat dilanjutkan nanti, jelaskan cara kembali tanpa kehilangan progres yang aman untuk disimpan.

### 2.6 Utamakan pengalaman mobile dan kondisi jaringan nyata

- Rancang pengalaman mobile sebagai pengalaman utama, bukan sekadar versi desktop yang diperkecil.
- Pastikan teks, formulir, navigasi, dan tombol nyaman digunakan pada layar kecil.
- Hindari interaksi yang hanya mengandalkan hover atau gerakan presisi.
- Pertimbangkan koneksi lambat, terputus sementara, dan perangkat dengan kemampuan terbatas.
- Hindari memuat media atau animasi berat tanpa manfaat yang jelas.
- Berikan status yang jujur ketika suatu tindakan membutuhkan koneksi dan belum berhasil disimpan.

## 3. Aksesibilitas dan keterbacaan

- Gunakan struktur heading, landmark, label formulir, dan elemen HTML semantik.
- Semua fungsi utama harus dapat digunakan dengan keyboard.
- Fokus keyboard harus terlihat dan urutannya masuk akal.
- Jangan menyampaikan status atau makna hanya melalui warna.
- Pastikan kontras teks dan komponen cukup, ukuran teks dapat dibaca, dan tata letak tetap berfungsi saat teks diperbesar.
- Sediakan nama aksesibel untuk kontrol, ikon, dan tombol yang tidak memiliki teks terlihat.
- Berikan teks alternatif yang bermakna untuk gambar informatif; gambar dekoratif tidak boleh menambah kebisingan bagi teknologi bantu.
- Berikan caption atau transkrip untuk media audio/video yang menyampaikan informasi penting.
- Hindari batas waktu yang tidak perlu. Jika batas waktu diperlukan untuk keamanan, jelaskan sebelumnya dan sediakan perpanjangan jika aman.
- Gunakan WCAG 2.2 Level AA sebagai acuan evaluasi aksesibilitas desain dan implementasi. Pengecualian harus dicatat beserta alasan dan alternatif yang tersedia.

## 4. Keamanan yang tidak membebani pengguna

Keamanan harus tetap kuat, tetapi tidak semua pengguna harus melewati tingkat pemeriksaan yang sama.

- Terapkan pemeriksaan berbasis risiko di backend.
- Untuk risiko rendah, pilih jalur aman dengan langkah sesedikit mungkin.
- Untuk risiko sedang, minta verifikasi tambahan yang relevan dan jelaskan alasan praktisnya.
- Untuk risiko tinggi, tahan tindakan sensitif atau minta pemeriksaan tambahan sesuai kebijakan keamanan.
- Jangan menampilkan detail sinyal keamanan, aturan deteksi, atau informasi yang membantu penyerang menghindari pemeriksaan.
- Jangan menganggap keberhasilan OTP atau kepemilikan satu kontak sebagai bukti tunggal kepemilikan akun.
- Jangan mengungkap keberadaan akun, jumlah akun yang terkait, atau identitas akun lain kepada pihak yang belum berwenang.
- Sediakan jalur pemulihan atau bantuan ketika proses otomatis tidak dapat menyelesaikan masalah.
- Jangan menambahkan langkah keamanan hanya demi terlihat lebih aman jika langkah itu tidak mengurangi risiko secara nyata.

Kompleksitas pemeriksaan boleh berada di belakang layar, tetapi pengguna harus tetap menerima penjelasan yang jujur dan instruksi yang dapat dilakukan.

## 5. Kepercayaan, privasi, dan kendali pengguna

- Jelaskan tujuan penggunaan data pada saat data diminta.
- Minta persetujuan secara spesifik untuk penggunaan data yang memang memerlukan persetujuan.
- Jangan menggunakan pilihan yang sudah dicentang untuk persetujuan pemasaran atau hal opsional.
- Berikan cara yang jelas untuk melihat, memperbarui, atau menghapus informasi sesuai kebijakan produk dan kewajiban retensi.
- Jangan menggunakan desain yang menekan pengguna untuk membeli Premium, membagikan data, atau menyetujui sesuatu.
- Tampilkan biaya, konsekuensi, batas, dan ketentuan penting sebelum pengguna mengambil keputusan.
- Bedakan informasi yang sudah diverifikasi, belum diverifikasi, dan hanya berupa klaim pengguna.

## 6. Bantuan dan pemulihan

- Sediakan jalur bantuan yang mudah ditemukan dari halaman yang relevan.
- Instruksi bantuan harus spesifik terhadap masalah, bukan hanya mengarahkan pengguna ke halaman umum.
- Jika masalah tidak dapat diselesaikan otomatis, jelaskan opsi bantuan berikutnya.
- Jangan meminta pengguna mengirim kata sandi, OTP, token sesi, atau rahasia akun kepada petugas.
- Petugas bantuan hanya boleh mengakses data dan melakukan tindakan sesuai kewenangan serta audit.
- Jangan menjanjikan pemulihan berhasil jika bukti kepemilikan belum cukup.

## 7. Pengujian usability

Pengujian tidak cukup dilakukan oleh developer atau orang yang sudah terbiasa dengan produk digital.

Sebelum alur penting dianggap siap, lakukan pengujian dengan peserta yang mewakili variasi:

- Usia dan pengalaman menggunakan teknologi.
- Tingkat literasi digital.
- Perangkat dan ukuran layar.
- Kondisi jaringan.
- Kebutuhan aksesibilitas, jika memungkinkan dan relevan.

Utamakan tugas nyata, misalnya membuat akun, memverifikasi kontak, masuk, menemukan peluang, mengirim permintaan koneksi, memahami status permintaan, dan memulihkan akses akun.

Amati dan catat:

- Apakah peserta memahami tujuan halaman dan instruksi?
- Apakah peserta dapat menyelesaikan tugas tanpa bantuan?
- Di bagian mana peserta ragu, salah memilih, atau mengulang langkah?
- Apakah pesan kesalahan membantu peserta memperbaiki masalah?
- Berapa banyak bantuan yang diperlukan?
- Apakah peserta memahami konsekuensi tindakan penting?

Jangan hanya mengandalkan pendapat bahwa desain terlihat bagus. Gunakan keberhasilan tugas, kesalahan, kebutuhan bantuan, dan umpan balik peserta sebagai bukti untuk perbaikan.

Ambang keberhasilan kuantitatif ditetapkan setelah baseline pengujian awal tersedia; jangan mengarang target angka tanpa data.

## 8. Daftar pemeriksaan sebelum fitur dinyatakan siap

- [ ] Tujuan fitur dan tindakan utama mudah dipahami pengguna baru.
- [ ] Bahasa jelas, konsisten, dan tidak bergantung pada pemahaman istilah teknis.
- [ ] Formulir hanya meminta data yang diperlukan pada tahap tersebut.
- [ ] Label tombol menjelaskan tindakan yang akan terjadi.
- [ ] Loading, berhasil, gagal, kosong, dan gangguan jaringan memiliki umpan balik yang sesuai.
- [ ] Kesalahan dapat diperbaiki tanpa mengulang proses yang tidak perlu.
- [ ] Input aman yang sudah benar dipertahankan jika memungkinkan.
- [ ] Tampilan dan interaksi nyaman pada perangkat mobile.
- [ ] Keyboard, pembaca layar, kontras, dan pembesaran teks telah diperiksa.
- [ ] Pemeriksaan keamanan proporsional terhadap risiko dan tetap ditegakkan di backend.
- [ ] Privasi, persetujuan, dan konsekuensi tindakan dijelaskan.
- [ ] Alur penting telah diuji dengan pengguna yang mewakili target produk, atau keterbatasan pengujiannya dicatat beserta rencana tindak lanjut.

## 9. Penerapan dan peninjauan

Prinsip ini berlaku lintas fitur dan role. Setiap spesifikasi UX, implementasi frontend, pesan backend yang ditampilkan kepada pengguna, serta alur bantuan harus mengacu pada dokumen ini.

Dokumen ini merupakan baseline yang dapat direvisi berdasarkan pengujian, umpan balik pengguna, perubahan kebutuhan produk, dan tinjauan keamanan. Perubahan tidak boleh mengurangi aksesibilitas, kejelasan, atau perlindungan pengguna tanpa alasan yang terdokumentasi.
