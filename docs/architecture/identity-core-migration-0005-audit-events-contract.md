# Kontrak Skema Migration 0005 — Peristiwa Audit Identity Core

> Status: diimplementasikan sebagai migration SQLx dan lulus CI, termasuk pengujian integrasi PostgreSQL.
> Tanggal: 22 November 2026
> Cakupan: jejak audit identitas dan keamanan yang append-only

Dokumen ini menerjemahkan baseline audit Identity Core ke dalam skema PostgreSQL. Migration ini menyediakan penyimpanan dan perlindungan struktur data. Backend telah menyediakan API terkontrol untuk menambahkan event audit; hak INSERT aplikasi baru tersedia setelah skrip bootstrap privilege dijalankan. Event tidak ditulis otomatis tanpa pemanggilan API tersebut.

## 1. Batas implementasi

Migration `20261122130000_create_identity_audit_events.sql` menambahkan tabel `identity_audit_events` dan fungsi validasi metadata.

Cakupan event meliputi registrasi/verifikasi, perubahan identitas dan kontak, lifecycle akun, sesi autentikasi, recovery, penolakan kebijakan/role, provisioning Admin, serta tindakan istimewa yang relevan. Event code dan reason code ditentukan oleh kontrak aplikasi; migration tidak membuat katalog event yang mengunci seluruh domain.

Migration tidak menambahkan endpoint, middleware, event publisher, integrasi Admin, sink eksternal, retensi otomatis, grant runtime, maupun deployment.

## 2. Struktur event

Kolom utama:

- `id`: UUID yang dibuat database.
- `occurred_at`: waktu kejadian yang ditetapkan database saat INSERT.
- `actor_type`: `user`, `admin`, `system`, atau `operator`.
- `actor_id`: UUID aktor bila tersedia. Tidak memakai FK karena aktor system/operator dapat berasal dari principal di luar tabel users.
- `subject_user_id`: akun yang menjadi subjek event, nullable untuk event yang belum terkait akun.
- `action_code`: kode aksi terstruktur dengan format identifier.
- `outcome`: `success`, `failure`, atau `denied`.
- `reason_code`: kode alasan yang aman dan opsional.
- `request_id`: ID korelasi request, bila tersedia.
- `metadata`: objek JSONB kecil dengan key allowlist dan nilai skalar saja.

Indeks mendukung riwayat per subject, riwayat aktor, pencarian berdasarkan action, dan korelasi request. Tidak ada partisi atau retensi otomatis pada tahap ini.

## 3. Perlindungan metadata

Metadata bukan tempat menyimpan payload request atau salinan objek domain. Database membatasi:

1. Metadata harus berupa objek JSONB datar, bukan array atau scalar.
2. Key dibatasi pada allowlist seperti `channel`, `purpose`, `account_status`, `role`, `session_id`, `challenge_id`, `recovery_case_id`, `resource_type`, `resource_id`, `provider`, `policy_code`, `risk_level`, dan kode teknis terbatas lainnya.
3. Nilai hanya boleh scalar (string, number, boolean, atau null); nested object/array ditolak.
4. String dibatasi maksimal 128 karakter dan keseluruhan JSON maksimal 2 KiB.

Validasi struktur ini bukan pengganti validasi semantik runtime. Backend tetap wajib memakai allowlist per action, meminimalkan PII, dan tidak memasukkan password, OTP, session/reset token, secret, email/nomor HP mentah, isi chat, dokumen rahasia, bukti recovery, atau catatan investigasi.

Fungsi validasi metadata bersifat murni dan tidak mengakses data; hak EXECUTE-nya tidak memberikan akses baca/tulis ke tabel audit.

## 4. Append-only dan kontrol akses

Trigger menolak UPDATE, DELETE, dan TRUNCATE. Database menetapkan ID dan waktu event pada INSERT. Tabel tidak memberi privilege kepada PUBLIC.

Ketentuan privilege untuk tahap bootstrap:

- Principal runtime hanya memperoleh INSERT pada tabel audit dan hak EXECUTE minimum yang diperlukan oleh constraint metadata.
- Principal runtime tidak memperoleh UPDATE, DELETE, TRUNCATE, ownership, atau kemampuan mengubah trigger.
- Pembacaan audit dipisahkan ke principal/fitur internal yang benar-benar memerlukan akses; Admin umum tidak otomatis dapat membaca seluruh audit.
- Pemilik tabel/migration menggunakan principal terpisah dari runtime application.
- Provisioning privilege dilakukan eksplisit dan diuji pada tahap bootstrap database, bukan melalui migration ini.

Append-only berarti tidak dapat diubah oleh principal aplikasi normal. Pemilik tabel, operator database dengan hak tertinggi, atau superuser tetap berada di luar batas perlindungan trigger; operasi administratif database harus dikendalikan dan diaudit secara terpisah.

## 5. Retensi dan pengiriman eksternal

Retensi numerik ditunda sampai kebutuhan hukum, operasional, dan klasifikasi data ditinjau. Jangan menghapus event secara otomatis hanya karena tabel membesar. Rencana partisi/arsip harus mempertahankan integritas, akses terbatas, dan kemampuan investigasi yang sah.

Forwarding ke SIEM atau penyimpanan log terpisah dapat ditambahkan kemudian dengan event terpilih dan kontrol akses/retensi independen. Sink eksternal tidak menggantikan audit database.

## 6. Pengujian

Tes PostgreSQL harus membuktikan:

- Event valid diterima dan waktu/ID ditetapkan database.
- Metadata di luar allowlist, nested payload, dan format action/outcome yang tidak sah ditolak.
- Subject yang tidak ada ditolak oleh foreign key.
- UPDATE, DELETE, dan TRUNCATE ditolak.
- Migration berjalan pada PostgreSQL 16 kosong dan aman saat migrator dijalankan ulang.

Tes harus memakai database sementara yang dapat dibuang. Jangan menghubungkan tes destruktif ke database produksi.

## 7. Pekerjaan runtime yang belum termasuk

- Menghasilkan event dari alur registrasi, verifikasi, login/session, kontak, recovery, Admin, dan enforcement.
- Validasi aktor berdasarkan session/role dan penentuan subject yang benar.
- Action-specific metadata allowlist, redaksi, dan correlation ID.
- Transaksi atomik agar perubahan penting dan event audit berhasil/gagal bersama.
- Grant least-privilege, pemisahan principal pembaca, monitoring akses audit, dan bootstrap owner.
- Retensi, arsip, legal hold bila diperlukan, serta forwarding eksternal.
