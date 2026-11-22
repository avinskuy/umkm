# Kebijakan Privasi USahapintar — Draf Pemberitahuan Pengguna

- **Status:** Draf pemberitahuan publik; belum menjadi pemberitahuan yang berlaku.
- **Versi kerja:** `draft-privacy-0.1`
- **Tanggal draf:** 22 November 2026
- **Pengendali/pengelola data:** BELUM DITETAPKAN
- **Tanggal berlaku:** BELUM DITETAPKAN
- **Tinjauan:** Wajib ditinjau pemilik produk, penanggung jawab privasi, dan penasihat hukum sebelum dipublikasikan.

> Dokumen ini adalah draf produk dan engineering, bukan pendapat hukum atau pernyataan kepatuhan. Detail yang belum diketahui ditandai sebagai keputusan terbuka. Jangan gunakan versi draf ini sebagai versi penerimaan registrasi.

## 1. Siapa yang mengelola data

USahapintar (**Platform**) direncanakan dikelola oleh **[NAMA BADAN USAHA/PENGELOLA]**, berkedudukan di **[ALAMAT RESMI]**.

Untuk pertanyaan, permintaan hak data, atau keluhan privasi:
- Email: **[EMAIL PRIVASI/RESMI]**
- Kanal bantuan: **[URL/KANAL BANTUAN]**
- Petugas/penanggung jawab privasi: **[NAMA ATAU FUNGSI]**

Identitas dan kontak tersebut wajib dilengkapi sebelum pemberitahuan diterbitkan.

## 2. Ruang lingkup Platform

USahapintar dirancang sebagai platform promosi bisnis, intelijen, dan jejaring untuk Business Owner dan Investor. Platform bukan marketplace barang, penyelenggara transaksi investasi, pengelola dana, atau penjamin pendanaan maupun keuntungan.

Fitur dan kategori data yang dijelaskan di sini dapat berubah mengikuti fitur yang benar-benar diaktifkan. Data yang belum dikumpulkan atau fitur yang belum tersedia tidak boleh dipresentasikan sebagai aktivitas pemrosesan yang sudah berjalan.

## 3. Data yang dapat diproses

Bergantung pada fitur yang digunakan, Platform dapat memproses kategori berikut:

| Kategori | Contoh data | Catatan |
|---|---|---|
| Identitas akun | Nama tampilan, role utama, ID akun, status akun | Nama registrasi adalah nama tampilan publik, bukan bukti identitas legal |
| Kontak dan autentikasi | Email/nomor telepon utama, hash password, status verifikasi, sesi dan token dalam bentuk terlindungi | Password dan OTP plaintext tidak boleh dicatat atau ditampilkan |
| Verifikasi | Tujuan pengiriman OTP yang dilindungi, fingerprint, waktu, status dispatch, hasil percobaan | Kode OTP tidak disimpan sebagai plaintext |
| Profil dan konten | Foto, deskripsi, profil usaha, posting, peluang, preferensi | Mengikuti field dan pengaturan visibilitas |
| Koneksi dan komunikasi | Permintaan koneksi, status hubungan, pesan, laporan | Percakapan dibatasi pada peserta dan proses yang memiliki izin |
| Keanggotaan | Paket, hak fitur, kuota, status pembayaran jika fitur pembayaran diaktifkan | Detail pembayaran belum ditetapkan; data kartu tidak boleh diasumsikan disimpan Platform |
| Keamanan dan operasional | Log akses, perangkat/sesi, audit event, laporan penyalahgunaan, tindakan moderasi | Akses internal dibatasi dan dicatat |
| Dukungan | Isi pertanyaan, korespondensi, bukti yang relevan | Jangan mengirim password, OTP, atau token ke kanal bantuan |
| Preferensi dan consent | Pilihan marketing, versi pemberitahuan, waktu pemberian atau pencabutan | Marketing bersifat opsional dan terpisah |

Data bisnis atau dokumen yang diberikan pengguna dapat memuat informasi pihak lain. Pengguna harus memiliki dasar dan izin yang sesuai sebelum membagikannya.

## 4. Sumber data

Data dapat diperoleh:
- Langsung dari pengguna ketika membuat akun, melengkapi profil, membuat posting, mengirim pesan, atau meminta bantuan.
- Secara otomatis dari penggunaan Platform, seperti log keamanan, status sesi, dan aktivitas teknis yang diperlukan.
- Dari penyedia layanan yang digunakan untuk mengirim OTP, mengoperasikan infrastruktur, atau menyediakan fitur yang diminta.
- Dari pengguna lain ketika mereka mengirim permintaan koneksi, laporan, atau konten yang berkaitan dengan akun pengguna.

Jika sumber eksternal atau integrasi baru ditambahkan, Platform harus menjelaskan kategori data dan tujuan yang relevan sebelum mengaktifkannya.

## 5. Tujuan dan dasar pemrosesan

Data hanya boleh digunakan untuk tujuan yang relevan, seperti:

1. Membuat dan mengelola akun, autentikasi, serta verifikasi kontak.
2. Menyediakan profil, posting, penemuan peluang, koneksi, dan komunikasi sesuai pilihan pengguna.
3. Menyediakan fitur keanggotaan, kuota, dukungan, dan transaksi jika diaktifkan.
4. Menjaga keamanan, mencegah penyalahgunaan, menangani laporan, moderasi, audit, dan pemulihan akun.
5. Memenuhi kewajiban hukum dan menangani permintaan resmi yang sah.
6. Mengirim informasi layanan dan keamanan yang diperlukan.
7. Mengirim marketing hanya jika pengguna memberikan persetujuan terpisah yang dapat ditarik.

Dasar pemrosesan untuk setiap tujuan—misalnya pelaksanaan layanan, kewajiban hukum, kepentingan sah yang telah dinilai, atau persetujuan—harus dipetakan dan disetujui sebelum publikasi. Jangan menganggap semua pemrosesan didasarkan pada persetujuan.

## 6. Visibilitas dan pembagian data

- Nama tampilan dan field profil yang dinyatakan publik dapat dilihat sesuai pengaturan serta status akun.
- Email, nomor telepon, kontak pemulihan, kredensial, dokumen privat, dan percakapan tidak otomatis menjadi publik.
- Informasi bisnis privat hanya dapat dibagikan kepada penerima yang ditentukan setelah izin yang sesuai.
- Admin hanya dapat mengakses data privat untuk tujuan operasional yang sah, dengan izin dan audit.
- Platform tidak menjual data pribadi pengguna.
- Data tidak boleh digunakan untuk marketing pihak ketiga atau tujuan lain yang tidak sesuai dengan pemberitahuan ini.

## 7. Penyedia layanan dan penerima

Untuk menjalankan fitur, Platform dapat menggunakan kategori penyedia berikut:
- Infrastruktur hosting, database, penyimpanan, dan backup.
- Penyedia pengiriman email dan WhatsApp OTP.
- Penyedia keamanan, pemantauan, dan pencatatan operasional.
- Penyedia AI atau pemrosesan analitik jika fitur tersebut diaktifkan.
- Penyedia pembayaran jika keanggotaan berbayar diaktifkan.
- Penasihat profesional, auditor, atau otoritas jika pengungkapan memiliki dasar yang sah.

Nama vendor, lokasi pemrosesan, subprocessor, peran masing-masing, dan pengaturan kontraktual harus dicantumkan atau dijelaskan sesuai kewajiban yang berlaku sebelum layanan publik diaktifkan.

## 8. Transfer lintas wilayah

**[PERLU KEPUTUSAN]** Lokasi hosting, backup, vendor, dan kemungkinan transfer data ke luar Indonesia belum ditetapkan dalam dokumen ini. Sebelum peluncuran, identifikasi negara tujuan, kategori data, mekanisme perlindungan, serta pemberitahuan atau persyaratan hukum yang berlaku.

## 9. Keamanan data

Platform merancang kontrol seperti pembatasan akses berdasarkan kebutuhan, enkripsi, perlindungan kredensial, pencatatan audit, pembatasan akses Admin, dan prosedur penanganan insiden. Penerapan aktual harus dibuktikan melalui konfigurasi, pengujian, dan prosedur operasional.

Tidak ada sistem yang dapat menjamin keamanan absolut. Jika terjadi insiden yang memenuhi kriteria pemberitahuan, pengelola akan menanganinya sesuai ketentuan hukum dan prosedur respons insiden yang berlaku.

## 10. Retensi dan penghapusan

Data disimpan selama diperlukan untuk tujuan pengumpulan, layanan, keamanan, penyelesaian sengketa, atau kewajiban hukum. Jadwal retensi numerik per kategori masih harus ditetapkan setelah tinjauan hukum dan operasional.

Pengguna dapat mengajukan penghapusan akun. Baseline produk menetapkan masa tunggu pembatalan 30 hari sebelum proses penghapusan dimulai. Masa tunggu ini bukan janji bahwa seluruh data dan salinan backup hilang tepat pada hari ke-30. Data tertentu dapat dipertahankan secara terbatas jika memiliki dasar yang sah; akses dan penggunaannya harus dibatasi.

## 11. Pilihan dan hak pengguna

Sesuai hukum yang berlaku dan kondisi pemrosesan, pengguna dapat meminta:
- Informasi mengenai pemrosesan data.
- Akses atau salinan data yang berkaitan dengan akunnya.
- Koreksi data yang tidak akurat.
- Penarikan persetujuan opsional.
- Pembatasan atau penghentian pemrosesan tertentu jika memenuhi syarat.
- Penghapusan data atau akun jika memenuhi syarat.
- Penanganan keluhan melalui kanal resmi.

Permintaan dapat memerlukan verifikasi yang proporsional untuk mencegah pengungkapan data kepada pihak yang tidak berhak. Pengelola harus menetapkan proses, tenggat respons, pengecualian, dan kanal eskalasi sebelum publikasi.

## 12. Marketing dan komunikasi

Persetujuan marketing bersifat opsional, tidak dicentang sebelumnya, dan tidak menjadi syarat layanan inti. Penolakan atau pencabutan persetujuan marketing tidak mematikan notifikasi keamanan atau pesan layanan yang diperlukan.

Riwayat pemberian dan pencabutan consent dicatat secara terpisah. Tidak adanya catatan pemberian berarti pengguna tidak memberikan persetujuan marketing.

## 13. Anak dan kelayakan pengguna

**[PERLU KEPUTUSAN]** Usia minimum pengguna, proses penanganan akun di bawah usia minimum, serta langkah verifikasi kelayakan belum ditetapkan. Tentukan sebelum registrasi publik dibuka dan selaraskan dengan Ketentuan Layanan serta hukum yang berlaku.

## 14. Cookie, analitik, dan teknologi serupa

**[PERLU KEPUTUSAN]** Daftar cookie, local storage, SDK, analytics, telemetry, dan teknologi serupa harus diaudit dari implementasi aktual. Jelaskan kategori, tujuan, masa berlaku, dan pilihan pengguna jika teknologi tersebut digunakan. Jangan menyatakan tidak ada tracking sebelum audit teknis selesai.

## 15. Perubahan pemberitahuan

Versi dan tanggal berlaku pemberitahuan harus terlihat. Perubahan material diberitahukan melalui kanal yang sesuai. Jika perubahan membutuhkan penerimaan baru, Platform akan meminta penerimaan versi baru sebelum penggunaan fitur yang mensyaratkannya.

## Daftar keputusan sebelum publikasi

- [ ] Nama legal pengelola, alamat, dan kontak privasi.
- [ ] Pemetaan tujuan dan dasar pemrosesan untuk setiap kategori data.
- [ ] Daftar vendor/subprocessor, negara pemrosesan, dan transfer lintas wilayah.
- [ ] Jadwal retensi per kategori dan pengecualian.
- [ ] Usia minimum dan tata kelola data anak.
- [ ] Audit cookie, analytics, telemetry, dan SDK aktual.
- [ ] Proses hak pengguna, tenggat, dan eskalasi.
- [ ] Tinjauan hukum serta persetujuan pemilik produk.
- [ ] Versi final dan tanggal berlaku ditetapkan melalui registry kebijakan.
