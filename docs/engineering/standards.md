# Standar Engineering

## Umum

- Utamakan solusi yang sederhana, eksplisit, dan mudah dipelihara.
- Setiap file harus memiliki satu tanggung jawab utama yang jelas.
- Jaga agar modul tetap kohesif dan API tidak terlalu besar.
- Hindari abstraksi yang dibuat terlalu dini serta direktori placeholder yang belum memiliki fungsi.
- Hindari file raksasa; pisahkan berdasarkan tanggung jawab ketika modul mulai sulit dipahami atau dinavigasi.
- Jangan menggandakan aturan bisnis antara frontend dan backend. Backend menjadi sumber kebenaran.

## Penamaan

- Komponen React dan nama file komponennya: PascalCase.
- Hooks: `useSomething`.
- Variabel dan fungsi TypeScript: camelCase.
- Modul, fungsi, dan variabel Rust: snake_case.
- Tipe dan trait Rust: PascalCase.
- Gunakan istilah domain secara konsisten.

## Frontend

- Aktifkan TypeScript strict mode.
- Gunakan struktur feature-first di dalam `src/features/<feature>`.
- Tempatkan komponen dasar yang dapat digunakan ulang dan tidak terikat domain di `src/shared/ui`.
- Tempatkan komposisi route di `src/app`.
- Tempatkan akses API dan integrasi eksternal dalam modul service dengan tanggung jawab yang jelas.
- Utamakan CSS Modules untuk style komponen dan halaman.
- Batasi CSS global pada design tokens, reset, dan aturan dokumen global.
- Hindari satu stylesheet seluruh aplikasi yang berisi selector khusus untuk banyak fitur.
- Komponen tidak boleh menangani urusan jaringan, domain, dan presentasi yang tidak saling berkaitan.
- Sediakan label yang aksesibel, dukungan keyboard, serta state loading, error, dan empty yang jelas.
- Terapkan prinsip pengalaman pengguna yang inklusif dalam [Prinsip Pengalaman Pengguna yang Inklusif](../product/prinsip-pengalaman-pengguna.md).

## Backend

- Jaga handler tetap tipis: baca input transport, panggil perilaku application, lalu petakan hasilnya ke HTTP.
- Validasi input yang tidak tepercaya pada batas masuk sistem.
- Tegakkan otorisasi dan kepemilikan sumber daya pada jalur application backend.
- Gunakan error bertipe dan respons API yang konsisten.
- Hindari `unwrap()` dan `expect()` pada jalur request normal.
- Tempatkan query database dan client penyedia eksternal di balik batas infrastructure.
- Jangan pernah mengembalikan rahasia internal atau field sensitif.
- Pesan error yang ditampilkan kepada pengguna harus jelas, dapat ditindaklanjuti, dan tidak membocorkan detail internal atau keberadaan akun.

## Kepemilikan style

- Design tokens global: `src/styles/tokens/`.
- Reset dan style dokumen global: `src/styles/base/`.
- Style komponen: file `.module.css` yang diletakkan berdekatan dengan komponennya.
- Style halaman fitur: diletakkan bersama halaman atau fitur yang memilikinya.
- Jangan membuat stylesheet global baru hanya untuk memperbaiki masalah pada satu komponen lokal.

## Pengujian

- Uji logika domain dan perilaku UI yang dapat digunakan ulang melalui unit test.
- Uji API, persistence, dan batas otorisasi melalui integration test.
- Tambahkan regression test untuk setiap bug yang diperbaiki.
- Uji akses role/sumber daya yang diizinkan maupun yang harus ditolak.
- Tambahkan E2E test untuk alur pengguna yang penting ketika alur tersebut sudah tersedia.
- Uji alur penting dari sudut pandang pengguna baru dan pengguna dengan tingkat literasi digital yang beragam; catat hambatan dan perbaikannya.
- Periksa aksesibilitas keyboard, fokus, label, kontras, dan pembesaran teks pada komponen serta alur penting.

## Gerbang kualitas

Jalankan pemeriksaan yang relevan sebelum membuka pull request:

- Frontend: pemeriksaan format, lint, typecheck, test, dan production build.
- Backend: rustfmt, Clippy dengan peringatan dianggap gagal, test, dan build.
- Tinjau perubahan dependency dan lockfile yang dihasilkan.
- Seluruh CI harus berstatus hijau sebelum merge.
- Fitur yang menyentuh alur utama pengguna harus memiliki bukti pengujian usability yang relevan atau mencatat keterbatasan serta rencana pengujian lanjutan.

## Standar keamanan, privasi, dan tata kelola data

Kebijakan keamanan dan perlindungan data dirangkum dalam [Indeks Kebijakan Keamanan, Privasi & Perlindungan Data](../security/README.md). Cakupannya meliputi kontrol akses, klasifikasi L1–L4, enkripsi, logging dan audit, API, database, infrastruktur, supply chain, privasi, persetujuan, retensi, pembagian data pihak ketiga, notifikasi keamanan, kewenangan Admin, moderasi, banding, dan transparansi aktivitas akun.

Persetujuan kebijakan merupakan baseline desain dan implementasi; hal tersebut tidak otomatis membuktikan kontrol telah diterapkan, diuji, atau tersertifikasi. Setiap perubahan harus mengikuti ruang lingkup, prioritas risiko, dan bukti penerapan yang relevan.

## Standar kualitas dan operasional

- [Strategi Pengujian & Quality Assurance](testing-strategy-quality-assurance.md) mengatur tingkat pengujian, security testing, regression, accessibility, bukti QA, dan release quality gate.
- [Performance, Scalability & Reliability](performance-scalability-reliability.md) mengatur pengukuran, kapasitas, ketahanan terhadap kegagalan, dan optimasi berbasis bukti.
- [Secure Software Development Lifecycle](secure-software-development-lifecycle.md) mengatur integrasi keamanan dari perencanaan sampai pemeliharaan.
- [Kebijakan Keamanan Dependency & Komponen Pihak Ketiga](../security/dependency-third-party-security.md) mengatur supply chain software, package, SDK, dan komponen vendor.
- [Kesiapan Operasional](../operations/README.md) mengatur respons insiden, kesinambungan layanan, observability, release/change control, dan runbook operasional.
- [Pemeliharaan Repository](repository-maintenance.md) menetapkan praktik audit, dokumentasi, kualitas kode, dan pemeriksaan sebelum merge.

Status implementasi setiap kontrol harus dibuktikan melalui kode, konfigurasi, hasil pengujian, atau catatan operasional yang relevan. Dokumen kebijakan dan prosedur tidak dengan sendirinya membuktikan kontrol sudah berjalan.
