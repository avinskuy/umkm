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

### Backend

- Rust dan Axum.
- Modul berorientasi domain dengan pemisahan tanggung jawab API, application, domain, dan infrastructure secara eksplisit sesuai kebutuhan kompleksitas.
- PostgreSQL direncanakan sebagai database relasional utama.
- SQLx direncanakan sebagai lapisan akses database.
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

Keputusan pendaftaran, verifikasi kontak, dan siklus hidup akun yang sudah disetujui maupun yang masih terbuka dicatat dalam [Keputusan Pendaftaran Identity Core](identity-core-registration-decisions.md). Migrasi Identity Core belum boleh dibuat sebelum seluruh persyaratan gerbang implementasinya terpenuhi.

## Skema database

Model relasional yang diusulkan, batasan integritas, batas privasi, dan urutan migrasi dijelaskan dalam [Rancangan Skema Database PostgreSQL](database-schema-design.md). Dokumen tersebut masih berupa usulan sampai tahap peninjauan migrasi pertama selesai.

## Akses data dan privasi

Pemilik bisnis menentukan informasi bisnis mana yang boleh dibagikan. Data yang ditampilkan kepada investor harus mendapat izin secara eksplisit dan tidak boleh mencakup detail transaksi pada tingkat pelanggan. Data perlu menyertakan asal-usul, periode pelaporan, waktu pembaruan, dan status verifikasi.

## Batasan arsitektur

- Tidak menggunakan microservices untuk produk tahap awal.
- Hindari folder penampung umum seperti `utils` atau `common` tanpa tanggung jawab yang jelas.
- Hindari ketergantungan melingkar antarfitur.
- Kode bersama harus memiliki lebih dari satu pengguna nyata atau tanggung jawab tingkat platform yang jelas.
- Pisahkan kebutuhan transport/API dari aturan domain.
- Jangan mempercayai klaim role atau kepemilikan yang berasal dari frontend.
