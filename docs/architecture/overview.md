# Gambaran Arsitektur

## Arah pengembangan

USahapintar dimulai dengan pendekatan **modular monolith**. Frontend dan backend merupakan aplikasi terpisah yang dapat di-deploy secara mandiri, sedangkan modul-modul domain backend tetap berada dalam satu layanan. Pendekatan ini menjaga batas tanggung jawab tetap jelas tanpa menambah kerumitan operasional yang belum diperlukan.

## Aplikasi

### Frontend

- React dan TypeScript.
- Struktur kode berorientasi fitur.
- Komponen UI umum ditempatkan terpisah dari komponen khusus domain.
- Style komponen diletakkan berdekatan dengan komponennya dan dibatasi cakupannya (gunakan CSS Modules jika sesuai).
- Style global dibatasi pada reset, design tokens, dan aturan yang benar-benar berlaku secara global.
- Implementasi frontend masih berupa shell awal; alur produk seperti profil, discovery, koneksi, pesan, membership, dan Admin belum tersedia.

### Backend

- Rust dan Axum.
- Modul berorientasi domain dengan pemisahan tanggung jawab API, application, domain, dan infrastructure secara eksplisit sesuai kebutuhan kompleksitas.
- PostgreSQL 16 menjadi baseline database relasional.
- Fondasi runtime SQLx 0.8 sudah tersedia. HTTP server membuat connection pool saat startup menggunakan `APP_DATABASE_URL` milik principal aplikasi dengan hak minimum.
- Endpoint `GET /api/v1/health` menyediakan liveness check yang tidak bergantung pada database. Endpoint `GET /api/v1/readyz` memeriksa kesiapan koneksi database melalui pool runtime.
- Migration dijalankan secara eksplisit menggunakan principal migrator yang terpisah. HTTP server tidak menjalankan migration otomatis saat startup.
- Modul `identity` menyediakan hashing password, `VerificationService` untuk registrasi email/WhatsApp dengan OTP, dan router HTTP registrasi/resend/verifikasi yang diuji tetapi belum dipasang pada aplikasi HTTP default. Kebijakan produk mewajibkan OTP saat registrasi; login dan login kembali setelah logout menggunakan password tanpa OTP rutin. Login password Owner/Investor dan logout sudah tersedia dengan cookie aman, sesi server-side, serta rate limit PostgreSQL. Middleware validasi sesi, pembaruan idle timeout, refresh/rotasi token, dan logout semua perangkat tersedia pada endpoint identitas. Penerapan middleware pada endpoint fitur, adapter provider nyata, dan runtime recovery belum tersedia.
- Otorisasi ditegakkan di sisi server.

## Batas domain awal

- Identitas dan autentikasi
- Profil pemilik usaha dan bisnis
- Profil investor
- Peluang dan permintaan kemitraan
- Koneksi dan pesan
- Keanggotaan
- Data dan verifikasi bisnis
- Asisten dan intelijen berbasis AI
- Moderasi dan administrasi

Daftar ini menunjukkan batas tanggung jawab secara logis, bukan kewajiban untuk membuat modul kosong sejak awal. Buat modul ketika sudah ada perilaku nyata dan kontrak yang jelas.

## Keputusan Identity Core

Keputusan pendaftaran, verifikasi kontak, dan siklus hidup akun yang sudah disetujui maupun yang masih terbuka dicatat dalam [Keputusan Pendaftaran Identity Core](identity-core-registration-decisions.md). Skema Identity Core tersedia melalui migration 0001–0013, mencakup akun, verifikasi, sesi, pemulihan, audit, penghapusan dukungan OAuth, operasi database verifikasi, pengikatan ID verifikasi, audit runtime verifikasi, profil dan penerimaan kebijakan, serta kebijakan resend OTP saat lock verifikasi. Runtime menyediakan orkestrasi registrasi dan verifikasi OTP. Login dan login kembali setelah logout ditetapkan menggunakan password tanpa OTP rutin, sementara login/logout dan pembuatan sesi tersedia melalui runtime; middleware sesi, refresh token, dan logout semua perangkat tersedia pada endpoint identitas; penerapan pada endpoint fitur, adapter provider nyata, dan recovery runtime belum tersedia.

## Skema database

Model relasional, batasan integritas, batas privasi, dan urutan migrasi dijelaskan dalam [Rancangan Skema Database PostgreSQL](database-schema-design.md). Migration Identity Core 0001–0013 tersedia dan tercakup dalam pengujian PostgreSQL di CI. Migration tersebut mencakup akun, verifikasi, sesi, pemulihan, audit, serta operasi runtime registrasi; migration domain produk lainnya belum tersedia. Penerapan migration dilakukan secara eksplisit melalui principal migrator dan tetap terpisah dari startup HTTP maupun deployment produksi.

## Akses data dan privasi

Pemilik bisnis menentukan informasi bisnis mana yang boleh dibagikan. Data yang ditampilkan kepada investor harus mendapat izin secara eksplisit dan tidak boleh mencakup detail transaksi pada tingkat pelanggan. Data perlu menyertakan asal-usul, periode pelaporan, waktu pembaruan, dan status verifikasi.

## Batasan arsitektur

- Tidak menggunakan microservices untuk produk tahap awal.
- Hindari folder penampung umum seperti `utils` atau `common` tanpa tanggung jawab yang jelas.
- Hindari ketergantungan melingkar antarfitur.
- Kode bersama harus memiliki lebih dari satu pengguna nyata atau tanggung jawab tingkat platform yang jelas.
- Pisahkan kebutuhan transport/API dari aturan domain.
- Jangan mempercayai klaim role atau kepemilikan yang berasal dari frontend.
