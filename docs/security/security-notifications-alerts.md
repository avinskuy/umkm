# Kebijakan Notifikasi Keamanan & Peringatan

- **Status:** Disetujui sebagai baseline produk dan keamanan.
- **Cakupan:** Notifikasi keamanan untuk pengguna, termasuk autentikasi, perubahan identitas, pemulihan akun, dan tindakan pembatasan oleh Admin.
- **Hubungan:** Melengkapi Identity Core, Access Control, Logging & Audit, serta Incident Response.

## 1. Tujuan dan prinsip

Notifikasi keamanan membantu pengguna mengenali perubahan penting pada akun dan mengambil tindakan ketika aktivitas tidak dikenali.

Terapkan prinsip:

- Kirim notifikasi untuk peristiwa keamanan penting dan relevan.
- Gunakan kanal yang tersedia dan telah diizinkan atau dikonfigurasi secara sah.
- Sampaikan informasi minimum yang cukup untuk mengenali peristiwa.
- Jangan membocorkan rahasia, data sensitif, atau informasi akun lain.
- Berikan langkah yang dapat dilakukan pengguna jika aktivitas tidak dikenali.
- Pisahkan notifikasi keamanan dari komunikasi marketing.

## 2. Peristiwa yang wajib memicu notifikasi

Beri notifikasi kepada akun yang terdampak untuk peristiwa berikut:

1. Login dari perangkat baru.
2. Perubahan password.
3. Perubahan email utama.
4. Perubahan nomor telepon utama.
5. Perubahan kontak pemulihan.
6. Permintaan pemulihan akun dan hasil akhirnya, termasuk persetujuan, penundaan untuk pemeriksaan, atau penolakan.
7. Pembatasan fitur atau suspensi akun yang dilakukan Admin, termasuk perubahan status atau pemulihannya.

Notifikasi dikirim setelah peristiwa tercatat atau statusnya berubah secara sah. Permintaan pemulihan tidak boleh dianggap berhasil hanya karena notifikasi permintaan telah dikirim.

## 3. Kanal pengiriman

Gunakan kombinasi kanal yang tersedia dan relevan:

- **In-app notification** sebagai kanal dalam aplikasi.
- **Email** jika alamat yang sesuai tersedia dan dapat digunakan secara sah.
- **Push notification** jika didukung dan telah diizinkan pengguna/perangkat.
- **SMS** hanya jika diperlukan dan sesuai konteks keamanan.

Tidak setiap peristiwa harus dikirim melalui semua kanal. Pilih kanal berdasarkan jenis peristiwa, ketersediaan kontak, risiko, dan kemungkinan kontak tersebut sedang dibagikan dengan akun lain.

Jangan mengirim informasi akun spesifik kepada kontak pemulihan bersama jika informasi tersebut berkaitan dengan akun lain yang tidak dipilih atau tidak relevan.

## 4. Isi dan perlindungan notifikasi

Notifikasi harus:

- Menjelaskan jenis peristiwa, waktu yang relevan, dan status tindakan.
- Menampilkan detail perangkat atau lokasi secara minimum dan perkiraan bila tersedia; jangan menyajikan lokasi sebagai kepastian jika sumbernya hanya estimasi.
- Menghindari isi password, OTP, token, percakapan privat, dokumen, bukti pemulihan, atau detail investigasi.
- Tidak menampilkan data sensitif pada layar kunci atau preview push secara default.
- Tidak mengungkap keberadaan akun lain yang menggunakan kontak pemulihan bersama.
- Menggunakan tautan atau tindakan yang mengarah ke halaman resmi aplikasi dan tetap memerlukan autentikasi sesuai risiko.

Jangan memasukkan secret atau data L3/L4 yang tidak diperlukan ke payload notifikasi maupun log penyedia.

## 5. Panduan untuk aktivitas yang tidak dikenali

Untuk peristiwa yang mungkin tidak dilakukan pengguna, berikan tindakan yang jelas, misalnya:

- Periksa aktivitas akun atau sesi yang aktif.
- Amankan akun melalui alur resmi.
- Cabut sesi yang tidak dikenal jika fitur tersedia.
- Hubungi dukungan melalui kanal resmi jika pengguna tidak dapat mengamankan akun.

Jangan menyertakan instruksi yang mengungkap sinyal deteksi internal, detail mitigasi, atau cara menghindari kontrol keamanan.

## 6. Preferensi dan prioritas

- Notifikasi keamanan wajib tidak boleh dinonaktifkan melalui pengaturan marketing.
- Preferensi kanal boleh mengatur kanal opsional selama setidaknya mekanisme keamanan yang diwajibkan tetap tersedia.
- Notifikasi operasional dan marketing harus dipisahkan secara teknis maupun pengalaman pengguna.
- Hindari notifikasi duplikat yang tidak perlu, tetapi jangan meniadakan pemberitahuan penting karena optimasi frekuensi.
- Kegagalan satu kanal tidak boleh mengubah status keamanan peristiwa atau dianggap sebagai bukti bahwa pengguna telah menerima pemberitahuan.

## 7. Pencatatan dan keandalan

- Catat peristiwa pemicu, waktu, akun terdampak, kanal yang dicoba, dan hasil pengiriman dengan data minimum.
- Jangan menyimpan isi rahasia atau token di log pengiriman.
- Gunakan mekanisme yang mencegah pengiriman ganda yang tidak disengaja untuk satu peristiwa.
- Tangani kegagalan kanal secara terukur dan sediakan status pengiriman yang dapat ditinjau oleh petugas berwenang.
- Batasi akses Admin terhadap riwayat dan payload notifikasi sesuai tujuan kerja.

## 8. Pengujian minimum

- Setiap peristiwa wajib menghasilkan notifikasi untuk akun yang benar.
- Kontak pemulihan bersama tidak menerima informasi akun lain yang tidak relevan.
- Preview notifikasi tidak membocorkan rahasia atau data sensitif.
- Notifikasi keamanan tetap berjalan meskipun marketing dinonaktifkan.
- Kegagalan email/push/SMS tidak mengubah hasil tindakan keamanan.
- Notifikasi perubahan status moderasi dan pemulihan akun mencerminkan status aktual.
- Tautan tindakan menuju domain/aplikasi resmi dan tetap melewati autentikasi serta otorisasi.

## 9. Referensi

- NIST SP 800-63-4, Digital Identity Guidelines.
- NIST SP 800-53, Audit and Accountability serta Incident Response.
- OWASP Authentication Cheat Sheet.
- Praktik notifikasi keamanan dari platform konsumen mapan, disesuaikan dengan kebutuhan dan risiko USahapintar.

Referensi digunakan sebagai panduan praktik. Kanal, penyedia, template, retry, dan jadwal pengiriman ditentukan saat implementasi; kebijakan ini tidak menetapkan vendor tertentu.
