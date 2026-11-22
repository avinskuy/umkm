# Keputusan Teknis Identity Core

> Cakupan: baseline teknis lintas migration Identity Core.
> Registrasi email/WhatsApp + password wajib OTP; login rutin menggunakan password tanpa OTP.

Dokumen ini menetapkan baseline teknis Identity Core. Migration 0001–0011 dan bootstrap privilege tersedia serta diuji di CI. Runtime menyediakan orkestrasi registrasi/verifikasi OTP dan router registrasi yang belum dipasang pada aplikasi HTTP default. Login/logout, sesi HTTP, provider nyata, key produksi, dan recovery runtime belum tersedia. HTTP startup tidak menjalankan migration otomatis.

Registrasi publik menggunakan email + password dengan OTP email atau nomor telepon/WhatsApp + password dengan OTP WhatsApp. Login rutin menggunakan email atau nomor telepon + password tanpa OTP setiap kali login. Migration 0006 menghapus dukungan Google OAuth dari skema aktif. Resend dan FlowKirim menjadi target adapter pengiriman; integrasi nyata belum tersedia. Detail kebijakan ada di [Keputusan Autentikasi Identity Core — Password dan OTP](identity-core-authentication-stage-one-decision.md).

## 1. Perlindungan tujuan pengiriman OTP — tahap berikutnya

### Keputusan

Bagian ini menjadi baseline untuk OTP registrasi email/WhatsApp + password dan verifikasi kontak lainnya. OTP tidak digunakan untuk login rutin.

Gunakan dua representasi yang berbeda untuk dua kebutuhan berbeda:

1. **Ciphertext tujuan** untuk mengirim ulang kode melalui provider.
2. **HMAC tujuan yang dinormalisasi** untuk pencarian, deduplikasi, dan pembatasan lintas sesi.

Hash biasa tidak dapat dipakai untuk pengiriman karena tidak dapat dipulihkan menjadi alamat email atau nomor telepon. Jangan menyimpan tujuan OTP dalam plaintext pada tabel verifikasi.

Kolom konseptual untuk sesi verifikasi dan relasi kontak pemulihan:

- `destination_ciphertext` atau `contact_ciphertext`: bytea berisi ciphertext.
- `destination_nonce` atau `contact_nonce`: nonce AEAD yang unik sesuai algoritma yang dipilih.
- `encryption_key_version`: identifier versi key/envelope yang dipakai.
- `destination_hmac` atau `contact_hmac`: HMAC atas nilai tujuan yang telah dinormalisasi.
- `hmac_key_version`: versi key HMAC untuk pencarian.

### Batas keamanan

- Gunakan enkripsi terautentikasi (AEAD) dari library kriptografi yang terawat. Jangan membuat algoritma atau format kriptografi sendiri.
- Kunci enkripsi dan HMAC harus berbeda, disimpan di luar database, dan tidak masuk source control, log, atau frontend.
- Desain produksi mengutamakan envelope encryption dengan KMS/secret-management yang sesuai. Backend mengakses operasi enkripsi/dekripsi melalui abstraksi key provider; kode domain tidak membaca key mentah secara bebas.
- Akses dekripsi hanya diberikan pada komponen yang benar-benar perlu mengirim OTP atau memproses kontak. Admin operasional tidak memperoleh akses plaintext hanya karena memiliki role Admin.
- HMAC hanya untuk pencocokan; HMAC tidak boleh dipakai sebagai bukti kepemilikan akun.
- Normalisasi email dan nomor telepon dilakukan sebelum enkripsi dan HMAC. Untuk nomor telepon gunakan format E.164; normalisasi email harus konsisten dan tidak boleh menerapkan transformasi provider-specific yang mengubah identitas.
- Rotasi key harus mendukung pembacaan versi lama selama masa transisi, penulisan dengan versi aktif, backfill terkontrol, verifikasi hasil, dan pencabutan versi lama setelah seluruh data relevan selesai. Rotasi HMAC memerlukan strategi dual-read/backfill agar pencarian dan deduplikasi tidak terputus. Untuk rate limit, perubahan versi fingerprint tidak boleh membuka jendela bypass: selama transisi, transaksi harus mengunci dan menghitung fingerprint versi lama maupun aktif dalam urutan deterministik, atau memakai fingerprint lock yang stabil sampai seluruh event lama melewati jendela limit.

Nilai kontak utama yang dibutuhkan untuk login tetap mengikuti model identitas akun dan perlindungan database/storage sesuai klasifikasi data. Keputusan ini terutama mengatur salinan tujuan sementara yang diperlukan untuk verifikasi/pengiriman dan kontak pemulihan yang perlu dicari lintas akun.

### Batas migration

Migration hanya menyediakan kolom ciphertext, nonce, key version, HMAC, dan HMAC key version yang telah disepakati. Migration tidak menyimpan key, tidak melakukan enkripsi sendiri, dan tidak mengunci algoritma library sebelum dependency serta key provider dipilih pada tahap implementasi.

## 2. Rate limit pengiriman OTP lintas sesi dan instance

### Keputusan

Gunakan PostgreSQL sebagai sumber kebenaran untuk baseline batas per kontak. Catat setiap percobaan penerbitan/pengiriman dalam tabel event terbatas, dengan fingerprint HMAC tujuan, kanal, purpose, dan timestamp. Jangan mengandalkan penghitung di memory proses atau satu baris verification session.

Untuk setiap permintaan pengiriman:

1. Normalisasi tujuan dan hitung fingerprint HMAC menggunakan key versi aktif.
2. Mulai transaksi PostgreSQL.
3. Ambil transaction-scoped advisory lock yang stabil berdasarkan fingerprint tujuan dan kanal. Benturan hash hanya boleh menyebabkan serialisasi tambahan, bukan melemahkan limit.
4. Periksa event untuk fingerprint dan kanal yang sama pada 60 menit berjalan, tanpa memisahkan berdasarkan verification session. Batas berlaku lintas purpose agar pembuatan sesi baru tidak menghindarinya.
5. Periksa waktu pengiriman terakhir untuk cooldown 60 detik.
6. Jika batas terpenuhi, tolak permintaan tanpa mengirim. Jika diizinkan, catat event dalam transaksi sebelum pekerjaan pengiriman diteruskan.
7. Commit transaksi; pengiriman provider dilakukan melalui alur yang menangani kegagalan secara eksplisit. Percobaan yang sudah mengonsumsi kuota tidak boleh mengembalikan kuota secara otomatis hanya karena hasil provider tidak pasti.

Aturan produk yang harus ditegakkan tetap:
- Maksimal lima pengiriman per kontak dalam jendela bergulir 60 menit.
- Cooldown minimal 60 detik antar-pengiriman.
- Batas tersebut berlaku lintas sesi, proses, dan instance aplikasi.
- Reset lock percobaan verifikasi setelah satu jam tidak menghapus event pengiriman dan tidak mengubah kuota kontak.
- Penerbitan OTP baru saat sesi terkunci boleh dilakukan bila cooldown dan limit pengiriman mengizinkan, tetapi tidak membuka verifikasi sebelum lock berakhir.

### Data dan operasi

- Tabel event menyimpan fingerprint HMAC, kanal, purpose, timestamp, dan status dispatch minimum; tidak menyimpan alamat tujuan plaintext atau kode OTP.
- Gunakan indeks yang mendukung filter fingerprint + kanal + timestamp.
- Bersihkan event hanya setelah tidak lagi diperlukan untuk evaluasi jendela limit dan kebutuhan operasional yang disetujui. Jangan menghapus event yang masih berada dalam jendela 60 menit.
- Limit per IP/network, per akun, serta guard biaya/throughput provider tetap menjadi lapisan tambahan. Implementasinya harus mengikuti kebijakan privasi dan tidak menjadikan IP sebagai satu-satunya identitas abuse control.

PostgreSQL advisory lock dipilih karena bersifat lintas koneksi dan lintas instance yang memakai database yang sama. Pengujian wajib memakai beberapa koneksi bersamaan untuk membuktikan bahwa permintaan paralel tidak melewati batas.

### Batas percobaan verifikasi lintas sesi

Batas kegagalan OTP tidak boleh hanya bergantung pada `failed_attempt_count` satu sesi. Jika pengguna dapat membuat sesi baru untuk tujuan yang sama, penghitung sesi baru tidak boleh memberi anggaran percobaan kontak yang baru.

Baseline implementasi:
- Simpan event percobaan verifikasi yang benar-benar diproses secara persisten, dengan fingerprint HMAC tujuan, versi key, kanal, purpose, sesi, challenge, outcome, dan timestamp.
- Hitung kegagalan lintas sesi berdasarkan fingerprint tujuan/kanal; jangan mempartisi limit berdasarkan purpose.
- Catat event gagal dan berhasil. Hanya outcome gagal mengonsumsi anggaran kegagalan; sesi tetap memiliki counter dan lock satu jam sendiri sesuai kebijakan OTP.
- Penolakan karena sesi terkunci/kedaluwarsa/terminal atau rate limit tidak dihitung sebagai tebakan kode yang diproses.
- Penghitungan event lintas sesi, penambahan counter sesi, dan konsumsi challenge harus konsisten dalam transaksi PostgreSQL.
- Batas lintas sesi ditetapkan maksimal **5 kegagalan per kontak dalam jendela bergulir 60 menit**, melintasi seluruh sesi dan purpose. Hanya event `failed` mengonsumsi anggaran; `succeeded` tetap dicatat. Runtime harus mengunci fingerprint kontak/kanal, menghitung event gagal pada jendela tersebut, lalu mencatat outcome dan memperbarui counter sesi dalam transaksi PostgreSQL yang sama. Migration hanya menyediakan tabel, constraint, dan indeks; enforcement baru aktif setelah runtime OTP diimplementasikan.

Kontrak tabel dan tes ada pada [Kontrak Skema Migration 0002 — Verifikasi Kontak dan OTP](identity-core-migration-0002-verification-contract.md).

## 3. Status akun dan reservasi kontak utama

### Keputusan

Gunakan `account_status` hanya untuk siklus hidup akun. Pembatasan akses oleh Admin (misalnya suspend atau limit fitur) dicatat terpisah melalui moderation/enforcement records. Pemisahan ini memungkinkan akun yang sedang dibatasi tetap mengajukan penghapusan tanpa kehilangan riwayat atau mencampur dua state machine. Nilai lifecycle yang diizinkan:

| Status | Arti dan aturan |
| --- | --- |
| `pending_verification` | Status akun baru selama proses registrasi menunggu verifikasi OTP. Akun belum dapat login atau menggunakan fitur akun aktif sampai verifikasi berhasil. |
| `active` | Akun aktif dan dapat memakai fitur sesuai role/entitlement. |
| `deactivated` | Penonaktifan sementara oleh pengguna; profil disembunyikan, data dipertahankan, kontak tetap direservasi. |
| `deletion_pending` | Permintaan penghapusan dalam masa tunggu pembatalan 30 hari; akun dibatasi dan kontak tetap direservasi. |
| `deletion_processing` | Masa tunggu selesai dan proses penghapusan sedang berlangsung; kontak tetap direservasi sampai penghapusan benar-benar selesai. |
| `pending_expired` | Status terminal ketika registrasi tidak menyelesaikan verifikasi OTP dalam batas waktu yang ditetapkan. |
| `closed` | Penghapusan telah selesai sesuai proses retensi; kontak utama telah dilepas dan data personal telah dihapus atau dianonimkan sesuai kebijakan. |

Status suspend/limit fitur, lock verifikasi, lock keamanan sementara, dan status OTP tidak boleh dimasukkan ke `account_status`. Enforcement aktif dapat membatasi akses meskipun lifecycle akun tetap `active` atau `deletion_pending`. Status moderasi konten juga bukan status lifecycle akun.

### Aturan reservasi

Unique partial index kontak utama hanya mencakup akun yang masih memegang reservasi: `pending_verification`, `active`, `deactivated`, `deletion_pending`, dan `deletion_processing`. Akun yang terkena suspend tetap berstatus lifecycle `active` (atau status lifecycle lain yang sah), sehingga reservasi tetap berlaku tanpa status `suspended` tersendiri.

Kontak dilepas hanya dalam dua kondisi:
- Registrasi pending telah kedaluwarsa dan transisi ke `pending_expired` selesai secara atomik.
- Penghapusan permanen benar-benar selesai dan transisi ke `closed` dilakukan setelah kewajiban retensi yang berlaku ditangani.

Registrasi tahap pertama membuat akun berstatus `pending_verification`. Akun hanya berubah menjadi `active` setelah OTP valid untuk kontak awal diverifikasi dan aktivasi diselesaikan secara atomik. Proses cleanup mengubah registrasi yang melewati batas waktu menjadi `pending_expired`; jangan melepas kontak sebelum transisi tersebut selesai. Saat deletion processing, jangan melepas kontak hanya karena 30 hari telah berlalu.

Transisi status harus dilakukan oleh operasi backend yang tervalidasi dan tercatat. CHECK constraint menjaga nilai lifecycle yang sah; active moderation/enforcement records mengatur pembatasan akses; partial unique index menjaga reservasi; tes transaksi membuktikan tidak ada race antara aktivasi, cleanup, pembatalan penghapusan, dan registrasi baru.

## 4. Provisioning Admin

### Keputusan

Tidak ada endpoint registrasi publik untuk role `admin`. Akun Admin dibuat melalui jalur provisioning internal yang terpisah dari alur registrasi Business Owner dan Investor.

Baseline provisioning:

- Operator yang berwenang membuat akun melalui prosedur internal/CLI atau workflow provisioning yang terautentikasi; bukan melalui payload registrasi publik.
- Gunakan kredensial database/principal provisioning yang terpisah dan hanya tersedia pada lingkungan operator yang dikendalikan. Kredensial tersebut tidak boleh tersedia bagi frontend atau proses API publik.
- Runtime principal aplikasi tidak boleh memiliki jalur umum untuk menaikkan role akun menjadi Admin. Role tidak dapat diubah setelah INSERT. Pada tingkat database, hindari hak INSERT langsung yang memungkinkan runtime memilih sembarang role: gunakan fungsi pembuatan akun klien yang hanya menerima `business_owner`/`investor`, sedangkan provisioning Admin memakai principal/fungsi internal terpisah. Fungsi `SECURITY DEFINER` (jika dipakai) harus dimiliki role non-login, memakai `search_path` tetap, tidak memakai SQL dinamis, dan hak EXECUTE publik dicabut.
- Provisioning mencatat aktor, alasan, waktu, identitas target, dan hasil ke audit trail.
- Akun Admin tidak memperoleh akses istimewa sebelum menyelesaikan bootstrap keamanan yang diwajibkan, termasuk MFA. Mekanisme MFA dan penyedia final dipilih pada tahap implementasi Admin.
- Undangan/bootstrap harus sekali pakai, kedaluwarsa, dan tidak memuat password atau OTP plaintext dalam log.
- Pembuatan akun Admin tidak boleh mengubah atau mengambil alih akun klien yang sudah ada. Tidak ada role migration.

### Batas implementasi

Migration boleh mendukung nilai role `admin`, tetapi nilai tersebut bukan izin bagi endpoint publik untuk membuat Admin. Pengujian harus membuktikan bahwa jalur registrasi publik hanya menerima `business_owner` atau `investor`, sementara provisioning Admin membutuhkan principal/workflow internal terpisah dan tercatat.

## 5. Batas audit Identity Core

### Keputusan

Gunakan tabel audit Identity Core khusus untuk peristiwa keamanan dan perubahan identitas yang penting. Audit trail berbeda dari log aplikasi/observability dan bukan tempat menyimpan salinan payload request.

Event minimum:
- Registrasi dibuat, diverifikasi, kedaluwarsa, atau dibatalkan.
- Upaya perubahan role, termasuk penolakan.
- Perubahan status akun, termasuk suspend, restore, deactivate, deletion request, cancellation, processing, dan closed.
- Perubahan kontak utama atau recovery contact.
- Penerbitan, verifikasi, supersede, expiry, dan kegagalan challenge yang relevan.
- Login/session creation, revocation, logout-all, dan penolakan karena sesi tidak valid.
- Recovery request, account selection, hasil pemeriksaan, assignment/review, dan outcome.
- Provisioning Admin, perubahan izin Admin, dan tindakan istimewa terkait Identity Core.

Kolom konseptual:
- ID event UUID.
- Waktu kejadian `timestamptz`.
- Jenis aktor (`user`, `admin`, `system`, atau `operator`) dan actor ID bila tersedia.
- Subject user ID bila relevan.
- Kode aksi, outcome, reason code yang aman, dan request/correlation ID.
- Metadata JSONB yang dibatasi allowlist dan tidak berisi rahasia.

### Perlindungan

- Principal aplikasi memperoleh hak INSERT saja untuk audit event dan SELECT hanya pada kebutuhan internal yang jelas; tidak memperoleh UPDATE/DELETE.
- Pemilik tabel adalah migration/database owner yang terpisah dari runtime application principal.
- Perubahan hak akses dan operasi administratif database diaudit melalui jalur operasional terpisah.
- Audit tidak menyimpan password, OTP, session/reset token, secret, data pembayaran sensitif, isi percakapan privat, dokumen rahasia, bukti recovery mentah, atau catatan investigasi internal.
- PII diminimalkan; gunakan ID internal, reason code, dan fingerprint bila cukup.
- Akses baca audit dibatasi berdasarkan kebutuhan tugas dan dicatat. Role Admin umum tidak otomatis memperoleh akses ke seluruh metadata sensitif.
- Untuk kebutuhan deteksi dan ketahanan terhadap manipulasi yang lebih tinggi, event terpilih dapat diteruskan ke penyimpanan log terpisah yang akses dan retensinya dikelola independen. Ini tidak menggantikan kontrol database.

Append-only di sini berarti tidak dapat diubah oleh principal aplikasi normal; bukan klaim bahwa data mustahil diubah oleh pemilik database atau operator dengan hak tertinggi.

## 6. Kontrak skema lintas migration

Keputusan berikut mengikat rancangan migration yang relevan:

1. `users.primary_role` hanya `business_owner`, `investor`, atau `admin`; tidak ada operasi update role.
2. `account_status` memakai lifecycle pada Bagian 3; suspend dan limit fitur berasal dari moderation/enforcement records, bukan lifecycle. Temporary lock berada di tabel/sesi yang sesuai.
3. Kontak utama unik secara global selama reservasi. Recovery contact dapat dipakai bersama lintas akun, tetapi unique constraint per akun/channel/fingerprint mencegah duplikasi relasi.
4. `verification_sessions.user_id` nullable hanya untuk recovery pra-pemilihan akun. Registration dan verifikasi account-specific wajib memiliki user ID.
5. Satu verification session memiliki maksimal satu recovery case; setiap recovery case wajib memiliki satu verification session. Target user nullable sebelum account selection.
6. OTP challenge mempunyai state `pending`, `consumed`, `expired`, `superseded`, atau `cancelled`; hanya satu challenge pending per sesi.
7. Limit pengiriman per kontak menggunakan event lintas sesi yang diserialisasi melalui PostgreSQL; penghitung gagal verifikasi dan lock tetap melekat pada verification session.
8. Token OTP, session, dan password reset hanya disimpan sebagai hash yang sesuai; tujuan pengiriman yang perlu dipulihkan disimpan terenkripsi.
9. Semua perubahan lifecycle, kontak, sesi, recovery, dan provisioning Admin yang penting menghasilkan audit event.
10. Migration yang sudah diterapkan tidak diedit; perubahan berikutnya menggunakan migration baru.

## 7. Gerbang migration dan status implementasi

### Migration 0001 — identitas akun

Gerbang migration akun pertama telah dipenuhi dan hasilnya sudah di-merge ke `main`:

- [x] Kontrak kolom, tipe PostgreSQL, CHECK constraint, partial unique index, foreign key, dan lifecycle ditinjau.
- [x] Principal migration dipisahkan secara konseptual dari runtime application dan provisioning Admin; GRANT lingkungan tetap belum dibuka.
- [x] Tes PostgreSQL nyata untuk role immutable, penolakan pembuatan Admin melalui jalur klien, reservasi kontak, cleanup pending, invariant password-only, dan konkurensi berhasil di CI.
- [x] Migration tidak memuat key, data pribadi, atau kredensial.
- [x] HTTP runtime menginisialisasi pool PostgreSQL menggunakan principal aplikasi dengan hak minimum; proses startup tidak menjalankan migration otomatis.
- [x] Migration tidak dijalankan pada database produksi sebagai bagian dari pekerjaan ini.

### Migration 0002 — verifikasi kontak dan OTP

Migration 0002 telah di-merge ke `main` dan lulus CI PostgreSQL. Skema menyediakan sesi verifikasi, challenge, serta event percobaan/pengiriman. Runtime OTP, provider, abstraksi key provider, dan enforcement rate limit masih merupakan pekerjaan lanjutan; batas lima kegagalan per kontak dalam 60 menit tetap wajib ditegakkan secara atomik lintas sesi dan instance.

### Migration 0003 — sesi autentikasi

Migration 0003 telah di-merge dan lulus CI PostgreSQL. Skema menyediakan hash token sesi, batas absolut 30 hari, idle timeout 7 hari, rotasi token, pencabutan satu arah, dan pencabutan otomatis ketika status lifecycle akun berubah menjadi nonaktif. Endpoint, cookie, middleware, validasi sesi per request, audit event, serta pencabutan karena reset password/restriksi Admin tetap merupakan pekerjaan runtime.

### Migration 0004 — pemulihan akun

Migration 0004 menambahkan relasi kontak pemulihan yang boleh digunakan lintas akun, kasus recovery yang baru mengikat akun setelah kontak diverifikasi, dan token reset kata sandi berbentuk hash unik. Database mengikat kontak ke sesi `recovery_contact_add` yang terverifikasi, membatasi duplikasi aktif per akun, menyediakan indeks pencarian fingerprint lintas akun, mengunci target kasus setelah dipilih, serta mensyaratkan kasus `password_reset` yang disetujui dan akun aktif sebelum token dibuat. Runtime tetap wajib mencocokkan fingerprint dengan kontak milik akun target, menjalankan pemeriksaan risiko dan limit, mengelola key/provider, mengonsumsi token dan mengganti password secara atomik, mencabut sesi, serta menulis audit.

### Migration 0005 — Identity Audit Events

Migration 0005 menambahkan `identity_audit_events` sebagai jejak audit Identity Core. Event menyimpan actor type/ID, subject akun, action/outcome/reason code, request correlation ID, metadata terbatas, dan waktu kejadian yang ditetapkan database. Metadata JSONB dibatasi pada objek datar dengan key allowlist, nilai skalar, panjang string maksimum 128 karakter, dan ukuran maksimum 2 KiB. Trigger menolak UPDATE, DELETE, dan TRUNCATE; tabel tidak memberikan privilege kepada PUBLIC.

Runtime saat ini menyediakan operasi terbatas `append_audit_event` dan principal aplikasi memperoleh hak INSERT-only sesuai bootstrap privilege. Pemanggil tetap wajib memvalidasi aktor dan action-specific metadata serta menulis event dalam transaksi bersama perubahan penting. Integrasi audit untuk seluruh lifecycle identitas, principal pembaca, dan retensi/forwarding eksternal masih perlu diselesaikan. Fungsi validasi metadata hanya melakukan pemeriksaan struktur dan tidak memberi akses ke tabel audit.

Jangan menjalankan migration 0001–0005 pada database produksi. Migration yang telah diterapkan pada database bersama tidak boleh diedit; semua perubahan berikutnya memakai migration baru.

## 8. Keputusan yang tetap ditunda

Hal berikut bukan penghalang untuk menyepakati model konseptual, tetapi harus diputuskan sebelum fitur terkait diaktifkan:

- Provider email/WhatsApp, sender/template, retry, monitoring, biaya, dan fallback.
- Library AEAD, KMS/key provider, parameter key wrapping, dan prosedur operasional rotasi.
- Detail implementasi advisory-lock helper, cleanup event, dan limit tambahan IP/account/provider.
- Penyedia dan metode MFA Admin.
- Retensi numerik audit, verification delivery event, dan data lifecycle sesuai kajian hukum/operasional.
- Parameter password hashing dan konfigurasi sesi/cookie produksi.

Keputusan ini tidak membuka ulang kebijakan OTP atau recovery yang telah disetujui; ini adalah detail implementasi dan operasi yang perlu diverifikasi sebelum rilis fitur terkait.

## Database principal dan privilege bootstrap

Provisioning role PostgreSQL tidak dimasukkan ke SQLx schema migration. Database khusus memakai role group `usahapintar_app`, `usahapintar_backup`, `usahapintar_monitor`, dan `usahapintar_audit_reader`, serta login terpisah untuk runtime, migrator, backup, dan monitoring. Semua login non-superuser dan tanpa CREATEDB/CREATEROLE/BYPASSRLS. DBA tetap dikelola di luar skrip aplikasi.

Semua migration harus dijalankan oleh `usahapintar_migrator`. Hal ini kritis karena fungsi `create_client_user` menggunakan SECURITY DEFINER: owner fungsi harus migrator non-superuser, tidak memiliki role membership lain, dan fungsi harus memakai `search_path=pg_catalog`. Grant aplikasi gagal tertutup jika ownership atau konfigurasi ini tidak cocok.

Aplikasi hanya mendapat USAGE schema, EXECUTE fungsi pembuatan akun, INSERT audit event, dan EXECUTE validator metadata. Tidak ada akses langsung ke tabel akun, verifikasi, sesi, kontak recovery, kasus recovery, token reset, atau SELECT audit. Backup menggunakan `pg_read_all_data` tanpa BYPASSRLS; monitoring menggunakan `pg_monitor`; keduanya memakai login dan credential terpisah. Audit reader NOLOGIN belum diberikan ke Admin mana pun.

Skrip dan runbook: [Database Principal Bootstrap](../security/database-principal-bootstrap.md). Bootstrap belum dijalankan pada database produksi.
