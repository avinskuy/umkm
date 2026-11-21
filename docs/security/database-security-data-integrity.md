# Database Security & Data Integrity Policy

- **Status:** Disetujui sebagai baseline
- **Tanggal:** 21 November 2026
- **Cakupan:** PostgreSQL, SQLx, database access, schema, data, backup, dan pemulihan

## 1. Referensi utama

- PostgreSQL Official Security Documentation.
- CIS PostgreSQL Benchmark.
- OWASP Database Security Cheat Sheet.
- NIST SP 800-53.

## 2. Database access control

- Terapkan least privilege dan deny-by-default.
- Pisahkan identitas/role untuk aplikasi, migration, backup, monitoring, dan administrasi database.
- Akun aplikasi tidak boleh menjadi superuser.
- Batasi akses jaringan database hanya untuk host dan layanan yang diperlukan.
- Pisahkan kredensial setiap environment dan simpan sebagai secret.
- Tinjau privilege dan cabut akses yang tidak lagi dibutuhkan.

## 3. Hardening PostgreSQL

- Gunakan versi PostgreSQL yang masih didukung dan patch keamanan yang relevan.
- Terapkan konfigurasi aman berdasarkan CIS Benchmark dan dokumentasi resmi.
- Nonaktifkan fitur, extension, akun, dan koneksi yang tidak diperlukan.
- Gunakan autentikasi kuat dan koneksi terenkripsi sesuai topologi.
- Lindungi file konfigurasi, data, WAL, dan backup.
- Jangan membuka database langsung ke internet publik tanpa kebutuhan dan tinjauan risiko yang jelas.

## 4. Data integrity dan transaksi

- Gunakan primary key, foreign key, unique constraint, check constraint, dan NOT NULL sesuai aturan domain.
- Terapkan transaksi untuk operasi yang harus berhasil atau gagal sebagai satu kesatuan.
- Pilih isolation level berdasarkan kebutuhan konsistensi dan pola konkurensi.
- Lindungi operasi sensitif dari race condition melalui constraint, locking, atau mekanisme atomik yang sesuai.
- Validasi di backend dan tegakkan invariant penting di database; validasi frontend bukan kontrol integritas.
- Perubahan data harus mempertahankan relasi, kepemilikan, dan aturan lifecycle.

## 5. SQL safety

- Gunakan parameterized query melalui SQLx.
- Jangan menggabungkan input pengguna langsung ke string SQL.
- Untuk identifier atau bagian query yang tidak dapat diparameterisasi, gunakan allowlist yang eksplisit.
- Hindari hak database yang lebih luas daripada kebutuhan query aplikasi.
- Jangan menampilkan query internal atau detail database dalam respons error publik.

## 6. Schema dan migration

- Semua perubahan schema dikelola melalui migration berversi.
- Migration yang sudah diterapkan pada database bersama tidak boleh diedit; buat migration baru.
- Tinjau dan uji migration sebelum penerapan.
- Pertimbangkan kompatibilitas aplikasi, ukuran data, locking, dan strategi perubahan bertahap.
- Siapkan prosedur pemulihan ketika migration gagal; jangan menganggap rollback otomatis selalu aman.
- Perubahan langsung di production harus dihindari dan, jika keadaan darurat mengharuskannya, dicatat serta direkonsiliasi ke migration resmi.

## 7. Data classification

Perlindungan database mengikuti klasifikasi L1–L4. L3 dan L4 memerlukan kontrol akses dan perlindungan lebih ketat. Jangan menyimpan secret, OTP plaintext, atau token yang dapat digunakan kembali. Lihat [Data Classification & Data Sensitivity Policy](data-classification.md).

## 8. Backup & restore

- Enkripsi backup dan batasi aksesnya.
- Simpan salinan terpisah dari lingkungan production.
- Dokumentasikan cakupan, retensi, dan tanggung jawab backup.
- Uji restore serta verifikasi integritas dan konsistensi data.
- Pastikan proses restore tidak mengaktifkan kembali data yang sudah dihapus atau akun yang sudah ditutup.
- Sinkronkan proses backup/restore dengan kebijakan BCDR dan data retention.

## 9. Audit dan monitoring database

Pantau akses dan aktivitas penting, perubahan privilege, perubahan schema, kegagalan autentikasi, serta pola akses tidak biasa. Lindungi audit trail. Jangan mencatat data L3/L4 yang tidak diperlukan, password, OTP, token, atau secret.

## 10. Pengujian wajib

- Uji role dan privilege database.
- SQL injection dan query parameterization.
- Constraint, relasi, dan invariant data.
- Transaksi bersamaan dan race condition.
- Migration pada database pengujian yang bersih dan pada versi schema sebelumnya.
- Backup restoration dan pemeriksaan integritas.
- Isolasi data antar-akun serta akses L3/L4.
- Kegagalan koneksi, timeout, dan penanganan error.

## 11. Penerapan bertahap

Prioritas awal: role database terpisah, hardening dasar, query SQLx yang aman, migration berversi, constraint yang tepat, backup terenkripsi, dan automated database tests. Monitoring lanjutan dan isolasi tambahan diterapkan sesuai risiko dan kemampuan infrastruktur.

## Referensi

- PostgreSQL Official Security Documentation.
- CIS PostgreSQL Benchmark.
- OWASP Database Security Cheat Sheet.
- NIST SP 800-53.
