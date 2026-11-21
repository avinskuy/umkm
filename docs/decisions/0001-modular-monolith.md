# ADR 0001: Memulai dengan Modular Monolith

- Status: Disetujui
- Tanggal: 2026-11-21

## Konteks

USahapintar masih berada pada tahap awal implementasi. Produk memiliki beberapa domain, tetapi belum ada kebutuhan operasional untuk men-deploy setiap domain secara terpisah.

## Keputusan

Bangun satu layanan backend Rust yang disusun ke dalam modul-modul domain dengan batas tanggung jawab yang jelas, serta satu aplikasi frontend React yang terpisah. Gunakan PostgreSQL sebagai database utama yang direncanakan. Pertahankan batas domain yang jelas tanpa memperkenalkan microservices.

## Konsekuensi

### Dampak positif

- Kompleksitas deployment dan operasional lebih rendah.
- Pengembangan lokal dan pengujian integrasi lebih mudah.
- Domain dapat dipisahkan di kemudian hari jika kebutuhan skala atau kepemilikan tim benar-benar membenarkannya.

### Kompromi

- Batas antar-modul harus dijaga melalui code review dan pengujian.
- Pada tahap awal, seluruh backend di-deploy dan diskalakan sebagai satu kesatuan.

## Tinjau kembali jika

Kebutuhan penskalaan, frekuensi deployment, kepemilikan tim, atau isolasi layanan memberikan alasan yang terukur untuk memisahkan suatu domain.
