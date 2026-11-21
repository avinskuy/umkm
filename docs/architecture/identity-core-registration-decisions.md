# Identity Core — Keputusan Pendaftaran

> Status: batas awal kebijakan produk Identity Core telah disetujui; rancangan teknis terperinci, pemilihan penyedia layanan, dan tinjauan keamanan tetap harus diselesaikan sebelum implementasi.

## Kebijakan pendaftaran yang disetujui

1. Pendaftaran publik hanya tersedia bagi Business Owner dan Investor.
2. Pendaftar memilih satu role utama yang bersifat tetap, lalu tepat satu metode kontak awal:
   - Email: kode sekali pakai dikirim ke alamat email yang diberikan.
   - Telepon: kode sekali pakai dikirim melalui WhatsApp.
3. Email dan telepon merupakan pilihan alternatif. Keduanya tidak wajib diberikan saat pendaftaran awal.
4. Pendaftaran melalui email atau telepon wajib menggunakan kata sandi. Pendaftaran melalui Google OAuth tidak mewajibkan pengguna membuat atau memasukkan kata sandi lokal terpisah.
5. Akun tetap berstatus menunggu verifikasi sampai kode dari kanal yang dipilih berhasil diverifikasi. Verifikasi merupakan kontrol aktivasi akun dan anti-bot, bukan metode login tanpa kata sandi.
6. Kontak yang dipilih dinormalisasi dan direservasi secara unik. Kontak yang sudah terhubung ke akun lain tidak dapat digunakan kembali untuk membuat atau menggabungkan akun.
7. Kontak yang tidak dipilih dapat ditambahkan kemudian, tetapi harus diverifikasi secara terpisah sebelum dianggap terverifikasi atau digunakan untuk pemulihan yang sensitif terhadap keamanan.
8. Nomor telepon dinormalisasi ke format E.164. Pengiriman WhatsApp merupakan integrasi penyedia di sisi server; kredensial tidak boleh masuk ke source control atau konfigurasi frontend.
9. OTP tidak pernah disimpan atau dicatat dalam log sebagai teks asli. Gunakan keyed hash, masa berlaku, penggunaan satu kali, batas percobaan, jeda kirim ulang, dan rate limit berdasarkan tujuan/akun/jaringan. Kegagalan pengiriman tidak pernah menandai kontak sebagai terverifikasi.
10. Respons pendaftaran dan pemulihan tidak boleh mengungkap apakah kontak sudah terhubung ke suatu akun.

## Invariant role dan identitas

- Setiap akun klien hanya memiliki satu role utama yang disimpan secara permanen dan tidak dapat diubah: `business_owner` atau `investor`.
- Seseorang yang ingin menggunakan role lainnya harus mendaftar akun terpisah dengan identitas kontak berbeda yang memenuhi syarat dan belum terhubung ke akun lain.
- Akun Admin disediakan melalui proses internal dan tidak dapat menggunakan pendaftaran publik.
- Pengaitan identitas harus membuktikan kendali atas akun yang sudah ada dan identitas baru. Kecocokan teks alamat email saja tidak pernah cukup.
- Penggabungan akun atau perubahan role tidak diizinkan melalui pendaftaran, OAuth, bantuan, maupun operasi Admin rutin.

## Rancangan siklus pendaftaran

1. Pengguna memilih Owner atau Investor.
2. Pengguna memilih Email atau Telepon/WhatsApp.
3. Server memvalidasi dan menormalisasi kontak, menerapkan batas penyalahgunaan, lalu membuat pendaftaran/challenge berstatus pending.
4. Server mengirim kode sekali pakai melalui kanal pengiriman yang dipilih.
5. Pengguna mengirim kode; server memvalidasi masa berlaku, batas percobaan, status challenge, tujuan penggunaan, dan keterikatan ke tujuan pengiriman. Proses ini memverifikasi kendali atas kontak dan kelayakan anti-bot; bukan autentikasi sesi login.
6. Dalam satu transaksi, konsumsi challenge, tandai kontak terpilih sebagai terverifikasi, dan aktifkan akun hanya jika seluruh persyaratan pembuatan akun terpenuhi.
7. Pengguna melanjutkan onboarding sesuai role.

Pendaftaran tertunda kedaluwarsa setelah **30 menit** jika verifikasi kontak yang diwajibkan belum selesai. Proses kedaluwarsa dan pembersihan harus atomik serta idempotent, melepaskan reservasi kontak tertunda yang memenuhi syarat dengan aman, dan tidak pernah menghapus akun yang sudah aktif. Cegah pendaftaran tertunda aktif yang duplikat dan kondisi balapan saat reservasi.

## Implikasi model data (usulan tahap verifikasi dan sesi)

The eventual schema should distinguish normalized email and E.164 phone values, the initial registration contact method, independent verification timestamps, challenge delivery channel (email or WhatsApp), challenge purpose and destination binding, keyed code hash, attempts, expiry, consumption, and pending-registration expiry.

Do not infer that the user supplied both contact methods. The first account/Google-identity migration has been implemented after schema review (`backend/migrations/20261121170000_create_identity_accounts.sql`). Verification sessions, OTP challenges, recovery, and runtime endpoints remain separate implementation slices and must pass their own schema review and tests.

## Kebijakan Identity Core yang telah disetujui

### Google OAuth dan model kata sandi

Google OAuth tetap digunakan untuk pendaftaran dan login. Pengguna baru harus memilih tepat satu role klien yang tidak dapat diubah (Owner atau Investor) sebelum memulai OAuth; pilihan role diikat ke state OAuth yang dikelola server. Pendaftaran yang hanya menggunakan Google tidak mewajibkan pembuatan kata sandi lokal. Pengguna baru tetap harus memilih dan memverifikasi satu kontak pendaftaran terpisah (email atau nomor HP melalui WhatsApp) sebelum akun diaktifkan. Email yang dinyatakan oleh Google tidak menggantikan verifikasi kontak tersebut.

Pendaftaran menggunakan email atau nomor HP wajib memiliki kata sandi lokal. Login kata sandi menggunakan email atau nomor HP terdaftar beserta kata sandinya. OTP pendaftaran hanya digunakan untuk verifikasi kontak, aktivasi akun, dan perlindungan anti-bot; OTP bukan kredensial login dan tidak dapat digunakan untuk pemulihan kata sandi. Alur penambahan kata sandi lokal untuk akun yang hanya menggunakan Google masih menjadi keputusan produk terpisah jika kelak diperlukan.

Identitas Google yang sudah terhubung hanya dapat mengautentikasi akun yang sudah terhubung tersebut. Jangan menghubungkan akun secara otomatis hanya karena alamat email cocok. Penghubungan memerlukan sesi akun yang sudah terautentikasi serta bukti terbaru atas kendali identitas baru, atau proses pemulihan aman yang secara khusus disetujui. Validasi state, nonce, issuer, audience, signature, expiry, redirect URI yang tepat, provider subject, dan perlindungan terhadap callback yang diputar ulang. Akun Admin tidak boleh dibuat melalui OAuth publik.

### OTP dan siklus pendaftaran

- Masa berlaku OTP adalah **10 menit**.
- Maksimal **5 percobaan gagal** untuk setiap sesi verifikasi.
- Setelah kegagalan kelima, sesi verifikasi dikunci selama **1 jam**. Selama penguncian, semua percobaan verifikasi pada sesi tersebut ditolak, termasuk dengan OTP baru.
- Setelah satu jam berlalu, sesi yang sama boleh digunakan kembali dan penghitung kegagalan sesi direset. Reset hanya terjadi setelah masa penguncian benar-benar berakhir; permintaan atau pengiriman OTP baru tidak dapat mempercepatnya.
- Batas pengiriman per kontak dan rate limit lintas sesi tetap berlaku setelah sesi dibuka kembali. Membuat sesi baru tidak boleh menghindari batas percobaan atau pengiriman yang berlaku pada konteks kontak/tujuan.
- Jeda pengiriman ulang adalah **60 detik**.
- Maksimal **5 pengiriman per jam untuk setiap alamat kontak**, ditambah pembatasan adaptif berdasarkan IP/jaringan, akun, tujuan, dan aktivitas mencurigakan.
- Pengiriman OTP baru langsung membatalkan kode sebelumnya untuk tujuan dan alamat kontak yang sama.
- Kode baru tidak menghapus status penguncian sesi verifikasi, tidak mereset penghitung kegagalan, dan tidak menghapus batas pengiriman.
- Semua penghitung, masa berlaku, pembatalan, dan jeda harus ditegakkan di server serta aman terhadap proses bersamaan. Hitung mundur di sisi klien hanya untuk tampilan.
- OTP berupa kode angka dengan jumlah digit yang belum ditetapkan. Kode dibuat menggunakan sumber acak kriptografis yang aman, disimpan sebagai keyed hash, tidak pernah dicatat dalam log sebagai teks asli, terikat pada tujuan dan alamat penerima, serta hanya dapat digunakan satu kali secara atomik.
- Kegagalan pengiriman tidak pernah menandai kontak sebagai terverifikasi. Respons tidak boleh mengungkap apakah kontak sudah terdaftar.
- Masa berlaku pendaftaran tertunda adalah **30 menit**, terpisah dari masa berlaku OTP 10 menit. Pembersihan kedaluwarsa harus idempotent dan tidak boleh menghapus akun yang sudah aktif.

### Pengelolaan sesi

- Beberapa perangkat boleh memiliki sesi aktif secara bersamaan.
- Sediakan halaman Sesi Aktif pada Pengaturan Keamanan, pencabutan sesi individual, dan fitur logout dari semua perangkat.
- Pencabutan sesi ditegakkan oleh server; sesi mencurigakan dapat dicabut berdasarkan sinyal risiko.
- Batas awal sesi adalah **7 hari tanpa aktivitas** dan **30 hari secara absolut**.
- Fitur “Ingat perangkat ini” dapat mengaktifkan sesi browser persisten, tetapi tidak boleh melewati kedua batas waktu atau pencabutan dari server. Menutup browser saja tidak mengakhiri sesi yang memang dibuat persisten.
- Gunakan cookie Secure dan HttpOnly serta strategi SameSite/CSRF yang sesuai. Tindakan sensitif dapat meminta autentikasi ulang. Logout manual langsung mencabut sesi.

### Siklus hidup akun

- Penonaktifan sementara dan penghapusan permanen merupakan dua alur terpisah yang dimulai oleh pengguna.
- Penonaktifan sementara menyembunyikan profil publik dan membatasi akses normal, tetapi data akun serta role tetap disimpan. Pengaktifan kembali memerlukan login dan pemeriksaan keamanan.
- Penghapusan permanen memerlukan konfirmasi eksplisit dan autentikasi ulang, lalu **masa tunggu 30 hari**. Selama masa tersebut, akun disembunyikan/dibatasi dan pengguna dapat membatalkan permintaan melalui pemeriksaan keamanan ulang.
- Setelah masa tunggu, proses penghapusan dimulai. Catatan yang wajib dipertahankan untuk kebutuhan hukum/keamanan dan cadangan mengikuti kebijakan retensi transparan berdasarkan kategori data; jangan menjanjikan penghapusan seketika dari cadangan.
- Kontak tidak otomatis dapat digunakan kembali setelah akun ditutup/dihapus. Prosedur pelepasan harus diverifikasi dan tahan terhadap penyalahgunaan sebelum diterapkan.

### Pemulihan akun

Dukung alur **lupa kata sandi** dan **lupa email** melalui email pemulihan dan/atau nomor HP pemulihan yang sudah diverifikasi (dengan kode melalui WhatsApp). Kontak pemulihan bersifat opsional saat pendaftaran, dapat ditambahkan setelah aktivasi, dan harus diverifikasi secara terpisah sebelum digunakan. Sarankan pengguna menambahkannya saat onboarding tanpa menjadikannya syarat aktivasi.

- Lupa kata sandi: verifikasi kepemilikan melalui metode pemulihan yang memenuhi syarat sebelum mengizinkan pengaturan ulang kata sandi. OTP pendaftaran tidak dapat digunakan sebagai kredensial reset.
- Lupa email: setelah kontak pemulihan yang terhubung ke akun berhasil diverifikasi, pengguna boleh melihat alamat email utama secara lengkap. Jangan tampilkan sebelum verifikasi berhasil.
- Gunakan respons publik yang seragam, pembatasan permintaan, notifikasi, dan penilaian risiko untuk mencegah enumerasi akun serta penyalahgunaan.
- Pemulihan dilakukan berlapis dan berdasarkan risiko. Pertimbangkan perangkat tepercaya, sesi sebelumnya, faktor pemulihan yang sudah diverifikasi, aktivitas mencurigakan, dan perubahan faktor pemulihan terbaru. Terapkan jeda perlindungan atau pemeriksaan tambahan jika diperlukan.
- Jika pengguna kehilangan akses ke kontak utama dan seluruh kontak pemulihan, terima permintaan pemulihan alternatif dan periksa perangkat tepercaya, sesi sebelumnya, serta sinyal kepemilikan lainnya. Jika bukti otomatis belum cukup, eskalasikan ke pemeriksaan manual oleh petugas support dengan kewenangan khusus. Pemulihan berisiko tinggi dapat ditunda atau ditolak. Beri tahu pengguna melalui kanal aman yang tersedia dan catat setiap tindakan dalam audit.
- Pemeriksaan manual harus menerapkan hak akses minimum, kriteria bukti yang terdokumentasi, persetujuan dua pihak untuk pengecualian/risiko tinggi jika diperlukan, serta catatan audit yang tidak dapat diubah sembarangan. Petugas support tidak boleh mengubah role utama yang bersifat tetap melalui proses pemulihan.
- Perlindungan setelah pemulihan menyesuaikan risiko: cabut sesi mencurigakan, beri notifikasi kepada pengguna, dan minta verifikasi tambahan untuk perubahan sensitif jika diperlukan.

### Operasional penyedia layanan dan keputusan teknis tersisa

Penyedia email dan WhatsApp Business/API belum dipilih. Gunakan adapter penyedia yang dapat diganti, simpan kredensial di sisi server, dan tetapkan pemantauan pengiriman, percobaan ulang, batas biaya, persyaratan pengirim/template, penanggung jawab operasional, serta penanganan insiden sebelum produksi. Parameter hashing kata sandi, konfigurasi penyedia, jadwal retensi, MFA/pemulihan Admin, dan ambang operasional juga harus ditetapkan melalui tinjauan keamanan. Jangan membuat kredensial produksi atau menyimpan rahasia di source control.

## Gerbang implementasi

Migration akun Identity Core pertama sudah selesai. Migration verifikasi/OTP, sesi, recovery, serta endpoint runtime berikutnya hanya boleh dimulai setelah batasan skema slice terkait ditinjau, batas tanggung jawab penyedia layanan ditentukan, rancangan konkurensi/kedaluwarsa/pembersihan/privasi diperiksa, dan rencana pengujian disetujui. Pemilihan penyedia layanan, ambang operasional terperinci, jadwal retensi, panjang kode OTP, dan pengaturan kata sandi lokal untuk akun Google saja masih menjadi keputusan teknis/produk yang terbuka. Pendaftaran melalui email/nomor HP wajib menggunakan kata sandi; pendaftaran Google OAuth tidak memerlukan kata sandi lokal. OTP melalui telepon/WhatsApp bukan metode login. Seluruh kebijakan tetap dapat ditinjau ulang melalui evaluasi produk dan keamanan.


## Tambahan keputusan: kontak pemulihan bersama dan UX pemilihan akun

- Satu email atau nomor HP pemulihan yang sudah diverifikasi boleh digunakan oleh beberapa akun. Jangan menerapkan keunikan global pada nilai email maupun nomor HP di `recovery_contacts`. Aturan ini tidak mengubah keunikan dan reservasi kontak utama pendaftaran.
- Jika satu kontak pemulihan terhubung ke beberapa akun, gunakan pendekatan hybrid: verifikasi kontak terlebih dahulu, lalu tampilkan pilihan akun dengan petunjuk pembeda yang aman. Jangan mengungkap jumlah atau daftar akun sebelum verifikasi, dan jangan menampilkan data sensitif atau kontak lengkap.
- Keberhasilan verifikasi kontak hanya membuktikan kendali atas kontak tersebut; bukan bukti tunggal kepemilikan semua akun yang terhubung. Akun yang dipilih tetap menjalani pemeriksaan kepemilikan dan risiko.
- Kolom verification_sessions.user_id boleh NULL sementara hanya pada alur pemulihan sebelum akun tujuan dipilih. Untuk pendaftaran dan verifikasi akun tertentu, user_id wajib. Setelah akun dipilih dan pemeriksaan yang sesuai selesai, sesi/kasus dikaitkan ke akun tersebut secara atomik dan satu kali.
- Batas percobaan dan pengiriman harus berlaku lintas sesi dan instance, sehingga membuat sesi baru tidak mengatur ulang batas. Penghitung dan penguncian ditegakkan di server serta tahan terhadap request bersamaan.
- Kontak utama tetap direservasi selama masa tunggu penghapusan permanen 30 hari. Kontak tersebut baru dapat dilepas setelah proses penghapusan selesai dan sesuai kebijakan retensi/pelepasan identitas. Aturan ini tidak berlaku pada kontak pemulihan bersama.

- `account_recovery_cases` boleh dibuat sebelum akun tujuan diketahui. `target_user_id` sementara NULL dan kasus tetap terhubung ke tepat satu `verification_session_id` melalui foreign key wajib dan unique. Satu sesi verifikasi hanya boleh memiliki satu kasus pemulihan; pengiriman ulang OTP menggunakan sesi/kasus yang sama, sedangkan percobaan pemulihan baru membuat sesi dan kasus baru. Setelah pemilihan akun serta pemeriksaan keamanan selesai, target dikaitkan secara atomik dan satu kali. Kasus tanpa target tetap tercatat dan dapat masuk antrean pemeriksaan tanpa membuka identitas akun.

Rincian teknis dan checklist migrasi terdapat pada [Tinjauan Skema Identity Core — Verifikasi dan Pemulihan](identity-core-shared-recovery-contacts.md).

### Pemisahan status siklus hidup dan penguncian sesi

Status siklus hidup sesi verifikasi dan kondisi penguncian sementara merupakan dua dimensi yang terpisah.

Status siklus hidup yang menjadi baseline:
- `pending`: sesi masih menunggu proses verifikasi yang sah.
- `verified`: tujuan verifikasi telah berhasil dipenuhi; status terminal.
- `expired`: masa berlaku sesi berakhir; status terminal.
- `cancelled`: sesi dibatalkan; status terminal.

Kondisi penguncian sementara dicatat secara terpisah melalui `failed_attempt_count` dan `locked_until`, bukan dengan mengganti status siklus hidup menjadi `locked`.

Konsekuensinya:
- Sesi `pending` tanpa penguncian dapat menerima percobaan verifikasi sesuai aturan OTP.
- Sesi `pending` dengan `locked_until` yang masih berlaku tetap berstatus `pending`, tetapi semua percobaan verifikasi ditolak sampai waktu penguncian berakhir.
- Setelah penguncian berakhir, server secara atomik mereset penghitung kegagalan sesi dan sesi `pending` yang sama dapat digunakan kembali. Batas pengiriman dan rate limit lintas sesi tetap berlaku.
- Status `verified`, `expired`, dan `cancelled` tidak dapat dibuka kembali. Penguncian yang berakhir tidak menghidupkan kembali sesi atau challenge yang sudah terminal.
- Status challenge OTP individual tetap dikelola terpisah dari status sesi verifikasi. Challenge yang kedaluwarsa, dibatalkan/digantikan, atau telah digunakan tidak diaktifkan kembali.

Status lifecycle akun sudah diterapkan pada migration akun pertama. Daftar status sesi verifikasi dan challenge di atas tetap merupakan baseline konseptual; nama kolom, constraint, transisi lengkap, dan pemetaan status challenge harus difinalisasi pada rancangan migration verifikasi tersendiri.



### Mesin status challenge OTP individual

Challenge OTP individual memiliki siklus hidup terpisah dari sesi verifikasi. Baseline status:

- `pending`: challenge masih aktif dan dapat diverifikasi selama sesi induk mengizinkan.
- `consumed`: kode berhasil diverifikasi dan hanya dapat digunakan satu kali.
- `expired`: masa berlaku challenge berakhir.
- `superseded`: challenge digantikan oleh challenge baru.
- `cancelled`: sesi verifikasi induk dibatalkan sehingga challenge tidak lagi berlaku.

Transisi yang diizinkan:
- `pending` → `consumed` setelah verifikasi berhasil.
- `pending` → `expired` setelah masa berlaku berakhir.
- `pending` → `superseded` ketika challenge baru diterbitkan untuk sesi dan tujuan yang sama.
- `pending` → `cancelled` ketika sesi induk dibatalkan.

Semua status selain `pending` bersifat terminal. Satu sesi hanya boleh memiliki satu challenge aktif pada satu waktu. Kegagalan memasukkan kode dihitung pada sesi verifikasi, bukan dengan membuat challenge baru. Verifikasi berhasil harus mengonsumsi challenge dan menyelesaikan sesi secara atomik. Challenge terminal tidak boleh diaktifkan kembali.

### Prinsip UX OTP yang diadopsi

Gunakan pola yang familier dari alur verifikasi aplikasi yang telah digunakan luas, tanpa menyalin implementasi internal penyedia tertentu:

- Tampilkan tujuan pengiriman dengan penyamaran yang cukup agar pengguna tahu ke mana kode dikirim.
- Sediakan input kode yang mendukung autofill sistem operasi/browser jika tersedia, paste, dan input manual.
- Tampilkan masa berlaku kode dan hitung mundur sebelum resend tersedia. Waktu di UI hanya informatif; server tetap menjadi sumber kebenaran.
- Sediakan cara memperbaiki tujuan pengiriman jika pengguna salah memasukkan email/nomor.
- Berikan umpan balik kesalahan yang jelas dan pertahankan input ketika aman, misalnya saat terjadi gangguan jaringan.
- Saat sesi dikunci, jelaskan bahwa pengguna perlu menunggu dan tampilkan sisa waktu secara jelas. Jangan memberi kesan bahwa meminta kode baru dapat melewati penguncian.
- Ingatkan pengguna agar tidak membagikan kode kepada siapa pun.
- Jangan tampilkan status internal seperti `consumed` atau `superseded` kepada pengguna; gunakan bahasa tindakan yang sederhana.

Panjang kode OTP tetap belum ditetapkan. Pilihan kanal alternatif, termasuk apakah tersedia metode pengiriman cadangan, juga belum ditetapkan. Penerapan autofill harus mengikuti kemampuan platform dan tidak boleh meminta izin luas untuk membaca SMS. Verifikasi kode selalu dilakukan di server.

Acuan pola UX dan kemampuan platform:
- WhatsApp Help Center, [Cara Mendaftarkan Nomor Telepon](https://faq.whatsapp.com/684051319521343/?cms_platform=android&locale=id_ID).
- Google for Developers, [Automatic SMS Verification with the SMS Retriever API](https://developers.google.com/identity/sms-retriever/overview).
- Android Developers, [Secure user authentication](https://developer.android.com/security/fraud-prevention/authentication).

Kebijakan penerbitan OTP selama sesi terkunci telah ditetapkan pada bagian **Keputusan: penerbitan OTP selama penguncian sesi** di bawah. Tidak ada keputusan kebijakan OTP tambahan yang perlu dibuka pada tahap dokumentasi ini. Untuk pengalaman pengguna, USahapintar mengadopsi pola verifikasi yang telah digunakan luas dan mengikuti dokumentasi resmi platform serta penyedia yang kelak dipilih; detail integrasi dan konfigurasi dibahas saat implementasi.


### Keputusan: penerbitan OTP selama penguncian sesi

Pengguna **boleh meminta dan menerima OTP baru selama sesi verifikasi terkunci**, selama jeda resend, batas pengiriman per kontak, dan rate limit lain terpenuhi.

Ketentuan:
- Pengiriman OTP baru tidak menghapus, memperpendek, atau mengatur ulang penguncian satu jam.
- Semua percobaan verifikasi tetap ditolak sampai `locked_until` berakhir, termasuk untuk challenge baru yang diterbitkan selama penguncian.
- Challenge baru menggantikan challenge sebelumnya sesuai aturan `superseded`; challenge lama tidak dapat digunakan kembali.
- Jika challenge baru kedaluwarsa sebelum sesi terbuka, pengguna dapat meminta OTP berikutnya setelah batas resend/pengiriman mengizinkan.
- Setelah penguncian berakhir, sesi `pending` yang sama dapat digunakan kembali sesuai aturan reset penghitung kegagalan yang sudah disetujui.
- Penerbitan OTP tetap tunduk pada batas lima pengiriman per kontak per jam dan rate limit lintas sesi/instance.

Tujuan UX: pengguna tetap dapat memperoleh kode terbaru saat menunggu, tanpa membuat kode tersebut menjadi jalan pintas untuk melewati penguncian keamanan.
