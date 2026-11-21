# Kebijakan Keamanan

## Melaporkan kerentanan

Jangan memublikasikan detail kerentanan yang dapat dieksploitasi melalui issue, diskusi, atau pull request publik.

Laporkan masalah keamanan secara privat kepada pengelola proyek melalui kanal komunikasi privat yang tersedia. Sertakan komponen yang terdampak, potensi dampak, konteks reproduksi yang aman, dan saran mitigasi jika tersedia. Jangan membagikan data pengguna asli, kredensial, token, atau bukti sensitif.

Pengelola akan menilai laporan, memvalidasi dampak, menentukan prioritas, dan mengoordinasikan perbaikan serta pemberitahuan yang diperlukan. Jangan menguji sistem produksi atau mengakses data pihak lain tanpa izin.

## Prinsip keamanan

- Tegakkan autentikasi, role, kepemilikan, dan otorisasi objek di backend.
- Simpan kata sandi menggunakan algoritma hashing modern seperti Argon2id.
- Simpan rahasia di luar source control.
- Terapkan pembatasan permintaan untuk autentikasi, OTP, pemulihan, dan endpoint yang rentan disalahgunakan.
- Gunakan sesi yang aman, dapat dicabut, dan mengikuti batas waktu yang disetujui.
- Minimalkan pengumpulan data pribadi dan terapkan kontrol akses secara eksplisit.
- Catat tindakan administratif yang sensitif dalam audit trail tanpa menyimpan secret.
- Jangan tampilkan data bisnis privat atau data pelanggan tingkat individu kepada investor tanpa izin yang jelas.

## Dokumen acuan

- [Indeks Kebijakan Keamanan, Privasi & Perlindungan Data](docs/security/README.md)
- [Kesiapan Operasional dan Incident Response](docs/operations/README.md)
- [Panduan Kontribusi](CONTRIBUTING.md)

Persetujuan kebijakan tidak berarti seluruh kontrol telah diterapkan, diuji, atau tersertifikasi. Status implementasi harus dibuktikan melalui kode, konfigurasi, hasil pengujian, dan catatan operasional.
