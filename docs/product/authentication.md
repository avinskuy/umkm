# Aturan Produk Autentikasi

> Keputusan terbaru (22 November 2026): registrasi email + password wajib OTP email; registrasi telepon/WhatsApp + password wajib OTP WhatsApp. Login rutin menggunakan email/telepon + password tanpa OTP. Google OAuth tidak termasuk cakupan. Lihat [Keputusan Autentikasi Identity Core](../architecture/identity-core-authentication-stage-one-decision.md).

## Model akun

- Akun publik hanya memiliki satu role utama: `business_owner` atau `investor`.
- Pendaftaran publik wajib meminta pengguna memilih role secara eksplisit.
- Role tidak dapat diubah melalui pengaturan profil, support, tindakan Admin, migrasi akun, atau proses internal lainnya.
- Pengguna yang ingin menggunakan role klien lainnya harus membuat akun terpisah dengan identitas login yang memenuhi syarat dan belum terhubung ke akun lain.
- Akun Admin disediakan melalui proses internal dan tidak pernah ditawarkan dalam pendaftaran publik.
- Identitas utama yang dipakai untuk pendaftaran tunduk pada keunikan dan reservasi kontak akun; kontak pemulihan opsional dapat digunakan bersama oleh beberapa akun sesuai kebijakan Identity Core.

Role dan keanggotaan merupakan dua hal terpisah. Paket Free maupun Premium tidak mengubah izin berdasarkan role dan tidak memberikan akses ke sumber daya tanpa pemeriksaan kepemilikan, persetujuan, serta otorisasi yang diwajibkan. Lihat [Kebijakan Membership dan Entitlements](membership-and-entitlements.md).

## Halaman dan rute

- Business Owner: `/masuk/pemilik`
- Investor: `/masuk/investor`
- Admin: `/admin/login`
- Pendaftaran publik: `/daftar`
- Pendaftaran Owner: `/daftar/pemilik`
- Pendaftaran Investor: `/daftar/investor`

Halaman login klien yang terpisah merupakan kebutuhan pengalaman pengguna. Keduanya tetap menggunakan sistem identitas backend yang sama dan backend wajib memeriksa role yang sesuai. Route frontend bukan batas keamanan.

## Metode autentikasi dan verifikasi

- Registrasi publik hanya menyediakan email + password dengan OTP email, atau nomor telepon/WhatsApp + password dengan OTP WhatsApp.
- Pengguna memilih tepat satu kontak awal dan satu role klien (Business Owner atau Investor). Role utama bersifat tetap.
- Akun berstatus `pending_verification` sampai OTP pada kontak awal berhasil diverifikasi; akun baru aktif setelah verifikasi berhasil.
- Login rutin menggunakan email atau nomor telepon + password tanpa OTP setiap kali login.
- Google OAuth dan jalur registrasi tanpa OTP tidak termasuk cakupan yang disetujui.
- Kontak awal yang belum diverifikasi tidak boleh digunakan sebagai bukti pemulihan atau untuk tindakan sensitif.
- Password harus disimpan menggunakan Argon2id. Usulan panjang minimum, maksimum, blocklist, dan pengalaman pengguna masih menunggu persetujuan pada [Usulan Kebijakan Password Pengguna](../security/password-authentication-policy-proposal.md).
- OTP tidak dapat digunakan sebagai kredensial login atau pengganti password; OTP hanya untuk verifikasi kontak dan alur keamanan yang telah ditetapkan.
- MFA Admin wajib tersedia sebelum sistem digunakan di produksi. Metode dan penyedianya masih merupakan keputusan implementasi.

Baseline masa berlaku, batas percobaan, lock, cooldown, dan rate limit OTP mengikuti [Keputusan Pendaftaran Identity Core](../architecture/identity-core-registration-decisions.md) serta implementasi runtime yang telah diuji. Nilai konfigurasi produksi tetap harus divalidasi sebelum endpoint publik diaktifkan.

## Sesi dan pemulihan akun

- Sesi mendukung batas 7 hari tanpa aktivitas dan 30 hari absolut.
- Pengguna dapat melihat sesi aktif, mencabut sesi tertentu, dan logout dari semua perangkat.
- Fitur “Ingat perangkat ini” tidak melewati batas sesi atau pencabutan server.
- Pemulihan akun hanya boleh menggunakan kanal yang sudah dibuktikan kepemilikannya. Kontak awal yang belum diverifikasi tidak boleh menjadi tujuan reset; fitur reset tidak boleh dirilis sebelum pengiriman token aman tersedia.
- Kepemilikan kontak bersama tidak membuktikan kepemilikan seluruh akun yang terhubung. Pemilihan akun dan pemeriksaan risiko tetap diperlukan.
- Jika kontak utama dan pemulihan tidak tersedia, gunakan pemulihan alternatif berbasis risiko dan pemeriksaan manual oleh petugas khusus jika diperlukan.
- Pemulihan berisiko tinggi dapat ditunda atau ditolak; role utama tidak pernah berubah.

Rincian kebijakan terdapat pada [Identity Core](../architecture/identity-core-registration-decisions.md) dan [Tinjauan Kontak Pemulihan Bersama](../architecture/identity-core-shared-recovery-contacts.md).

## Otorisasi

Backend wajib memeriksa:

1. Sesi masih valid, belum kedaluwarsa, dan belum dicabut.
2. Akun berstatus aktif serta tidak dibatasi untuk tindakan yang diminta.
3. Role akun sesuai dengan halaman atau layanan klien yang diminta.
4. Akun memiliki izin untuk mengakses sumber daya yang diminta.
5. Kepemilikan, status koneksi, persetujuan berbagi, klasifikasi data, dan konteks tindakan sesuai kebijakan.

Gunakan model RBAC + ABAC, least privilege, dan deny-by-default. Route guard frontend hanya membantu pengalaman pengguna dan bukan batas keamanan. Lihat [Access Control & Authorization Policy](../security/access-control-authorization.md).

## Notifikasi keamanan dan transparansi

Notifikasi wajib untuk login dari perangkat baru, perubahan password/kontak utama/kontak pemulihan, permintaan dan hasil pemulihan akun, serta pembatasan atau suspensi Admin. Notifikasi keamanan tidak boleh dinonaktifkan melalui preferensi marketing. Lihat [Security Notifications & Alerts](../security/security-notifications-alerts.md).

Pengguna dapat melihat tindakan moderasi pada halaman Riwayat Aktivitas Akun dan mengajukan satu banding untuk setiap tindakan yang memenuhi syarat. Informasi internal Admin, identitas pelapor, dan catatan investigasi tidak ditampilkan.
