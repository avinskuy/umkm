# Keputusan Autentikasi Identity Core — Tahap Pertama

> Status: disetujui pada 22 November 2026  
> Cakupan: metode registrasi/login awal, pemilihan role, status kontak, dan penundaan OTP  
> Implementasi runtime: belum tersedia; dokumen ini menetapkan keputusan produk dan kontrak desain, bukan klaim fitur sudah aktif.

## 1. Keputusan

Tahap pertama USahapintar menyediakan dua metode autentikasi:

1. **Email atau nomor telepon + password** untuk registrasi dan login biasa.
2. **Google OAuth** sebagai alternatif registrasi dan login.

OTP tidak menjadi syarat registrasi maupun login pada tahap pertama. Implementasi OTP ditunda ke tahap berikutnya. Kode OTP tidak boleh digunakan sebagai pengganti password atau autentikasi Google.

Password lokal wajib untuk akun yang dibuat melalui formulir email/telepon. Akun yang dibuat hanya melalui Google tidak wajib memiliki password lokal. Password disimpan menggunakan Argon2id; password plaintext tidak pernah disimpan atau dicatat.

## 2. Pemilihan role tetap wajib

- Pendaftaran publik hanya menerima `business_owner` atau `investor`.
- Untuk pendaftaran Google baru, pengguna harus memilih role secara eksplisit **sebelum** memulai alur OAuth.
- Pilihan role diikat ke state OAuth yang diterbitkan dan divalidasi server; callback tidak boleh mengganti role.
- Login Google ke identitas yang sudah terhubung menggunakan role yang tersimpan pada akun. Pemilihan role bukan sarana mengganti role akun.
- Setiap akun hanya memiliki satu role utama yang immutable. Pengguna yang membutuhkan role lain harus membuat akun terpisah dengan identitas login lain.
- Tidak ada pendaftaran Admin publik, termasuk melalui Google OAuth.

## 3. Perilaku registrasi dan kontak

### Email/telepon + password

- Pengguna memilih satu identifier awal: email atau nomor telepon.
- Pengguna menetapkan password dan menyetujui Ketentuan serta Kebijakan Privasi.
- Akun dapat dibuat dan diaktifkan tanpa memasukkan kode OTP.
- Kontak yang belum dibuktikan kepemilikannya harus tetap ditandai belum terverifikasi. Jangan mengisi `email_verified_at`, `phone_verified_at`, atau `initial_contact_verified_at` hanya karena pengguna mengetik suatu alamat/nomor.
- Kontak yang belum terverifikasi tidak boleh dipakai sebagai bukti kepemilikan untuk pemulihan akun, perubahan kredensial, atau tindakan berisiko tinggi.
- Perubahan kontak utama tetap memerlukan pembuktian kepemilikan atas kontak baru sebelum nilai tersebut menggantikan kontak aktif.

### Google OAuth

- Untuk akun baru, role dipilih sebelum OAuth dan disimpan pada state server yang singkat, sekali pakai, serta terlindung dari CSRF/replay.
- Identitas Google dikenali melalui stable provider subject, bukan alamat email.
- Validasi issuer, audience/client ID, signature, expiry, state, nonce, redirect URI, dan subject wajib dilakukan.
- Klaim email dari Google tidak boleh dianggap terverifikasi kecuali token tervalidasi menyatakan `email_verified=true`. Klaim ini hanya membuktikan status email menurut penyedia pada saat autentikasi; jangan menggunakannya untuk menggabungkan akun.
- Jika email Google cocok dengan akun lokal yang sudah ada, jangan membuat tautan otomatis atau menggabungkan akun. Pengguna harus masuk ke akun yang ada dan menjalankan proses pengaitan eksplisit dengan autentikasi ulang.
- Akun Google-only tetap dapat login melalui identitas Google yang sudah terhubung walaupun tidak memiliki password lokal.

### Risiko identifier yang belum diverifikasi

Tanpa pembuktian kontak saat pendaftaran, seseorang dapat mencoba mendaftarkan email/nomor milik orang lain dan menahan identifier tersebut. Ini adalah trade-off nyata dari keputusan tanpa OTP, bukan risiko yang hilang hanya karena login memakai password.

- Terapkan rate limit dan deteksi pola registrasi massal/penyalahgunaan berdasarkan sinyal yang proporsional dan minim data.
- Pertahankan respons generik agar sistem tidak membocorkan apakah identifier digunakan akun lain.
- Jangan mengirim notifikasi yang mengungkap data akun atau mengizinkan pengambilalihan melalui dukungan tanpa pemeriksaan kepemilikan.
- Rancang prosedur pelepasan reservasi dan penanganan klaim identifier yang aman sebelum pendaftaran publik dibuka luas.
- Threat model dan uji penyalahgunaan harus secara eksplisit mengevaluasi account squatting. Jika mitigasinya tidak memadai, metode pembuktian alternatif (misalnya tautan verifikasi, bukan OTP) harus dibahas sebagai keputusan produk terpisah sebelum produksi.

## 4. Batas pemulihan akun pada tahap pertama

- Password reset harus menggunakan token acak, sekali pakai, kedaluwarsa, disimpan dalam bentuk hash, dan dikirim melalui kanal yang telah dibuktikan kepemilikannya.
- Kontak yang belum terverifikasi tidak boleh menjadi tujuan pemulihan yang dipercaya.
- Akun yang hanya memiliki nomor telepon belum terverifikasi tidak boleh menerima pemulihan melalui nomor tersebut. Sediakan jalur pemulihan yang aman setelah kanal verifikasi tersedia; jangan membuat bypass manual yang tidak diaudit.
- Akun Google-only dapat memulihkan akses melalui proses pemulihan akun Google. Jangan membuat password lokal otomatis.
- Jika kanal pemulihan yang aman belum tersedia, fitur reset untuk akun tersebut harus dinyatakan belum tersedia dan diarahkan ke proses bantuan yang memiliki pemeriksaan kepemilikan serta audit; jangan mengirim token ke alamat yang belum diverifikasi.
- Recovery contact bersama tetap mengikuti kebijakan yang sudah disetujui dan harus diverifikasi secara terpisah sebelum dipakai.

## 5. Kontrol keamanan yang tetap wajib

Penundaan OTP tidak menunda kontrol berikut:

- Rate limit registrasi dan login serta perlindungan brute-force/credential stuffing.
- Respons yang tidak membocorkan keberadaan akun.
- Hash password Argon2id dengan parameter yang diukur pada lingkungan deployment.
- Sesi server-side opaque, penyimpanan hash token, rotasi, expiry, dan pencabutan.
- Cookie `Secure`, `HttpOnly`, `SameSite`, perlindungan CSRF, pemeriksaan origin, dan TLS.
- RBAC + ABAC, least privilege, deny-by-default, dan pemeriksaan kepemilikan objek di backend.
- Audit event untuk registrasi, login, kegagalan autentikasi, pengaitan/pemutusan Google, reset password, dan tindakan sensitif.
- Notifikasi keamanan untuk peristiwa penting sesuai kebijakan.
- MFA Admin tetap wajib sebelum akses Admin produksi; penundaan OTP untuk pengguna klien tidak menghapus kewajiban MFA Admin.

## 6. Konsekuensi terhadap skema yang sudah ada

Migration Identity Core 0001 saat ini mengikat aktivasi akun pada verifikasi kontak awal, status `pending_verification`, dan deadline 30 menit. Constraint tersebut **belum sesuai** dengan keputusan autentikasi tahap pertama ini.

Migration 0001–0005 yang sudah ada tidak diubah dalam keputusan ini. Sebelum runtime registrasi/login diimplementasikan, diperlukan audit dan migration lanjutan yang diuji untuk:

1. Mengizinkan akun email/telepon + password dibuat langsung aktif tanpa mengisi timestamp verifikasi kontak.
2. Mempertahankan status verifikasi kontak sebagai fakta terpisah dari status aktif akun.
3. Menyesuaikan trigger, fungsi `create_client_user`, invariant kontak, cleanup, dan pengujian PostgreSQL yang saat ini mengasumsikan registrasi pending.
4. Mempertahankan keunikan/reservasi identifier dan mencegah race saat dua registrasi memakai identifier yang sama.
5. Memastikan akun Google baru tetap dibuat dalam transaksi dengan role yang dipilih dan identitas provider subject yang unik.
6. Memastikan akun terminal tidak menyisakan kredensial atau identitas provider yang tidak semestinya.

Jangan mengubah migration lama yang mungkin sudah diterapkan pada lingkungan bersama. Gunakan migration baru yang eksplisit dan strategi transisi data bila ternyata ada database yang telah memakai migration lama. Database produksi saat ini belum diprovisikan/menerapkan migration menurut status repository terakhir.

## 7. Tahapan berikutnya

1. Selaraskan spesifikasi produk, wireframe, rancangan autentikasi, dan tinjauan skema dengan keputusan ini.
2. Rancang kontrak API dan threat model untuk password login serta Google OAuth.
3. Rancang migration lanjutan dan tes PostgreSQL sebelum implementasi runtime.
4. Implementasikan autentikasi dan uji negatif/positif pada staging serta lingkungan demo juri.
5. Tambahkan verifikasi kontak/OTP pada tahap berikutnya tanpa mengubah OTP menjadi metode login utama.

## 8. Di luar keputusan ini

Keputusan ini tidak memilih penyedia email, SMS, WhatsApp, Google client configuration, secret manager, atau KMS. Keputusan ini juga tidak mengaktifkan autentikasi, menjalankan migration, mengubah database produksi, atau melakukan deployment.
