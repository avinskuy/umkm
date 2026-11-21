# Kebijakan Keamanan

## Melaporkan kerentanan

Jangan memublikasikan detail kerentanan yang dapat dieksploitasi melalui issue atau pull request publik.

Laporkan masalah keamanan secara privat kepada pengelola proyek melalui kanal komunikasi privat. Sertakan komponen yang terdampak, potensi dampak, konteks untuk mereproduksi masalah, dan saran mitigasi jika tersedia. Hindari membagikan data pengguna asli atau kredensial.

## Prinsip keamanan

- Tegakkan autentikasi, pemeriksaan role, dan kepemilikan sumber daya di backend.
- Simpan kata sandi menggunakan algoritma hashing modern seperti Argon2id.
- Simpan rahasia di luar source control.
- Terapkan pembatasan permintaan pada autentikasi, OTP, dan endpoint lain yang rentan disalahgunakan.
- Gunakan cookie sesi yang aman dan perlindungan CSRF yang sesuai.
- Minimalkan pengumpulan data pribadi dan terapkan kontrol akses secara eksplisit.
- Catat tindakan administratif yang sensitif dalam audit trail.
- Jangan tampilkan data bisnis privat atau data pelanggan pada tingkat individu kepada investor tanpa izin yang jelas.

Dokumen ini merupakan kebijakan awal dan akan dilengkapi sebelum rilis produksi.
