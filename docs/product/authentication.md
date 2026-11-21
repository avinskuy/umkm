# Aturan Produk Autentikasi

## Model akun

- Akun publik hanya memiliki satu role utama: `business_owner` atau `investor`.
- Pendaftaran publik wajib meminta pengguna memilih role secara eksplisit.
- Pengguna tidak dapat mengganti role akunnya sendiri.
- Akun Admin disediakan melalui proses internal dan tidak pernah ditawarkan dalam pendaftaran publik.
- Identitas email yang sama bersifat unik secara global; proses penghubungan OAuth tidak boleh diam-diam membuat akun atau role kedua.

Role klien seseorang bersifat tetap untuk akun tersebut. Jika ingin menggunakan role klien yang lain, pengguna harus mendaftar akun terpisah dengan identitas yang memenuhi syarat dan belum terhubung ke akun lain. Tidak ada alur profil, support, Admin, atau OAuth yang boleh mengubah role akun yang sudah ada.

Role dan keanggotaan merupakan dua hal terpisah. Paket Free maupun Premium tidak mengubah izin berdasarkan role dan tidak memberikan akses ke sumber daya tanpa pemeriksaan kepemilikan, persetujuan, serta otorisasi yang diwajibkan. Lihat [Kebijakan Membership dan Entitlements](membership-and-entitlements.md).

## Halaman login

- Business Owner: `/login/owner`
- Investor: `/login/investor`
- Admin: `/admin/login`
- Pendaftaran publik: `/register`

Halaman login klien yang terpisah merupakan kebutuhan pengalaman pengguna. Keduanya tetap harus menggunakan sistem identitas backend yang sama dan backend wajib memeriksa role yang sesuai.

## Persyaratan autentikasi

- Verifikasi kontak yang dipilih saat pendaftaran sebelum fitur akun yang dilindungi dapat digunakan. Kontak awal dapat berupa email atau nomor HP yang diverifikasi melalui WhatsApp.
- Password harus disimpan menggunakan Argon2id.
- Sesi harus aman dan dapat dicabut.
- Terapkan rate limit untuk login, pemulihan password, dan permintaan kode verifikasi.
- Pemulihan password menggunakan token sekali pakai dengan masa berlaku singkat.
- Google OAuth harus terikat pada identitas yang terverifikasi dan mempertahankan role akun yang sudah ada.
- MFA Admin wajib tersedia sebelum sistem digunakan di produksi.

## Otorisasi

Backend wajib memeriksa:

1. Sesi masih valid.
2. Akun berstatus aktif.
3. Role akun sesuai dengan halaman atau layanan klien yang diminta.
4. Akun memiliki izin untuk mengakses sumber daya yang diminta.

Route guard frontend hanya membantu pengalaman pengguna dan bukan batas keamanan.

## Privasi

Investor hanya boleh melihat informasi bisnis yang secara eksplisit dibagikan oleh pemiliknya. Data transaksi pada tingkat pelanggan tidak boleh ditampilkan kepada investor.
