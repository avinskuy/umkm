# Kontrak Skema Migration 0001 — Identitas Akun

> Status: diimplementasikan sebagai migration SQLx; dokumen ini menjadi kontrak dan referensi tes
> Tanggal: 21 November 2026
> Basis: keputusan teknis Identity Core yang telah disetujui sebagai baseline
> Cakupan: akun, role permanen, kontak utama, dan identitas Google

Dokumen ini menetapkan kontrak skema Identity Core slice pertama. Implementasi SQL tersedia di `backend/migrations/20261121170000_create_identity_accounts.sql` dan diuji melalui `backend/tests/postgres.rs`. Migration belum dijalankan pada database produksi dan belum mengaktifkan runtime registrasi/login.

Istilah **migration 0001** di dokumen ini adalah nama slice implementasi. SQLx menggunakan nama file migration berversi berbasis timestamp, bukan nomor urut literal `0001`; nama file aktual akan ditentukan saat migration dibuat.

## 1. Batas migration pertama

Migration pertama hanya membangun:

1. `users` sebagai sumber kebenaran akun, role utama, lifecycle, kontak utama, dan referensi kredensial lokal.
2. `user_identities` untuk identitas provider eksternal yang stabil, dimulai dari Google OAuth. Tabel ini masuk slice pertama karena Google OAuth sudah menjadi metode registrasi/login yang disetujui dan stable provider subject harus dilindungi keunikannya sejak akun pertama dibuat.

Migration pertama **tidak** membangun:

- `verification_sessions` atau `verification_challenges`;
- `sessions` autentikasi;
- `recovery_contacts`, `account_recovery_cases`, atau token reset;
- tabel event rate limit OTP;
- audit Identity Core;
- profil Owner/Investor, bisnis, post, koneksi, chat, membership, notifikasi, deletion request, atau data retention.

Migration ini tidak mengaktifkan koneksi database pada HTTP startup, tidak menjalankan migration terhadap database produksi, dan tidak membuat endpoint registrasi/login.

## 2. Tabel `users`

### 2.1 Kolom

| Kolom | Tipe PostgreSQL | Null | Default / aturan |
| --- | --- | --- | --- |
| `id` | `uuid` | Tidak | Primary key; `gen_random_uuid()` |
| `primary_role` | `text` | Tidak | Salah satu `business_owner`, `investor`, `admin` |
| `account_status` | `text` | Tidak | Default `pending_verification`; dibatasi CHECK |
| `email_normalized` | `text` | Ya | Email yang telah dinormalisasi server |
| `phone_e164` | `text` | Ya | Nomor telepon dalam format E.164 |
| `initial_contact_channel` | `text` | Tidak | `email` atau `phone`; ditetapkan saat pendaftaran |
| `email_verified_at` | `timestamptz` | Ya | Diisi hanya setelah email tersebut berhasil diverifikasi |
| `phone_verified_at` | `timestamptz` | Ya | Diisi hanya setelah nomor tersebut berhasil diverifikasi |
| `initial_contact_verified_at` | `timestamptz` | Ya | Waktu kontak awal pendaftaran berhasil diverifikasi; provenance yang tidak berubah saat kontak utama kelak diganti |
| `password_hash` | `text` | Ya | Hash password lokal; bukan plaintext |
| `pending_expires_at` | `timestamptz` | Ya | Wajib selama `pending_verification`; target 30 menit dari pembuatan |
| `created_at` | `timestamptz` | Tidak | `now()` |
| `updated_at` | `timestamptz` | Tidak | `now()`; diperbarui backend pada perubahan |
| `status_changed_at` | `timestamptz` | Tidak | `now()`; diperbarui pada transisi lifecycle |

Tidak ada kolom nama tampilan, bio, foto profil, preferensi, data bisnis, atau atribut profil pada tabel ini. Data profil ditempatkan di tabel domain profil pada migration berikutnya.

### 2.2 Role utama

Nilai yang diterima:
- `business_owner`
- `investor`
- `admin`

Ketentuan:
- Satu akun hanya memiliki satu role utama.
- Role ditetapkan pada pembuatan akun dan tidak dapat diubah sesudah INSERT.
- Public registration hanya dapat membuat `business_owner` atau `investor`.
- `admin` hanya dibuat melalui jalur provisioning internal dengan principal/workflow terpisah.
- Tidak ada endpoint, dukungan operasional biasa, OAuth linking, atau proses recovery yang dapat memindahkan role.
- Tambahkan trigger database yang menolak perubahan `primary_role` dan `initial_contact_channel` pada UPDATE sebagai defense in depth. `initial_contact_verified_at` hanya boleh diisi sekali oleh operasi aktivasi tepercaya setelah verifikasi berhasil dan tidak boleh diubah setelah terisi.
- Hak akses database harus mencegah runtime principal membuat role Admin dengan mengirim nilai role arbitrer. Gunakan jalur pembuatan akun klien yang hanya menerima dua role klien dan jalur provisioning Admin yang terpisah. Jangan memberi runtime principal hak INSERT generik pada tabel yang memungkinkan pemilihan role bebas.
- Fungsi database `SECURITY DEFINER` (jika dipakai) harus dimiliki role non-login, memakai `search_path` tetap, tidak memakai SQL dinamis, dan hak EXECUTE publik dicabut. Nama role database serta GRANT aktual ditangani pada bootstrap lingkungan, bukan ditebak dalam migration.

### 2.3 Lifecycle akun

Nilai `account_status` yang diizinkan:

| Status | Makna |
| --- | --- |
| `pending_verification` | Registrasi menunggu verifikasi kontak awal; batas 30 menit. |
| `active` | Akun aktif. |
| `deactivated` | Penonaktifan sementara oleh pengguna; data dan kontak utama tetap dipertahankan. |
| `deletion_pending` | Permintaan penghapusan dalam masa tunggu pembatalan 30 hari; kontak tetap direservasi. |
| `deletion_processing` | Penghapusan sedang diproses; kontak tetap direservasi sampai penghapusan selesai. |
| `pending_expired` | Registrasi tidak selesai dalam 30 menit; kontak identitas aktif telah dilepas. |
| `closed` | Penghapusan permanen selesai dan kontak utama telah dilepas sesuai proses retensi. |

Status suspend atau pembatasan fitur oleh Admin **bukan** nilai `account_status`. Kondisi tersebut direpresentasikan oleh moderation/enforcement records terpisah. Dengan demikian, akun yang dibatasi tetap dapat memiliki lifecycle `active` atau `deletion_pending` tanpa menggabungkan dua state machine.

Status `deletion_pending` dan `deletion_processing` disediakan sebagai vocabulary lifecycle. Transisi penghapusan belum boleh diaktifkan sebelum tabel permintaan penghapusan, retensi, dan proses terkait tersedia pada migration domain tersendiri.

### 2.4 CHECK constraint

Kontrak constraint minimum:

1. `primary_role` hanya menerima tiga nilai role kanonis.
2. `account_status` hanya menerima nilai lifecycle pada Bagian 2.3.
3. `initial_contact_channel` hanya menerima `email` atau `phone`.
4. `initial_contact_channel` immutable setelah INSERT karena mencatat metode registrasi awal, bukan kanal kontak yang harus dipertahankan selamanya. `initial_contact_verified_at` hanya boleh berubah dari NULL menjadi timestamp ketika aktivasi yang sah terjadi, lalu immutable.
5. Selama `pending_verification`, kontak sesuai `initial_contact_channel` wajib tersedia; `pending_expires_at` wajib terisi; `initial_contact_verified_at` dan timestamp verifikasi kontak awal harus NULL.
6. Untuk status pasca-verifikasi (`active`, `deactivated`, `deletion_pending`, `deletion_processing`), `initial_contact_verified_at` wajib terisi dan setidaknya satu kontak utama saat ini harus tersedia serta terverifikasi.
7. Setiap kontak utama saat ini yang disimpan pada status pasca-verifikasi harus memiliki timestamp verifikasi yang sesuai. Kontak baru tidak boleh ditulis ke kolom identitas aktif sebelum diverifikasi.
8. Timestamp verifikasi email hanya boleh terisi bila `email_normalized` tersedia; timestamp verifikasi telepon hanya boleh terisi bila `phone_e164` tersedia.
9. Selama `pending_verification`, kontak non-awal tidak boleh terisi. Kontak kedua dapat ditambahkan setelah aktivasi melalui alur verifikasi terpisah.
10. `pending_expires_at` harus NULL setelah akun keluar dari `pending_verification`; transisi expired dicatat melalui event/audit pada tahap terkait.
11. Pada `pending_expired` dan `closed`, kolom kontak utama, timestamp verifikasi kontak, `initial_contact_verified_at`, dan `password_hash` harus NULL. Identitas provider yang tidak lagi diperlukan harus dilepas melalui proses terminal yang sesuai.
12. `password_hash`, bila terisi, tidak boleh berupa string kosong.

Constraint lintas tabel yang tidak dapat diekspresikan dengan aman sebagai CHECK harus dijaga oleh operasi backend/transaksi dan tes integrasi. Khusus akun klien: `password_hash` boleh NULL hanya jika identitas Google yang sah dibuat/ditautkan dalam transaksi pembuatan akun yang sama; registrasi email/telepon wajib memiliki password hash. Jangan membuat CHECK yang membaca tabel lain. Aturan kredensial Admin mengikuti jalur provisioning internal dan harus dipastikan sebelum akun Admin diaktifkan.

### 2.5 Keunikan dan reservasi kontak

Gunakan unique partial index terpisah:

- Email: `email_normalized` unik ketika tidak NULL dan `account_status` termasuk `pending_verification`, `active`, `deactivated`, `deletion_pending`, atau `deletion_processing`.
- Telepon: `phone_e164` unik ketika tidak NULL dan `account_status` termasuk status reservasi yang sama.

Aturan:
- Normalisasi dilakukan backend sebelum insert/update. Database tetap menjadi penjaga keunikan terakhir.
- Email dan nomor telepon adalah dua kolom terpisah. Pendaftaran hanya mewajibkan satu kontak awal; kontak kedua dapat ditambahkan dan diverifikasi kemudian.
- Kontak utama tetap direservasi ketika akun dinonaktifkan, disuspend melalui enforcement, atau berada dalam masa tunggu/proses penghapusan.
- Transisi ke `pending_expired` hanya boleh melepas reservasi setelah pembersihan nilai kontak dilakukan atomik.
- Transisi ke `closed` hanya boleh melepas reservasi setelah proses penghapusan benar-benar selesai; berlalunya 30 hari saja tidak cukup.
- Jangan mengandalkan soft-delete flag atau validasi frontend sebagai pengganti partial unique index.

### 2.6 Password dan verifikasi

- `password_hash` menyimpan encoded hash hasil password-hashing library yang dipilih kemudian. Parameter dan library final bukan bagian dari migration.
- Registrasi email/telepon wajib menyediakan password lokal.
- Registrasi Google OAuth tidak mewajibkan password lokal, tetapi tetap wajib memverifikasi kontak awal yang dipilih secara terpisah sebelum aktivasi.
- OTP pendaftaran bukan password, bukan login credential, dan bukan token reset password.
- `email_verified_at` dan `phone_verified_at` adalah fakta verifikasi untuk nilai kontak saat ini; ketika kontak diganti, timestamp lama tidak boleh dipakai untuk menyatakan nilai baru telah terverifikasi.
- `initial_contact_channel` dan `initial_contact_verified_at` menyimpan provenance metode registrasi awal. Keduanya tidak berubah saat pengguna mengganti atau menghapus salah satu kontak utama setelah akun aktif.
- Perubahan kontak harus memverifikasi nilai baru sebelum mengganti nilai aktif; jangan menaruh kontak baru yang belum terverifikasi langsung pada kolom identitas utama.
- Verifikasi OTP, masa berlaku 10 menit, batas percobaan, lock, resend, dan rate limit disediakan pada migration verifikasi berikutnya. Migration pertama tidak membuat kolom OTP.

## 3. Tabel `user_identities`

### 3.1 Kolom

| Kolom | Tipe PostgreSQL | Null | Aturan |
| --- | --- | --- | --- |
| `id` | `uuid` | Tidak | Primary key; `gen_random_uuid()` |
| `user_id` | `uuid` | Tidak | FK ke `users(id)`, ON DELETE RESTRICT |
| `provider` | `text` | Tidak | Untuk slice awal dibatasi ke `google` |
| `provider_subject` | `text` | Tidak | Stable subject dari provider; bukan alamat email |
| `created_at` | `timestamptz` | Tidak | `now()` |

Constraint dan indeks:
- Unique (`provider`, `provider_subject`) agar satu identitas provider tidak dapat terhubung ke dua akun.
- CHECK provider hanya `google` untuk migration awal; provider lain ditambahkan melalui migration baru setelah kebutuhan dan validasinya ditinjau.
- CHECK `provider_subject` tidak kosong.
- Index pada `user_id` untuk pencarian identitas milik akun.
- Tidak menyimpan access token, refresh token, ID token, atau email provider di tabel ini. Jika token provider kelak benar-benar diperlukan, desain penyimpanannya harus ditinjau terpisah.
- Jangan auto-link berdasarkan kecocokan email. Linking memerlukan sesi akun yang sudah terautentikasi dan bukti kontrol terbaru atas identitas provider.
- Untuk akun klien tanpa `password_hash`, identitas Google adalah metode autentikasi yang diperlukan. Jangan mengizinkan unlink identitas Google terakhir sebelum akun memiliki kredensial lokal atau metode autentikasi lain yang telah diverifikasi.

Keunikan satu provider subject global tidak berarti satu email provider dapat menggabungkan beberapa akun. Identitas ditentukan oleh stable provider subject.

## 4. Foreign key dan perilaku penghapusan

- `user_identities.user_id` memakai ON DELETE RESTRICT. Penghapusan user row tidak boleh diam-diam menghapus identitas provider atau audit/history yang masih diperlukan.
- Lifecycle akun normal menggunakan status dan proses retensi, bukan hard delete langsung.
- Sebelum akun mencapai `closed`, identitas provider yang tidak lagi diperlukan harus dilepas sesuai proses penghapusan.
- Tidak ada ON DELETE CASCADE dari `users` ke tabel yang akan menjadi sumber audit atau retensi.
- Profil, recovery contact, session, dan domain lain akan menentukan FK masing-masing pada migration terkait.

## 5. Konvensi tipe dan migration

- Gunakan `uuid` dengan `gen_random_uuid()` yang tersedia pada PostgreSQL 16; tidak perlu menambahkan extension hanya untuk generator UUID.
- Gunakan `timestamptz` untuk semua instant.
- Gunakan `text + CHECK` untuk role/status pada baseline awal agar perubahan vocabulary dapat dilakukan melalui migration terkontrol tanpa ketergantungan pada PostgreSQL enum yang sulit diubah.
- Nama migration aktual mengikuti konvensi SQLx berbasis timestamp, misalnya `YYYYMMDDHHMMSS_create_identity_accounts.sql`; contoh ini bukan timestamp yang harus dipakai.
- Migration dijalankan menggunakan migration/database-owner principal yang terpisah dari runtime application principal.
- Jangan membuat role database atau GRANT yang bergantung pada nama akun lingkungan di dalam migration umum. Bootstrap privilege dilakukan secara terkontrol per lingkungan.
- Jangan edit migration setelah diterapkan pada database bersama. Perubahan skema berikutnya harus menggunakan migration baru.

## 6. Operasi yang belum termasuk

| Operasi | Alasan ditunda |
| --- | --- |
| Pendaftaran dan aktivasi akun | Memerlukan verification sessions/challenges, provider, rate limit, dan transaksi aktivasi pada slice berikutnya. |
| Google OAuth callback/linking | Memerlukan validasi state/nonce/token, endpoint, dan pengelolaan rahasia. Tabel identitas hanya menyiapkan relasi. |
| Deaktivasi/penghapusan akun | Memerlukan deletion request, retensi, pembatalan, dan proses penghapusan terpisah. |
| Suspend/limit fitur | Mengikuti moderation/enforcement records dan tidak menjadi status lifecycle. |
| Session login/logout | Memerlukan tabel session dan kebijakan cookie/revocation. |
| Recovery contact dan pemulihan akun | Memerlukan model kontak bersama, sesi pra-pemilihan, recovery case, serta pemeriksaan risiko. |
| Audit Identity Core | Memerlukan tabel audit dan privilege append-only pada migration tersendiri. |

## 7. Tes PostgreSQL wajib sebelum migration dianggap siap

### Struktur dan constraint
- Migration berjalan pada PostgreSQL 16 kosong.
- SQLx migrator mengenali versi dan checksum migration.
- Role tidak dikenal ditolak.
- Lifecycle tidak dikenal ditolak.
- Initial contact channel tidak dikenal ditolak.
- Akun pending tanpa kontak awal atau tanpa expiry ditolak.
- Akun pending yang kontak awalnya sudah ditandai verified atau memiliki `initial_contact_verified_at` ditolak.
- Akun aktif tanpa `initial_contact_verified_at` atau tanpa setidaknya satu kontak utama terverifikasi ditolak.
- Mengganti kontak utama tanpa verifikasi nilai baru ditolak; provenance `initial_contact_channel` tidak dapat diubah.
- Timestamp verified tanpa nilai kontak ditolak.
- Pending expired/closed dengan kontak, verification timestamp, atau password hash masih terisi ditolak.
- Email dan nomor telepon duplikat ditolak pada seluruh status yang masih memegang reservasi.
- Email/nomor yang sama dapat digunakan kembali setelah transisi terminal yang sah melepaskan reservasi.

### Role dan provisioning
- Primary role dan `initial_contact_channel` tidak dapat diubah setelah INSERT; `initial_contact_verified_at` hanya dapat diisi sekali pada aktivasi sah. Percobaan perubahan berikutnya ditolak.
- Jalur registrasi klien hanya dapat membuat Business Owner atau Investor.
- Runtime application principal tidak dapat membuat Admin melalui insert langsung atau parameter role yang dimanipulasi.
- Provisioning Admin hanya dapat dilakukan melalui principal/workflow internal yang terpisah dan tercatat.
- Tidak ada endpoint publik untuk provisioning Admin.

### Provider identity
- Provider subject yang sama tidak dapat terhubung ke dua akun.
- Akun klien tanpa password hash ditolak kecuali identitas Google yang sah dibuat/ditautkan dalam transaksi yang sama.
- Provider yang tidak diizinkan ditolak.
- FK menolak user yang tidak ada.
- Identitas provider tidak dapat dipindahkan otomatis hanya karena email cocok.
- Linking identitas memerlukan otorisasi akun yang ada dan bukti kontrol provider.
- Akun tanpa password hash tidak dapat melepas identitas Google terakhir sebelum metode autentikasi pengganti tersedia.

### Concurrency dan lifecycle
- Dua registrasi paralel untuk email yang sama: hanya satu reservasi berhasil.
- Dua registrasi paralel untuk nomor yang sama: hanya satu reservasi berhasil.
- Cleanup pending yang bersamaan dengan aktivasi tidak dapat menghapus akun yang sudah aktif.
- Pending expiry melepaskan kontak dan menutup kredensial/identitas terkait secara atomik.
- Akun deactivated/deletion_pending/deletion_processing tetap memegang reservasi kontak.
- Pembatalan deletion_pending tidak membuat reservasi kontak hilang.
- Transisi closed tidak terjadi sebelum proses penghapusan dan pelepasan kontak selesai.

Semua tes concurrency menggunakan beberapa koneksi PostgreSQL nyata. Unit test atau mock tidak cukup untuk membuktikan partial unique index, transaksi, dan race behavior.

## 8. Gate keputusan

Sebelum SQL migration dibuat, review harus memastikan:

- [x] Nama kolom dan tipe data final ditetapkan pada migration pertama.
- [x] Constraint dan partial unique index telah diterapkan dan diuji terhadap aturan lifecycle.
- [ ] Jalur pembuatan akun klien dan provisioning Admin memiliki batas privilege yang dapat ditegakkan.
- [x] Tabel `user_identities` masuk migration pertama karena Google OAuth merupakan metode registrasi/login yang sudah disetujui.
- [x] Foreign key dan perilaku terminal deletion menggunakan ON DELETE RESTRICT serta cleanup identitas eksplisit.
- [x] Tes PostgreSQL untuk constraint, role immutability, provider identity, reservasi kontak, dan konkurensi telah dijalankan di CI.
- [x] Tidak ada kebutuhan verifikasi, session, recovery, atau audit yang terselip ke migration pertama.

Migration pertama telah dibuat dan diuji pada PostgreSQL sementara. Sebelum runtime diaktifkan, tahap berikutnya tetap harus menyelesaikan bootstrap privilege untuk runtime/provisioning Admin, mengintegrasikan operasi registrasi dengan fungsi database, dan menambahkan migration verifikasi secara terpisah. Tidak ada database produksi atau deployment pada tahap ini.

## 9. Referensi

- [Tinjauan Teknis Skema Identity Core](identity-core-schema-review.md)
- [Keputusan Teknis Identity Core](identity-core-technical-decisions.md)
- [Rancangan Skema Database](database-schema-design.md)
- [Keputusan Registrasi Identity Core](identity-core-registration-decisions.md)
- [Tinjauan Skema Verifikasi dan Pemulihan](identity-core-shared-recovery-contacts.md)
- [Kebijakan Keamanan Database dan Integritas Data](../security/database-security-data-integrity.md)
