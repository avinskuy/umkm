# Provisioning Principal dan Privilege PostgreSQL

- **Status:** baseline implementasi untuk database khusus USahapintar
- **Tanggal:** 22 November 2026
- **Cakupan:** PostgreSQL 16, SQLx, Identity Core migrations 0001–0005
- **Penerapan produksi:** belum dilakukan

## 1. Tujuan dan batas aman

Provisioning role PostgreSQL adalah operasi tingkat cluster/database, bukan SQLx schema migration. Skrip di `backend/database/bootstrap/` dijalankan secara eksplisit oleh DBA pada database khusus aplikasi. Jangan menjalankannya pada database bersama atau produksi tanpa rencana perubahan, backup, dan persetujuan deployment terpisah.

Password tidak ditulis di repository. Skrip membuat login role tanpa password; kredensial harus ditetapkan melalui secret manager atau prosedur interaktif `psql \\password`. Jangan menaruh password production di command history, file SQL, CI log, atau commit.

## 2. Principal

| Principal | Jenis | Tanggung jawab | Batas |
| --- | --- | --- | --- |
| `usahapintar_app` | NOLOGIN group | Hak minimum aplikasi | Tidak memiliki tabel; hanya hak eksplisit |
| `usahapintar_app_login` | LOGIN | Koneksi runtime aplikasi | Mewarisi hak group aplikasi; bukan owner |
| `usahapintar_migrator` | LOGIN | Menjalankan SQLx migrations | CREATE pada schema `public`; bukan superuser dan bukan pemilik cluster |
| `usahapintar_backup` | NOLOGIN group | Membaca data untuk backup | Mewarisi `pg_read_all_data`; tidak BYPASSRLS dan tidak punya hak tulis |
| `usahapintar_backup_login` | LOGIN | Koneksi proses backup | Hanya anggota group backup |
| `usahapintar_monitor` | NOLOGIN group | Monitoring PostgreSQL | Mewarisi `pg_monitor`; tidak mendapat akses tabel aplikasi |
| `usahapintar_monitor_login` | LOGIN | Koneksi monitoring | Hanya anggota group monitoring |
| `usahapintar_audit_reader` | NOLOGIN group | Pembacaan audit terbatas | SELECT hanya pada audit event; belum diberikan kepada akun Admin |
| DBA | Dikelola di luar skrip | Administrasi cluster dan pemulihan | Provisioning terpisah, MFA/akses jaringan/audit sesuai kebijakan |

Jangan memakai satu credential untuk aplikasi, migrasi, backup, monitoring, dan DBA. Tidak ada role aplikasi yang boleh memiliki `SUPERUSER`, `CREATEDB`, `CREATEROLE`, atau `BYPASSRLS`.

## 3. Urutan provisioning

1. Buat database khusus aplikasi dan siapkan principal DBA.
2. Jalankan `001_create_principals.sql` sebagai DBA pada database target. Skrip ini idempotent untuk role/privilege yang dikelolanya.
3. Tetapkan password setiap LOGIN role dari secret manager atau melalui sesi `psql` interaktif `\\password role_name`. Untuk autentikasi production, utamakan SCRAM-SHA-256 atau sertifikat sesuai kebijakan jaringan.
4. Jalankan seluruh SQLx migrations sebagai `usahapintar_migrator`, bukan superuser/DBA. Dengan demikian tabel dan fungsi dimiliki migrator yang tidak memiliki hak superuser.
5. Setelah Identity Core 0001–0005 terpasang, jalankan `002_identity_core_grants.sql` sebagai DBA. Skrip menolak melanjutkan bila `create_client_user`, `users`, atau `identity_audit_events` bukan milik migrator non-superuser.
6. Verifikasi koneksi dan privilege memakai tes PostgreSQL pada CI atau database disposable sebelum perubahan lingkungan bersama.

Untuk database yang sudah dimigrasikan oleh superuser/owner lain, jangan langsung memberikan EXECUTE pada fungsi `SECURITY DEFINER`. Skrip 002 sengaja gagal tertutup. Lakukan rencana transfer ownership yang ditinjau dan diuji secara terpisah; jangan melakukan `REASSIGN OWNED` secara membabi buta.

## 4. Hak akses runtime yang dibuka

Aplikasi mendapat:
- `USAGE` pada schema `public`.
- `EXECUTE` pada `create_client_user(...)` setelah guard memastikan fungsi dimiliki migrator non-superuser.
- `INSERT` pada `identity_audit_events`.
- `EXECUTE` pada validator metadata audit yang dipakai constraint.

Aplikasi **tidak** mendapat SELECT/INSERT/UPDATE/DELETE/TRUNCATE langsung pada tabel akun, identitas provider, verifikasi, sesi, kontak pemulihan, kasus recovery, token reset, atau audit. Aplikasi juga tidak mendapat CREATE pada schema, akses fungsi trigger, atau hak baca audit. Akses domain lain harus ditambahkan melalui grant eksplisit per use case setelah kontrak runtime dan tes otorisasi tersedia.

Fungsi `create_client_user` adalah SECURITY DEFINER dan hanya aman diberikan kepada aplikasi jika owner-nya `usahapintar_migrator` yang bukan superuser/BYPASSRLS/CREATEDB/CREATEROLE serta memiliki tabel terkait. Jangan pernah menjalankan migration production dengan superuser.

## 5. Backup, monitoring, dan audit reader

- `usahapintar_backup_login` menjadi anggota `usahapintar_backup`, yang mewarisi role bawaan `pg_read_all_data`. Ini memberikan akses baca lintas tabel/sequences, bukan tulis dan bukan BYPASSRLS. Perilaku backup terhadap row-level security harus diuji; jangan menambah BYPASSRLS otomatis.
- `usahapintar_monitor_login` menjadi anggota `usahapintar_monitor`, yang mewarisi `pg_monitor`. Role ini ditujukan untuk statistik/monitoring, bukan membaca data bisnis. Karena `pg_monitor` juga mencakup akses baca konfigurasi dan statistik yang lebih luas, batasi host, credential, dan operatornya; jangan menyimpan secret di parameter konfigurasi PostgreSQL.
- `usahapintar_audit_reader` adalah NOLOGIN dan hanya dapat membaca `identity_audit_events`. Belum ada anggota; pemberian akses kepada fungsi/Admin tertentu memerlukan keputusan dan grant terpisah.

## 6. Default privilege dan deny-by-default

Skrip mencabut CONNECT/TEMPORARY dari PUBLIC pada database target, mencabut akses PUBLIC ke schema `public` dan objek yang sudah ada, serta mengatur default privilege untuk objek baru yang dibuat oleh migrator. Aplikasi diberi CONNECT secara eksplisit. Database owner/DBA tetap menjadi jalur administrasi.

Default privilege berlaku untuk objek yang dibuat oleh role migrator tersebut, bukan objek yang dibuat role lain. Semua migration produksi karena itu harus berjalan sebagai `usahapintar_migrator`.

## 7. CI dan pengujian

CI memakai tiga identitas terpisah:
- DBA sementara untuk membuat database disposable dan menjalankan skrip bootstrap.
- `usahapintar_migrator` non-superuser untuk SQLx migrations dan tes invariant schema.
- `usahapintar_app_login` untuk membuktikan hak runtime terbatas.

Tes harus membuktikan aplikasi dapat memanggil fungsi pembuatan akun dan menulis audit, tetapi tidak dapat membaca/mengubah tabel identitas secara langsung, membaca audit, membuat objek, atau memakai privilege cluster. Backup hanya membaca, monitoring tidak membaca tabel aplikasi, dan migrator bukan superuser.

## 8. Belum termasuk

Provisioning ini tidak mengaktifkan HTTP database startup, endpoint autentikasi, login/session runtime, OTP provider, Admin console, backup scheduler, monitoring exporter, rotasi password otomatis, RLS policy domain, production migration, atau deployment. Hak baca audit untuk Admin dan grant CRUD domain harus ditinjau secara terpisah.
