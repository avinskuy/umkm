# Migrasi Database

Direktori ini berisi migration SQLx berversi. Migration pertama untuk Identity Core adalah:

- `20261121170000_create_identity_accounts.sql` — membuat `users` dan `user_identities`, constraint lifecycle/role/kontak, unique partial index reservasi kontak, trigger immutability, pemeriksaan metode autentikasi, serta fungsi `create_client_user` yang hanya menerima role Business Owner atau Investor.

Migration pertama tidak membuat sesi login, OTP, recovery, audit Identity Core, profil, atau domain produk lain. Tidak ada grant runtime atau provisioning Admin yang dibuka secara otomatis oleh migration ini; hak akses tiap principal harus disiapkan terpisah sesuai kebijakan least privilege.

Migrasi dijalankan secara eksplisit melalui `database::migrations::run(&pool)`. HTTP server belum menjalankan migrasi atau menginisialisasi database pada startup. Migration ini belum dijalankan terhadap database produksi dan tidak boleh dianggap sebagai aktivasi fitur registrasi/login.

Pengujian PostgreSQL untuk migration dan invariant identitas berada di `backend/tests/postgres.rs`. Gunakan database sementara yang dapat dibuang, bukan database produksi.

Jangan pernah mengubah migration setelah diterapkan pada database bersama. Perubahan skema berikutnya harus menggunakan migration baru dengan versi SQLx berikutnya.
