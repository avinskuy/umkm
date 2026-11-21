# Kebijakan Enkripsi & Manajemen Kunci

- **Status:** Disetujui sebagai baseline
- **Tanggal:** 21 November 2026
- **Cakupan:** Komunikasi, penyimpanan, password, token, secret, dan kunci kriptografi

## 1. Enkripsi saat transit

- Komunikasi aplikasi melalui jaringan wajib menggunakan HTTPS/TLS.
- Komunikasi antarlayanan dilindungi sesuai risiko dan topologi deployment.
- Sertifikat dan konfigurasi TLS dikelola dengan aman; protokol dan cipher lemah dinonaktifkan sesuai dukungan platform.
- Jangan mengirim kredensial atau data sensitif melalui kanal yang tidak terlindungi.

## 2. Enkripsi saat tersimpan

- Database, file privat, backup, dan penyimpanan data sensitif dienkripsi sesuai klasifikasi dan risiko.
- L3 dan L4 memerlukan perlindungan yang lebih ketat, termasuk pembatasan akses dan pengelolaan kunci.
- Enkripsi tidak menggantikan otorisasi, minimisasi data, atau pengelolaan siklus hidup.
- Jangan menganggap seluruh penyimpanan otomatis terenkripsi tanpa memverifikasi konfigurasi layanan yang digunakan.

## 3. Password, OTP, dan token

- Password disimpan menggunakan password hashing yang kuat dan sesuai praktik terkini, bukan plaintext atau enkripsi yang dapat dibalik.
- Gunakan parameter hashing yang ditinjau dan dapat ditingkatkan mengikuti kemampuan platform.
- OTP dan token sensitif tidak disimpan dalam bentuk plaintext yang dapat digunakan kembali.
- Token sesi dan reset disimpan dalam bentuk hash atau representasi terlindungi yang sesuai dengan desain token.
- Jangan pernah menulis password, OTP, token sesi, reset token, atau secret ke log.

## 4. Secret dan kunci

- Secret tidak boleh di-hardcode atau di-commit ke repository.
- Pisahkan secret development, test, staging, dan production.
- Pisahkan kunci dari data terenkripsi dan batasi akses ke kunci berdasarkan kebutuhan.
- Catat akses dan perubahan kunci tanpa mencatat nilai rahasianya.
- Tetapkan prosedur pembuatan, penyimpanan, rotasi, pencabutan, backup, dan pemulihan kunci.
- Jika kunci diduga bocor, batasi penggunaan, cabut atau rotasi, nilai dampak data, dan ikuti prosedur respons insiden.
- Gunakan pengelola secret/KMS yang sesuai ketika infrastruktur dan kebutuhan sudah ditentukan.

## 5. Pemilihan teknologi

Algoritma, ukuran kunci, KMS, penyedia sertifikat, dan jadwal rotasi final ditetapkan pada tahap implementasi berdasarkan rekomendasi resmi yang berlaku, dukungan platform, serta peninjauan keamanan. Jangan mengarang konfigurasi kriptografi sendiri atau menganggap satu konfigurasi cocok untuk semua data.

## 6. Pengujian

- Verifikasi HTTPS/TLS dan konfigurasi sertifikat.
- Pastikan secret tidak muncul pada repository, log, artefak, atau pesan error.
- Uji pemisahan secret antar-environment.
- Uji pencabutan sesi/token dan prosedur rotasi kunci.
- Verifikasi backup dan file privat terlindungi sesuai klasifikasi.
- Tinjau proses pemulihan kunci agar pemulihan tidak melemahkan kontrol akses.

## Referensi

- NIST SP 800-57, Recommendation for Key Management.
- OWASP Cryptographic Storage Cheat Sheet.
- OWASP Password Storage Cheat Sheet.
- NIST SP 800-53.
