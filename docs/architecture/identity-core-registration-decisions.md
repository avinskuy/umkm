# Identity Core — Keputusan Pendaftaran

> **Status:** Keputusan aktif per 22 November 2026. Dokumen ini merangkum keputusan pendaftaran yang berlaku dan menggantikan rumusan lama yang bertentangan. Detail skema, runtime, recovery, dan migration dirujuk ke kontrak masing-masing.

## 1. Metode registrasi dan login

Registrasi publik hanya menyediakan dua jalur:

1. **Email + password** — alamat email diverifikasi dengan OTP email.
2. **Nomor telepon/WhatsApp + password** — nomor telepon diverifikasi dengan OTP melalui WhatsApp.

Tidak ada jalur registrasi tanpa OTP dan tidak ada Google OAuth. Login rutin menggunakan email atau nomor telepon + password tanpa OTP setiap kali login.

Akun dibuat dengan status `pending_verification`. Akun hanya menjadi `active` setelah OTP untuk kontak awal berhasil diverifikasi. Akun pending tidak boleh login atau memakai fitur yang mensyaratkan akun aktif.

Password:
- Disimpan menggunakan Argon2id.
- Password plaintext tidak boleh disimpan, dicatat, atau dikirim ke provider.
- Password hash hanya diproses di backend dan diteruskan ke fungsi database terkontrol.

OTP:
- Dibuat menggunakan sumber acak kriptografis dan disimpan sebagai keyed hash.
- Tidak boleh dicatat atau dikirim ke log dalam bentuk plaintext.
- Terikat pada tujuan, kontak, sesi, dan challenge yang benar.
- Hanya dapat digunakan satu kali melalui transisi atomik.
- Bukan kredensial login dan bukan pengganti password.

## 2. Role dan identitas akun

- Pendaftaran publik hanya tersedia untuk `business_owner` dan `investor`.
- Pengguna memilih tepat satu role utama sebelum akun dibuat.
- Role utama bersifat tetap dan tidak dapat diubah melalui pengaturan, support, tindakan Admin rutin, migrasi akun, atau alur autentikasi.
- Pengguna yang membutuhkan role klien lain harus membuat akun terpisah dengan kontak utama yang memenuhi aturan reservasi.
- Admin dibuat melalui provisioning internal dan tidak tersedia pada pendaftaran publik.
- Kontak utama yang dipilih saat registrasi dinormalisasi dan tunduk pada aturan keunikan/reservasi.
- Kontak pemulihan opsional boleh digunakan bersama oleh beberapa akun; aturan ini tidak mengubah keunikan kontak utama.

## 3. Alur registrasi

1. Pengguna memilih Business Owner atau Investor.
2. Pengguna memilih satu kontak awal: email atau nomor telepon/WhatsApp.
3. Pengguna memasukkan nama tampilan publik, kontak, password, konfirmasi password, dan penerimaan Terms of Service serta Privacy Policy. Nama registrasi adalah nama tampilan publik, bukan klaim identitas legal.
4. Consent marketing ditawarkan secara terpisah, opsional, dan tidak dicentang sebelumnya.
5. Backend menormalisasi kontak, memvalidasi input, menerapkan kontrol anti-penyalahgunaan, dan membuat akun pending beserta profil dan catatan penerimaan kebijakan secara atomik.
6. Backend membuat verification session dan challenge OTP. Tujuan pengiriman dilindungi enkripsi; fingerprint tujuan menggunakan HMAC.
7. Provider mengirim OTP melalui kanal yang dipilih setelah reservasi pengiriman dicatat.
8. Pengguna mengirim kode. Backend memeriksa tujuan, expiry, status challenge, batas percobaan, lock, dan status akun.
9. OTP valid mengonsumsi challenge, memverifikasi sesi, dan mengaktifkan akun secara atomik.
10. Setelah aktif, pengguna melanjutkan onboarding sesuai role.

Penerimaan Terms dan Privacy disimpan terpisah per versi. Consent marketing hanya dicatat sebagai `granted` bila pengguna secara eksplisit opt-in. Versi kebijakan ditentukan backend dari konfigurasi server-side, bukan dipercaya dari input browser. Versi final belum boleh diaktifkan sebelum dokumen publik disetujui; lihat [Registry Versi Kebijakan Publik](../legal/policy-version-registry.md).

## 4. Baseline OTP dan masa pending

Baseline runtime yang disetujui dan diuji:

- Kode OTP numerik enam digit.
- Masa berlaku challenge: 10 menit.
- Jeda resend: 60 detik.
- Maksimal lima pengiriman per kontak dalam 60 menit, lintas sesi.
- Maksimal lima percobaan verifikasi gagal dalam konteks sesi/kontak yang ditetapkan.
- Setelah lima kegagalan, sesi dikunci selama satu jam.
- Penguncian tidak direset oleh pengiriman challenge baru, refresh halaman, atau pembuatan sesi baru.
- Pengguna boleh meminta OTP baru saat sesi terkunci jika cooldown dan rate limit pengiriman mengizinkan; semua percobaan verifikasi tetap ditolak sampai lock berakhir.
- Setelah lock berakhir, penghitung kegagalan sesi direset secara atomik. Batas pengiriman lintas sesi tetap berlaku.
- Masa pending akun: 30 menit. Akun pending yang kedaluwarsa ditangani melalui cleanup terkontrol; jangan mengaktifkan akun setelah batas tersebut.

Semua batas harus ditegakkan di server dan aman terhadap request bersamaan. Hitung mundur frontend hanya informatif. Detail status sesi/challenge, event, rate limit, dan SQL function terdapat pada [Runtime Registrasi & Verifikasi](identity-core-registration-verification-runtime.md), [Kontrak Migration 0002](identity-core-migration-0002-verification-contract.md), [Kontrak Migration 0007](identity-core-migration-0007-verification-operations-contract.md) jika tersedia pada branch yang memuatnya, dan [Tinjauan Kontak Pemulihan Bersama](identity-core-shared-recovery-contacts.md).

## 5. Provider OTP

Kandidat awal untuk demonstrasi:
- Resend untuk email OTP.
- FlowKirim untuk WhatsApp OTP.

Keduanya merupakan kandidat integrasi, bukan provider produksi yang telah dikonfigurasi atau diuji dengan pengiriman nyata. Adapter provider harus dapat diganti. Business logic OTP tetap berada di backend Rust.

Ketentuan:
- Kredensial hanya berada pada secret/environment server.
- CI memakai mock provider; tidak mengirim pesan nyata.
- Gunakan penerima demo yang menyetujui pengiriman.
- Simpan ID pesan, status dispatch, dan kategori kegagalan tanpa OTP plaintext.
- Timeout dengan hasil tidak pasti dicatat sebagai `unknown`; jangan mengembalikan kuota secara otomatis.
- Harga, kuota, watermark, masa token, syarat pengirim, dan kebijakan provider diverifikasi ulang sebelum demo.

## 6. Kontak pemulihan dan pemulihan akun

- Kontak pemulihan bersifat opsional dan harus diverifikasi secara terpisah.
- Email atau nomor telepon pemulihan yang sama boleh terhubung ke beberapa akun. Jangan menerapkan unique constraint global pada nilai kontak pemulihan.
- Verifikasi kontak bersama hanya membuktikan kendali atas kontak tersebut; bukan kepemilikan semua akun yang terhubung.
- Jangan mengungkap daftar atau jumlah akun sebelum kontak diverifikasi. Setelah verifikasi, pemilihan akun sasaran tetap memerlukan pemeriksaan kepemilikan dan risiko.
- `verification_sessions.user_id` boleh NULL sementara hanya untuk alur recovery sebelum akun sasaran dipilih. Registrasi dan verifikasi spesifik akun tetap memerlukan `user_id`.
- Reset password hanya boleh dilakukan melalui kanal pemulihan yang sudah diverifikasi.
- Jika seluruh kanal tidak tersedia, gunakan pemulihan alternatif berbasis risiko dan pemeriksaan manual oleh petugas berwenang. Pemulihan berisiko tinggi dapat ditunda atau ditolak.
- Rate limit dan lock berlaku lintas sesi; membuat sesi baru tidak boleh mengatur ulang anggaran percobaan atau pengiriman.

Detail lengkap berada di [Tinjauan Skema Identity Core — Verifikasi dan Pemulihan](identity-core-shared-recovery-contacts.md) dan [Kontrak Migration 0004](identity-core-migration-0004-account-recovery-contract.md).

## 7. Sesi dan siklus hidup akun

- Beberapa sesi aktif diperbolehkan.
- Batas sesi: tujuh hari tanpa aktivitas dan 30 hari absolut.
- Pengguna harus dapat melihat sesi aktif, mencabut sesi individual, dan logout dari semua perangkat.
- Cookie sesi menggunakan atribut Secure dan HttpOnly serta perlindungan CSRF/origin yang sesuai.
- Penonaktifan sementara, suspensi Admin, dan penghapusan permanen adalah proses yang berbeda.
- Penghapusan permanen yang diminta pengguna memiliki masa tunggu 30 hari untuk pembatalan sebelum proses penghapusan dimulai.
- Masa tunggu 30 hari bukan janji bahwa seluruh data dan backup hilang tepat pada hari ke-30. Retensi numerik per kategori masih memerlukan tinjauan hukum dan operasional.
- Admin tidak boleh menghapus akun secara permanen secara sepihak.

## 8. Status implementasi repository

Pada snapshot keputusan ini:
- Migration Identity Core 0001–0010 tersedia di repository dan diuji melalui PostgreSQL CI.
- Rust `VerificationService` mengorkestrasi registrasi, issue/resend OTP, dispatch, verifikasi, dan aktivasi dengan fake provider pada tes.
- Migration 0010 menyimpan nama tampilan publik, penerimaan Terms/Privacy per versi, dan consent marketing opsional secara atomik dengan pembuatan akun.
- Endpoint HTTP registrasi/verifikasi belum dibuat.
- Provider Resend/FlowKirim nyata belum diimplementasikan atau dikonfigurasi.
- Versi Terms/Privacy final belum ditetapkan; registry belum memiliki versi penerimaan aktif.
- Migration belum diterapkan ke database produksi. Bootstrap privilege, provisioning, dan deployment merupakan operasi terpisah.

## 9. Dokumen rujukan

- [Keputusan Autentikasi Identity Core — Password dan OTP](identity-core-authentication-stage-one-decision.md).
- [Runtime Registrasi & Verifikasi](identity-core-registration-verification-runtime.md).
- [Tinjauan Kontak Pemulihan Bersama](identity-core-shared-recovery-contacts.md).
- [Keputusan Teknis Identity Core](identity-core-technical-decisions.md).
- [Kontrak Migration 0001–0005](identity-core-migration-0001-contract.md).
- [Kontrak Migration 0010 — Profil Publik dan Penerimaan Kebijakan](identity-core-migration-0010-contract.md).
- [Registry Versi Kebijakan Publik](../legal/policy-version-registry.md).

Dokumen ini adalah keputusan produk dan engineering. Dokumen ini bukan pendapat hukum atau klaim bahwa endpoint, provider, atau deployment produksi sudah aktif.
