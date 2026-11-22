# Keputusan Autentikasi Identity Core — Password dan OTP

> Cakupan: registrasi, login, verifikasi kontak, pemulihan akun, dan provider OTP.
> Status implementasi: login password Owner/Investor, validasi sesi pada endpoint identitas, refresh/rotasi token, logout, dan logout semua perangkat tersedia; registrasi HTTP publik, penerapan middleware ke endpoint fitur, dan provider OTP nyata belum diaktifkan.

## 1. Keputusan autentikasi

Registrasi publik hanya menyediakan dua jalur:

1. **Email + password** dengan verifikasi OTP melalui email.
2. **Nomor telepon/WhatsApp + password** dengan verifikasi OTP melalui WhatsApp.

Akun dibuat dengan status pending_verification dan hanya menjadi active setelah OTP pada kontak awal berhasil diverifikasi. Password disimpan menggunakan Argon2id; password plaintext tidak pernah disimpan atau dicatat.

Login rutin menggunakan email atau nomor telepon + password. OTP tidak diminta pada setiap login. Login sosial/OAuth tidak termasuk cakupan autentikasi saat ini.

## 2. Pemilihan role

- Pendaftaran publik hanya menerima business_owner atau investor.
- Role wajib dipilih sebelum akun dibuat dan menjadi role utama yang immutable.
- Satu akun hanya memiliki satu role utama. Pengguna yang membutuhkan role klien lain harus membuat akun terpisah dengan kontak utama yang memenuhi aturan reservasi.
- Tidak ada pendaftaran Admin publik. Akun Admin hanya dibuat melalui provisioning internal dan mengikuti kebijakan MFA Admin.

## 3. Alur registrasi email/WhatsApp + password

1. Pengguna memilih role Owner atau Investor.
2. Pengguna memilih tepat satu kontak awal: email atau nomor telepon/WhatsApp.
3. Pengguna mengisi nama tampilan publik, kontak, password, konfirmasi password, serta persetujuan Ketentuan dan Kebijakan Privasi. Nama yang dimasukkan menjadi nama tampilan publik, bukan klaim identitas legal. Persetujuan marketing ditawarkan terpisah dan opsional.
4. Backend menormalisasi kontak, memeriksa reservasi identifier, menerapkan rate limit, dan membuat akun pending_verification.
5. Backend membuat verification session dan challenge OTP. Tujuan pengiriman dilindungi enkripsi; fingerprint tujuan menggunakan HMAC.
6. Provider mengirim kode melalui Resend (email) atau FlowKirim (WhatsApp) setelah reservasi pengiriman dicatat secara atomik.
7. Pengguna memasukkan OTP. Backend memverifikasi tujuan, expiry, status challenge, batas percobaan, dan status akun dalam transaksi.
8. Setelah OTP valid, akun diaktifkan dan timestamp verifikasi kontak dicatat secara atomik.
9. Pengguna melanjutkan onboarding sesuai role. Nama tampilan, penerimaan kebijakan wajib beserta versinya, dan consent marketing bila dipilih sudah tercatat atomik bersama pembuatan akun.

Kegagalan atau timeout provider tidak boleh mengaktifkan akun. Respons publik untuk konflik identifier harus generik dan tidak menjadi oracle enumerasi akun.

## 4. Login

- Pengguna masuk menggunakan email atau nomor telepon + password.
- Login rutin tidak meminta OTP.
- Terapkan Argon2id, rate limit PostgreSQL berbasis HMAC (10 percobaan per identifier dan 30 per IP dalam 15 menit), perlindungan brute-force/credential stuffing, respons generik, sesi server-side 30 hari dengan idle limit 7 hari, cookie `__Host-` HttpOnly/Secure/SameSite=Lax, TLS, dan audit event. Middleware validasi sesi, pembaruan idle timeout, dan rotasi token tersedia pada endpoint identitas. Endpoint state-changing menolak Fetch Metadata cross-site; kebijakan CSRF/origin lengkap untuk API fitur tetap harus diterapkan sebelum fitur akun lain dibuka.
- Ketidaksesuaian role ditangani backend; role tidak dapat diganti dari halaman login.

## 5. Provider OTP

Provider awal untuk demonstrasi kompetisi:
- **Resend** untuk email OTP.
- **FlowKirim** untuk WhatsApp OTP.

Provider merupakan adapter yang dapat diganti. Business logic OTP tetap berada di backend Rust: pembuatan kode, keyed hash, expiry, batas percobaan, cooldown, rate limit lintas sesi, lock, dan verifikasi.

Ketentuan:
- API key/token hanya di secret/environment server, tidak di frontend, repository, log, atau database.
- Gunakan nomor demonstrasi dan penerima yang menyetujui pengiriman.
- Jangan kirim password, token sesi, atau informasi sensitif lain melalui pesan.
- Catat ID pesan, status dispatch, dan kategori kegagalan tanpa menyimpan OTP plaintext.
- Timeout provider dengan hasil tidak pasti dicatat sebagai unknown; jangan mengembalikan kuota secara otomatis.
- CI memakai mock provider dan tidak mengirim pesan nyata.
- Kuota, watermark, masa token, harga, dan ketentuan provider harus diverifikasi ulang sebelum demo.

## 6. Pemulihan akun dan perubahan kontak

- OTP tetap digunakan untuk membuktikan penguasaan kontak pada recovery dan perubahan kontak sesuai kebijakan yang disetujui.
- Recovery email/telepon yang dapat dipakai bersama beberapa akun tetap memerlukan verifikasi terpisah dan pemeriksaan kepemilikan akun sasaran; possession saja tidak membuktikan kepemilikan akun.
- Reset password hanya dikirim ke kanal pemulihan yang telah diverifikasi.
- Jangan membuat bypass manual yang tidak diaudit.
- Membuat verification session baru tidak boleh mengatur ulang anggaran percobaan atau pengiriman lintas sesi.

## 7. Batas implementasi database

Migration 0001–0005 tetap immutable. Migration 0001 awalnya memuat tabel identitas provider dan parameter google_subject; keduanya tidak lagi menjadi bagian dari desain aktif. Migration 0006 menghapus tabel tersebut, mengganti fungsi pembuatan akun menjadi fungsi lima parameter yang mewajibkan password hash, dan memperketat invariant akun klien.

Migration 0006 harus gagal secara aman bila masih ada baris identitas provider yang perlu ditinjau. Migration dan bootstrap tetap merupakan operasi terpisah dari HTTP startup dan deployment produksi.

## 8. Hal yang belum diaktifkan

Router registrasi/resend/verifikasi tersedia dan diuji, tetapi belum dipasang pada aplikasi HTTP default. Login password Owner/Investor dan logout sudah dipasang pada aplikasi HTTP utama; aktivasi production tetap memerlukan konfigurasi `AUTH_RATE_LIMIT_HMAC_KEY` dan penerapan migration secara terpisah. Penerapan middleware sesi pada endpoint fitur, kebijakan CSRF/origin lengkap, provider OTP nyata, credential provider, key OTP produksi, dan deployment belum aktif. Setiap aktivasi fitur berikutnya harus melalui audit, pengujian PostgreSQL, CI, dan tinjauan keamanan.
