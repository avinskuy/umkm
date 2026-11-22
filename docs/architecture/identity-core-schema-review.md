# Tinjauan Teknis Skema Identity Core

> Cakupan: hasil review skema, migration, privilege, dan kontrak runtime Identity Core.
> Status implementasi harus dibaca bersama hasil CI dan kondisi deployment; migration yang tersedia belum berarti telah diterapkan pada database produksi.

Dokumen ini mencatat tinjauan desain dan status implementasi Identity Core. Migration 0001–0014 serta skrip bootstrap principal dan grant tersedia di repository dan diuji melalui CI. Migration dan bootstrap tidak dijalankan otomatis oleh HTTP runtime; penerapan pada database produksi merupakan operasi terpisah.

Registrasi publik menggunakan email + password dengan OTP email atau nomor telepon/WhatsApp + password dengan OTP WhatsApp. Login rutin menggunakan email atau nomor telepon + password tanpa OTP setiap kali login. Migration 0001–0014 membentuk baseline skema saat ini; seluruh migration yang sudah diterapkan pada database bersama harus diperlakukan immutable. Lihat [Keputusan Autentikasi Identity Core — Password dan OTP](identity-core-authentication-stage-one-decision.md).

## 1. Hasil pemeriksaan repository

- Backend menggunakan Rust, Axum, Tokio, dan SQLx 0.8 dengan PostgreSQL 16 sebagai baseline.
- Konfigurasi database, connection pool, serta SQLx migrator sudah tersedia.
- `backend/migrations/` memiliki migration Identity Core 0001–0014, termasuk penghapusan OAuth, operasi database verifikasi, binding ID runtime, audit runtime verifikasi, profil publik, dan penerimaan kebijakan.
- HTTP server menyediakan `GET /api/v1/health` dan `GET /api/v1/readyz`. Pool PostgreSQL diinisialisasi saat startup menggunakan principal aplikasi.
- Tes integrasi PostgreSQL memeriksa invariant akun, role immutable, reservasi kontak, autentikasi password-only, sesi verifikasi, challenge OTP, event fingerprint, lock, sesi autentikasi, rotasi token, expiry, pencabutan, serta audit event append-only dan validasi metadata.
- Migration 0001–0014 diuji pada PostgreSQL sementara melalui CI. Pengujian principal/privilege menambahkan identitas migrator non-superuser dan login aplikasi terpisah. Tidak ada migration yang dijalankan pada database produksi.
- `VerificationService` dan router HTTP registrasi/resend/verifikasi tersedia; router belum dipasang pada aplikasi HTTP default. Login password Owner/Investor, validasi sesi, refresh/rotasi token, logout, dan logout semua perangkat tersedia pada endpoint identitas dengan cookie aman dan rate limit PostgreSQL. Penerapan middleware sesi pada endpoint fitur, adapter delivery OTP produksi, integrasi KMS, pemasangan router registrasi, dan runtime recovery belum tersedia. Key ring environment dan guard abuse PostgreSQL sudah tersedia. Grant database minimal tidak berarti seluruh autentikasi telah aktif.

## 2. Prinsip desain yang harus dipertahankan

1. PostgreSQL menjadi sumber kebenaran untuk identitas, role, status akun, verifikasi, dan sesi.
2. Backend tetap menjadi otoritas validasi dan otorisasi; frontend tidak menjadi batas keamanan.
3. Gunakan UUID, `timestamptz`, foreign key eksplisit, constraint, dan indeks sesuai kebutuhan query nyata.
4. Role utama hanya satu: `business_owner`, `investor`, atau `admin`. Role tidak dapat berubah setelah akun dibuat. Admin hanya dibuat melalui provisioning internal.
5. Kontak utama email dan nomor HP harus dinormalisasi sebelum penyimpanan dan unik secara global pada akun yang masih memegang reservasi identitas.
6. Kontak pemulihan berbeda dari kontak utama: kontak pemulihan dapat dipakai bersama oleh beberapa akun, tetapi relasi duplikat pada akun yang sama ditolak.
7. Rahasia OTP, token sesi, token reset, dan kata sandi tidak boleh disimpan dalam bentuk plaintext.
8. Perubahan multi-baris yang berkaitan harus atomik. Constraint database menjadi pertahanan terakhir terhadap race condition.
9. Status siklus hidup tidak boleh dicampur dengan kondisi lock sementara.
10. Migration yang telah diterapkan pada database bersama tidak boleh diedit.

## 3. Batas domain dan urutan migration yang direkomendasikan

Jangan membuat satu migration besar yang sekaligus membangun semua fitur akun. Gunakan urutan kecil yang bisa diuji terpisah:

| Urutan | Cakupan | Isi utama |
| --- | --- | --- |
| 0001 | Identitas akun | `users`, role/status kanonis, kontak utama, aturan reservasi, dan perlindungan role immutable |
| 0002 | Verifikasi kontak | `verification_sessions`, `verification_challenges`, event pengiriman dan percobaan verifikasi persisten, serta indeks pencarian/rate limit; skema dan fungsi database digunakan oleh `VerificationService` untuk registrasi OTP |
| 0003 | Sesi autentikasi | `sessions`, hash token SHA-256 unik, idle timeout 7 hari, absolute expiry 30 hari, rotasi token, dan pencabutan server-side; skema, operasi pembuatan/pencabutan sesi, middleware validasi, pembaruan idle timeout, rotasi token, logout semua perangkat, dan tes runtime tersedia pada endpoint identitas |
| 0004 | Pemulihan akun | `recovery_contacts`, `account_recovery_cases`, dan `password_reset_tokens` |
| 0005 | Audit Identity Core | audit event yang dibutuhkan untuk perubahan identitas, sesi, verifikasi, dan pemulihan |

Urutan ini adalah pemisahan implementasi, bukan keharusan membuat semua tabel pada hari yang sama. Setiap migration harus memiliki tes PostgreSQL yang fokus pada invariant yang diperkenalkannya. Outbox pengiriman, notifikasi, account deletion, data retention, dan data export tidak dimasukkan secara otomatis ke migration Identity Core pertama; tambahkan saat kontrak domain dan kebutuhan alurnya siap.

## 4. Rekomendasi struktur entitas dan invariant

### 4.1 `users`

Tanggung jawab: identitas akun, role permanen, status siklus hidup, kontak utama, dan referensi kredensial lokal.

Kolom konseptual yang diperlukan:

- `id`: UUID primary key.
- `primary_role`: constrained value `business_owner`, `investor`, `admin`.
- `account_status`: constrained lifecycle value `pending_verification`, `active`, `deactivated`, `deletion_pending`, `deletion_processing`, `pending_expired`, dan `closed`. Suspend/limit fitur dicatat terpisah melalui moderation/enforcement records.
- `email_normalized` dan `phone_e164`: nullable secara individual; akun klien memilih satu identifier awal. Kontak awal baru dianggap terverifikasi setelah OTP email/WhatsApp valid.
- `initial_contact_channel`: mencatat identifier awal (email atau phone) yang dipilih saat pendaftaran; akun hanya aktif setelah kontak awal berhasil diverifikasi melalui OTP.
- `email_verified_at` dan `phone_verified_at`: waktu verifikasi yang sesuai; jangan menyimpulkan kontak kedua telah diverifikasi.
- `password_hash`: wajib untuk akun klien aktif maupun pending; akun terminal yang telah ditutup atau kedaluwarsa mengikuti aturan pembersihan kredensial.
- `pending_expires_at`: masih bagian dari migration saat ini untuk lifecycle pending lama; tidak digunakan sebagai syarat registrasi tahap pertama dan perlu ditinjau pada migration lanjutan.
- `created_at`, `updated_at`, dan timestamp lifecycle yang memang diperlukan.

Invariant database:

- Unique partial index pada kontak utama ternormalisasi yang tidak null, selama kontak masih direservasi.
- Role dibatasi oleh CHECK constraint dan tidak dapat diubah setelah INSERT. Terapkan perlindungan database yang sempit dan teruji, bukan memberi hak UPDATE role secara luas kepada akun aplikasi.
- Akun klien wajib memiliki password lokal. Akun Admin dibuat melalui provisioning internal dan mengikuti aturan kontak/MFA Admin tersendiri.
- Status akun tidak boleh menjadi jalan pintas untuk melepas kontak. Kontak utama tetap direservasi selama masa tunggu penghapusan dan baru dilepas setelah penghapusan benar-benar selesai sesuai kebijakan retensi.
- Cleanup untuk status pending lama tetap diperlukan bila masih ada record yang memakai lifecycle tersebut; jangan menerapkan kedaluwarsa 30 menit pada registrasi tahap pertama yang langsung aktif.
- Jika baris pending dipertahankan untuk audit setelah kedaluwarsa, kontak yang dilepas harus dihapus dari kolom identitas aktif atau dipindahkan ke representasi audit yang tidak dapat digunakan untuk login. Jangan membiarkan baris expired tetap memblokir registrasi tanpa batas.

### 4.2 Autentikasi lokal

- Registrasi publik hanya menggunakan email atau nomor telepon/WhatsApp bersama password lokal.
- Tidak ada tabel identitas OAuth atau provider eksternal pada skema aktif.
- Password hash dibuat di backend dengan Argon2id; password plaintext tidak pernah dikirim ke fungsi database.

### 4.3 `verification_sessions`

Sesi verifikasi adalah induk dari satu atau beberapa challenge OTP berurutan. Batas percobaan dan lock melekat pada sesi, bukan pada challenge.

Kolom konseptual:

- `id`, `user_id`, `purpose`, `channel`, `destination_hash`, status lifecycle, `failed_attempt_count`, `locked_until`, `resend_available_at`, `created_at`, `expires_at`, dan timestamp penyelesaian/pembatalan.
- Data tujuan yang diperlukan untuk pengiriman ulang harus disimpan dengan perlindungan enkripsi yang sesuai. Hash/HMAC tujuan digunakan untuk pencarian dan pembatasan lintas sesi; hash tidak dapat menggantikan nilai tujuan yang dibutuhkan provider untuk pengiriman.
- `user_id` wajib untuk registrasi dan verifikasi yang sudah terkait ke akun. Nilai NULL hanya diizinkan pada alur pemulihan sebelum akun tujuan dipilih.
- CHECK constraint harus mengikat nullable `user_id` pada purpose pemulihan yang tepat; jangan mengizinkan sesi registrasi tanpa akun.
- Lifecycle awal: `pending`, `verified`, `expired`, `cancelled`. Lock sementara dicatat terpisah melalui `failed_attempt_count` dan `locked_until`; jangan menambah lifecycle status `locked`.
- Setelah lock satu jam berakhir, reset penghitung dilakukan secara atomik saat sesi pending digunakan kembali. Penerbitan OTP baru tidak mengubah lock.
- Resend menggunakan sesi yang sama. Sesi baru tidak boleh menjadi cara untuk menghindari limit lintas sesi.

### 4.4 `verification_challenges`

- Challenge memiliki FK wajib ke `verification_sessions`; simpan `code_hash` berbasis keyed hash, status, `expires_at`, dan timestamp consumed/superseded/cancelled.
- Status: `pending`, `consumed`, `expired`, `superseded`, `cancelled`. Hanya pending yang dapat diverifikasi, selama sesi induk mengizinkan.
- Hanya satu challenge pending per sesi pada satu waktu; gunakan partial unique index.
- Kode berlaku 10 menit, maksimal lima kegagalan per sesi, dan hanya dapat digunakan sekali melalui transaksi/operasi atomik.
- Pengiriman kode baru menandai challenge pending sebelumnya sebagai superseded.
- **Jangan menambahkan kolom `user_id` pada challenge jika hanya menduplikasi relasi sesi induk.** Jika kelak ada kebutuhan query yang terukur untuk menyimpan salinan user_id, constraint harus memastikan nilainya selalu cocok dengan user_id sesi induk. Challenge untuk alur pemulihan pra-pemilihan akun tidak boleh memaksa user_id terisi.
- Jumlah digit OTP tetap keputusan konfigurasi implementasi; jangan mengarang nilai di migration.

### 4.5 Batas pengiriman dan abuse control lintas sesi

Batas 60 detik per resend dan lima pengiriman per kontak per jam tidak boleh hanya disimpan pada satu verification session. Satu kontak dapat memulai banyak sesi, sehingga limit harus dihitung lintas sesi dan instance aplikasi.

Rekomendasi:

- Gunakan tabel/penyimpanan rate-limit persisten dengan key berbasis HMAC dari kanal dan tujuan yang dinormalisasi, serta jendela waktu yang eksplisit.
- Perubahan penghitung harus atomik (misalnya row-level locking atau operasi UPSERT yang benar) agar permintaan paralel tidak melampaui batas.
- Tetapkan juga batas per IP/network, akun, purpose, dan batas global provider. Data IP harus diminimalkan dan memiliki retensi terbatas.
- Reset lock sesi tidak boleh mereset penghitung pengiriman per kontak.
- Jangan mengandalkan cache proses lokal untuk kontrol yang harus berlaku lintas instance atau setelah restart.

Kontrak event pengiriman dan indeks pencarian telah tersedia pada migration 0002. Batas lima pengiriman per kontak dalam 60 menit dan cooldown 60 detik tetap harus ditegakkan oleh runtime melalui transaksi PostgreSQL yang diserialisasi berdasarkan fingerprint kontak/kanal. Rotasi HMAC harus mempertahankan penghitungan lintas versi agar perubahan key tidak membuka bypass limit.

### 4.6 `sessions`

- Simpan hash SHA-256 32-byte yang unik secara global, bukan token asli; setiap sesi wajib terkait ke satu user aktif.
- Database menetapkan `created_at`, `last_seen_at`, `token_rotated_at`, dan `expires_at` (30 hari sejak pembuatan). Backend wajib menolak sesi dengan aktivitas terakhir lebih dari 7 hari.
- Rotasi hash token mengganti token lama secara atomik; timestamp rotasi dikelola database.
- `remember_device` hanya memengaruhi persistensi cookie browser dan tidak mengubah expiry.
- Mendukung pencabutan individual, logout semua perangkat, reset password, dan pencabutan karena peristiwa keamanan. Revocation tidak dapat dibatalkan atau ditulis ulang.
- Perubahan status lifecycle akun ke status nonaktif otomatis mencabut sesi yang masih aktif. Restriksi moderasi/suspensi dan sinyal risiko lain harus memicu pencabutan oleh runtime.
- Metadata perangkat dibatasi pada ringkasan user-agent opsional dan hash prefix jaringan opsional; jangan menyimpan alamat IP mentah atau fingerprint lengkap.
- Penutupan browser tidak otomatis mengakhiri sesi yang memang dibuat persisten.

### 4.7 `recovery_contacts`

- Kontak pemulihan bersifat opsional, diverifikasi terpisah, dan terhubung melalui FK ke akun.
- Kontak email atau nomor HP yang sama boleh memiliki relasi pada beberapa akun.
- **Tidak boleh ada unique constraint global pada `contact_hash` atau alamat kontak.**
- Gunakan unique constraint komposit (`user_id`, `channel`, `contact_hash`) agar satu akun tidak menyimpan relasi duplikat.
- Gunakan indeks non-unik untuk pencarian lintas akun setelah verifikasi kontak. Keberhasilan verifikasi hanya membuktikan kendali kontak, bukan kepemilikan seluruh akun terkait.
- Perubahan/penghapusan relasi harus memicu audit dan notifikasi yang sesuai; jangan mengirim informasi akun lain ke kontak yang dipakai bersama.

### 4.8 `account_recovery_cases`

- Setiap kasus memiliki satu `verification_session_id` wajib dengan unique constraint; satu sesi memiliki maksimal satu kasus.
- `target_user_id` nullable hanya sebelum target dipilih. Kasus pra-pemilihan tetap tercatat dan dapat masuk antrean pemeriksaan tanpa mengungkap akun.
- Setelah akun dipilih dan pemeriksaan yang diwajibkan selesai, target dikaitkan atomik dan hanya sekali.
- OTP resend menggunakan sesi dan kasus yang sama; percobaan pemulihan baru membuat sesi dan kasus baru.
- Simpan status, tujuan recovery, hasil/ringkasan penilaian risiko, assignment petugas khusus bila diperlukan, dan referensi audit. Jangan simpan bukti mentah atau catatan investigasi sensitif di log umum.
- Verifikasi kontak bersama bukan bukti tunggal kepemilikan akun. Pemilihan akun dan pemeriksaan risiko tetap wajib.

### 4.9 `password_reset_tokens` dan audit

- Token reset disimpan sebagai hash unik, memiliki expiry dan waktu konsumsi, dan hanya dapat dipakai sekali.
- Jangan gunakan OTP pendaftaran sebagai token reset kata sandi.
- Audit event untuk perubahan role (upaya ditolak), status akun, kontak utama/pemulihan, sesi, verifikasi, recovery, dan tindakan Admin yang relevan.
- Audit harus append-only pada batas aplikasi dan tidak memuat password, OTP, token, isi chat privat, atau bukti pemulihan mentah.

## 5. Konsistensi state dan transaksi yang wajib diuji

| Operasi | Sifat atomik yang dibutuhkan |
| --- | --- |
| Reservasi kontak saat registrasi | Dua pendaftaran bersamaan untuk kontak yang sama: hanya satu berhasil memperoleh reservasi |
| Aktivasi akun | Challenge dikonsumsi, kontak ditandai verified, dan status akun berubah menjadi active dalam satu transaksi |
| Cleanup pending | Hanya akun pending yang benar-benar kedaluwarsa yang dapat dibersihkan; tidak boleh menghapus akun yang aktif |
| Penerbitan OTP baru | Challenge lama menjadi superseded dan challenge baru menjadi satu-satunya pending |
| Verifikasi OTP | Konsumsi challenge dan pembaruan sesi tidak dapat terjadi dua kali |
| Percobaan gagal ke-5 | Lock sesi satu jam tercatat konsisten meskipun permintaan paralel |
| Setelah lock berakhir | Penghitung sesi reset satu kali secara atomik; limit kontak lintas sesi tidak direset |
| Recovery contact bersama | Kontak yang sama dapat terkait ke dua akun, tetapi duplikat pada akun yang sama ditolak |
| Pemilihan target recovery | Target hanya dapat ditetapkan satu kali; kasus tetap terhubung ke satu sesi |
| Pencabutan sesi | Sesi yang dicabut tidak dapat digunakan kembali walau token belum mencapai expiry |

Tes concurrency harus menggunakan beberapa koneksi PostgreSQL, bukan hanya unit test atau mock.

## 6. Penyelesaian penghalang teknis

Lima penghalang teknis dari tinjauan awal telah memiliki baseline keputusan dalam [Keputusan Teknis Identity Core](identity-core-technical-decisions.md):

1. **Perlindungan tujuan OTP:** ciphertext AEAD untuk kebutuhan pengiriman ulang, HMAC terpisah untuk pencarian dan pembatasan, key di luar database, serta rotasi versi secara terencana.
2. **Rate limit lintas sesi:** event pengiriman persisten di PostgreSQL, jendela bergulir 60 menit, cooldown 60 detik, dan serialisasi transaksi per fingerprint tujuan agar berlaku lintas koneksi/instance.
3. **Status akun final:** `pending_verification`, `active`, `deactivated`, `deletion_pending`, `deletion_processing`, `pending_expired`, dan `closed`; suspend/limit fitur dicatat terpisah sebagai enforcement agar tidak bertabrakan dengan lifecycle penghapusan. Reservasi kontak hanya dilepas setelah transisi terminal yang sah.
4. **Provisioning Admin:** jalur internal terpisah, principal/kredensial provisioning terpisah dari runtime API publik, role immutable, audit, dan bootstrap MFA sebelum akses istimewa.
5. **Audit boundary:** tabel audit khusus, principal aplikasi insert-only, pemilik tabel terpisah, metadata allowlist, dan larangan menyimpan rahasia atau isi privat.

Keputusan tersebut menetapkan arah desain, bukan mengklaim seluruh mekanisme sudah diimplementasikan. Library AEAD/KMS, provider OTP, detail cleanup, MFA Admin, dan retensi numerik tetap diverifikasi pada tahap implementasi terkait.

## 7. Rencana pengujian sebelum migration dianggap siap

### Migration dan constraint

- Migration berjalan pada PostgreSQL 16 kosong.
- SQLx migrator dapat dijalankan ulang tanpa mengubah hasil skema.
- Unique partial index menolak kontak utama duplikat.
- Kontak pemulihan bersama lintas akun diterima; duplikat dalam akun yang sama ditolak.
- Role check menolak nilai tidak dikenal; percobaan mengubah role setelah insert ditolak database.
- Foreign key dan CHECK constraint menolak relasi/status yang tidak valid.
- Partial unique index hanya mengizinkan satu challenge pending per verification session.
- Kasus recovery wajib memiliki session unik; target boleh NULL sebelum pemilihan.

### Konkurensi dan lifecycle

- Reservasi kontak diuji dengan transaksi paralel.
- Challenge tidak dapat dikonsumsi dua kali.
- Dua permintaan verifikasi paralel tidak dapat melewati batas lima percobaan.
- Resend bersamaan tidak menghasilkan dua challenge pending.
- Batas pengiriman per kontak berlaku lintas sesi dan lintas koneksi.
- Unlock/reset counter tidak berjalan sebelum waktunya dan hanya terjadi satu kali setelah lock berakhir.
- Cleanup pending tidak menyentuh akun aktif.
- Sesi revoked/expired tidak dapat dipakai kembali.

### Keamanan dan privasi

- Tidak ada OTP/token/password plaintext di database atau log.
- Tujuan pengiriman tidak dapat dibaca tanpa akses key yang sah.
- Error registrasi/recovery tidak mengungkap keberadaan akun.
- Pemilihan akun setelah verifikasi tidak membocorkan daftar akun atau data privat lain.
- Audit tidak menyimpan rahasia atau konten privat yang tidak diperlukan.

Semua pengujian database harus memakai database PostgreSQL sementara yang dapat dibuang, dengan kredensial khusus test. Jangan menghubungkan tes destruktif ke database produksi.

## 8. Gate untuk pekerjaan berikutnya

Migration akun Identity Core pertama sudah diimplementasikan dan gerbang berikut telah terpenuhi:

- [x] Lima penghalang teknis memiliki baseline keputusan dan batas implementasi.
- [x] Kontrak kolom, tipe PostgreSQL, constraint, partial unique index, dan foreign key untuk slice akun telah diterapkan.
- [x] Migration 0006 menghapus tabel identitas provider yang tidak digunakan; skema aktif hanya mendukung password lokal.
- [x] Tes PostgreSQL untuk constraint, role immutability, kontak, metode autentikasi, provider identity, dan concurrency berhasil di CI.
- [x] Migration pertama dibatasi pada akun dan identitas provider; verifikasi, sesi, recovery, dan audit tidak ikut dicampur.

Migration 0001–0014 dan bootstrap principal/privilege telah diuji melalui CI pada PostgreSQL sementara. Runtime menyediakan login password Owner/Investor, validasi sesi pada endpoint identitas, pembaruan idle timeout, rotasi token, logout, dan logout semua perangkat. Router registrasi publik, adapter delivery produksi, kebijakan password registrasi, penerapan middleware pada endpoint fitur, recovery runtime, dan cleanup job masih memerlukan implementasi serta pengujian tersendiri. Migration dan bootstrap belum diterapkan ke database produksi; jangan menjalankan migration otomatis pada HTTP startup.

## 9. Referensi internal

- [Kontrak Skema Migration 0001 — Identitas Akun](identity-core-migration-0001-contract.md)
- [Kontrak Skema Migration 0002 — Verifikasi Kontak dan OTP](identity-core-migration-0002-verification-contract.md)
- [Kontrak Skema Migration 0003 — Sesi Autentikasi](identity-core-migration-0003-session-contract.md)
- [Kontrak Skema Migration 0004 — Pemulihan Akun](identity-core-migration-0004-account-recovery-contract.md)
- [Kontrak Skema Migration 0005 — Identity Audit Events](identity-core-migration-0005-audit-events-contract.md)
- [Rancangan Skema Database](database-schema-design.md)
- [Keputusan Registrasi Identity Core](identity-core-registration-decisions.md)
- [Tinjauan Skema Verifikasi dan Pemulihan](identity-core-shared-recovery-contacts.md)
- [Desain Arsitektur Autentikasi dan Keamanan Identitas](authentication-and-identity-security-design.md)
- [Kebijakan Keamanan Database dan Integritas Data](../security/database-security-data-integrity.md)
- [Strategi Pengujian dan Quality Assurance](../engineering/testing-strategy-quality-assurance.md)

## 10. Status provisioning principal PostgreSQL

Bootstrap principal dipisahkan dari schema migration karena role PostgreSQL berlaku pada tingkat cluster/database. Skrip `backend/database/bootstrap/001_create_principals.sql` membuat login aplikasi, migrator, backup, dan monitoring beserta role group; PUBLIC kehilangan CONNECT/TEMPORARY dan akses schema `public`. Skrip `002_identity_core_grants.sql` hanya memberikan hak pembuatan akun melalui fungsi `SECURITY DEFINER` yang ownership, privilege pemilik, dan `search_path`-nya telah diverifikasi; aplikasi mendapat INSERT audit tanpa hak SELECT/UPDATE/DELETE/TRUNCATE.

CI menjalankan SQLx migrations sebagai `usahapintar_migrator` non-superuser, lalu menguji fungsi/grant melalui `usahapintar_app_login`. Backup dan monitoring diuji sebagai principal terpisah. Skrip tidak dijalankan pada production; kredensial production dan provisioning DBA tetap di luar repository.
