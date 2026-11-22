# Pemeliharaan Repository

## Tujuan

Menjaga repository tetap mudah dipahami, diuji, ditinjau, dan dikembangkan tanpa duplikasi kode atau dokumentasi yang tidak sesuai dengan implementasi.

## Struktur

- `frontend/` berisi aplikasi web dan pengujian frontend.
- `backend/` berisi API, modul domain, migration, bootstrap database, dan pengujian backend.
- `docs/` berisi spesifikasi produk, arsitektur, kebijakan keamanan, standar engineering, dan prosedur operasional.
- Hindari folder placeholder dan direktori generik tanpa tanggung jawab yang jelas.

## Audit sebelum perubahan

Sebelum menambah atau mengubah implementasi:

1. Periksa implementasi, abstraksi, endpoint, migration, dan pengujian yang sudah ada.
2. Cari fungsi atau tanggung jawab yang tumpang tindih.
3. Periksa dependensi antar-modul dan batas domain.
4. Pastikan perubahan tidak mengubah perilaku fitur yang sudah berjalan tanpa alasan yang jelas.
5. Periksa apakah dokumentasi yang ada dapat diperbarui daripada membuat dokumen baru.
6. Pisahkan perubahan dokumentasi, perubahan runtime, dan perubahan skema agar mudah ditinjau.

## Kualitas kode

- Gunakan nama yang menjelaskan tanggung jawab.
- Jaga fungsi dan modul tetap fokus.
- Hindari duplikasi validasi, aturan bisnis, dan kontrol keamanan.
- Letakkan logika domain pada backend, bukan pada komponen UI.
- Gunakan transaksi untuk perubahan data yang harus atomik.
- Terapkan least privilege dan deny-by-default.
- Jangan menyimpan secret, password, OTP, token, atau data pribadi sensitif di log.

## Dokumentasi

- Tulis informasi substantif yang dibutuhkan untuk memahami, menjalankan, menguji, atau mengoperasikan sistem.
- Jangan memasukkan kronologi percakapan, catatan persetujuan, narasi proses, atau tanggal yang tidak diperlukan secara teknis.
- Bedakan dengan jelas spesifikasi, implementasi yang tersedia, dan kemampuan yang belum tersedia.
- Hindari mengulang isi dokumen lain; gunakan tautan ke sumber yang menjadi acuan.
- Perbarui dokumen terkait ketika perubahan kode mengubah perilaku, konfigurasi, endpoint, atau skema.
- Gunakan nama file dan heading yang konsisten serta tautan relatif yang valid.

## Pemeriksaan kualitas

### Frontend

Jalankan dari `frontend/`:

```sh
npm ci
npm run format:check
npm run typecheck
npm run lint
npm test
npm run build
```

### Backend

Jalankan dari root repository:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
```

### PostgreSQL

- Jalankan migration dan integration test pada database disposable.
- Uji constraint, transaksi, concurrency, privilege, dan akses dengan principal yang sesuai.
- Jangan menggunakan database production sebagai target pengujian.
- Jangan mengubah migration yang telah diterapkan pada database bersama.

## Pull request dan rilis

- Kerjakan perubahan pada branch terpisah dari `main`.
- Sertakan ringkasan perubahan, cakupan pengujian, dan batasan yang belum diselesaikan.
- Periksa diff, file yang berubah, hasil CI, dan status branch sebelum merge.
- Jangan menggabungkan perubahan jika pemeriksaan wajib gagal atau dampak ke fitur yang sudah berjalan belum dipahami.
- Deployment dan migration production merupakan operasi terpisah dari merge kode.
