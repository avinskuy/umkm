# Frontend

Aplikasi web USahapintar menggunakan React, TypeScript, dan Vite.

## Persyaratan

- Node.js 22 (versi yang digunakan oleh CI).
- npm yang kompatibel dengan lockfile proyek.

## Menyiapkan lingkungan lokal

Jalankan dari direktori `frontend/`:

```sh
npm ci
npm run dev
```

Vite akan menampilkan alamat lokal untuk membuka aplikasi di browser. Gunakan terminal yang sama untuk melihat output development server.

## Pemeriksaan kualitas

Jalankan dari direktori `frontend/`:

```sh
npm run format:check
npm run typecheck
npm run lint
npm test
npm run build
```

Untuk menjalankan test dalam mode watch:

```sh
npm run test:watch
```

Untuk melihat hasil production build secara lokal:

```sh
npm run preview
```

## Catatan proyek

Frontend saat ini masih berupa shell awal proyek, belum mengimplementasikan alur produk seperti autentikasi, profil, discovery, koneksi, pesan, membership, atau Admin.

Ikuti [Standar Engineering](../docs/engineering/standards.md), [Prinsip Pengalaman Pengguna yang Inklusif](../docs/product/prinsip-pengalaman-pengguna.md), dan [Panduan Kontribusi](../CONTRIBUTING.md).

Jangan menaruh secret di source code atau variabel yang dikirim ke browser. Konfigurasi yang bersifat rahasia harus tetap berada di backend atau lingkungan server yang sesuai.
