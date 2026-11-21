# Kontrak Skema Migration 0002 — Verifikasi Kontak dan OTP

> Status: rancangan teknis untuk review sebelum migration SQL dibuat

Dokumen ini menerjemahkan kebijakan Identity Core dan [Keputusan Teknis Identity Core](identity-core-technical-decisions.md) menjadi kontrak skema untuk slice verifikasi kontak. Dokumen ini belum mengimplementasikan tabel, endpoint, provider OTP, enkripsi, atau koneksi database pada HTTP startup.

Migration 0002 hanya mencakup sesi verifikasi, challenge OTP, dan event pengiriman untuk rate limit kontak lintas sesi/instance. Recovery contacts, account recovery cases, password reset, login sessions, audit, dan notifikasi tetap berada pada slice terpisah.

## 1. Tujuan dan batas

Migration ini harus menyediakan:

1. Satu sesi verifikasi yang memiliki lifecycle dan penghitung kegagalan sendiri.
2. Satu atau beberapa challenge OTP berurutan di bawah sesi yang sama, tetapi hanya satu challenge berstatus `pending` pada satu waktu.
3. Tujuan pengiriman yang dapat dipulihkan oleh komponen pengiriman, tetapi tidak tersimpan plaintext.
4. Fingerprint HMAC untuk pencarian dan pembatasan pengiriman tanpa membuka alamat/nomor telepon.
5. Event pengiriman persisten agar limit berlaku lintas sesi, proses, instance, dan restart.
6. Foreign key dan constraint yang menjaga hubungan akun, sesi, challenge, purpose, serta lifecycle.

Di luar cakupan: memilih vendor/provider, membuat kode OTP, mengirim pesan, algoritma/library AEAD final, key management/KMS, HTTP endpoint, UI, serta proses recovery lengkap.

## 2. Tabel `verification_sessions`

Satu baris mewakili satu proses verifikasi. Resend tidak membuat sesi baru.

### Kolom yang diusulkan

| Kolom | Tipe PostgreSQL | Aturan |
| --- | --- | --- |
| `id` | `uuid` | Primary key, default UUID |
| `user_id` | `uuid` | FK ke `users(id)`, nullable hanya untuk `account_recovery` sebelum target dipilih |
| `purpose` | `text` | CHECK: `registration`, `primary_contact_change`, `recovery_contact_add`, `account_recovery` |
| `channel` | `text` | CHECK: `email` atau `whatsapp`; WhatsApp adalah kanal pengiriman untuk tujuan nomor telepon |
| `destination_ciphertext` | `bytea` | Ciphertext AEAD tujuan yang sudah dinormalisasi |
| `destination_nonce` | `bytea` | Nonce AEAD; keunikan nonce dijaga sesuai kebutuhan algoritma/key |
| `encryption_key_version` | `text` | Identifier versi key/envelope; bukan key material |
| `destination_hmac` | `bytea` | HMAC tujuan ternormalisasi untuk pencarian dan pembatasan |
| `hmac_key_version` | `text` | Versi key HMAC yang menghasilkan fingerprint |
| `status` | `text` | CHECK: `pending`, `verified`, `expired`, `cancelled` |
| `failed_attempt_count` | `smallint` | Default 0; rentang 0–5; counter berada pada sesi, bukan challenge |
| `locked_until` | `timestamptz` | NULL kecuali sesi mencapai lima kegagalan |
| `resend_available_at` | `timestamptz` | Batas waktu resend berikutnya; dikendalikan transaksi server |
| `created_at` | `timestamptz` | Waktu pembuatan dari database |
| `expires_at` | `timestamptz` | Deadline sesi yang ditentukan flow; harus setelah pembuatan |
| `verified_at` | `timestamptz` | Terisi hanya saat status menjadi verified |
| `expired_at` | `timestamptz` | Terisi hanya saat status menjadi expired |
| `cancelled_at` | `timestamptz` | Terisi hanya saat status menjadi cancelled |

### Invariant

- `user_id` wajib untuk `registration`, `primary_contact_change`, dan `recovery_contact_add`. Nilai NULL hanya sah untuk `account_recovery` pra-pemilihan akun.
- `account_recovery` boleh dibuat sebelum akun tujuan diketahui. Setelah OTP berhasil, pemilihan akun dan pemeriksaan risiko dilakukan. Pengaitan `user_id` hanya boleh dari NULL menjadi satu akun, satu kali, secara atomik setelah pemeriksaan yang diwajibkan selesai.
- `user_id` yang sudah terisi tidak dapat diganti atau dikosongkan. Pengaitan NULL → target hanya diizinkan sekali pada purpose `account_recovery`, setelah sesi verified dan pemeriksaan pemilihan akun diselesaikan dalam transaksi.
- Tujuan dinormalisasi sebelum dienkripsi dan dihitung HMAC. Kanal `email` hanya untuk alamat email; `whatsapp` hanya untuk nomor telepon format E.164. Validasi format dilakukan sebelum sesi dibuat. Backend harus memastikan tujuan dan purpose cocok dengan kontak/aksi yang sedang diverifikasi.
- Ciphertext, nonce, dan versi key harus tersedia bersama; HMAC dan versi HMAC harus tersedia bersama. Key material tidak pernah disimpan di PostgreSQL.
- Status awal `pending`. Transisi yang diizinkan hanya `pending` → `verified`, `expired`, atau `cancelled`. Status terminal tidak dapat dibuka kembali.
- `verified_at`, `expired_at`, dan `cancelled_at` konsisten dengan status; tepat satu timestamp terminal terisi sesuai status, dan tidak dapat diubah setelah ditetapkan. Perubahan ke status expired mencatat `expired_at` dari waktu server.
- `failed_attempt_count` tidak boleh negatif atau lebih dari lima. Saat mencapai lima, `locked_until` ditetapkan satu jam ke depan menggunakan waktu server. Selama lock, semua percobaan verifikasi ditolak.
- Setelah lock benar-benar berakhir, counter reset ke nol secara atomik ketika sesi pending digunakan kembali. Penerbitan/resend OTP tidak mengubah atau mempercepat lock.
- Sesi yang terminal atau telah melewati `expires_at` tidak dapat diverifikasi kembali.
- Masa berlaku sesi ditetapkan oleh flow yang membuatnya; jangan menyamakan masa berlaku sesi dengan masa berlaku OTP 10 menit. Untuk purpose `registration`, akun target harus masih `pending_verification` dan deadline sesi tidak boleh melampaui `users.pending_expires_at`. Karena ini lintas tabel, enforce dalam operasi transaksi backend dan trigger database yang memvalidasi pembuatan sesi; jangan mengandalkan CHECK biasa.
- Resend menggunakan sesi yang sama dan tunduk pada `resend_available_at`, rate limit lintas sesi, serta batas lain yang berlaku.

## 3. Tabel `verification_challenges`

Satu baris mewakili satu kode OTP yang diterbitkan untuk sesi tertentu.

| Kolom | Tipe PostgreSQL | Aturan |
| --- | --- | --- |
| `id` | `uuid` | Primary key |
| `verification_session_id` | `uuid` | FK wajib ke `verification_sessions(id)`, ON DELETE RESTRICT |
| `code_hash` | `bytea` | Keyed hash kode OTP; bukan OTP plaintext |
| `hash_key_version` | `text` | Versi key yang digunakan untuk keyed hash |
| `status` | `text` | CHECK: `pending`, `consumed`, `expired`, `superseded`, `cancelled` |
| `created_at` | `timestamptz` | Waktu penerbitan dari database |
| `expires_at` | `timestamptz` | Lebih dari created_at dan maksimal 10 menit setelahnya |
| `consumed_at` | `timestamptz` | Terisi hanya untuk status consumed |
| `expired_at` | `timestamptz` | Terisi hanya untuk status expired |
| `superseded_at` | `timestamptz` | Terisi hanya untuk status superseded |
| `cancelled_at` | `timestamptz` | Terisi hanya untuk status cancelled |

### Invariant

- Tidak ada kolom `user_id`; identitas akun diturunkan dari sesi induk. Ini juga menjaga sesi recovery pra-pemilihan akun tetap mungkin.
- Hanya satu challenge `pending` per sesi, dijaga dengan partial unique index.
- Status awal `pending`; transisi hanya menuju satu status terminal. Timestamp terminal yang sesuai wajib dicatat dan tidak dapat diubah.
- Challenge hanya dapat dikonsumsi sekali. Verifikasi yang berhasil harus mengonsumsi challenge dan menyelesaikan sesi dalam satu transaksi.
- Penerbitan challenge baru dalam sesi yang sama harus mengubah challenge pending sebelumnya menjadi `superseded` dan membuat challenge baru dalam transaksi yang sama.
- Challenge kedaluwarsa setelah 10 menit. Kedaluwarsa challenge tidak otomatis mengakhiri sesi induk.
- Kegagalan memasukkan kode menambah counter sesi induk secara atomik, bukan counter challenge.
- Challenge baru yang diterbitkan saat sesi terkunci tetap tidak dapat diverifikasi sampai `locked_until` berakhir.
- Kode OTP tidak pernah disimpan atau dicatat dalam log sebagai plaintext. Panjang digit tetap konfigurasi implementasi, bukan nilai yang ditetapkan migration.
- Perubahan status challenge harus menjaga timestamp terminal yang sesuai dan menolak reaktivasi.

## 4. Tabel `otp_delivery_events`

Tabel ini adalah catatan percobaan penerbitan/pengiriman yang menjadi sumber kebenaran rate limit per kontak lintas sesi dan instance.

| Kolom | Tipe PostgreSQL | Aturan |
| --- | --- | --- |
| `id` | `uuid` | Primary key |
| `destination_hmac` | `bytea` | Fingerprint HMAC tujuan ternormalisasi |
| `hmac_key_version` | `text` | Versi key fingerprint |
| `channel` | `text` | `email` atau `whatsapp` |
| `purpose` | `text` | Purpose yang sama dengan sesi |
| `dispatch_status` | `text` | `pending`, `accepted`, `failed`, atau `unknown` |
| `created_at` | `timestamptz` | Waktu event dibuat sebelum pemanggilan provider |
| `updated_at` | `timestamptz` | Waktu status dispatch terakhir diperbarui |

### Invariant dan operasi

- Event dibuat di dalam transaksi yang memegang lock rate limit, sebelum pekerjaan pengiriman diteruskan ke provider.
- Status dispatch hanya berubah dari `pending` ke salah satu status terminal `accepted`, `failed`, atau `unknown`; status terminal tidak dibuka kembali. Event yang sudah dibuat mengonsumsi kuota, termasuk status `failed` atau `unknown`. Hasil provider yang tidak pasti tidak boleh otomatis mengembalikan kuota.
- Event yang ditolak sebelum penerbitan tidak dibuat sebagai event dispatch.
- Event tidak menyimpan tujuan plaintext, OTP, isi pesan, token provider, atau data pribadi lain.
- Rate limit dasar berlaku lintas purpose dan sesi: maksimal lima event untuk fingerprint tujuan/kanal pada jendela bergulir 60 menit dan cooldown minimal 60 detik antar-event.
- Gunakan indeks komposit yang mendukung pencarian berdasarkan fingerprint, kanal, dan waktu.
- Serialisasi dilakukan dengan transaction-scoped PostgreSQL advisory lock berbasis fingerprint dan kanal. Benturan hash lock hanya menyebabkan serialisasi tambahan, tidak boleh melemahkan batas.
- Penghitungan dan insert event terjadi dalam transaksi yang sama agar permintaan paralel tidak melewati kuota.
- Saat rotasi HMAC, operasi harus mengunci dan menghitung fingerprint dari seluruh versi key yang masih aktif dengan urutan deterministik sampai masa transisi selesai. Jangan sampai pergantian versi membuka jendela bypass.
- Event hanya boleh dibersihkan setelah keluar dari jendela rate limit dan tidak lagi diperlukan untuk investigasi operasional yang sah. Retensi akhir ditetapkan dalam kebijakan operasional, bukan diasumsikan dari migration.
- Rate limit per IP/network, akun, purpose, dan batas global provider adalah lapisan tambahan. Jangan menambahkan penyimpanan IP ke migration ini sebelum tujuan, minimisasi, dan retensinya ditetapkan.

## 5. Indeks dan foreign key

Migration SQL harus sekurang-kurangnya meninjau dan menerapkan:

- FK `verification_sessions.user_id` → `users.id` dengan perilaku penghapusan yang tidak menghapus histori verifikasi secara diam-diam.
- FK `verification_challenges.verification_session_id` → `verification_sessions.id`.
- Indeks sesi berdasarkan `user_id`, `purpose`, `status`, dan waktu kedaluwarsa sesuai pola cleanup/query.
- Partial unique index untuk satu challenge pending per sesi.
- Indeks rate-limit berdasarkan `destination_hmac`, `channel`, dan `created_at`.
- Indeks pendukung cleanup challenge/session kedaluwarsa tanpa menghapus sesi yang masih dibutuhkan.
- CHECK constraints untuk vocabulary, rentang counter, pasangan timestamp/status, pasangan ciphertext/key version/HMAC, dan batas waktu OTP. Tambahkan trigger untuk aturan lintas tabel pada sesi registrasi.

Nama indeks dan detail kolom final ditetapkan saat review SQL. Jangan membuat indeks duplikat yang tidak punya kebutuhan query terukur.

## 6. Transaksi wajib

| Operasi | Batas atomik |
| --- | --- |
| Membuat sesi | Validasi purpose, user_id, channel, destination; simpan ciphertext/HMAC dan deadline secara konsisten |
| Mengirim OTP | Lock fingerprint; hitung limit; perbarui cooldown; supersede challenge lama; buat challenge dan delivery event |
| Verifikasi berhasil | Lock sesi/challenge; cek status, expiry, purpose, destination binding, dan lock; konsumsi challenge serta tandai sesi verified satu kali |
| Verifikasi gagal | Lock sesi; tambah counter; kegagalan kelima menetapkan lock satu jam |
| Membuka kembali sesi setelah lock | Periksa deadline dan status; reset counter satu kali secara atomik; jangan menghapus delivery events |
| Mengaitkan recovery account | Setelah pemilihan akun dan pemeriksaan risiko, ubah user_id NULL menjadi target satu kali dalam transaksi |
| Cleanup | Hanya mengubah status yang benar-benar kedaluwarsa; operasi idempotent dan tidak mengaktifkan kembali status terminal |

Operasi yang mengirim pesan eksternal tidak boleh menahan transaksi database terbuka selama menunggu provider. Catat event dan reservasi kuota secara atomik, commit, lalu lakukan dispatch dan perbarui statusnya secara terpisah. Penanganan crash/retry provider harus menjaga idempotensi dan tidak mengirim ulang kode lama yang sudah superseded.

## 7. Rencana pengujian PostgreSQL

Migration belum siap dibuat/dianggap selesai sebelum pengujian nyata membuktikan:

### Constraint dan lifecycle
- Migration berjalan pada PostgreSQL 16 kosong dan SQLx mengenali checksum/version.
- `user_id` NULL ditolak untuk purpose selain account recovery; user_id wajib pada registrasi.
- Status dan purpose/channel yang tidak dikenal ditolak.
- Ciphertext/nonce/key version/HMAC yang tidak lengkap ditolak.
- Challenge tanpa sesi induk ditolak.
- Dua challenge pending pada sesi yang sama ditolak.
- Challenge dengan expiry lebih dari 10 menit ditolak.
- Status terminal, timestamp terminal, dan transisi sesi/challenge tidak dapat dibuka kembali; timestamp expired harus tersedia untuk kedua tabel.

### Rate limit dan concurrency
- Lima pengiriman dalam jendela 60 menit diterima; pengiriman keenam ditolak.
- Cooldown 60 detik ditegakkan bahkan pada sesi berbeda.
- Limit berlaku lintas purpose dan sesi.
- Permintaan paralel menggunakan beberapa koneksi PostgreSQL tidak dapat melampaui lima event.
- Rotasi HMAC tidak membuat bypass selama masa transisi.
- Event gagal/unknown tetap dihitung.
- Event yang masih diperlukan tidak dihapus cleanup.

### Verifikasi
- Challenge hanya berlaku 10 menit dan hanya dapat dikonsumsi sekali.
- Resend bersamaan menghasilkan tepat satu challenge pending.
- Kegagalan kelima mengunci sesi satu jam meskipun percobaan paralel.
- Challenge baru selama lock tidak dapat diverifikasi.
- Setelah lock berakhir, sesi pending yang sama dapat digunakan kembali dan counter reset satu kali.
- Reset counter tidak mereset rate limit kontak.
- Sesi/challenge expired, verified, atau cancelled tidak dapat digunakan kembali.

### Privasi dan keamanan
- Database tidak menyimpan tujuan plaintext atau OTP plaintext.
- Log dan error tidak memuat ciphertext, fingerprint mentah, OTP, atau secret.
- Key material tidak ada dalam migration, fixture, source control, atau database.
- Recovery pra-pemilihan akun tidak mengungkap daftar akun atau mengaitkan sesi ke akun sebelum pemeriksaan selesai.

Semua tes race/concurrency harus memakai beberapa koneksi PostgreSQL nyata, bukan mock.

## 8. Gate sebelum implementasi

Migration SQL 0002 baru boleh dibuat setelah review memastikan:

- [ ] Kontrak purpose dan channel sesuai alur produk yang disetujui.
- [ ] Representasi ciphertext/HMAC/key version cocok dengan abstraksi key provider yang akan dipakai backend.
- [ ] Operasi advisory lock dan rotasi HMAC tidak membuka bypass.
- [ ] Semua constraint, partial index, FK, dan state transition sudah ditinjau.
- [ ] Rencana transaksi resend, verifikasi, reset lock, dan cleanup jelas.
- [ ] Tes PostgreSQL untuk concurrency dan batas waktu sudah dirancang.
- [ ] Tidak ada key, OTP, tujuan plaintext, atau data produksi dalam migration/test fixture.
- [ ] Migration tetap terpisah dari runtime endpoint, provider, login session, recovery case, dan deployment.

Migration ini tidak boleh dijalankan terhadap database produksi. Jangan mengaktifkan migration otomatis pada HTTP startup.

## 9. Referensi internal

- [Keputusan Teknis Identity Core](identity-core-technical-decisions.md)
- [Tinjauan Skema Identity Core](identity-core-schema-review.md)
- [Keputusan Pendaftaran Identity Core](identity-core-registration-decisions.md)
- [Kontrak Skema Migration 0001 — Identitas Akun](identity-core-migration-0001-contract.md)
- [Kebijakan OTP dan recovery pada Identity Core](identity-core-registration-decisions.md)
