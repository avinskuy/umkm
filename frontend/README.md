# Frontend

Aplikasi web USahapintar menggunakan React, TypeScript, dan Vite.

## Persyaratan

- Node.js 22.
- npm yang kompatibel dengan lockfile repository.

## Menyiapkan lingkungan lokal

Jalankan dari direktori `frontend/`:

```sh
npm ci
npm run dev
```

Vite menampilkan alamat lokal untuk membuka aplikasi. Output development server tersedia pada terminal yang sama.

## Pemeriksaan kualitas

Jalankan dari direktori `frontend/`:

```sh
npm run format:check
npm run typecheck
npm run lint
npm test
npm run build
```

Perintah tambahan:

```sh
npm run test:watch
npm run preview
```

## Struktur dan batas frontend

- Kelompokkan komponen, halaman, hooks, dan layanan berdasarkan fitur.
- Tempatkan komponen UI lintas fitur pada area bersama hanya jika memiliki tanggung jawab dan pemakai yang jelas.
- Hindari duplikasi komponen, validasi keamanan, dan aturan bisnis yang seharusnya menjadi tanggung jawab backend.
- Frontend tidak boleh menjadi sumber kebenaran untuk role, status akun, kepemilikan resource, atau keputusan otorisasi.
- Secret dan credential server tidak boleh dimasukkan ke bundle browser.

Frontend saat ini merupakan shell awal. Alur autentikasi, profil, discovery, koneksi, pesan, membership, dan administrasi belum tersedia sebagai fitur produk lengkap.

Ikuti [Engineering Standards](../docs/engineering/standards.md), [Inclusive UX Principles](../docs/product/prinsip-pengalaman-pengguna.md), dan [Contributing Guide](../CONTRIBUTING.md).
