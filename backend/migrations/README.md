# Migrasi Database

Direktori ini sengaja belum berisi migrasi skema selama model domain masih diselesaikan. SQLx migrator sudah terhubung dan diuji melalui pengujian integrasi PostgreSQL. Migrasi SQL berversi pertama baru boleh ditambahkan bersamaan dengan skema domain pertama yang telah disetujui.

Jangan pernah mengubah migrasi setelah diterapkan pada database bersama. Gunakan migrasi baru dengan nomor versi berikutnya untuk setiap perubahan skema selanjutnya.
