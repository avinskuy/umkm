# Tinjauan Skema Identity Core — Verifikasi dan Pemulihan

> Cakupan: kontrak kontak pemulihan bersama, pemilihan akun sasaran, dan perlindungan terhadap penyalahgunaan. Registrasi email/WhatsApp tetap mewajibkan OTP; login rutin menggunakan password tanpa OTP.

> Status implementasi: skema recovery, operasi PostgreSQL, service core, dan router HTTP telah tersedia serta diuji. Sinyal sesi aktif sebelumnya dan service review Admin juga tersedia; keputusan review memerlukan sesi Admin aktif dan tercatat append-only. Router belum dipasang pada server default. Password reset dan pencabutan sesi setelah reset belum tersedia.

Dokumen ini menjelaskan invariant kontak pemulihan bersama, pemilihan akun sasaran, dan perlindungan penyalahgunaan. Skema, operasi PostgreSQL, service core, dan router HTTP tersedia; aktivasi router dan provider produksi tetap terpisah.

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
- Gunakan unique constraint komposit `(user_id, channel, contact_hash)` agar satu akun tidak memiliki relasi duplikat untuk kontak/kanal yang sama. Constraint ini tetap mengizinkan kontak yang sama dipakai oleh akun berbeda.
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

Operasi verifikasi OTP menggunakan PostgreSQL untuk menegakkan batas lintas sesi dengan fingerprint kontak berbasis HMAC. Kontrol tambahan berdasarkan IP, perangkat, jaringan, dan risiko harus diterapkan melalui abuse guard pada lapisan HTTP; router recovery saat ini menerima guard melalui dependency injection.

## 6. Reservasi kontak utama selama penghapusan

- Kontak utama email/nomor HP tetap direservasi selama masa tunggu penghapusan permanen 30 hari.
- Selama masa tunggu, kontak tersebut tidak dapat dipakai untuk membuat akun baru.
- Pembatalan penghapusan memerlukan verifikasi keamanan; akun dipulihkan sesuai pemeriksaan status dan risiko.
- Setelah penghapusan permanen selesai, penggunaan ulang kontak mengikuti kebijakan retensi dan pelepasan identitas yang terpisah.
- Kebijakan reservasi ini hanya berlaku pada kontak utama yang menjadi identitas pendaftaran. Email dan nomor HP pemulihan yang boleh digunakan bersama tidak tunduk pada unique constraint global.
- Proses pembersihan harus mengunci/memeriksa status akun secara aman dan tidak boleh melepaskan reservasi jika penghapusan dibatalkan atau akun aktif kembali.

Rancangan constraint dan indeks harus membedakan kontak utama pada users dari relasi di recovery_contacts. Jangan mengandalkan validasi frontend untuk menjamin reservasi.

## 7. Invariant database dan runtime

- verification_sessions.user_id nullable hanya untuk purpose pemulihan sebelum pemilihan akun.
- Setiap account_recovery_cases memiliki tepat satu verification_session_id yang wajib dan unique; satu sesi hanya memiliki maksimal satu kasus. OTP resend memakai kasus/sesi yang sama, percobaan baru membuat keduanya baru.
- account_recovery_cases.target_user_id nullable hanya sebelum akun tujuan dipilih; target dikaitkan atomik setelah pemeriksaan.
- Constraint/check atau validasi transaksi menolak kombinasi purpose/status/relasi yang tidak sah.
- Email dan nomor HP pemulihan dapat terhubung ke beberapa akun tanpa unique constraint global.
- Relasi kontak yang sama tidak dapat didaftarkan dua kali pada akun yang sama, tetapi tetap boleh dipakai akun berbeda.
- Pencarian kontak pemulihan menggunakan keyed hash yang aman dan indeks non-unik.
- Nilai kontak yang perlu dipakai kembali dilindungi dengan enkripsi yang sesuai.
- OTP terikat pada purpose, kanal, dan kontak; tidak dapat dipakai lintas akun atau tujuan.
- Pemilihan akun tidak membocorkan daftar atau jumlah akun kepada pihak yang belum berwenang.
- Pengaitan akun tujuan atomik, satu kali, dan diaudit.
- Rate limit berlaku lintas sesi dan instance serta tahan terhadap request bersamaan.
- Kontak utama tetap direservasi selama masa tunggu penghapusan 30 hari.
- Kontak pemulihan email dan nomor HP tidak ikut terkena aturan keunikan kontak utama.
- Tes integrasi mencakup kontak bersama, beberapa akun, pemilihan akun, enumerasi, concurrency, penguncian, kedaluwarsa, pembatalan penghapusan, dan pelepasan reservasi.

Migration dan operasi database untuk recovery telah tersedia. Perubahan skema berikutnya harus menggunakan migration baru dan mempertahankan invariant yang dijelaskan pada dokumen ini.


## 8. Pembukaan kembali sesi setelah penguncian OTP

Setelah lima percobaan verifikasi gagal, sesi dikunci selama satu jam. Aturan pembukaan kembali:

- Selama satu jam penguncian, seluruh percobaan verifikasi pada sesi ditolak, termasuk ketika pengguna meminta atau menerima OTP baru.
- Setelah satu jam benar-benar berlalu, sesi yang sama boleh digunakan kembali dan penghitung kegagalan sesi direset.
- Reset penghitung hanya terjadi setelah masa penguncian berakhir. Penerbitan OTP baru, resend, refresh halaman, atau pembuatan challenge baru tidak dapat mempercepat pembukaan kunci.
- Challenge individual tetap mengikuti status dan masa berlakunya sendiri. Challenge yang kedaluwarsa, dibatalkan/digantikan, atau sudah digunakan tidak pernah diaktifkan kembali.
- Batas pengiriman per kontak serta rate limit lintas sesi dan instance tetap berlaku. Sesi baru tidak boleh menjadi cara untuk menghindari pembatasan.
- Pemeriksaan waktu dan reset penghitung dilakukan di server secara atomik. Request bersamaan pada batas akhir penguncian tidak boleh menghasilkan lebih dari satu reset yang tidak konsisten.

Reset penghitung sesi setelah masa penguncian dilakukan secara atomik oleh operasi verifikasi. Penyimpanan rate limit OTP menggunakan operasi PostgreSQL terkontrol agar batas lintas sesi tetap konsisten.

## 9. Status siklus hidup dan kondisi penguncian sesi

Status siklus hidup sesi verifikasi harus dipisahkan dari kondisi penguncian sementara. Jangan menambahkan `locked` sebagai status siklus hidup sesi.

| Dimensi | Nilai baseline | Makna |
| --- | --- | --- |
| Status siklus hidup | `pending` | Sesi masih menunggu verifikasi yang sah. |
| Status siklus hidup | `verified` | Tujuan verifikasi berhasil dipenuhi; terminal. |
| Status siklus hidup | `expired` | Masa berlaku sesi berakhir; terminal. |
| Status siklus hidup | `cancelled` | Sesi dibatalkan; terminal. |
| Kondisi penguncian | `failed_attempt_count` dan `locked_until` | Menentukan apakah percobaan pada sesi pending untuk sementara harus ditolak. |

Aturan perilaku:

- Sesi `pending` yang tidak sedang terkunci dapat menerima percobaan verifikasi sesuai kebijakan OTP.
- Sesi `pending` yang waktu `locked_until`-nya masih di masa depan tetap berstatus `pending`, tetapi semua percobaan verifikasi ditolak, termasuk menggunakan OTP baru.
- Setelah waktu penguncian berakhir, server mereset penghitung kegagalan sesi secara atomik. Sesi `pending` yang sama dapat digunakan kembali.
- Reset tersebut tidak menghapus batas pengiriman per kontak atau rate limit lintas sesi/instance.
- Sesi `verified`, `expired`, dan `cancelled` bersifat terminal dan tidak dapat dibuka kembali hanya karena waktu penguncian telah berakhir.
- Status challenge OTP individual tetap terpisah. Challenge yang kedaluwarsa, dibatalkan/digantikan, atau sudah digunakan tidak pernah diaktifkan kembali.
- Pemeriksaan status, waktu penguncian, dan reset penghitung harus dilakukan di server dalam operasi yang aman terhadap request bersamaan.

Status di atas merupakan kontrak lifecycle sesi dan challenge yang harus dipertahankan oleh implementasi database dan runtime.



## 10. State machine challenge OTP individual

Siklus hidup challenge OTP individual dipisahkan dari siklus hidup sesi verifikasi.

| Status | Makna | Terminal |
| --- | --- | --- |
| `pending` | Challenge masih aktif dan dapat diverifikasi jika sesi induk mengizinkan. | Tidak |
| `consumed` | Kode berhasil diverifikasi dan telah digunakan satu kali. | Ya |
| `expired` | Masa berlaku challenge berakhir. | Ya |
| `superseded` | Challenge digantikan oleh challenge baru. | Ya |
| `cancelled` | Sesi verifikasi induk dibatalkan. | Ya |

Transisi baseline yang diizinkan:
- `pending` → `consumed`: verifikasi kode berhasil.
- `pending` → `expired`: masa berlaku challenge berakhir.
- `pending` → `superseded`: challenge baru diterbitkan untuk sesi dan tujuan yang sama.
- `pending` → `cancelled`: sesi induk dibatalkan.

Aturan integritas:
- Semua status selain `pending` bersifat terminal dan tidak boleh diaktifkan kembali.
- Satu sesi verifikasi hanya boleh memiliki satu challenge aktif pada satu waktu.
- Kegagalan memasukkan kode dihitung pada sesi verifikasi, bukan sebagai perubahan status challenge atau pembuatan challenge baru.
- Verifikasi berhasil harus mengonsumsi challenge dan menyelesaikan sesi secara atomik.
- Challenge harus tetap terikat pada sesi, purpose, kanal, dan tujuan penerima yang benar.
- Sesi yang sedang terkunci tetap menolak verifikasi meskipun challenge berstatus `pending`.

## 11. Prinsip UX OTP yang diadopsi

UX mengikuti pola yang familier dari aplikasi verifikasi yang telah digunakan luas, dengan penyesuaian pada kebijakan keamanan USahapintar:

1. Tampilkan alamat email atau nomor tujuan dalam bentuk tersamarkan agar pengguna dapat mengenali tujuan pengiriman.
2. Gunakan input kode yang mendukung autofill sistem operasi/browser bila tersedia, paste, dan input manual.
3. Tampilkan masa berlaku kode serta hitung mundur sebelum pengguna dapat meminta kode baru.
4. Sediakan tindakan untuk memperbaiki alamat email atau nomor jika salah.
5. Berikan pesan kesalahan yang jelas; jangan menghapus input tanpa alasan ketika terjadi gangguan jaringan.
6. Ketika sesi dikunci, jelaskan bahwa pengguna harus menunggu dan tampilkan sisa waktu. Meminta kode baru tidak boleh memberi kesan bahwa penguncian dapat dilewati.
7. Ingatkan pengguna agar tidak membagikan kode kepada siapa pun.
8. Jangan tampilkan istilah status internal seperti `consumed` atau `superseded`; tampilkan pesan dan tindakan yang dipahami pengguna.

Kode OTP menggunakan enam digit numerik sesuai kebijakan verifikasi. Autofill mengikuti kemampuan platform dan tidak boleh meminta izin luas untuk membaca SMS. Validasi kode wajib dilakukan di server.

Acuan:
- WhatsApp Help Center, [Cara Mendaftarkan Nomor Telepon](https://faq.whatsapp.com/684051319521343/?cms_platform=android&locale=id_ID).
- Google for Developers, [Automatic SMS Verification with the SMS Retriever API](https://developers.google.com/identity/sms-retriever/overview).
- Android Developers, [Secure user authentication](https://developer.android.com/security/fraud-prevention/authentication).




## 12. Penerbitan challenge selama sesi terkunci

Pengguna boleh meminta dan menerima OTP baru selama sesi verifikasi terkunci, selama jeda resend 60 detik, batas lima pengiriman per kontak per jam, serta rate limit lain terpenuhi.

Aturan keamanan:
- Penerbitan challenge baru tidak menghapus, memperpendek, atau mengatur ulang `locked_until` maupun penghitung kegagalan sesi.
- Semua percobaan verifikasi tetap ditolak sampai waktu penguncian berakhir, termasuk percobaan dengan challenge baru yang diterbitkan selama penguncian.
- Challenge baru menggantikan challenge sebelumnya sesuai transisi `pending` → `superseded`. Challenge yang sudah terminal tidak dapat digunakan kembali.
- Jika challenge baru kedaluwarsa sebelum sesi terbuka, pengguna dapat meminta challenge berikutnya setelah batas resend/pengiriman mengizinkan.
- Setelah penguncian berakhir, sesi `pending` yang sama dapat digunakan kembali sesuai aturan reset penghitung kegagalan.
- Penerbitan tetap tunduk pada rate limit lintas sesi dan instance; membuat sesi baru tidak boleh menghindari batas tersebut.

Pengguna tetap dapat memperoleh kode terbaru saat menunggu, tetapi tidak dapat menggunakannya untuk melewati penguncian verifikasi.
