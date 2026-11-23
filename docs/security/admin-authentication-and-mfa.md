# Autentikasi Admin dan MFA

## 1. Model akun

USahapintar menggunakan tepat satu akun Admin platform. Akun ini dibuat melalui provisioning internal dan tidak dapat dibuat melalui registrasi publik.

- Tidak ada Super Admin, Verification Admin, sub-role Admin, atau pengelolaan role Admin bertingkat.
- Role utama akun Admin tetap `admin`; akun klien tetap `business_owner` atau `investor`.
- Database menolak akun Admin kedua melalui unique partial index.
- Akun Admin tidak boleh mengubah role utama, menghapus akun secara permanen, atau melewati proses keamanan akun.
- Izin operasional Admin ditetapkan secara eksplisit pada backend dan tidak berasal dari klaim frontend.

## 2. Login dan MFA

Alur login Admin dibuat ringkas untuk penggunaan dan pengujian:

1. Admin memasukkan email dan password.
2. Backend memverifikasi kredensial.
3. Admin memasukkan kode TOTP enam digit dari aplikasi authenticator.
4. Backend memverifikasi kode, time-step, batas percobaan, dan status akun.
5. Sesi Admin baru diterbitkan hanya setelah kedua faktor berhasil.

Ketentuan:
- TOTP mengikuti RFC 6238 dengan secret acak yang dibuat oleh CSPRNG.
- Secret TOTP disimpan hanya dalam bentuk ciphertext AES-256-GCM, nonce 96-bit, dan versi key.
- Secret tidak boleh dikirim kembali setelah enrollment selesai, ditampilkan pada log, atau disimpan di frontend.
- QR enrollment hanya ditampilkan saat proses enrollment yang terautentikasi dan belum aktif; aktivasi memerlukan verifikasi kode pertama.
- Kode TOTP harus dibandingkan secara constant-time dan diterima hanya dalam jendela waktu terbatas.
- Time-step yang telah dipakai tidak boleh diterima kembali. Pembaruan replay-counter harus atomik di PostgreSQL.
- Terapkan rate limit untuk password dan TOTP serta pesan kegagalan generik.
- Sesi Admin harus dipisahkan dari sesi Business Owner/Investor dan menggunakan cookie HttpOnly, Secure, SameSite serta proteksi CSRF yang sesuai.

Tidak diperlukan SMS, WhatsApp, email OTP, atau integrasi vendor MFA. Aplikasi authenticator standar dapat digunakan.

## 3. Provisioning dan pemulihan

- Provisioning akun Admin dilakukan operator internal melalui prosedur terkontrol; tidak ada endpoint publik untuk membuat Admin.
- Password awal diberikan melalui kanal rahasia di luar repository dan chat, lalu wajib diganti saat aktivasi.
- Enrollment TOTP harus diselesaikan sebelum akun dapat melakukan tindakan operasional.
- Jika authenticator hilang atau akun terkunci, operator internal melakukan verifikasi dan reset enrollment melalui prosedur terdokumentasi dengan audit trail. Tidak boleh ada bypass MFA melalui endpoint publik.
- Secret baru harus dienkripsi dengan key provider yang tersedia. Jika key tidak tersedia, enrollment dan login gagal tertutup.

## 4. Otorisasi dan audit

Satu akun Admin memiliki izin operasional platform yang ditentukan backend, termasuk meninjau dan memutuskan kasus pemulihan akun. Tidak ada pemetaan izin berbasis sub-role.

Setiap login berhasil/gagal, enrollment, reset MFA, penolakan otorisasi, dan keputusan pemulihan harus menghasilkan audit event yang tidak memuat password, kode TOTP, secret, token sesi, atau token reset.

## 5. Status implementasi dan batas aktivasi

Migration Identity Core 0018 menyediakan:
- Batas database untuk satu akun Admin.
- Tabel kredensial TOTP dengan secret terenkripsi, nonce, versi key, status enrollment, dan replay-counter.
- Trigger untuk memastikan kredensial terikat pada akun Admin aktif, melindungi identitas kredensial, mencegah reset langsung setelah enrollment, menjaga replay-counter agar tidak mundur, dan melarang penghapusan/truncate.

Migration ini adalah fondasi skema, bukan implementasi login Admin yang siap digunakan. Login dua faktor, provisioning internal, enrollment, verifikasi TOTP, operasi atomik replay protection, sesi Admin terpisah, endpoint Admin, dan UI Admin harus selesai serta lulus pengujian sebelum akun Admin diaktifkan. Migration production dan deployment tetap merupakan proses terpisah.

## 6. Pengujian wajib

- Database menolak akun Admin kedua.
- Kredensial TOTP hanya dapat dikaitkan dengan akun Admin aktif.
- Secret tidak pernah disimpan plaintext.
- Enrollment hanya aktif setelah kode pertama terverifikasi.
- Kode salah, kedaluwarsa, atau time-step yang sudah dipakai ditolak.
- Dua permintaan bersamaan untuk time-step yang sama tidak dapat sama-sama berhasil.
- Password benar tanpa TOTP tidak menerbitkan sesi.
- Akun Owner/Investor tidak dapat menggunakan endpoint Admin.
- Admin tidak dapat memperoleh sesi melalui endpoint login klien.
- Rate limit, logout, pencabutan sesi, CSRF, audit event, dan kegagalan key provider diuji.
- Principal aplikasi tidak memiliki akses tabel langsung ke kredensial Admin.
