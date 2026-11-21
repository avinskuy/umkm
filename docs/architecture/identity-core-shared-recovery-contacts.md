# Tinjauan Skema Identity Core — Verifikasi dan Pemulihan

> Status: keputusan produk dan implikasi skema yang disetujui sebagai baseline; rancangan SQL final tetap memerlukan tinjauan keamanan dan pengujian.
> Cakupan: sesi verifikasi, kontak pemulihan bersama, pemilihan akun tujuan, batas penyalahgunaan, dan masa tunggu penghapusan.

Dokumen ini melengkapi [Rancangan Skema Database](database-schema-design.md). Dokumen ini tidak menyatakan bahwa tabel atau alur sudah diimplementasikan.

## 1. Prinsip utama

Pengalaman pengguna harus sederhana, tetapi sistem tetap melindungi akun dan privasi di sisi server. Satu kontak pemulihan dapat digunakan oleh beberapa akun. Karena itu, keberhasilan verifikasi kontak hanya membuktikan kendali atas kontak tersebut; hal itu tidak otomatis membuktikan kepemilikan semua akun yang terkait.

## 2. Sesi verifikasi yang belum memiliki akun tujuan

Kolom verification_sessions.user_id boleh bernilai NULL sementara hanya untuk alur pemulihan yang memverifikasi kontak bersama sebelum akun tujuan dipilih.

| Jenis alur | user_id | Ketentuan |
| --- | --- | --- |
| Pendaftaran tertunda | Wajib | Akun pending sudah dibuat dan sesi terikat ke akun tersebut. |
| Verifikasi kontak pada akun tertentu | Wajib | Akun tujuan sudah diketahui dan otorisasi konteksnya sudah diperiksa. |
| Pemulihan sebelum pemilihan akun | Boleh NULL | Sesi hanya terikat pada tujuan, kanal, kontak, dan konteks pemulihan; belum menunjuk akun tertentu. |
| Pemulihan setelah akun dipilih | Wajib | Sesi/kasus dikaitkan ke akun yang dipilih setelah pemeriksaan yang sesuai. |

Ketentuan implementasi:

- NULL bukan nilai default untuk semua alur. Backend harus memvalidasi kombinasi purpose, status, dan relasi akun yang diperbolehkan.
- Jangan mengubah sesi pemulihan tanpa akun menjadi sesi login atau sesi terautentikasi.
- Pengaitan akun tujuan harus dilakukan secara atomik dan hanya sekali. Perubahan tujuan setelah pengaitan memerlukan alur baru atau prosedur terkontrol yang diaudit.
- Challenge tetap terikat pada sesi, tujuan, kanal, dan alamat penerima yang sama. OTP tidak boleh berlaku untuk tujuan lain.
- Sesi yang kedaluwarsa, dibatalkan, atau dikunci tidak boleh digunakan untuk memilih atau memulihkan akun.
- account_recovery_cases dibuat sejak awal proses dan tetap menjadi catatan audit. Setiap kasus wajib terhubung ke tepat satu `verification_session_id`, dan satu sesi verifikasi hanya boleh memiliki maksimal satu kasus pemulihan (foreign key wajib dengan unique constraint). Pengiriman ulang OTP menggunakan sesi dan kasus yang sama; percobaan pemulihan baru membuat sesi serta kasus baru. `target_user_id` boleh NULL sampai pengguna memilih akun tujuan. Setelah akun dipilih dan pemeriksaan yang sesuai selesai, pengaitan `target_user_id` dilakukan secara atomik dan hanya sekali. Kasus tanpa target tetap dapat diproses melalui antrean berdasarkan status dan waktu, tanpa mengungkap identitas akun.

## 3. Kontak pemulihan dapat digunakan bersama

Satu email atau nomor HP pemulihan yang sudah diverifikasi boleh dihubungkan ke lebih dari satu akun. Ini berbeda dari kontak utama pendaftaran, yang tetap tunduk pada kebijakan reservasi dan keunikan akun.

Implikasi:

- Jangan membuat unique constraint global pada nilai email maupun nomor HP pemulihan; setiap kontak dapat memiliki beberapa baris relasi pemilik.
- Gunakan indeks non-unik berdasarkan kanal dan keyed hash kontak untuk menemukan akun-akun yang terhubung tanpa membuka nilai kontak.
- Simpan relasi pemilik kontak melalui foreign key ke akun; relasi satu email atau nomor HP ke beberapa akun direpresentasikan melalui beberapa baris.
- Simpan nilai kontak secara aman. Untuk pencarian dan deduplikasi, gunakan representasi normalisasi serta keyed hash yang stabil dan dikelola server. Jika nilai asli perlu dikirim ulang atau ditampilkan kepada pemiliknya, lindungi dengan enkripsi yang sesuai; jangan gunakan hash sebagai pengganti data yang harus dipulihkan.
- Perubahan, penghapusan, verifikasi ulang, dan pelepasan kontak harus dicatat serta diberi notifikasi yang sesuai.
- Kontak pemulihan yang sama pada beberapa akun tidak boleh menyebabkan akun-akun tersebut digabungkan, saling berbagi sesi, atau mewarisi role.

## 4. Pemilihan akun tujuan setelah verifikasi

Alur yang disetujui adalah pendekatan hybrid yang mengutamakan kemudahan:

1. Pengguna memasukkan kontak pemulihan.
2. Sistem memverifikasi kendali atas kontak.
3. Jika hanya ada satu akun yang memenuhi syarat, sistem melanjutkan ke pemeriksaan akun tersebut tanpa membocorkan informasi akun sebelum verifikasi.
4. Jika ada beberapa akun yang memenuhi syarat, sistem menawarkan pemilihan akun dengan petunjuk pembeda yang aman dan tidak sensitif.
5. Pengguna memilih akun yang dimaksud.
6. Sistem melakukan pemeriksaan kepemilikan dan risiko untuk akun terpilih sebelum mengizinkan tindakan pemulihan.
7. Jika bukti tidak cukup atau risikonya tinggi, sistem meminta pemeriksaan tambahan, menunda, atau mengarahkan ke pemeriksaan manual sesuai kebijakan.

Petunjuk akun tidak boleh menampilkan email/nomor HP lengkap, data bisnis privat, informasi sensitif, atau detail yang mengungkap akun lain. Jangan menampilkan jumlah akun terkait sebelum verifikasi. Bahkan setelah verifikasi kontak, daftar pilihan hanya boleh memuat petunjuk minimum yang aman. Keberhasilan OTP bukan satu-satunya bukti kepemilikan akun.

Jika tidak ada akun yang dapat dipastikan dengan aman, gunakan respons yang menjaga privasi dan tawarkan jalur bantuan. Jangan mengonfirmasi keberadaan akun yang tidak berhak diketahui pemohon.

## 5. Batas penyalahgunaan lintas sesi

Batas percobaan dan pengiriman tidak boleh dapat diakali dengan membuat sesi baru.

- Batas lima percobaan gagal dan penguncian satu jam berlaku pada konteks verifikasi yang ditentukan kebijakan, bukan hanya pada satu baris challenge.
- Pengiriman ulang 60 detik dan maksimal lima pengiriman per alamat kontak per jam tetap berlaku meski sesi aplikasi dibuat ulang.
- Terapkan batas terdistribusi yang persisten berdasarkan kombinasi yang relevan, sekurang-kurangnya kanal, keyed hash tujuan, dan purpose; tambahkan IP/jaringan, akun, perangkat, serta sinyal risiko sesuai kebutuhan.
- Jangan menyimpan penghitung hanya di memori proses atau mengandalkan timer frontend.
- Perubahan penghitung, konsumsi challenge, pembatalan challenge lama, dan penerbitan challenge baru harus aman terhadap request bersamaan.
- Batas lintas sesi harus mencegah penggandaan kesempatan, tetapi jangan sampai pengguna yang sah terkunci permanen karena penyalahgunaan pihak lain. Tetapkan mekanisme pemulihan pembatasan yang aman dan dapat diaudit.
- Jangan membocorkan aturan deteksi atau nilai ambang internal melalui pesan error.

Strategi penyimpanan akhir (tabel penghitung, bucket waktu, atau layanan rate-limit terdistribusi) belum dipilih. Pilihan tersebut harus dibuat saat desain implementasi dengan mempertimbangkan konkurensi, retensi, privasi, dan operasi multi-instance.

## 6. Reservasi kontak utama selama penghapusan

- Kontak utama email/nomor HP tetap direservasi selama masa tunggu penghapusan permanen 30 hari.
- Selama masa tunggu, kontak tersebut tidak dapat dipakai untuk membuat akun baru.
- Pembatalan penghapusan memerlukan verifikasi keamanan; akun dipulihkan sesuai pemeriksaan status dan risiko.
- Setelah penghapusan permanen selesai, penggunaan ulang kontak mengikuti kebijakan retensi dan pelepasan identitas yang terpisah.
- Kebijakan reservasi ini hanya berlaku pada kontak utama yang menjadi identitas pendaftaran. Email dan nomor HP pemulihan yang boleh digunakan bersama tidak tunduk pada unique constraint global.
- Proses pembersihan harus mengunci/memeriksa status akun secara aman dan tidak boleh melepaskan reservasi jika penghapusan dibatalkan atau akun aktif kembali.

Rancangan constraint dan indeks harus membedakan kontak utama pada users dari relasi di recovery_contacts. Jangan mengandalkan validasi frontend untuk menjamin reservasi.

## 7. Checklist sebelum migrasi

- [ ] verification_sessions.user_id nullable hanya untuk purpose pemulihan sebelum pemilihan akun.
- [ ] Setiap account_recovery_cases memiliki tepat satu verification_session_id yang wajib dan unique; satu sesi hanya memiliki maksimal satu kasus. OTP resend memakai kasus/sesi yang sama, percobaan baru membuat keduanya baru.
- [ ] account_recovery_cases.target_user_id nullable hanya sebelum akun tujuan dipilih; target dikaitkan atomik setelah pemeriksaan.
- [ ] Constraint/check atau validasi transaksi menolak kombinasi purpose/status/relasi yang tidak sah.
- [ ] Email dan nomor HP pemulihan dapat terhubung ke beberapa akun tanpa unique constraint global.
- [ ] Pencarian kontak pemulihan menggunakan keyed hash yang aman dan indeks non-unik.
- [ ] Nilai kontak yang perlu dipakai kembali dilindungi dengan enkripsi yang sesuai.
- [ ] OTP terikat pada purpose, kanal, dan kontak; tidak dapat dipakai lintas akun atau tujuan.
- [ ] Pemilihan akun tidak membocorkan daftar atau jumlah akun kepada pihak yang belum berwenang.
- [ ] Pengaitan akun tujuan atomik, satu kali, dan diaudit.
- [ ] Rate limit berlaku lintas sesi dan instance serta tahan terhadap request bersamaan.
- [ ] Kontak utama tetap direservasi selama masa tunggu penghapusan 30 hari.
- [ ] Kontak pemulihan email dan nomor HP tidak ikut terkena aturan keunikan kontak utama.
- [ ] Tes integrasi mencakup kontak bersama, beberapa akun, pemilihan akun, enumerasi, concurrency, penguncian, kedaluwarsa, pembatalan penghapusan, dan pelepasan reservasi.

Tidak ada SQL migration yang dibuat oleh dokumen ini. Keputusan tentang state machine final, kolom tambahan, strategi rate-limit, enkripsi, dan retensi tetap memerlukan review sebelum implementasi.
