# Identity Core — Registration Decisions

> Status: batas awal kebijakan produk Identity Core telah disetujui; rancangan teknis terperinci, pemilihan penyedia layanan, dan tinjauan keamanan tetap harus diselesaikan sebelum implementasi.

## Approved registration policy

1. Public registration is available only for Business Owner and Investor.
2. The registrant chooses one permanent primary role, then exactly one initial contact method:
   - Email: a one-time code is delivered to the supplied email address.
   - Phone: a one-time code is delivered through WhatsApp.
3. Email and phone are alternatives. Both are not required during initial registration.
4. Email- or phone-based registration requires a password. Registration through Google OAuth does not require the user to create or enter a separate local password.
5. The account remains pending verification until the selected channel's code is successfully verified. Verification is an account-activation and anti-bot control, not a passwordless login method.
6. The selected contact is normalized and uniquely reserved. A contact already associated with another account cannot be reused to create or merge an account.
7. The non-selected contact may be added later, but must be verified independently before being treated as verified or used for security-sensitive recovery.
8. Phone numbers use E.164 normalization. WhatsApp delivery is a server-side provider integration; credentials must never enter source control or frontend configuration.
9. OTPs are never stored or logged in plaintext. Use keyed hashes, expiry, one-time use, bounded attempts, resend cooldown, and destination/account/network rate limits. Delivery failure never marks a contact verified.
10. Registration and recovery responses must not reveal whether a contact is already associated with an account.

## Role and identity invariants

- Setiap akun klien hanya memiliki satu role utama yang disimpan secara permanen dan tidak dapat diubah: `business_owner` atau `investor`.
- A person who wants the other role must register a separate account using a different, eligible, unlinked contact identity.
- Admin accounts are provisioned internally and cannot use public registration.
- Identity linking must prove control of both the existing account and the new identity. Matching email text alone is never sufficient.
- No account merge or role mutation is permitted through registration, OAuth, support, or routine Admin operations.

## Proposed registration lifecycle

1. User selects Owner or Investor.
2. User selects Email or Phone/WhatsApp.
3. Server validates and normalizes the contact, applies abuse limits, and creates a pending registration/challenge.
4. Server sends a one-time code through the selected delivery channel.
5. User submits the code; the server validates expiry, attempt budget, challenge state, purpose, and destination binding. This verifies contact ownership and anti-bot eligibility; it does not authenticate a login session.
6. In one transaction, consume the challenge, mark the selected contact verified, and activate the account only when all account-creation conditions are satisfied.
7. User continues to role-specific onboarding.

Pendaftaran tertunda kedaluwarsa setelah **30 menit** jika verifikasi kontak yang diwajibkan belum selesai. Proses kedaluwarsa dan pembersihan harus atomik serta idempotent, melepaskan reservasi kontak tertunda yang memenuhi syarat dengan aman, dan tidak pernah menghapus akun yang sudah aktif. Cegah pendaftaran tertunda aktif yang duplikat dan kondisi balapan saat reservasi.

## Data-model implications (proposal only)

The eventual schema should distinguish normalized email and E.164 phone values, the initial registration contact method, independent verification timestamps, challenge delivery channel (email or WhatsApp), challenge purpose and destination binding, keyed code hash, attempts, expiry, consumption, and pending-registration expiry.

Do not infer that the user supplied both contact methods. Do not create a migration until the complete Identity Core decisions and schema review gate are approved.

## Kebijakan Identity Core yang telah disetujui

### Google OAuth dan model kata sandi

Google OAuth tetap digunakan untuk pendaftaran dan login. Pengguna baru harus memilih tepat satu role klien yang tidak dapat diubah (Owner atau Investor) sebelum memulai OAuth; pilihan role diikat ke state OAuth yang dikelola server. Pendaftaran yang hanya menggunakan Google tidak mewajibkan pembuatan kata sandi lokal. Pengguna baru tetap harus memilih dan memverifikasi satu kontak pendaftaran terpisah (email atau nomor HP melalui WhatsApp) sebelum akun diaktifkan. Email yang dinyatakan oleh Google tidak menggantikan verifikasi kontak tersebut.

Pendaftaran menggunakan email atau nomor HP wajib memiliki kata sandi lokal. Login kata sandi menggunakan email atau nomor HP terdaftar beserta kata sandinya. OTP pendaftaran hanya digunakan untuk verifikasi kontak, aktivasi akun, dan perlindungan anti-bot; OTP bukan kredensial login dan tidak dapat digunakan untuk pemulihan kata sandi. Alur penambahan kata sandi lokal untuk akun yang hanya menggunakan Google masih menjadi keputusan produk terpisah jika kelak diperlukan.

Identitas Google yang sudah terhubung hanya dapat mengautentikasi akun yang sudah terhubung tersebut. Jangan menghubungkan akun secara otomatis hanya karena alamat email cocok. Penghubungan memerlukan sesi akun yang sudah terautentikasi serta bukti terbaru atas kendali identitas baru, atau proses pemulihan aman yang secara khusus disetujui. Validasi state, nonce, issuer, audience, signature, expiry, redirect URI yang tepat, provider subject, dan perlindungan terhadap callback yang diputar ulang. Akun Admin tidak boleh dibuat melalui OAuth publik.

### OTP dan siklus pendaftaran

- Masa berlaku OTP adalah **10 menit**.
- Maksimal **5 percobaan gagal** untuk setiap sesi verifikasi.
- Setelah kegagalan kelima, sesi verifikasi dikunci selama **1 jam**. Pengiriman ulang atau permintaan kode pengganti tidak boleh mengatur ulang atau melewati penguncian.
- Jeda pengiriman ulang adalah **60 detik**.
- Maksimal **5 pengiriman per jam untuk setiap alamat kontak**, ditambah pembatasan adaptif berdasarkan IP/jaringan, akun, tujuan, dan aktivitas mencurigakan.
- Pengiriman OTP baru langsung membatalkan kode sebelumnya untuk tujuan dan alamat kontak yang sama.
- Kode baru tidak menghapus status penguncian sesi verifikasi atau batas pengiriman.
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

Migrasi Identity Core dan endpoint runtime baru boleh dimulai setelah kebijakan yang disetujui diterjemahkan menjadi batasan skema yang telah ditinjau, batas tanggung jawab penyedia layanan ditentukan, rancangan konkurensi/kedaluwarsa/pembersihan/privasi diperiksa, dan rencana pengujian disetujui. Pemilihan penyedia layanan, ambang operasional terperinci, jadwal retensi, panjang kode OTP, dan pengaturan kata sandi lokal untuk akun Google saja masih menjadi keputusan teknis/produk yang terbuka. Pendaftaran melalui email/nomor HP wajib menggunakan kata sandi; pendaftaran Google OAuth tidak memerlukan kata sandi lokal. OTP melalui telepon/WhatsApp bukan metode login. Seluruh kebijakan tetap dapat ditinjau ulang melalui evaluasi produk dan keamanan.


## Tambahan keputusan: kontak pemulihan bersama dan UX pemilihan akun

- Satu email atau nomor HP pemulihan yang sudah diverifikasi boleh digunakan oleh beberapa akun. Jangan menerapkan keunikan global pada nilai email maupun nomor HP di `recovery_contacts`. Aturan ini tidak mengubah keunikan dan reservasi kontak utama pendaftaran.
- Jika satu kontak pemulihan terhubung ke beberapa akun, gunakan pendekatan hybrid: verifikasi kontak terlebih dahulu, lalu tampilkan pilihan akun dengan petunjuk pembeda yang aman. Jangan mengungkap jumlah atau daftar akun sebelum verifikasi, dan jangan menampilkan data sensitif atau kontak lengkap.
- Keberhasilan verifikasi kontak hanya membuktikan kendali atas kontak tersebut; bukan bukti tunggal kepemilikan semua akun yang terhubung. Akun yang dipilih tetap menjalani pemeriksaan kepemilikan dan risiko.
- Kolom verification_sessions.user_id boleh NULL sementara hanya pada alur pemulihan sebelum akun tujuan dipilih. Untuk pendaftaran dan verifikasi akun tertentu, user_id wajib. Setelah akun dipilih dan pemeriksaan yang sesuai selesai, sesi/kasus dikaitkan ke akun tersebut secara atomik dan satu kali.
- Batas percobaan dan pengiriman harus berlaku lintas sesi dan instance, sehingga membuat sesi baru tidak mengatur ulang batas. Penghitung dan penguncian ditegakkan di server serta tahan terhadap request bersamaan.
- Kontak utama tetap direservasi selama masa tunggu penghapusan permanen 30 hari. Kontak tersebut baru dapat dilepas setelah proses penghapusan selesai dan sesuai kebijakan retensi/pelepasan identitas. Aturan ini tidak berlaku pada kontak pemulihan bersama.

- `account_recovery_cases` boleh dibuat sebelum akun tujuan diketahui. `target_user_id` sementara NULL dan kasus tetap terhubung ke sesi verifikasi; setelah pemilihan akun serta pemeriksaan keamanan selesai, target dikaitkan secara atomik dan satu kali. Kasus tanpa target tetap tercatat dan dapat masuk antrean pemeriksaan tanpa membuka identitas akun.

Rincian teknis dan checklist migrasi terdapat pada [Tinjauan Skema Identity Core — Verifikasi dan Pemulihan](identity-core-shared-recovery-contacts.md).