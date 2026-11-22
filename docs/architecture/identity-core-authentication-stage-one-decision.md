# Keputusan Autentikasi Identity Core — Registrasi OTP dan Google OAuth

> Status: disetujui pada 22 November 2026 (amendemen setelah keputusan OTP ditunda)
> Cakupan: metode registrasi/login, verifikasi kontak, pemilihan role, dan provider pengiriman OTP
> Implementasi runtime: belum tersedia; dokumen ini menetapkan keputusan produk dan kontrak desain, bukan klaim fitur sudah aktif.

## 1. Keputusan autentikasi

USahapintar menyediakan dua jalur pendaftaran klien:

1. **Email + password dengan OTP email wajib**.
2. **Nomor telepon/WhatsApp + password dengan OTP WhatsApp wajib**.

Pengguna harus menyelesaikan verifikasi OTP pada kontak awal sebelum akun diaktifkan. Password disimpan menggunakan Argon2id; password plaintext tidak pernah disimpan atau dicatat.

**Google OAuth menjadi jalur alternatif** untuk pendaftaran dan login:
- Pendaftaran Google tidak memerlukan OTP tambahan.
- Token/identitas Google harus divalidasi sepenuhnya; klaim email hanya dianggap terverifikasi bila token tervalidasi menyatakan `email_verified=true`.
- Google-only account tidak wajib memiliki password lokal.
- Login Google untuk identitas yang sudah terhubung tidak meminta OTP rutin.

OTP adalah bukti penguasaan kanal kontak saat registrasi atau proses verifikasi yang memang memerlukannya; OTP bukan metode login rutin dan tidak menggantikan password atau Google OAuth.

## 2. Pemilihan role tetap wajib

- Pendaftaran publik hanya menerima `business_owner` atau `investor`.
- Role wajib dipilih sebelum pembuatan akun, termasuk sebelum memulai Google OAuth.
- Untuk Google OAuth, pilihan role diikat ke state OAuth yang diterbitkan dan divalidasi server; callback tidak boleh mengganti role.
- Setiap akun hanya memiliki satu role utama yang immutable.
- Login menggunakan role yang sudah tersimpan. Pemilihan role bukan sarana mengganti role akun.
- Tidak ada pendaftaran Admin publik, termasuk melalui Google OAuth.

## 3. Alur registrasi email/WhatsApp + password

1. Pengguna memilih role Owner atau Investor.
2. Pengguna memilih tepat satu kanal kontak awal: email atau WhatsApp/nomor telepon.
3. Pengguna mengisi nama, kontak, password, konfirmasi password, serta persetujuan Ketentuan dan Kebijakan Privasi.
4. Backend menormalisasi kontak, memeriksa reservasi identifier, menerapkan rate limit, dan membuat akun berstatus `pending_verification`.
5. Backend membuat sesi verifikasi dan challenge OTP dengan tujuan yang dilindungi enkripsi serta fingerprint HMAC.
6. Provider mengirim kode melalui Resend (email) atau FlowKirim (WhatsApp) setelah kelayakan pengiriman dicatat secara atomik.
7. Pengguna memasukkan OTP. Backend memverifikasi kode, expiry, status challenge, limit percobaan, serta status akun dalam transaksi.
8. Setelah OTP valid, akun diaktifkan dan timestamp verifikasi kontak dicatat secara atomik.
9. Pengguna melanjutkan onboarding Owner atau Investor.

Kegagalan atau timeout provider tidak boleh mengaktifkan akun. Respons publik untuk konflik identifier harus generik dan tidak menjadi oracle enumerasi akun.

## 4. Alur Google OAuth

- Pengguna memilih Owner atau Investor sebelum memulai OAuth.
- State OAuth singkat, sekali pakai, terikat pada role, dan terlindung dari CSRF/replay.
- Identitas Google dikenali melalui stable provider subject, bukan alamat email.
- Validasi issuer, audience/client ID, signature, expiry, state, nonce, redirect URI, dan subject wajib dilakukan.
- Klaim email hanya dianggap terverifikasi bila token tervalidasi menyatakan `email_verified=true`.
- Pendaftaran Google tidak meminta OTP tambahan. Jika email provider tidak terverifikasi, jangan perlakukan email sebagai kontak terverifikasi atau izinkan email itu menjadi kanal pemulihan.
- Identitas Google yang sudah terhubung hanya dapat masuk ke akun yang sama. Jangan menghubungkan atau menggabungkan akun otomatis hanya karena email cocok.
- Pengaitan identitas ke akun lokal yang sudah ada memerlukan autentikasi ulang dan bukti kendali atas kedua identitas.
- Admin tidak dapat didaftarkan melalui OAuth publik.

## 5. Login

- Akun password: email atau nomor telepon + password.
- Akun Google-only: Google OAuth.
- Tidak ada OTP rutin pada login biasa.
- MFA Admin tetap wajib sesuai kebijakan akses Admin dan tidak diturunkan oleh keputusan ini.
- Terapkan rate limit login, perlindungan brute-force/credential stuffing, respons generik, sesi server-side, cookie aman, CSRF/origin checks, TLS, dan audit event.

## 6. Provider OTP dan batas penggunaannya

Provider awal untuk demonstrasi kompetisi:
- **Resend** untuk email OTP.
- **FlowKirim** untuk WhatsApp OTP.

Provider merupakan adapter yang dapat diganti; business logic OTP tetap berada di backend Rust. Resend hanya mengirim email: backend membuat kode, menyimpan keyed hash, mengelola expiry/attempt/rate limit, dan memverifikasi OTP. FlowKirim diperlakukan sebagai provider pihak ketiga eksperimental untuk kebutuhan demonstrasi, bukan jaminan produksi atau jalur resmi Meta.

Ketentuan:
- API key/token hanya di secret/environment server, tidak di frontend, repository, log, atau database.
- Gunakan nomor demonstrasi dan penerima yang menyetujui pengiriman.
- Jangan kirim password, token sesi, atau informasi sensitif lain melalui pesan.
- Catat ID pesan, status dispatch, dan kategori kegagalan tanpa menyimpan OTP plaintext.
- Provider timeout dengan hasil tidak pasti dicatat sebagai `unknown`; jangan mengembalikan kuota secara otomatis.
- Siapkan provider mock untuk CI; tes tidak mengirim pesan nyata.
- Free tier, kuota, watermark, masa token, serta ketentuan provider harus diverifikasi ulang sebelum demo karena dapat berubah.

## 7. Batas pemulihan akun dan perubahan kontak

- OTP tetap menjadi mekanisme pembuktian kontak untuk recovery dan perubahan kontak sesuai kebijakan yang disetujui.
- Recovery contact email/telepon yang dapat dipakai bersama beberapa akun tetap memerlukan verifikasi terpisah dan pemeriksaan kepemilikan akun sasaran; possession saja tidak membuktikan kepemilikan akun.
- Reset password hanya dikirim ke kanal pemulihan yang telah diverifikasi.
- Jangan membuat bypass manual yang tidak diaudit.
- Rate limit OTP berlaku lintas sesi dan purpose; membuat sesi baru tidak boleh mengatur ulang anggaran percobaan atau pengiriman.

## 8. Konsekuensi terhadap skema

Migration 0001–0005 tidak boleh diedit setelah merge. Migration 0001 memodelkan akun `pending_verification` dan aktivasi setelah verifikasi kontak. Migration 0002 menyediakan sesi, challenge, event pengiriman, dan event percobaan untuk kanal `email` dan `whatsapp`.

**Temuan audit konkret:** fungsi `public.create_client_user` pada migration 0001 selalu membuat akun `pending_verification` dengan `pending_expires_at` 30 menit, termasuk ketika `p_google_subject` diberikan. Fungsi tersebut tidak mengisi `email_verified_at` atau `initial_contact_verified_at`. Karena itu, Google OAuth tanpa OTP belum dapat langsung menghasilkan akun aktif melalui fungsi yang ada. Akun Google-only juga tidak boleh sekadar dilewatkan ke aktivasi OTP karena tidak ada challenge registrasi. Diperlukan desain dan migration lanjutan (tanpa mengubah migration lama) untuk jalur pembuatan akun Google yang memvalidasi provider, mengikat stable subject, menetapkan status/verifikasi email yang benar, dan tetap menjaga invariant akun serta role. Sampai migration dan tes itu tersedia, Google OAuth tanpa OTP adalah keputusan produk, belum didukung oleh skema runtime saat ini.

Sebelum runtime dibuat, audit harus memastikan:
1. Fungsi `create_client_user` dan trigger mendukung jalur password pending serta jalur Google OAuth tanpa OTP secara benar.
2. Akun Google dengan `email_verified=true` memiliki representasi kontak/lifecycle yang konsisten dan tidak memerlukan challenge registrasi.
3. Constraint akun, provider identity, serta trigger aktivasi tidak memungkinkan bypass OTP pada akun password.
4. Fungsi verifikasi mengaktifkan akun hanya setelah challenge registrasi yang tepat berhasil.
5. Batas waktu pending 30 menit, expiry challenge 10 menit, cooldown resend 60 detik, maksimal lima pengiriman per kontak per 60 menit, maksimal lima kegagalan per kontak per 60 menit, dan lock satu jam ditegakkan konsisten.
6. Tes PostgreSQL mencakup email OTP, WhatsApp OTP, Google OAuth tanpa OTP, role wajib, race identifier, resend, expiry, lock, dan kegagalan provider.

Jika penyesuaian skema diperlukan, buat migration baru dan tes transisi; jangan mengubah migration lama. Migration/bootstrap produksi belum diterapkan dan tidak boleh dijalankan sebagai bagian dari pekerjaan dokumentasi.

## 9. Di luar keputusan ini

Keputusan ini tidak mengaktifkan endpoint/runtime, tidak mengirim pesan nyata, tidak memprovisikan credential provider, tidak menjalankan migration pada database, dan tidak melakukan deployment.