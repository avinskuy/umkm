# Kontrak Skema Migration 0004 — Pemulihan Akun

> Status: diimplementasikan sebagai migration SQLx dan lulus CI, termasuk pengujian integrasi PostgreSQL.
> Tanggal: 22 November 2026
> Cakupan: kontak pemulihan bersama, kasus pemulihan akun, dan token reset kata sandi

Dokumen ini menerjemahkan kebijakan pemulihan akun yang telah disetujui ke dalam kontrak PostgreSQL. Migration ini tidak mengaktifkan endpoint, provider, runtime kriptografi, atau pemulihan akun di aplikasi.

## 1. Batas implementasi

Migration `20261122120000_create_account_recovery.sql` menambahkan:

- `recovery_contacts` — kontak pemulihan opsional yang telah diverifikasi dan dapat dipakai pada lebih dari satu akun.
- `account_recovery_cases` — satu kasus untuk satu sesi verifikasi pemulihan, termasuk pemilihan akun setelah kontak terverifikasi.
- `password_reset_tokens` — hash token reset berumur terbatas, terikat pada satu kasus dan satu akun.

Migration tidak mengubah migration 0001–0003, tabel identitas utama, atau kebijakan role. Tidak ada grant, key kriptografi, data pengguna, kredensial, provider email/WhatsApp, endpoint, UI, atau konfigurasi produksi yang ditambahkan.

## 2. `recovery_contacts`

Kontak pemulihan bersifat opsional, ditambahkan dari pengaturan keamanan akun aktif, dan harus diverifikasi secara terpisah.

Kolom utama:

- `user_id`, `channel`: pemilik relasi dan kanal `email` atau `whatsapp`.
- `contact_ciphertext`, `contact_nonce`, `encryption_key_version`: representasi terenkripsi untuk kebutuhan pengiriman/pemrosesan.
- `contact_hmac`, `hmac_key_version`: fingerprint untuk pencarian dan deduplikasi.
- `verification_session_id`: sesi verifikasi `recovery_contact_add` yang menjadi bukti verifikasi.
- `verified_at`, `created_at`, `revoked_at`: waktu verifikasi, pembuatan, dan pencabutan.

Invariant:

1. Insert hanya diterima bila sesi verifikasi sudah `verified`, purpose-nya `recovery_contact_add`, akun dan kanal cocok, serta HMAC dan versi key sama dengan sesi.
2. Database menetapkan `verified_at` dari sesi verifikasi; caller tidak dapat mengklaim waktu verifikasi sendiri.
3. Unique partial index mencegah satu akun memiliki dua kontak aktif dengan kanal dan fingerprint yang sama. Indeks non-unik terpisah mendukung pencarian fingerprint setelah verifikasi untuk menemukan akun-akun yang terkait.
4. Tidak ada unique constraint global pada fingerprint. Kontak yang sama boleh terhubung ke beberapa akun; hal ini hanya membuktikan kendali atas kontak, bukan kepemilikan semua akun.
5. Data identitas dan bukti verifikasi tidak dapat diubah setelah dibuat. Pencabutan bersifat satu arah; penambahan kembali memerlukan verifikasi baru.
6. Kontak utama pendaftaran tetap mengikuti unique reservation pada `users`; kebijakan berbagi ini hanya berlaku untuk kontak pemulihan opsional.

Rotasi HMAC harus memakai prosedur dual-read/backfill yang tidak membuka celah duplikasi atau bypass limit. Karena indeks deduplikasi memakai fingerprint yang tersimpan, runtime harus memeriksa versi fingerprint lama dan aktif selama masa transisi.

## 3. `account_recovery_cases`

Satu `account_recovery_cases` terikat unik pada satu `verification_sessions` dengan purpose `account_recovery`. Kasus dibuat sebelum kontak diverifikasi, tanpa mengikat akun target dan tanpa mengungkap daftar akun.

Kolom utama:

- `verification_session_id`: sesi kontak pemulihan pra-pemilihan.
- `target_user_id`: nullable sampai kontak diverifikasi dan pengguna memilih akun.
- `requested_action`: `password_reset`, `forgot_email`, atau `account_access`.
- `status`: state machine kasus.
- `created_at`, `updated_at`, `resolved_at`: waktu lifecycle.

Status dan transisi yang diizinkan:

| Status | Makna |
| --- | --- |
| `pending_verification` | Menunggu verifikasi kontak; target belum dipilih. |
| `account_selection_required` | Kontak terverifikasi; pengguna perlu memilih akun yang aman ditampilkan. |
| `under_review` | Pemeriksaan risiko atau bukti kepemilikan sedang berlangsung. |
| `approved` | Pemeriksaan selesai dan kasus disetujui; target akun wajib sudah ditetapkan. |
| `rejected` | Kasus ditolak; terminal. |
| `completed` | Proses pemulihan selesai; terminal. |
| `cancelled` | Pengguna atau proses berwenang membatalkan; terminal. |
| `expired` | Kasus kedaluwarsa; terminal. |

Kasus tidak dapat maju dari `pending_verification` sebelum sesi kontak berstatus `verified`. Pemilihan target hanya dapat dilakukan setelah sesi terverifikasi dan `verification_sessions.user_id` diikat ke akun yang sama. Target yang telah dipilih tidak dapat diganti. Database memastikan konsistensi binding tersebut, tetapi runtime tetap wajib membuktikan bahwa fingerprint kontak terverifikasi benar-benar cocok dengan kontak utama atau kontak pemulihan milik akun target. Jangan memakai HMAC sebagai bukti kepemilikan dengan sendirinya.

Pemeriksaan risiko dapat mempertimbangkan sesi/perangkat tepercaya, perubahan kontak terbaru, sinyal kepemilikan lain, dan tinjauan manual. Bukti tidak cukup dapat menyebabkan penundaan atau penolakan. KTP/identitas resmi atau selfie bukan persyaratan standar; bukti Premium tidak pernah cukup sebagai satu-satunya dasar pemulihan.

## 4. `password_reset_tokens`

Token reset disimpan hanya sebagai hash SHA-256 32-byte dari token acak berentropi tinggi. Token plaintext hanya boleh ditampilkan/dikirim sekali oleh runtime dan tidak boleh disimpan atau dicatat.

Invariant:

1. Token harus terikat pada pasangan `recovery_case_id` dan `user_id` yang sama melalui composite foreign key.
2. Token hanya dapat dibuat untuk kasus berstatus `approved`, dengan action `password_reset`, target yang sama, dan akun yang masih aktif.
3. Hash token unik secara global; token hanya dapat dikonsumsi sekali dan hanya sebelum `expires_at`.
4. Token dapat dicabut satu arah dengan alasan terbatas: `superseded`, `security_event`, `account_status_changed`, atau `admin_action`.
5. Maksimal satu token yang belum dikonsumsi/dicabut per kasus. Sebelum menerbitkan token pengganti setelah token kedaluwarsa, runtime harus terlebih dahulu mencabut token lama agar indeks unik tidak menghalangi penerbitan ulang.
6. Masa berlaku spesifik token ditetapkan runtime melalui konfigurasi yang ditinjau; migration memastikan deadline di masa depan tetapi tidak menetapkan durasi produk yang belum diputuskan.

Runtime harus mengonsumsi token, mengganti password hash, menyelesaikan kasus, mencabut sesi autentikasi dengan alasan `password_reset`, dan menulis audit event dalam transaksi yang konsisten. Kegagalan di salah satu langkah tidak boleh meninggalkan token terpakai tanpa perubahan kredensial yang berhasil atau sebaliknya.

## 5. Tanggung jawab runtime yang belum termasuk

- Normalisasi email/nomor HP, AEAD, HMAC, key provider, rotasi key, dan akses dekripsi terbatas.
- Pembuktian bahwa kontak yang diverifikasi cocok dengan akun target, account-selection UX yang tidak melakukan enumeration, serta respons generik sebelum verifikasi.
- Rate limit lintas sesi/kanal/instance, kontrol abuse, expiry cleanup, dan transaksi atomik untuk verifikasi/resend/recovery.
- Pemeriksaan risiko, perangkat tepercaya, manual review, notifikasi keamanan, dan audit event.
- Endpoint lupa password/lupa email, penerbitan/pengiriman token, konsumsi token, hashing password, pencabutan sesi, dan UX.
- Grant database least-privilege untuk principal aplikasi, migration, backup, monitoring, dan Admin.

Migration dijalankan eksplisit melalui migrator yang sudah ada. HTTP server belum menjalankan migration pada startup. Jangan jalankan migration ini pada database produksi sebagai bagian dari PR ini.
