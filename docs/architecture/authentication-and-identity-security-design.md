# Autentikasi & Identitas — Rancangan Arsitektur dan Keamanan

> Status: migration Identity Core 0001–0005 tersedia; runtime autentikasi lengkap belum diimplementasikan  
> Referensi produk: `docs/product/public-and-auth-uiux-spec.md`  
> Referensi wireframe: `docs/product/low-fidelity-public-auth-wireframes.md`  
> Fondasi backend: Rust + Axum, PostgreSQL 16, dan SQLx 0.8. Skema akun, verifikasi, sesi, recovery, dan audit sudah tersedia sebagai migration.

Dokumen ini menjelaskan arsitektur autentikasi USahapintar. Migration Identity Core 0001–0005 sudah menyediakan skema akun, verifikasi, sesi, pemulihan, dan audit. Backend juga sudah memiliki pool database saat startup serta akses terbatas untuk membuat akun pending dan menambahkan audit event. Namun, alur registrasi dan aktivasi lengkap, OTP runtime, endpoint login/logout, pengelolaan cookie, middleware autentikasi, pemulihan akun, dan integrasi penyedia layanan belum tersedia. Status ini bukan pernyataan bahwa sistem siap produksi atau telah diaudit keamanannya. Penyedia layanan, secret operasional, masa retensi, dan kontrol akhir tetap harus ditinjau sebelum digunakan di produksi.

## 1. Cakupan dan invariant

### Termasuk dalam cakupan

- Pendaftaran publik hanya tersedia untuk Business Owner dan Investor.
- Setiap akun hanya memiliki satu role utama klien yang tetap (`business_owner` atau `investor`).
- Jalur login Owner dan Investor dipisahkan.
- Tahap pertama menyediakan email atau nomor telepon + password serta Google OAuth sebagai alternatif. OTP tidak menjadi syarat registrasi maupun login; implementasinya ditunda. Kontak yang belum diverifikasi tidak boleh diperlakukan sebagai bukti kepemilikan untuk pemulihan atau tindakan sensitif.
- Kontak awal pendaftaran dipilih salah satu: email atau telepon/WhatsApp.
- Google OAuth tetap tersedia untuk login dan pendaftaran. Untuk akun baru, pemilihan role Owner/Investor wajib dilakukan sebelum OAuth dan diikat ke state server. Tidak ada OTP tambahan pada tahap pertama.
- Password recovery.
- Siklus hidup sesi dan logout.
- Otorisasi role dan sumber daya di sisi server.
- Fondasi provisioning Admin internal, MFA, dan audit trail.

### Di luar cakupan tahap awal autentikasi

- Pendaftaran Admin publik.
- Beberapa role klien dalam satu akun.
- Social login providers other than Google.
- Magic link email tanpa kata sandi, kecuali disetujui secara terpisah.
- Transaksi investasi, penitipan dana pembayaran, atau koneksi ke rekening keuangan.
- Pemisahan menjadi microservice; autentikasi tetap menjadi bagian dari modular monolith.

### Invariant yang wajib dipertahankan

1. Backend menjadi sumber kebenaran untuk identitas, role utama, status akun, dan izin.
2. Setiap akun klien hanya memiliki satu role utama yang dipilih saat pendaftaran; role tersebut tidak dapat diubah selama akun masih ada.
3. Pengguna yang ingin memakai role klien lainnya harus membuat akun baru yang terpisah dengan identitas login yang belum terhubung ke akun lain.
4. Frontend tidak pernah memberikan akses hanya berdasarkan visibilitas rute atau state lokal.
5. Akun Owner tidak dapat login melalui portal Investor, begitu pula sebaliknya.
6. Akun Admin disediakan melalui proses internal; tidak ada endpoint publik yang dapat membuat Admin.
7. A conversation requires an accepted connection between its participants.
8. Keanggotaan Free tidak membatasi pengiriman pesan setelah koneksi disetujui.
9. OAuth tidak boleh diam-diam mengubah role akun atau menggabungkan identitas.
10. Kata sandi, OTP, access token, secret sesi, dan token reset tidak pernah dicatat dalam log aplikasi.
11. Setiap perubahan sensitif divalidasi terhadap pengguna yang terautentikasi dan sumber daya yang dituju.
12. Sistem tidak boleh mengungkap apakah suatu email atau nomor telepon terhubung ke akun melalui respons pendaftaran atau pemulihan yang belum terautentikasi.

## Batas keanggotaan dan otorisasi

Keanggotaan merupakan dimensi hak akses tersendiri dan tidak mengubah role utama yang bersifat tetap. Paket Free atau Premium tidak boleh memberikan kemampuan Owner kepada Investor atau sebaliknya, melewati pemeriksaan status akun, kepemilikan, consent, moderasi, maupun izin Admin, atau membuka informasi bisnis privat. Backend memeriksa autentikasi, status akun, role, izin terhadap sumber daya, dan status keamanan/moderasi sebelum mengevaluasi entitlement serta kuota keanggotaan. Lihat [Kebijakan Keanggotaan & Hak Fitur](../product/membership-and-entitlements.md) untuk kemampuan paket, aturan kuota, perilaku downgrade, dan kebutuhan pengujian.

## 2. Model identitas dan role

### Role utama

Gunakan satu role utama untuk MVP:

- `business_owner`
- `investor`
- `admin`

Role Owner/Investor ditetapkan saat pendaftaran dan tidak dapat diubah selama akun masih ada. Admin hanya ditetapkan melalui proses provisioning internal. Tidak ada endpoint profil, proses bantuan, migrasi administratif, atau alur OAuth yang boleh mengubah role utama akun yang sudah ada.

Pengguna yang ingin memakai role klien lainnya harus membuat akun baru yang terpisah. Pendaftaran harus menolak identitas login yang sudah terhubung ke akun lain tanpa mengungkap akun mana yang memilikinya. Jika pelepasan identitas dari akun yang ditutup kelak didukung, prosesnya harus mengikuti kebijakan penutupan akun, verifikasi, dan retensi yang ditetapkan secara terpisah; identitas tidak boleh dipindahkan atau digabungkan secara diam-diam.

### Lapisan otorisasi

Otorisasi dievaluasi dengan urutan berikut:

1. Apakah sesi valid dan belum dicabut/kedaluwarsa?
2. Apakah siklus hidup akun berstatus aktif serta tidak memiliki pembatasan moderasi atau penguncian keamanan yang masih berlaku?
3. Apakah akun yang terautentikasi memiliki role utama yang disyaratkan?
4. Apakah akun memiliki atau menjadi peserta pada sumber daya yang diminta?
5. Apakah tindakan yang diminta diizinkan untuk status sumber daya saat ini?
6. Apakah hak fitur keanggotaan memengaruhi kuota/fitur yang diminta? Keanggotaan tidak pernah melewati pemeriksaan kepemilikan atau role.

Jangan menerapkan otorisasi sebagai kondisi frontend yang tersebar. Gunakan extractor/middleware backend bersama dan fungsi kebijakan pada tingkat domain, disertai pengujian.

## 3. Rancangan model data

> **Catatan keselarasan keputusan 22 November 2026:** Migration 0001 saat ini masih mewajibkan status `pending_verification` dan bukti verifikasi kontak awal sebelum akun menjadi `active`. Ketentuan tersebut belum sesuai dengan autentikasi tahap pertama yang mengizinkan registrasi tanpa OTP. Jangan menganggap runtime dapat memakai skema sekarang tanpa penyesuaian. Sebelum runtime dibuat, siapkan migration lanjutan yang diuji untuk mengizinkan akun langsung aktif dengan kontak berstatus belum terverifikasi, tanpa mengubah migration lama. Lihat [Keputusan Autentikasi Identity Core — Tahap Pertama](identity-core-authentication-stage-one-decision.md).

PostgreSQL 16 merupakan dasar database relasional. Migration SQLx Identity Core 0001–0005 untuk akun, verifikasi, sesi, recovery, dan audit sudah tersedia serta tercakup dalam pengujian integrasi PostgreSQL. HTTP server menginisialisasi pool database saat startup melalui principal aplikasi; migration tetap dijalankan secara eksplisit oleh proses terpisah. Backend baru membuka operasi terbatas untuk membuat akun pending dan menambahkan audit event, belum menyediakan alur autentikasi lengkap. Migration harus berversi dan tidak boleh diubah setelah diterapkan pada database bersama. Gunakan primary key UUID dan `timestamptz`.

### `users`

Migration 0001 mengimplementasikan field akun berikut. Data profil dan riwayat permintaan penghapusan tetap berada dalam tabel domain terpisah pada tahap berikutnya.

| Kolom | Tujuan |
| --- | --- |
| `id` | UUID primary key |
| `primary_role` | Role tetap: `business_owner`, `investor`, atau `admin` |
| `account_status` | Lifecycle: `pending_verification`, `active`, `deactivated`, `deletion_pending`, `deletion_processing`, `pending_expired`, or `closed` |
| `email_normalized` | Email utama yang sudah dinormalisasi; boleh NULL jika nomor telepon menjadi satu-satunya kontak utama saat ini |
| `phone_e164` | Nomor telepon utama yang sudah dinormalisasi ke E.164; boleh NULL jika email menjadi satu-satunya kontak utama saat ini |
| `initial_contact_channel` | Kanal pendaftaran awal yang tetap: `email` atau `phone` |
| `email_verified_at`, `phone_verified_at` | Timestamp verifikasi terpisah untuk nilai kontak saat ini |
| `initial_contact_verified_at` | Bukti waktu verifikasi pendaftaran yang tetap, ditetapkan satu kali saat aktivasi dan hanya dihapus setelah proses penghapusan selesai |
| `password_hash` | Hash kata sandi lokal dalam format encoded yang boleh NULL; akun klien tanpa nilai ini wajib memiliki identitas Google yang terhubung |
| `pending_expires_at` | Immutable 30-minute deadline while registration is pending |
| `created_at`, `updated_at`, `status_changed_at` | Database-managed timestamps |

Constraint dan pengaman siklus hidup:
- Indeks unik parsial mereservasi kontak email dan telepon yang tidak NULL selama akun berstatus pending, active, deactivated, deletion-pending, atau deletion-processing.
- Akun pending memiliki tepat satu kontak awal yang belum diverifikasi. Akun active memiliki setidaknya satu kontak utama terverifikasi yang masih berlaku.
- Role dan kanal pendaftaran awal tidak dapat diubah. Verifikasi kontak awal hanya dapat dicatat satu kali selama aktivasi yang sah.
- Pendaftaran pending tidak dapat diaktifkan setelah 30 menit atau kedaluwarsa sebelum batas waktunya. Masa berlaku pending tidak dapat diperpanjang.
- Penghapusan harus menunggu 30 hari dalam status `deletion_pending` sebelum masuk ke `deletion_processing`; proses hanya dapat berakhir pada status `closed`. `pending_expired` dan `closed` bersifat terminal.
- Suspensi dan batas fitur dicatat sebagai record moderasi/penegakan, bukan sebagai nilai `account_status`.
- Kolom profil, riwayat permintaan penonaktifan/penghapusan, dan catatan retensi tidak disimpan dalam tahap pertama ini.
- Jangan pernah menyimpan kata sandi dalam bentuk teks asli. Jangan gunakan ID database mentah sebagai secret otorisasi.

### `user_identities`

Migration 0001 mengimplementasikan identitas penyedia Google yang terhubung.

| Kolom | Tujuan |
| --- | --- |
| `id` | UUID primary key |
| `user_id` | Pengguna pemilik; FK dengan ON DELETE RESTRICT |
| `provider` | Initially constrained to `google` |
| `provider_subject` | Identifier subject penyedia yang stabil; bukan email |
| `created_at` | Creation timestamp |

Constraints:
- Unique composite key on (`provider`, `provider_subject`).
- Subject dari penyedia, bukan teks email, menjadi key identitas yang stabil.
- Pengikatan identitas tidak dapat diubah setelah insert; pelepasan tautan merupakan tindakan terautentikasi yang terpisah.
- Akun klien tanpa kata sandi lokal tidak dapat melepas tautan identitas Google terakhirnya.
- Terminal accounts cannot retain provider identities.
- Email dari penyedia maupun OAuth access/refresh/ID token tidak disimpan dalam tabel ini.
- Pengaitan memerlukan bukti kendali atas akun yang sudah ada dan identitas penyedia yang baru.
- Jangan mengaitkan identitas secara otomatis hanya karena email dari penyedia tampak cocok.

### `sessions`

Opaque, server-managed sessions.

| Kolom | Tujuan |
| --- | --- |
| `id` | UUID primary key |
| `user_id` | Pemilik sesi |
| `token_hash` | Hash SHA-256 32 byte dari token sesi opaque berentropi tinggi; unik secara global |
| `created_at` | Waktu pembuatan sesi yang ditetapkan database |
| `last_seen_at` | Aktivitas terakhir yang diperbarui server untuk menegakkan batas idle 7 hari |
| `expires_at` | Batas absolut yang ditetapkan database, 30 hari sejak sesi dibuat |
| `token_rotated_at` | Waktu rotasi token terakhir yang ditetapkan database |
| `remember_device` | Penanda persistensi cookie browser; tidak mengubah batas idle maupun absolut |
| `revoked_at`, `revocation_reason` | Waktu dan alasan pencabutan server-side |
| `user_agent_summary` | Ringkasan perangkat opsional, diminimalkan dan dibatasi panjangnya |
| `ip_prefix_hash`, `ip_prefix_hmac_key_version` | HMAC prefix jaringan opsional beserta versi key; bukan alamat IP mentah |

Constraints:
- Simpan hanya hash kriptografis token sesi berentropi tinggi, bukan token asli. Skema mensyaratkan digest 32 byte dan keunikan global.
- Waktu pembuatan, aktivitas awal, rotasi token, dan batas absolut ditetapkan database. Batas absolut selalu 30 hari.
- Backend wajib menolak sesi yang tidak aktif lebih dari 7 hari, telah melewati `expires_at`, dicabut, atau dimiliki akun yang tidak aktif. Perangkat yang diingat tidak boleh melewati batas ini.
- Rotasi token dilakukan setelah autentikasi dan perubahan privilege; hash lama diganti secara atomik dan timestamp rotasi diperbarui database.
- Pencabutan sesi harus segera berlaku dari server. Logout, pencabutan satu perangkat, logout semua perangkat, reset password, perubahan status akun, tindakan Admin, dan peristiwa keamanan memiliki kode alasan yang eksplisit.
- Perubahan siklus hidup akun dari `active` ke status lain otomatis mencabut seluruh sesi yang belum dicabut. Restriksi/suspensi moderasi dan peristiwa keamanan lain tetap harus memicu pencabutan melalui runtime.
- Hash prefix jaringan, jika digunakan, harus berupa HMAC berkunci dan menyimpan versi key agar rotasi dapat dikelola. Jangan mengumpulkan alamat IP penuh atau fingerprint perangkat lengkap tanpa kebutuhan keamanan dan kebijakan retensi yang terdokumentasi.

### `verification_challenges`

Mendukung verifikasi email dan nomor telepon melalui WhatsApp untuk aktivasi akun/verifikasi kontak tanpa menyimpan kode yang masih dapat digunakan. Kode tersebut bukan kredensial login.

| Kolom | Tujuan |
| --- | --- |
| `id` | UUID primary key |
| `user_id` | Pengguna terkait; hanya boleh NULL pada alur pendaftaran sebelum akun dibuat jika memang diperlukan |
| `channel` | `email` atau `whatsapp`; nomor telepon merupakan tujuan yang dinormalisasi, sedangkan WhatsApp adalah kanal pengiriman |
| `destination_hash` | Hash or protected reference to destination |
| `code_hash` | Hash of a high-entropy one-time code |
| `purpose` | Verifikasi pendaftaran atau perubahan kontak |
| `verification_session_id` | Alur verifikasi induk; jumlah kegagalan dan penguncian tetap berlaku setelah OTP dikirim ulang |
| `attempt_count` | Jumlah percobaan gagal untuk challenge ini |
| `expires_at` | Masa berlaku 10 menit |
| `superseded_at` | Diisi ketika OTP pengganti membatalkan kode ini |
| `consumed_at` | One-time use marker |
| `created_at` | Creation timestamp |

Controls:
- Catatan sesi verifikasi terpisah harus menyimpan akumulasi batas lima kegagalan dan penguncian satu jam. Pembuatan challenge baru tidak boleh mengatur ulang batas tersebut.
- Simpan atau tegakkan secara persisten jeda pengiriman ulang 60 detik dan batas lima pengiriman per jam untuk setiap alamat tujuan di seluruh instance aplikasi.
- Masa berlaku singkat, konsumsi satu kali, batas percobaan, jeda kirim ulang, serta rate limit berdasarkan tujuan/akun/IP.
- Use a keyed hash/HMAC for short numeric OTPs so database disclosure alone does not make offline guessing trivial.
- Jangan mencatat nilai kode dalam log atau mengirimkannya ke analytics.
- Batalkan challenge aktif yang lebih lama ketika challenge baru diterbitkan untuk tujuan penggunaan yang sama.
- Kegagalan pengiriman oleh penyedia tidak boleh menandai tujuan sebagai terverifikasi.

### Kontak pemulihan dan kasus pemulihan akun

Kontak pemulihan bersifat opsional dan ditambahkan setelah akun aktif. Simpan kontak ini terpisah dari kontak utama untuk login agar masing-masing dapat diverifikasi, diberi notifikasi, dan dicabut secara terpisah. Setiap kontak pemulihan perlu memiliki nilai yang sudah dinormalisasi, jenis kanal, waktu verifikasi, status, dan relasi ke pemilik akun. Terapkan aturan keunikan serta pencegahan penyalahgunaan tanpa membocorkan hubungan kontak dengan akun.

Pemulihan akun memerlukan alur kasus dan verifikasi yang tercatat, bukan sekadar penanda benar/salah. Catat tujuan (lupa kata sandi, lupa email, atau pemulihan alternatif), status, waktu pembuatan/kedaluwarsa/peninjauan, hasil penilaian risiko, dan referensi audit secukupnya. Jangan simpan rahasia yang dikirim pengguna atau bukti identitas berlebihan di log umum. Penugasan pemeriksaan manual harus mengacu pada petugas support dengan kewenangan khusus, dan setiap pemeriksaan dicatat dalam riwayat yang tidak dapat diubah sembarangan. Email utama lengkap hanya boleh ditampilkan setelah kontak pemulihan yang sudah terhubung ke akun berhasil diverifikasi.

Jika kontak utama dan seluruh kontak pemulihan tidak dapat diakses, alur pemulihan alternatif harus memeriksa perangkat tepercaya, riwayat sesi, dan bukti kepemilikan lainnya. Jika pemeriksaan otomatis belum cukup, kasus dapat diteruskan kepada petugas yang berwenang. Permintaan berisiko tinggi dapat ditunda atau ditolak. Proses pemulihan tidak boleh mengubah role utama akun.

### `password_reset_tokens`

- Store a hash of a cryptographically random, single-use token.
- Sertakan ID pengguna, masa berlaku, waktu pembuatan, dan waktu konsumsi.
- Cabut sesi yang sudah ada setelah reset berhasil sesuai kebijakan keamanan.
- Jangan mengungkap keberadaan akun dalam respons permintaan.

### `admin_audit_logs`

- Catatan append-only untuk tindakan administratif yang sensitif.
- Sertakan actor ID, tindakan, jenis/ID target, timestamp, hasil, serta alasan/konteks minimum.
- Jangan menyimpan kata sandi, OTP, token sesi mentah, token reset, atau konten privat yang tidak diperlukan.
- Lindungi izin baca audit log secara terpisah dari izin Admin biasa.

### Tabel tambahan hanya jika diperlukan

- `admin_mfa_credentials` untuk metadata kredensial TOTP/passkey dan status pemulihan.
- `account_link_requests` jika pengaitan penyedia secara eksplisit masuk ke rilis pertama.
- `login_attempt_buckets` atau limiter berbasis Redis jika trafik operasional memerlukan rate limit terdistribusi.
- `user_consents` for versioned Terms/Privacy consent records.

Hindari membuat tabel yang belum digunakan sebelum ada fitur konkret yang membutuhkannya.

## Kebijakan awal Identity Core yang telah disetujui

Nilai kebijakan produk berikut merupakan batas awal yang telah disepakati dan harus ditegakkan oleh backend. Nilai ini dapat ditinjau ulang melalui evaluasi keamanan dan bukan jaminan dari penyedia layanan.

### OTP dan verifikasi kontak — tahap berikutnya

- OTP tidak digunakan untuk registrasi maupun login pada tahap pertama. Runtime OTP dan provider pengiriman belum diimplementasikan.
- Aturan masa berlaku, keyed hash, batas percobaan, lock, resend, serta rate limit di bawah ini merupakan baseline untuk tahap verifikasi kontak berikutnya; aturan tersebut tidak boleh ditafsirkan sebagai syarat aktivasi akun tahap pertama.
- Kontak yang belum diverifikasi tetap berstatus belum terverifikasi. Jangan mengisi timestamp verifikasi hanya karena kontak diberikan saat registrasi.
- Pendaftaran tahap pertama tidak menggunakan masa tunggu `pending_verification` selama 30 menit. Migration lanjutan harus menyelaraskan lifecycle dan constraint sebelum runtime registrasi diaktifkan.
- Saat OTP kelak diaktifkan, kode baru harus membatalkan kode sebelumnya dan kegagalan pengiriman tidak boleh dianggap sebagai keberhasilan verifikasi.

### Sesi

- Pengguna boleh memiliki beberapa sesi aktif sekaligus. Pengguna dapat melihat daftar sesi, mencabut satu sesi, atau mengakhiri semua sesi.
- Batas awal sesi adalah 7 hari tanpa aktivitas dan maksimal 30 hari sejak sesi dibuat.
- Fitur “Ingat perangkat ini” hanya membuat sesi browser bertahan. Fitur ini tidak boleh melewati kedua batas waktu atau pencabutan sesi dari server. Menutup browser tidak otomatis mengakhiri sesi yang memang dibuat persisten.
- Pencabutan sesi dari server berlaku segera. Gunakan cookie `Secure` dan `HttpOnly`, pengaturan `SameSite`, serta perlindungan CSRF dan pemeriksaan origin. Tindakan sensitif dapat meminta verifikasi ulang.

### Siklus hidup akun

Penonaktifan sementara berbeda dari penghapusan permanen. Saat dinonaktifkan, profil publik disembunyikan dan akses normal dibatasi, tetapi data serta role tetap disimpan. Untuk mengaktifkan kembali akun, pengguna harus login dan melewati pemeriksaan keamanan. Penghapusan permanen memerlukan konfirmasi dan autentikasi ulang, lalu masa tunggu 30 hari. Selama masa tunggu, akun disembunyikan atau dibatasi dan permintaan penghapusan dapat dibatalkan melalui pemeriksaan keamanan. Setelah itu, proses penghapusan mengikuti aturan retensi data dan penanganan cadangan yang telah didokumentasikan. Kontak tidak boleh otomatis digunakan kembali.

### Pemulihan akun

Kontak pemulihan tidak wajib saat pendaftaran. Kontak ini dapat ditambahkan setelah akun aktif dan harus diverifikasi secara terpisah. Alur lupa kata sandi dan lupa email menggunakan email atau nomor HP pemulihan yang sudah terverifikasi. Email utama lengkap hanya boleh ditampilkan setelah kontak pemulihan berhasil diverifikasi dan terbukti terhubung ke akun.

Jika kontak utama dan seluruh kontak pemulihan sama-sama tidak dapat diakses, gunakan proses pemulihan alternatif berlapis. Periksa perangkat tepercaya, sesi sebelumnya, dan sinyal kepemilikan lainnya. Jika bukti otomatis belum cukup, teruskan kasus kepada petugas support dengan kewenangan khusus. Permintaan berisiko tinggi dapat ditunda atau ditolak. Terapkan hak akses minimum, kriteria pemeriksaan yang terdokumentasi, audit log, dan notifikasi yang sesuai. Pemulihan tidak boleh mengubah role utama. Pencabutan sesi dan perlindungan tambahan setelah pemulihan ditentukan berdasarkan tingkat risiko.

## 4. Kontrak API (usulan)

Semua rute menggunakan versi `/api/v1`. Bentuk permintaan/respons JSON harus ditetapkan dalam OpenAPI sebelum implementasi. Tabel di bawah merupakan inventaris rute, bukan dokumentasi format data final.

### Pendaftaran tahap pertama

| Metode | Rute | Tujuan |
| --- | --- | --- |
| POST | `/auth/register/owner` | Membuat akun Owner dengan password dan kontak awal yang belum harus diverifikasi |
| POST | `/auth/register/investor` | Membuat akun Investor dengan password dan kontak awal yang belum harus diverifikasi |

Endpoint verifikasi email/WhatsApp tidak termasuk tahap pertama. Tambahkan rute verifikasi kontak pada tahap berikutnya setelah metode pembuktian kepemilikan dan provider pengiriman siap.

There is intentionally no `/auth/register/admin` endpoint.

### Autentikasi kata sandi dan sesi

Registrasi melalui email atau telepon wajib membuat kata sandi lokal. Registrasi Google OAuth tidak memerlukan kata sandi lokal; autentikasi dilakukan melalui stable provider subject Google yang terhubung. OTP tidak diminta pada registrasi atau login tahap pertama. Kontak yang belum diverifikasi tidak boleh digunakan untuk pemulihan atau tindakan sensitif.

| Metode | Rute | Tujuan |
| --- | --- | --- |
| POST | `/auth/login/owner` | Mengautentikasi akun Owner saja |
| POST | `/auth/login/investor` | Mengautentikasi akun Investor saja |
| POST | `/auth/logout` | Mencabut sesi saat ini |
| POST | `/auth/logout-all` | Mencabut seluruh sesi akun yang terautentikasi |
| GET | `/auth/session` | Mengembalikan pengguna yang sedang terautentikasi serta metadata role/sesi yang aman |
| POST | `/auth/password/forgot` | Meminta reset kata sandi |
| POST | `/auth/password/reset` | Mengonsumsi token reset dan menetapkan kata sandi baru |

Login menerima email atau nomor telepon terdaftar beserta kata sandi akun. Endpoint login tidak menerima kode verifikasi. Endpoint login wajib memeriksa role yang diharapkan di server. Ketidaksesuaian role menghasilkan respons panduan portal yang umum tanpa mengungkap detail akun tambahan.

### Google OAuth — alternatif tahap pertama

Google OAuth didukung untuk login dan pendaftaran. Untuk pendaftaran baru, pengguna wajib memilih Owner atau Investor sebelum memulai OAuth; role tersebut diikat pada state server dan tidak dapat diubah melalui callback. Akun Google-only tidak memerlukan password lokal. OTP tambahan tidak diminta pada tahap pertama.

| Metode | Rute | Tujuan |
| --- | --- | --- |
| GET | `/auth/oauth/google/start?role=business_owner` | Memulai OAuth dengan konteks Owner |
| GET | `/auth/oauth/google/start?role=investor` | Memulai OAuth dengan konteks Investor |
| GET | `/auth/oauth/google/callback` | Memvalidasi respons penyedia dan melanjutkan alur yang terikat pada role |

Untuk identitas Google baru, pengguna memilih Owner atau Investor sebelum OAuth. Setelah callback tervalidasi, buat akun dengan role yang terikat pada state dan simpan stable provider subject dalam transaksi yang sama. Jangan menyimpulkan email terverifikasi kecuali token tervalidasi menyatakan `email_verified=true`. Identitas yang sudah terhubung hanya dapat login ke akun lamanya dengan role yang tersimpan. Jika email Google cocok dengan akun lokal yang sudah ada, jangan menautkan atau menggabungkan akun otomatis; minta pengguna masuk ke akun lama dan menjalankan pengaitan eksplisit dengan autentikasi ulang. Akun Admin tidak dapat dibuat melalui OAuth publik. Validasi state, nonce, issuer, audience, signature, expiry, dan redirect URI yang tepat.

### Autentikasi Admin

Autentikasi Admin menggunakan kelompok rute dan kebijakan internal yang terpisah. Rute yang diusulkan:
- `POST /admin/auth/login`
- `POST /admin/auth/mfa/verify`
- `POST /admin/auth/logout`
- `GET /admin/auth/session`

Pembuatan akun Admin merupakan operasi internal dengan hak istimewa, bukan API autentikasi publik. Otorisasi Admin berbasis izin, bukan sekadar `primary_role=admin`.

## 5. Keamanan sesi dan browser

### Pola sesi browser yang disarankan

Utamakan sesi opaque yang dikelola server dan dikirim melalui cookie:

- `HttpOnly`
- `Secure` in production
- `SameSite=Lax` atau lebih ketat jika tetap kompatibel dengan rancangan callback OAuth.
- Narrow cookie path/domain
- Masa kedaluwarsa idle dan absolut yang ditetapkan secara eksplisit.
- Rotasi sesi setelah login dan perubahan yang sensitif terhadap hak akses.

Hindari menyimpan bearer token berumur panjang di localStorage. Jika arsitektur lintas situs memerlukan model token berbeda, dokumentasikan komprominya dan lakukan tinjauan keamanan terpisah.

### CSRF dan CORS

- Untuk permintaan yang mengubah status dan diautentikasi melalui cookie, gunakan perlindungan CSRF yang sesuai dengan topologi deployment (token atau strategi origin/referer yang kuat).
- Batasi CORS ke origin frontend tepercaya yang ditentukan secara tepat; jangan pernah menggabungkan permintaan dengan kredensial dengan wildcard origin.
- Validasi content type dan origin permintaan untuk endpoint sensitif.
- Simpan origin produksi dan OAuth redirect URI dalam konfigurasi environment, bukan hard-coded di source.

### Siklus hidup sesi

- Login membuat sesi baru dan merotasi state sesi pra-autentikasi yang ada.
- Logout mencabut sesi saat ini.
- Logout-all revokes all sessions.
- Password reset revokes sessions.
- Kode verifikasi email/WhatsApp tidak dapat digunakan untuk mengautentikasi sesi login.
- Penangguhan akun memblokir penggunaan sesi meskipun token sesi belum kedaluwarsa.
- Ketidaksesuaian role tidak pernah membuat sesi.
- Tindakan Admin yang sensitif dapat mewajibkan autentikasi ulang/MFA yang masih baru.

Tegakkan batas sesi yang telah disetujui: 7 hari tanpa aktivitas dan 30 hari secara absolut. Preferensi perangkat persisten tidak boleh melewati kedua batas tersebut atau pencabutan di sisi server. Tindakan sensitif dapat mewajibkan autentikasi baru.

## 6. Kebijakan kata sandi

- Gunakan Argon2id dengan parameter yang telah diukur untuk lingkungan deployment.
- Use a well-maintained password hashing library; do not design a custom algorithm.
- Terapkan panjang minimum yang wajar dan izinkan passphrase yang panjang.
- Do not force arbitrary periodic password changes absent compromise.
- Block known-compromised/common passwords where a safe, privacy-preserving check is available.
- Do not silently truncate passwords.
- Terapkan rate limit pada percobaan autentikasi dan gunakan kontrol penyalahgunaan bertahap.
- Pastikan token reset kata sandi hanya dapat digunakan satu kali dan berumur singkat.

Parameter Argon2id dan nilai kebijakan kata sandi yang tepat harus ditetapkan melalui tinjauan konfigurasi/keamanan, bukan ditebak sebagai konstanta dalam kode UI.

## 7. Kontrol penyalahgunaan OTP dan verifikasi — tahap berikutnya

Apply layered limits:
- Per destination (email/phone).
- Berdasarkan akun.
- Berdasarkan IP atau sinyal jaringan, dengan retensi yang memperhatikan privasi.
- Berdasarkan challenge dan tujuan penggunaannya.
- Global provider spend/throughput guard.

Also require:
- Short code expiry.
- One-time use.
- Bounded attempts.
- Resend cooldown.
- Pembatalan challenge setelah verifikasi berhasil atau digantikan oleh permintaan baru.
- Respons seragam yang tidak mengonfirmasi keberadaan akun.
- Pemantauan kegagalan pengiriman berulang, pola enumerasi akun, dan volume permintaan OTP yang tidak wajar.

Jangan gunakan hitung mundur di sisi klien sebagai mekanisme penegakan; masa berlaku dan rate limit ditentukan server.

## 8. Validasi Google OAuth dan pengaitan akun

On callback, validate at minimum:
- State cocok dengan state sekali pakai yang diterbitkan server.
- Nonce cocok jika digunakan oleh alur yang dipilih.
- Issuer, audience/client ID, signature, dan masa berlaku token.
- Exact redirect URI.
- Subject dari penyedia tersedia dan stabil.
- Konteks role yang diminta sama dengan alur yang dimulai.

Untuk identitas Google baru, lanjutkan hanya alur pendaftaran yang terikat pada role. Verifikasi kontak tambahan bukan syarat registrasi/login tahap pertama. Identitas penyedia yang sudah terhubung hanya dapat mengautentikasi akun yang sudah ada. Jangan mengubah role atau mengaitkan akun otomatis berdasarkan kecocokan email.

Aturan pengaitan akun:
1. Wajibkan sesi terautentikasi pada akun yang sudah ada.
2. Wajibkan bukti terbaru atas kendali identitas penyedia.
3. Tampilkan penyedia yang akan dihubungkan dan minta konfirmasi.
4. Enforce uniqueness on provider + subject.
5. Audit link/unlink actions.
6. Jangan pernah menggabungkan dua record pengguna secara otomatis hanya berdasarkan kecocokan teks email.
7. Jangan izinkan pengaitan yang mengubah role utama akun.

## 9. Matriks otorisasi

| Kemampuan | Pengunjung publik | Owner | Investor | Admin |
| --- | --- | --- | --- | --- |
| Melihat halaman publik/fitur | Ya | Ya | Ya | Ya |
| Pendaftaran publik | Owner/Investor saja | — | — | Tidak |
| Portal login Owner | Tanpa sesi | Owner saja | Tolak role yang tidak sesuai | Hanya portal Admin internal |
| Portal login Investor | Tanpa sesi | Tolak role yang tidak sesuai | Investor saja | Hanya portal Admin internal |
| Mengelola profil usaha sendiri | Tidak | Sumber daya sendiri | Tidak | Hanya dengan izin tinjauan eksplisit |
| Menerbitkan posting promosi Owner | Tidak | Ya, sesuai kuota/moderasi | Tidak | Hanya moderasi |
| Menerbitkan posting pencarian kemitraan | Tidak | Tidak | Ya, sesuai kuota/moderasi Investor | Hanya moderasi |
| Menemukan peluang yang dapat diakses | Hanya pratinjau publik | Terautentikasi | Terautentikasi | Tinjauan operasional |
| Mengajukan koneksi | Tidak | Ya | Ya | Bukan sebagai pengguna biasa |
| Menerima/menolak permintaan koneksi sendiri | Tidak | Permintaan sendiri | Permintaan sendiri | Tidak boleh rutin menyamar sebagai pengguna |
| Mengirim pesan | Tidak | Hanya setelah koneksi disetujui | Hanya setelah koneksi disetujui | Tidak ada akses rutin ke chat privat |
| Melihat data bisnis privat Owner | Tidak | Data sendiri | Hanya field yang dibagikan secara eksplisit | Tinjauan berizin dengan tujuan terbatas |
| Mengelola keanggotaan | Tidak | Paket/penggunaan sendiri | Paket/penggunaan sendiri | Administrasi hak fitur |
| Melakukan provisioning Admin | Tidak | Tidak | Tidak | Hanya super-admin |

Izin Admin harus terperinci (misalnya tinjauan pengguna, verifikasi usaha, moderasi posting, operasi keanggotaan, dan pembacaan audit). Role Admin yang luas saja tidak cukup.

## 10. Konvensi respons error

Use a consistent API error envelope, for example:

```json
{
  "error": {
    "code": "AUTH_INVALID_CREDENTIALS",
    "message": "Email/nomor atau kata sandi belum sesuai.",
    "request_id": "opaque-correlation-id"
  }
}
```

Requirements:
- Stable machine-readable error codes.
- Pesan yang aman dan mudah dipahami pengguna.
- Jangan sertakan stack trace, error SQL, token penyedia, atau nilai secret.
- Sertakan ID korelasi/permintaan untuk kebutuhan dukungan.
- Jangan memberikan respons pemulihan tanpa autentikasi yang berbeda sehingga mengungkap keberadaan akun.
- Error validasi menjelaskan masalah pada kolom input yang aman tanpa mengulang nilai sensitif.

## 11. Model ancaman dan mitigasi wajib

| Ancaman | Kontrol wajib |
| --- | --- |
| Credential stuffing | Rate limit, pemantauan, penguncian aman/kontrol bertahap, pemeriksaan kata sandi yang telah bocor |
| Enumerasi akun | Respons pendaftaran/pemulihan/login yang seragam jika sesuai |
| OTP brute force (tahap berikutnya) | Short expiry, keyed hash, attempt cap, resend limits, destination/IP controls |
| Pencurian sesi | Cookie HttpOnly/Secure, hashing token saat tersimpan, rotasi, pencabutan, TLS |
| CSRF | Kebijakan SameSite serta perlindungan CSRF/origin untuk perubahan yang diautentikasi melalui cookie |
| CSRF pada login OAuth | State sekali pakai, nonce, redirect URI yang tepat, validasi callback yang ketat |
| Pengambilalihan/pengaitan akun OAuth | Bukti pengaitan eksplisit; jangan pernah menggabungkan akun hanya berdasarkan kecocokan email |
| Eskalasi hak akses | Pemeriksaan role backend, kebijakan deny-by-default, pemeriksaan kepemilikan, cakupan izin Admin |
| IDOR/akses sumber daya | Query sumber daya melalui cakupan kepemilikan/partisipasi yang berizin; pengujian negatif |
| Kompromi Admin | MFA, least privilege, sesi singkat, audit log, autentikasi ulang untuk tindakan sensitif |
| Kebocoran secret | Secret manager/konfigurasi environment, penyamaran log, tanpa secret dalam bundle frontend |
| Penyalahgunaan penyedia verifikasi | Rate limit per tujuan/global, batas biaya, pemantauan |
| Replay token reset/verifikasi | Konsumsi satu kali, masa berlaku, penyimpanan dalam bentuk hash, pembaruan atomik |

Ini merupakan dasar model ancaman, bukan pengganti uji penetrasi atau penilaian keamanan formal.

## 12. Rencana pengujian wajib

### Pengujian unit

- Password hash/verify behavior.
- Normalisasi email dan nomor telepon.
- Parsing role dan perbandingan dengan role yang diharapkan.
- Role utama tidak dapat diubah setelah akun dibuat; tidak ada alur profil, bantuan, atau Admin yang boleh mengubahnya.
- Pendaftaran terpisah diwajibkan untuk membuat akun dengan role klien lainnya.
- Email, nomor telepon, atau identitas penyedia yang duplikat tidak dapat dihubungkan ke akun kedua.
- Masa berlaku challenge verifikasi, batas percobaan, dan konsumsi satu kali (pengujian tahap verifikasi kontak berikutnya).
- Masa berlaku token reset kata sandi dan perilaku penggunaan satu kali.
- Masa berlaku dan pencabutan sesi.
- Keputusan kebijakan otorisasi.

### Pengujian integrasi API

- Owner dapat mendaftar dan login melalui portal Owner.
- Investor dapat mendaftar dan login melalui portal Investor.
- Login Owner menolak akun Investor dan tidak membuat sesi.
- Login Investor menolak akun Owner dan tidak membuat sesi.
- Pendaftaran Admin publik tidak tersedia dan tidak dapat dipanggil.
- Perilaku kontak yang belum diverifikasi sesuai dengan kebijakan yang disetujui.
- Registrasi password dapat membuat akun aktif tanpa OTP; kontak yang belum diverifikasi tidak diterima sebagai bukti pemulihan.
- Pendaftaran Google baru wajib memilih role sebelum OAuth; state/nonce salah, kedaluwarsa, atau diputar ulang ditolak.
- Google `email_verified` hanya diterima dari token yang tervalidasi; kecocokan email tidak menggabungkan akun.
- Respons pemulihan tidak mengungkap keberadaan akun.
- Reset token cannot be reused.
- Logout dan logout semua perangkat mencabut sesi.
- Suspended accounts cannot use existing sessions.
- OAuth state mismatch/expired state/invalid audience/invalid issuer are rejected.
- Subject penyedia yang duplikat tidak dapat dihubungkan ke pengguna kedua dan pengaitan memerlukan bukti terbaru atas kedua identitas.
- OAuth tidak boleh membuat akun kedua menggunakan subject penyedia yang sudah terhubung ke akun dengan role lain.
- Pengguna tanpa izin tidak dapat membaca/memperbarui usaha atau profil milik pengguna lain.
- Non-participant cannot read a conversation.
- Akses chat hanya diberikan setelah koneksi disetujui.
- Anggota Free dapat mengirim pesan setelah koneksi disetujui.

### Pengujian keamanan dan operasional

- Rate limiting is enforced across multiple application instances if horizontally scaled.
- CORS rejects untrusted origins.
- Cookie flags are correct in production configuration.
- Log menyamarkan secret dan nilai autentikasi pribadi.
- Error responses do not expose internal details.
- MFA Admin diwajibkan sebelum mengakses rute Admin yang dilindungi.
- Audit log dibuat untuk tindakan dengan hak istimewa dan tidak dapat diubah oleh pengguna Admin biasa.

### Pengujian frontend

- Halaman login Owner/Investor yang terpisah menampilkan teks dan tujuan navigasi yang benar.
- Password show/hide is accessible.
- Pesan validasi dan error terhubung dengan field terkait.
- Tombol kirim tidak dapat dipicu berulang saat proses masih berjalan.
- UI tahap pertama tidak menampilkan input OTP; dukungan OTP/paste/keyboard diuji saat tahap verifikasi kontak.
- Progres pendaftaran dan perilaku tombol kembali/lanjut dapat diprediksi.
- OAuth cancel/failure state is recoverable.
- Mobile layouts do not overflow horizontally.

## 13. Urutan implementasi

1. **Reproducibilitas dependency — selesai:** lockfile frontend/backend tersedia dan CI menggunakan instalasi dependency yang terkunci.
2. **Fondasi database — selesai:** pool PostgreSQL saat runtime, pemisahan principal aplikasi/migrator, migration Identity Core 0001–0005, serta akses terbatas untuk pembuatan akun pending dan penambahan audit event sudah tersedia. Migration produksi tetap merupakan operasi terpisah.
3. **Penyelarasan skema:** rancang migration lanjutan agar akun dapat aktif tanpa OTP dan status verifikasi kontak terpisah; uji constraint, trigger, fungsi pembuatan akun, dan reservasi identifier.
4. **Tipe domain:** definisikan User, PrimaryRole, AccountStatus, Session, dan kode error bertipe.
5. **Pendaftaran dan password login:** implementasikan registrasi Owner/Investor, Argon2id, login sesuai role, cookie sesi, logout, serta kontrol anti-penyalahgunaan.
6. **Google OAuth:** implementasikan login/pendaftaran dengan pemilihan role wajib sebelum OAuth untuk akun baru, validasi OIDC lengkap, dan pengaitan akun eksplisit.
7. **Pemulihan akun:** implementasikan hanya setelah kanal pemulihan terbukti kepemilikannya dan pengiriman token reset aman tersedia.
8. **Verifikasi kontak/OTP:** implementasikan pada tahap berikutnya, bukan sebagai prasyarat registrasi/login.
8. **Otorisasi:** tambahkan kebijakan role/sumber daya di backend dan pengujian negatif.
9. **Fondasi Admin:** provisioning, MFA, izin, dan audit trail.
10. **Penguatan keamanan:** tinjau rate limit, pengujian keamanan, logging, aksesibilitas, dan konfigurasi deployment.

Implementasikan setiap tahap melalui pull request berukuran kecil. Jangan merge sebelum CI, pengujian integrasi yang terarah, dan pemeriksaan keamanan terkait lulus.

## 14. Keputusan tersisa sebelum implementasi

Hal-hal berikut harus ditetapkan sebelum sistem siap digunakan di lingkungan produksi:

- Pemilihan penyedia email dan WhatsApp Business/API, persyaratan pengirim/template, keandalan pengiriman di Indonesia, percobaan ulang, pemantauan, batas biaya, dan penanggung jawab operasional.
- Konfigurasi klien Google OAuth, alamat callback produksi, dan pengelolaan rahasia.
- Mitigasi account squatting/penyalahgunaan identifier yang belum diverifikasi, termasuk batas registrasi, kebijakan reservasi, dan jalur bantuan yang tidak membocorkan akun.
- Parameter hashing kata sandi yang tepat dan perilaku pemulihan kredensial untuk akun yang hanya menggunakan Google jika pengaturan kata sandi lokal kelak disediakan.
- Masa retensi numerik berdasarkan kategori data, jadwal backup, dan durasi penyelesaian penghapusan. Kebijakan produk telah menetapkan masa tunggu pembatalan 30 hari; kontak utama baru dapat digunakan kembali setelah penghapusan selesai dan kontak dilepas secara sah. Kontak pemulihan bersama tetap diperbolehkan.
- Metode dan penyedia MFA Admin, serta pemetaan izin teknis untuk petugas khusus yang menangani pemeriksaan pemulihan manual. Kebijakan pemulihan berbasis risiko dan standar bukti telah disetujui.
- Penyimpanan pembatasan permintaan terdistribusi dan ambang operasional; penegakannya harus berlaku di seluruh instance aplikasi.
- Skema metadata bukti pemulihan, pembobotan sinyal, kontrol eskalasi, dan kebutuhan persetujuan dua pihak untuk kasus berisiko tinggi; dokumen identitas atau selfie bukan metode standar pemulihan.

Keputusan tahap pertama adalah password + Google OAuth tanpa OTP registrasi/login. OTP dan verifikasi kontak ditunda; lifecycle `pending_verification` 30 menit pada migration saat ini perlu diselaraskan melalui migration lanjutan. Durasi sesi dan masa tunggu penonaktifan/penghapusan tetap mengikuti kebijakan yang telah disetujui.
