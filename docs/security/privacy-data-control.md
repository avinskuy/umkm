# Kebijakan Privasi & Kendali Data

- **Status:** Disetujui sebagai baseline produk dan keamanan.
- **Cakupan:** Data akun, profil, bisnis, peluang, koneksi, percakapan, dokumen, transaksi, dan data aktivitas pengguna.
- **Pemilik kebijakan:** Tim produk dan engineering USahapintar.
- **Penerapan:** Bertahap sesuai fitur, klasifikasi data, risiko, dan kewajiban yang berlaku.

## 1. Tujuan dan prinsip

USahapintar memberi pengguna kendali yang jelas atas informasi pribadi dan informasi bisnisnya. Terapkan:

- Privacy by Design dan Privacy by Default.
- Pengumpulan dan penggunaan data secukupnya untuk tujuan yang jelas.
- Transparansi tentang data yang diproses dan alasan pemrosesannya.
- Akses berdasarkan klasifikasi, kepemilikan, role, persetujuan, dan konteks.
- Perlindungan data sensitif tanpa bergantung pada pengaturan visibilitas biasa.
- Kendali pengguna untuk melihat, memperbaiki, mengunduh, dan meminta penghapusan data sesuai kebijakan.

Persetujuan kebijakan ini bukan klaim bahwa seluruh kontrol privasi telah diimplementasikan atau bahwa produk telah tersertifikasi.

## 2. Default visibilitas profil

Hanya informasi profil minimum yang memang ditujukan untuk publik yang boleh tampil secara publik secara default:

- Nama tampilan.
- Foto profil, jika pengguna memilih menggunakannya.
- Jenis akun (Business Owner atau Investor).
- Deskripsi yang secara eksplisit disediakan sebagai informasi publik.

Untuk field profil non-sensitif yang memenuhi syarat, sediakan pilihan visibilitas:

- **Publik:** dapat dilihat pengunjung tanpa login.
- **Pengguna terdaftar:** hanya dapat dilihat pengguna yang sudah masuk dan memiliki akses sesuai aturan produk.
- **Pribadi:** hanya pemilik dan proses internal yang memiliki dasar akses sah.

Pilihan visibilitas tidak menggantikan pemeriksaan otorisasi. Backend tetap memeriksa identitas, status akun, kepemilikan, role, hubungan, persetujuan, dan klasifikasi data pada setiap permintaan.

## 3. Data yang selalu dilindungi

Field sensitif tidak boleh dijadikan publik melalui pengaturan profil biasa. Termasuk:

- Email dan nomor telepon utama.
- Kontak pemulihan.
- Kredensial, token, dan informasi autentikasi.
- Dokumen identitas atau dokumen bisnis privat.
- Informasi transaksi dan pembayaran yang tidak ditujukan untuk publik.
- Percakapan privat.
- Bukti pemulihan dan informasi investigasi keamanan.
- Data pribadi pihak lain yang muncul dalam dokumen atau lampiran.

Kontrol khusus tetap berlaku walaupun data tersebut terhubung ke profil atau postingan publik.

## 4. Visibilitas posting dan peluang

- Postingan yang telah diterbitkan bersifat publik dan dapat dilihat pengunjung tanpa login, sesuai tujuan penemuan peluang di platform.
- Status publikasi dan status moderasi merupakan dua hal terpisah. Konten yang telah terbit dapat ditinjau, dibatasi, atau dihapus sesuai kebijakan moderasi.
- Pemilik dapat mengubah atau menghapus postingannya melalui alur produk yang disediakan.
- Informasi kontak pribadi, detail transaksi pelanggan, dokumen rahasia, dan data sensitif tidak otomatis menjadi publik karena disertakan dalam posting.
- Lampiran memiliki kontrol akses tersendiri. Lampiran privat hanya boleh dibagikan setelah pemilik memberi izin yang sesuai.
- Investor hanya menerima informasi bisnis yang secara eksplisit dibagikan pemiliknya. Data transaksi pada tingkat pelanggan tidak otomatis tersedia.

## 5. Persetujuan dan pembagian informasi

- Persetujuan pemilik harus spesifik terhadap data, penerima, tujuan, dan ruang lingkup pembagian yang relevan.
- Persetujuan berbagi data bisnis tidak berarti izin untuk membuka seluruh dokumen atau data terkait bisnis tersebut.
- Sistem harus dapat mencatat status, waktu, ruang lingkup, dan pencabutan persetujuan.
- Setelah persetujuan dicabut, hentikan akses atau pembagian berikutnya sesuai kemampuan dan kewajiban retensi; jelaskan jika salinan yang telah sah diterima pihak lain tidak dapat ditarik kembali.
- Premium tidak boleh membuka data privat yang belum dibagikan pemilik.

## 6. Akses dan kendali pengguna

Pengguna harus memiliki cara yang mudah dipahami untuk:

- Melihat data pribadi yang tersedia pada akunnya.
- Memperbaiki data yang tidak akurat melalui alur yang aman.
- Mengelola visibilitas field profil yang memenuhi syarat.
- Meninjau dan mencabut izin berbagi data yang masih aktif.
- Mengunduh salinan data miliknya melalui fitur **Unduh Data Saya**.
- Mengajukan penonaktifan atau penghapusan akun melalui alur yang terpisah.

Permintaan sensitif dapat memerlukan autentikasi ulang atau verifikasi keamanan. Respons harus menjelaskan status permintaan dan langkah berikutnya.

## 7. Unduh Data Saya

Fitur Unduh Data Saya menjadi bagian dari kendali privasi:

- Pengguna hanya dapat meminta salinan data milik akunnya sendiri.
- Lakukan pemeriksaan autentikasi dan otorisasi yang sesuai sebelum menyiapkan ekspor.
- Beri tahu pengguna ketika ekspor siap dan sediakan cara pengambilan yang aman serta terbatas waktunya.
- Ekspor tidak boleh menyertakan data pengguna lain, catatan investigasi internal, kredensial, token, rahasia keamanan, atau informasi yang tidak berhak diterima pemohon.
- Catat permintaan dan penyelesaian ekspor tanpa menyalin isi data sensitif ke log.
- Format, batas ukuran, masa berlaku unduhan, dan metode teknis ditetapkan saat implementasi.

## 8. Akses internal dan Admin

- Akses internal mengikuti RBAC + ABAC, least privilege, klasifikasi data, tujuan, dan kebutuhan tugas.
- Akses operasional luas untuk Admin pada tahap proyek tidak berarti akses bebas terhadap semua percakapan, dokumen, atau bukti sensitif.
- Akses percakapan privat hanya untuk laporan pengguna, permintaan dukungan, atau investigasi keamanan yang sah; cakupannya dibatasi dan dicatat.
- Password dan OTP plaintext tidak boleh dapat dilihat Admin.
- Akses dan perubahan sensitif harus dapat diaudit.

## 9. Antarmuka dan transparansi

- Gunakan Bahasa Indonesia yang jelas dan tidak menyesatkan.
- Jelaskan siapa yang dapat melihat informasi dan apa akibat dari pilihan visibilitas.
- Bedakan data publik, data privat, dan data yang dibagikan secara terbatas.
- Jangan menyamarkan persetujuan opsional sebagai syarat layanan inti.
- Jangan menggunakan pola manipulatif untuk mendorong pengguna membuka data lebih banyak.
- Berikan konfirmasi dan penjelasan sebelum tindakan yang berdampak besar.

## 10. Pengujian dan bukti

Pengujian minimum mencakup:

- Pengunjung publik tidak dapat membaca field privat.
- Pengguna terdaftar tidak dapat membaca data yang tidak diizinkan.
- Pengguna tidak dapat mengubah visibilitas field sensitif menjadi publik.
- Investor hanya dapat melihat data bisnis yang dibagikan secara sah.
- Lampiran mengikuti izin aksesnya sendiri, bukan visibilitas posting.
- Pencabutan persetujuan menghentikan akses berikutnya sesuai kebijakan.
- Ekspor hanya berisi data pemohon yang diizinkan.
- Akses Admin terhadap data privat dibatasi dan diaudit.
- Perubahan visibilitas dan persetujuan memiliki regression test.

## 11. Hubungan dengan kebijakan lain

Baca bersama [Klasifikasi Data](data-classification.md), [Kontrol Akses & Otorisasi](access-control-authorization.md), [Enkripsi & Pengelolaan Kunci](encryption-key-management.md), [Logging & Audit](logging-monitoring-audit.md), [Kebijakan Persetujuan & Pemrosesan Data](consent-data-processing.md), dan [Siklus Hidup Akun & Retensi Data](account-lifecycle-data-retention.md).

## Referensi

- NIST Privacy Framework.
- ISO/IEC 27701 sebagai referensi sistem manajemen informasi privasi.
- OWASP Application Security Verification Standard (ASVS).
- Prinsip privacy by design dan privacy by default.

Referensi digunakan sebagai panduan praktik. Penerapan kebijakan ini tidak menyatakan sertifikasi atau kepatuhan hukum secara otomatis.
