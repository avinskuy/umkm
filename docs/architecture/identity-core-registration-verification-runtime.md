# Desain Runtime Registrasi dan Verifikasi Identity Core

> **Status: rancangan untuk implementasi berikutnya.** Registrasi email + password wajib diverifikasi melalui OTP email; registrasi nomor telepon/WhatsApp + password wajib diverifikasi melalui OTP WhatsApp. Login rutin menggunakan email/nomor telepon + password tanpa OTP.

> **Status nama produk:** Nama produk belum final. Nama yang pernah digunakan di repository hanya working title dan dapat diganti. Dokumen ini menggunakan istilah netral seperti *aplikasi*, *platform*, dan *layanan* agar keputusan teknis tidak mengikat identitas merek.

- Status: rancangan teknis verifikasi OTP untuk pendaftaran password dan verifikasi kontak lainnya
- Cakupan: verifikasi kontak, penerbitan OTP, aktivasi/verifikasi tambahan, dan audit; bukan registrasi/login tahap pertama
- Basis: Identity Core migration 0001–0005 dan kebijakan registrasi/OTP yang telah disetujui
- Bukan: persetujuan migration database produksi, provisioning credential produksi, atau deployment

## 1. Tujuan dan prinsip

Runtime tahap berikutnya harus menyelesaikan verifikasi kontak Business Owner atau Investor tanpa membuka akses langsung aplikasi ke tabel identitas dan verifikasi. Registrasi email/WhatsApp + password menunggu verifikasi OTP sebelum aktivasi. Login sosial/OAuth tidak termasuk cakupan autentikasi saat ini.

Prinsip wajib:
1. Backend menjadi sumber kebenaran untuk validasi, role, status akun, expiry, limit, dan aktivasi.
2. Registrasi publik hanya menerima `business_owner` atau `investor`; Admin tidak dibuat melalui jalur ini.
3. Pendaftaran email/WhatsApp + password membuat akun `pending_verification`, berlaku maksimal 30 menit, dan belum dapat login atau memakai fitur akun aktif sampai OTP valid.
4. Aktivasi akun hanya terjadi setelah challenge registrasi yang tepat berhasil diverifikasi dan seluruh syarat database terpenuhi.
5. Pembuatan akun, sesi verifikasi, dan audit awal harus atomik.
6. Challenge OTP hanya dapat dipakai sekali; resend menggantikan challenge lama.
7. Penghitung dan batas lintas sesi disimpan serta diserialisasi di PostgreSQL, bukan hanya di memori proses.
8. Aplikasi tidak memperoleh SELECT/INSERT/UPDATE langsung ke tabel akun, challenge, sesi verifikasi, atau audit.
9. Rahasia kriptografi tidak disimpan di repository, database, log, atau konfigurasi frontend.
10. Kegagalan, timeout, hasil provider yang tidak pasti, dan retry ditangani fail-closed.

## 2. Alur registrasi

### 2.1 Validasi permintaan

Backend menerima role klien, satu kanal kontak awal, kontak yang sesuai kanal, dan satu metode autentikasi awal yang didukung.

- Email dinormalisasi konsisten sebelum reservasi.
- Nomor telepon dinormalisasi ke E.164.
- Tepat satu kontak awal dipilih: email atau WhatsApp.
- Tepat satu metode autentikasi awal digunakan sesuai kontrak Identity Core.
- Password plaintext hanya berada sementara di memori pada boundary request. Jangan teruskan ke repository, audit, tracing, atau database.
- Respons publik untuk konflik kontak harus generik dan tidak menjadi oracle enumerasi akun.

### 2.2 Password hashing

Gunakan Argon2id melalui library terawat dan ditinjau; jangan membuat algoritma sendiri.

Baseline awal mengikuti minimum OWASP Password Storage Cheat Sheet:
- Memory cost: 19 MiB.
- Iterations: 2.
- Parallelism: 1.
- Salt acak unik per password, minimal 16 byte.
- Output hash: 32 byte.
- Simpan encoded hash beserta parameter dan versi algoritma.

Parameter final harus diuji dengan benchmark pada lingkungan runtime sasaran. Jangan menurunkan parameter hanya untuk mempercepat CI. Hashing dilakukan sebelum transaksi database agar transaksi tidak menahan lock selama operasi CPU-intensif. Batas panjang input password harus ditetapkan dan diterapkan konsisten; jangan memotong password diam-diam. Hash palsu hanya boleh dipakai pada fixture tes.

### 2.3 Pembuatan akun pending

Dalam satu transaksi PostgreSQL:
1. Panggil fungsi terkontrol `create_client_user` untuk akun password dengan role yang diizinkan dan status pending.
2. Buat `verification_sessions` dengan `purpose='registration'`, `user_id` wajib, kanal email atau WhatsApp sesuai kontak awal, tujuan terenkripsi, fingerprint HMAC, dan expiry tidak melewati `users.pending_expires_at`.
3. Buat audit event `identity.registration.requested`.
4. Commit seluruh perubahan bersama-sama.

Jika satu langkah gagal, seluruh transaksi rollback. Unique reservation kontak utama tetap menjadi pengaman race antarregistrasi.

### 2.4 Pengiriman OTP

Penerbitan challenge dan kuota pengiriman direservasi atomik sebelum memanggil provider:
1. Normalisasi kontak dan hitung fingerprint HMAC menggunakan key versi aktif.
2. Ambil transaction-scoped advisory lock berdasarkan fingerprint dan kanal. Collision hanya boleh menambah serialisasi.
3. Periksa cooldown minimal 60 detik dan maksimal lima pengiriman per kontak dalam 60 menit, lintas sesi dan purpose.
4. Buat challenge dengan expiry maksimal 10 menit dan tidak melewati expiry sesi.
5. Supersede challenge pending sebelumnya.
6. Catat `otp_delivery_events` sebagai `pending` dalam transaksi yang sama.
7. Commit, lalu panggil adapter provider di luar transaksi.
8. Catat hasil dispatch sebagai `accepted`, `failed`, atau `unknown`.

Timeout provider yang hasilnya tidak diketahui diperlakukan sebagai `unknown`; kuota tidak dikembalikan otomatis. Retry harus punya identitas operasi/idempotency yang jelas dan tidak membuat challenge aktif ganda. Provider demonstrasi yang dipilih untuk dievaluasi adalah Resend (email) dan FlowKirim (WhatsApp). Adapter provider harus tetap dapat diganti. Biaya/free-tier, sender, template, retry policy, dan batas provider harus diverifikasi sebelum demo; CI memakai mock provider dan tidak mengirim pesan nyata. Fake provider hanya untuk tes; jangan mengirim OTP nyata sebelum provider resmi dikonfigurasi.

### 2.5 Verifikasi dan aktivasi

Verifikasi berjalan dalam satu transaksi dengan lock yang tepat:
1. Ambil lock kontak/kanal agar percobaan paralel lintas sesi terserialisasi.
2. Kunci sesi dan challenge; pastikan keduanya pending, belum kedaluwarsa, dan belum superseded.
3. Tolak jika sesi masih terkunci atau sudah mencapai batas percobaan.
4. Bandingkan keyed hash secara constant-time di lapisan aplikasi.
5. Catat event percobaan dengan fingerprint, kanal, purpose, sesi, challenge, outcome, dan waktu database.
6. Untuk kegagalan, tingkatkan counter sesi tepat satu. Kegagalan kelima mengunci sesi satu jam. Hitung juga kegagalan lintas sesi dalam rolling window 60 menit; maksimal lima per kontak.
7. Untuk keberhasilan, konsumsi challenge, tandai sesi verified, dan aktifkan akun beserta timestamp verifikasi kontak dalam transaksi yang sama.
8. Tulis audit event `identity.registration.verified` hanya ketika aktivasi benar-benar berhasil.
9. Commit.

Sesi terkunci tetap berstatus `pending`. Setelah satu jam, counter sesi dapat direset atomik pada sesi yang sama. Reset tidak menghapus event lintas sesi atau kuota pengiriman. Challenge baru boleh diterbitkan ketika sesi terkunci hanya jika cooldown dan batas pengiriman mengizinkan; challenge baru tidak mempersingkat lock.

Expiry diperiksa memakai waktu database. Aktivasi ditolak jika akun pending atau challenge sudah kedaluwarsa, termasuk bila transaksi selesai setelah deadline.

### 2.6 Kedaluwarsa pendaftaran tertunda

Akun yang tidak selesai dalam 30 menit dipindahkan ke `pending_expired` melalui proses cleanup terkontrol. Pelepasan reservasi kontak dan pembersihan nilai identitas aktif mengikuti kontrak migration 0001 dan harus atomik. Cleanup tidak boleh memengaruhi akun aktif, sesi recovery, atau akun lain. Cleanup job dan jadwalnya merupakan komponen operasional terpisah.

## 3. Kontrak kriptografi

Gunakan primitive standar dan library terawat. Jangan merancang cipher, KDF, generator acak, atau format hash sendiri.

### 3.1 Password
- Argon2id dengan salt unik per password.
- Encoded hash menyimpan parameter dan versi algoritma.
- Password dan encoded hash tidak pernah masuk log, audit metadata, tracing span, error response, atau telemetry.
- Dukung verifikasi hash lama yang masih diizinkan dan rehash setelah autentikasi bila parameter dinaikkan.

### 3.2 Tujuan pengiriman
- Tujuan yang perlu dipulihkan untuk pengiriman disimpan sebagai ciphertext AEAD.
- Gunakan implementasi standar seperti ChaCha20-Poly1305 atau AES-256-GCM dari library terawat; keputusan final ditetapkan bersama key-provider dan diuji pada target runtime.
- Nonce harus unik sesuai persyaratan algoritma dan disimpan bersama ciphertext.
- Gunakan associated data yang mengikat ciphertext ke identitas sesi, kanal, purpose, dan versi format.
- Key enkripsi terpisah dari key HMAC dan key OTP.

### 3.3 Fingerprint kontak
- Gunakan HMAC-SHA-256 dengan key khusus yang berbeda dari key enkripsi dan OTP.
- Input memiliki domain separation dan encoding kanonis, minimal mencakup versi format, kanal, dan kontak ternormalisasi.
- Simpan fingerprint dan versi key, bukan kontak plaintext, di event rate-limit.
- Rotasi key harus tetap memungkinkan evaluasi event dalam jendela 60 menit. Selama transisi, runtime harus dapat menghitung/mencari fingerprint versi lama dan aktif tanpa melemahkan limit.

### 3.4 OTP
- OTP dibuat menggunakan CSPRNG dari library sistem/crypto yang aman.
- OTP tidak disimpan plaintext.
- Simpan keyed hash yang mengikat kode pada challenge ID, session ID, purpose, kanal, dan versi key.
- Perbandingan hash menggunakan operasi constant-time.
- Jumlah digit, encoding, dan parameter keyed hash ditetapkan sebelum endpoint publik diaktifkan; jangan menganggap default UI sebagai kebijakan keamanan.
- Kode tidak boleh masuk audit, log, URL, analytics, atau error telemetry.

### 3.5 Manajemen kunci

Runtime bergantung pada abstraksi key provider yang menyediakan key berdasarkan purpose dan version.

- Local development dan CI boleh memakai key sementara yang dibuat untuk tes.
- Production key berasal dari secret manager/KMS yang disetujui; provider produksi belum dipilih.
- Key memiliki versi, rotasi, prosedur rollback, akses minimum, dan audit akses.
- Startup gagal tertutup bila key wajib tidak tersedia atau versi key tidak dikenal.
- Tidak ada fallback ke key hardcoded, default, atau kosong.

## 4. Batas modul backend

Struktur yang disarankan:
- `identity/password.rs`: hashing, verifikasi, parameter dan encoded hash.
- `identity/crypto.rs`: AEAD, HMAC fingerprint, keyed OTP hash, key-version interfaces.
- `identity/verification.rs`: orchestration sesi, challenge, rate limit, dan verifikasi.
- `identity/registration.rs`: validasi dan orchestration registrasi melalui operasi database terkontrol.
- `identity/delivery/`: trait provider dan fake provider; adapter nyata setelah vendor dipilih.
- `identity/audit.rs`: append-only audit yang sudah tersedia.

Modul domain tidak bergantung pada Axum. Handler HTTP hanya memetakan request/response dan memanggil service. Repository tidak mengembalikan rahasia yang tidak diperlukan caller. Hindari modul besar yang mencampur HTTP, crypto, SQL, dan provider.

## 5. Least privilege dan operasi database

Principal runtime hanya mendapat EXECUTE pada fungsi yang terdaftar, INSERT audit, dan EXECUTE validator metadata. Akses baru dibuka eksplisit dan fail-closed.

### Implementasi database operations 0007

Migration 0007 menambahkan operasi terkontrol untuk tahap registrasi OTP:

- `create_registration_verification_session`: membuat sesi verifikasi yang terikat pada akun pending dan kanal kontak awal.
- `reserve_otp_challenge`: mengganti challenge pending, mencatat reservasi pengiriman, dan menerapkan cooldown 60 detik serta batas lima pengiriman dalam 60 menit.
- `record_otp_delivery_result`: menyimpan hasil dispatch provider satu kali.
- `record_otp_verification_attempt`: mencatat hasil pemeriksaan OTP, menerapkan batas kegagalan lintas sesi, mengunci sesi setelah lima kegagalan, dan mengaktifkan akun secara atomik saat berhasil.

Rate limit lintas rotasi HMAC menerima daftar fingerprint aktif dan versi sebelumnya yang masih berada dalam jendela 60 menit. Runtime wajib menyertakan seluruh versi fingerprint yang masih relevan dan mengunci seluruh alias secara berurutan agar request paralel tidak melewati kuota.

Operasi verifikasi pada migration 0007 dibatasi untuk purpose registrasi. Perubahan kontak, penambahan recovery contact, dan account recovery belum dibuka melalui fungsi ini. Pemeriksaan hash OTP dilakukan di backend; fungsi database menerima hasil boolean dari service tepercaya dan hanya mengelola transaksi, lifecycle, serta limit.

Bootstrap memverifikasi owner, SECURITY DEFINER, dan search_path untuk seluruh fungsi tersebut. Principal aplikasi tetap tidak memiliki akses langsung ke tabel verifikasi.


- Jangan memberikan SELECT/INSERT/UPDATE/DELETE langsung pada tabel verifikasi atau identitas.
- Gunakan fungsi PostgreSQL `SECURITY DEFINER` yang sempit untuk operasi lintas tabel.
- Setiap fungsi dimiliki `usahapintar_migrator` non-login/non-superuser, memakai `search_path=pg_catalog`, menghindari SQL dinamis yang tidak diperlukan, dan PUBLIC EXECUTE dicabut.
- Bootstrap privilege baru memverifikasi owner, konfigurasi, role membership, dan privilege sebelum GRANT.
- Jangan memberi fungsi generik yang menerima SQL, nama tabel, role bebas, atau user ID tanpa validasi hubungan dan status.
- Fungsi aktivasi memeriksa purpose registration, account/session binding, status pending, expiry akun/sesi/challenge, dan kanal kontak.
- Perubahan data dan audit wajib berada dalam transaksi yang sama.
- Jangan menjalankan migration otomatis pada HTTP startup.

Operasi database untuk sesi/challenge, attempt/delivery, supersede, verifikasi, aktivasi, dan cleanup harus dirancang sebagai API database terkontrol sebelum runtime menggunakannya. Bootstrap 002 tidak boleh diperluas dengan grant tabel luas.

## 6. Kontrak provider OTP

Interface provider mengembalikan hasil eksplisit:
- `Accepted`: provider menerima permintaan.
- `Rejected`: provider menolak secara definitif.
- `Unknown`: timeout/koneksi terputus sehingga hasil tidak dapat dipastikan.

Provider tidak menerima password, session token, atau metadata yang tidak diperlukan. Tujuan hanya dikirim untuk dispatch dan tidak dicatat pada log. Pesan menjelaskan kode tidak boleh dibagikan dan tidak memuat data akun berlebihan. Fake provider dapat mensimulasikan accepted, rejected, timeout/unknown, retry, dan duplikasi. Tidak ada provider nyata yang diaktifkan pada tahap ini.

## 7. Kontrak API dan UX

Endpoint publik belum ditetapkan. Saat dirancang, gunakan prefix `/api/v1`, validasi backend, schema request/response eksplisit, rate limit, correlation ID, dan error response konsisten.

UX minimum:
- Tampilkan tujuan yang dimasking, expiry/countdown, dan waktu tunggu resend.
- Dukung paste dan autofill kode bila platform memungkinkan.
- Jelaskan bahwa kode tidak boleh dibagikan.
- Pertahankan input aman ketika terjadi kegagalan jaringan.
- Jelaskan lock dan waktu tunggu tanpa mengungkap counter internal atau deteksi abuse.
- Jangan memberi tahu apakah email/nomor tertentu sudah terdaftar.
- Jangan meminta permission membaca SMS secara luas.
- Tampilkan berhasil hanya setelah aktivasi backend terkonfirmasi.

## 8. Audit dan privasi

Event minimum:
- `identity.registration.requested`
- `identity.verification.delivery_requested`
- `identity.verification.delivery_result`
- `identity.verification.attempt_failed`
- `identity.registration.verified`
- `identity.registration.expired`

Gunakan actor `system` untuk proses tanpa aktor pengguna terautentikasi. Metadata hanya key allowlist dan nilai skalar non-sensitif. Jangan mencatat kontak plaintext, OTP, password, hash password, ciphertext, nonce, key, token, atau isi pesan.

## 9. Pengujian wajib

### Unit dan crypto
- Normalisasi email dan nomor telepon.
- Argon2id menghasilkan salt berbeda, dapat diverifikasi, dan menolak password salah.
- Validasi parameter dan format encoded hash.
- AEAD round-trip; autentikasi gagal saat ciphertext/AAD berubah; nonce tidak digunakan ulang dalam satu operasi.
- HMAC domain separation untuk kanal/kontak, key version, dan constant-time comparison helper.
- OTP menggunakan CSPRNG dan tidak pernah disimpan plaintext.
- Tidak ada secret/payload sensitif pada error, log, atau audit.

### Integrasi PostgreSQL
- Akun pending, sesi verifikasi, dan audit awal commit/rollback bersama.
- Dua registrasi bersamaan dengan kontak sama hanya menghasilkan satu reservasi.
- Challenge pending tunggal per sesi; resend supersede challenge lama.
- Challenge kedaluwarsa, superseded, consumed, atau sesi terminal tidak dapat dipakai.
- Aktivasi dan konsumsi challenge atomik; dua verifikasi paralel tidak mengaktifkan dua kali.
- Lima kegagalan mengunci sesi satu jam; lock tidak dapat dipersingkat.
- Reset counter hanya setelah lock habis dan tidak menghapus limit lintas sesi.
- Maksimal lima kegagalan per kontak dalam rolling 60 menit, lintas sesi/purpose dan koneksi.
- Maksimal lima pengiriman per kontak per jam dan cooldown 60 detik, termasuk concurrency.
- Pending registration tidak dapat diaktifkan setelah 30 menit.
- Cleanup melepas reservasi hanya untuk akun pending yang benar-benar kedaluwarsa.
- Application principal hanya menjalankan fungsi terdaftar; akses langsung ke tabel sensitif tetap ditolak.
- Fungsi SECURITY DEFINER menolak payload yang tidak memenuhi binding dan lifecycle.

### HTTP/runtime
- Input tidak valid, kontak terduplikasi, rate limit, expiry, provider rejected/unknown, dan database unavailable menghasilkan respons aman.
- Tidak ada endpoint yang mengembalikan OTP, password hash, ciphertext, key version internal, atau detail audit.
- Uji request paralel/retry untuk mencegah challenge ganda dan aktivasi ganda.
- Uji end-to-end memakai database disposable dan fake provider.

CI menjalankan format, Clippy, unit tests, PostgreSQL integration tests dengan principal least-privilege, dan build. Jangan memakai database superuser sebagai pengganti principal runtime.

## 10. Tahapan implementasi

Tahapan pada dokumen ini dimulai setelah fondasi registrasi password dan verifikasi OTP disetujui untuk implementasi runtime. Urutan tahap verifikasi berikutnya:
1. **Database operations:** migration dan operasi terkontrol untuk sesi/challenge/rate limit serta bootstrap privilege sempit.
2. **Verification orchestration:** sesi verifikasi, fake delivery provider, audit, dan transaksi atomik.
3. **OTP lifecycle:** issue/resend, dispatch result, verify, lock, dan cross-session limits.
4. **HTTP API:** endpoint verifikasi/resend dengan validasi, rate limit, error contract, dan tests.
5. **Operational readiness:** key provider produksi, provider OTP nyata, observability tanpa PII, cleanup job, alert biaya/abuse, runbook, dan security review.

Setiap tahap melalui PR, review patch, CI hijau pada head final, dan verifikasi merge. Migration produksi, provisioning credential produksi, dan deployment memerlukan persetujuan terpisah.

## 11. Batas yang belum diselesaikan

Hal berikut tetap terbuka dan tidak boleh ditebak:
- Provider email/WhatsApp dan sender resmi.
- Production KMS/secret manager.
- Library AEAD final dan prosedur rotasi key.
- Jumlah digit OTP dan format pesan.
- Batas password minimum/maksimum dan benchmark Argon2id pada runtime sasaran.
- Idempotency strategy provider dan retry/outbox operasional.
- Jadwal cleanup dan retensi event.
- Endpoint HTTP final dan kontrak frontend.

Tidak ada credential atau secret yang diminta melalui chat. Jangan mengirim OTP provider key, database password, atau production secret ke repository maupun percakapan.
