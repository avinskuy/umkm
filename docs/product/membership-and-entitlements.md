# Kebijakan Keanggotaan & Hak Fitur

> Status: baseline kebijakan produk  
> Cakupan: otorisasi MVP, hak fitur paket Free/Premium, dan penegakan kuota  
> Dokumen ini menetapkan aturan produk. Isinya tidak menyatakan bahwa penagihan atau penegakan keanggotaan sudah diimplementasikan.

## 1. Prinsip utama

1. **Free untuk membangun koneksi, Premium untuk memperluas dan mengoptimalkan koneksi.**
2. Keanggotaan tidak pernah mengubah role utama pengguna atau memberikan kemampuan milik role lain.
3. Anggota Free dan Premium dapat menemukan peluang yang sesuai, mengajukan koneksi, dan mengirim pesan kepada koneksi yang sudah disetujui.
4. Premium meningkatkan kapasitas, visibilitas, penemuan, dan wawasan; fitur ini tidak boleh menjadikan jejaring dasar sebagai layanan berbayar.
5. Izin role, kepemilikan sumber daya, consent, status moderasi, dan status akun ditegakkan secara terpisah dari keanggotaan.
6. Backend menjadi sumber kebenaran untuk setiap keputusan hak fitur dan kuota. Menyembunyikan atau menonaktifkan kontrol di frontend hanya bagian dari UX.

## 2. Dimensi otorisasi yang terpisah

Otorisasi merupakan hasil gabungan dari pemeriksaan yang berdiri sendiri:

- **Status akun:** apakah akun diizinkan menggunakan platform.
- **Role utama:** satu role klien akun yang bersifat tetap, yaitu `business_owner` atau `investor`.
- **Izin sumber daya:** kepemilikan, status peserta, consent berbagi yang eksplisit, dan status moderasi.
- **Hak fitur keanggotaan:** apakah paket mengizinkan suatu kemampuan atau kuota yang lebih tinggi.
- **Ketersediaan penggunaan:** apakah akun masih memiliki kuota untuk periode terkait.

Langganan Premium tidak dapat mengesampingkan ketidaksesuaian role, akun yang ditangguhkan, consent yang tidak ada, izin sumber daya yang ditolak, atau pembatasan moderasi.

Setiap akun klien hanya memiliki satu role utama yang bersifat tetap. Seseorang yang ingin menggunakan role klien lainnya harus mendaftar akun terpisah dengan identitas yang memenuhi syarat dan belum terhubung ke akun lain. Upgrade atau downgrade keanggotaan tidak pernah mengubah role.

## 3. Baseline kemampuan setiap role

| Kemampuan | Pengunjung publik | Business Owner | Investor | Admin |
| --- | --- | --- | --- | --- |
| Melihat halaman publik, fitur, dan informasi keanggotaan | Ya | Ya | Ya | Ya |
| Mendaftar akun klien | Memilih Owner atau Investor | Tidak dapat berganti role | Tidak dapat berganti role | Tidak ada pendaftaran publik |
| Mengelola profil sesuai role sendiri | Tidak | Profil/usaha sendiri | Profil Investor sendiri | Hanya dengan izin operasional eksplisit |
| Membuat posting promosi/peluang Owner | Tidak | Ya, sesuai kuota paket dan aturan moderasi | Tidak | Hanya moderasi |
| Membuat posting pencarian kemitraan Investor | Tidak | Tidak | Ya, sesuai kuota paket dan aturan moderasi | Hanya moderasi |
| Menemukan profil/peluang yang dapat diakses | Hanya pratinjau publik | Field untuk pengguna terautentikasi | Field untuk pengguna terautentikasi | Akses operasional dengan tujuan terbatas |
| Mengajukan koneksi | Tidak | Ya, sesuai kuota paket | Ya, sesuai kuota paket | Bukan sebagai anggota biasa |
| Menerima/menolak permintaan koneksi sendiri | Tidak | Ya | Ya | Tidak boleh rutin menyamar sebagai pengguna |
| Mengirim pesan kepada koneksi yang disetujui | Tidak | Ya | Ya | Tidak memiliki akses rutin ke pesan privat |
| Melihat informasi bisnis privat | Tidak | Data sendiri | Hanya informasi yang secara eksplisit dibagikan Owner | Akses terbatas sesuai tujuan, berizin, dan diaudit |
| Mengelola keanggotaan dan penggunaan sendiri | Tidak | Ya | Ya | Hanya administrasi operasional |
| Mengubah role utama pengguna | Tidak | Tidak | Tidak | Tidak boleh mengubah role secara rutin; role bersifat tetap |

Akses Admin dibatasi berdasarkan izin dan wajib diaudit. Admin bukan tingkatan keanggotaan Free/Premium dan tidak boleh diperlakukan sebagai role klien biasa dalam perhitungan hak fitur.

## 4. Keanggotaan Free dan Premium

### Free

Free merupakan paket awal yang tetap lengkap untuk berpartisipasi dalam jejaring:

- Membuat dan mengelola profil sesuai role.
- Membuat posting sesuai role dalam batas kuota posting Free.
- Menemukan anggota dan peluang yang sesuai melalui fitur penemuan standar.
- Mengirim dan menerima permintaan koneksi dalam batas kuota permintaan koneksi Free.
- Menerima atau menolak permintaan masuk.
- Mengirim pesan setelah koneksi disetujui.
- Menggunakan kontrol inti akun, privasi, dan keamanan.
- Melihat informasi dasar aktivitas dan performa jika didukung produk.

### Premium

Premium mempertahankan seluruh kemampuan Free serta menambahkan kapasitas dan optimalisasi:

- Kuota posting sesuai role yang lebih tinggi.
- Kuota permintaan koneksi yang lebih tinggi.
- Prioritas visibilitas atau promosi unggulan, mengikuti aturan pemeringkatan yang transparan dan kelayakan moderasi.
- Tampilan profil yang lebih kaya dan fitur profil tambahan.
- Filter penemuan lanjutan.
- Analitik dan wawasan performa yang lebih terperinci.

Premium tidak menjamin jumlah tayangan, koneksi, investasi, pendanaan, pendapatan, atau hasil bisnis. Penempatan unggulan harus diberi label jika diperlukan dan tidak boleh melewati moderasi atau pemeringkatan keselamatan.

### Fitur yang tidak boleh dibatasi untuk Premium

Kemampuan berikut tetap tersedia bagi anggota Free maupun Premium, dengan tetap mengikuti role, consent, keselamatan, dan status akun:

- Membuat akun dan menyelesaikan profil sesuai role.
- Penemuan peluang inti.
- Mengirim dan menerima permintaan koneksi (dengan kuota yang berbeda).
- Menerima atau menolak permintaan koneksi.
- Mengirim pesan setelah koneksi disetujui.
- Memblokir, melaporkan, mengatur privasi, dan menggunakan fitur keamanan akun.
- Mengakses informasi yang secara eksplisit dibagikan anggota lain kepada mereka.

Jangan menjual pesan langsung sebagai fitur khusus Premium. Status Premium tidak boleh membuka data privat yang belum dibagikan pemiliknya.

## 5. Posting sesuai role

- Business Owner membuat posting promosi/peluang bisnis.
- Investor membuat posting pencarian kemitraan atau minat investasi menggunakan format khusus Investor.
- Kedua jenis posting merupakan kemampuan utama platform, bukan eksperimen fitur berdasarkan role.
- Kuota posting dihitung berdasarkan hak fitur keanggotaan pembuat dan jenis posting terkait.
- Keanggotaan tidak melewati validasi konten, aturan publikasi, moderasi, atau penangguhan akun.
- Status publikasi dan status moderasi tetap terpisah. Posting yang sudah terbit dapat ditandai, ditinjau, dibatasi, atau dihapus kemudian sesuai kebijakan platform.

## 6. Permintaan koneksi dan pesan

- Anggota Free maupun Premium dapat mengajukan koneksi.
- Keanggotaan menentukan jumlah permintaan yang boleh dimulai anggota dalam periode kuota yang dikonfigurasi; keanggotaan tidak menentukan apakah anggota boleh terhubung sama sekali.
- Permintaan harus disetujui sebelum percakapan dapat dibuat atau pesan dapat dikirim.
- Setelah koneksi disetujui, fitur pesan tersedia bagi peserta Free maupun Premium.
- Koneksi yang telah disetujui dan percakapan yang sudah berjalan tidak otomatis dicabut hanya karena anggota melakukan downgrade.
- Pemblokiran, pelaporan, pencegahan penyalahgunaan, penangguhan akun, dan kontrol keselamatan lainnya dapat membatasi koneksi atau pesan tanpa memandang paket.
- Pada MVP, jangan mengenakan kuota pesan terpisah atau mewajibkan Premium untuk melanjutkan percakapan yang sudah diizinkan.

## 7. Model hak fitur dan kuota

Representasikan hak fitur sebagai data paket yang dikelola backend, bukan sebagai pemeriksaan nama paket yang tersebar di berbagai bagian kode.

Contoh key hak fitur:

| Key hak fitur | Arti | Free | Premium |
| --- | --- | --- | --- |
| `posts.owner.monthly_limit` | Jumlah posting Owner yang diizinkan dalam periode yang dikonfigurasi | Kuota dasar | Kuota lebih tinggi |
| `posts.investor.monthly_limit` | Jumlah posting Investor yang diizinkan dalam periode yang dikonfigurasi | Kuota dasar | Kuota lebih tinggi |
| `connections.requests.monthly_limit` | Jumlah permintaan koneksi baru yang dimulai dalam periode yang dikonfigurasi | Kuota dasar | Kuota lebih tinggi |
| `discovery.advanced_filters` | Filter penemuan lanjutan | Tidak | Ya |
| `visibility.priority_eligible` | Kelayakan untuk visibilitas prioritas/promosi unggulan | Tidak | Ya |
| `profile.enhanced_features` | Fitur tampilan profil Premium | Tidak | Ya |
| `analytics.advanced` | Analitik terperinci | Tidak | Ya |

Jumlah kuota pasti, mata uang/harga, siklus penagihan, dan kebijakan reset kuota sengaja belum ditetapkan dalam dokumen ini. Semuanya memerlukan keputusan produk dan operasional terpisah sebelum implementasi. Jangan menyimpulkan batas numerik dari teks UI atau menanamkannya langsung dalam logika aplikasi.

### Persyaratan penegakan

- Backend harus menentukan paket aktif dan nilai hak fiturnya untuk setiap tindakan yang menggunakan kuota.
- Hitung penggunaan berdasarkan akun yang terautentikasi, bukan ID pengguna yang dikirim oleh klien.
- Konsumsi kuota harus atomik agar permintaan bersamaan tidak melampaui batas.
- Sebelum implementasi, tetapkan apakah tindakan yang gagal, ditolak, dihapus, atau dimoderasi mengonsumsi kuota. Terapkan satu kebijakan yang konsisten dan dokumentasikan.
- Kembalikan kode error yang stabil dan pesan aman bagi pengguna ketika kuota habis. Sertakan informasi reset/perpanjangan hanya jika waktunya diketahui.
- Jangan mereset penggunaan berdasarkan jam perangkat.
- Simpan ledger penggunaan yang dapat diaudit atau riwayat penghitung lain yang andal untuk kebutuhan dukungan dan investigasi sengketa.
- Hindari menghapus atau menyembunyikan konten yang sudah ada secara diam-diam ketika paket diturunkan.

## 8. Siklus langganan dan perilaku downgrade

Perilaku berikut wajib diterapkan pada tingkat kebijakan produk:

- Paket efektif anggota saat ini dan snapshot hak fiturnya harus ditentukan di server.
- Jika Premium kedaluwarsa, dibatalkan, atau tidak lagi aktif, tindakan baru menggunakan hak fitur Free sejak waktu downgrade efektif.
- Posting, profil, koneksi yang disetujui, dan percakapan yang sudah ada tetap tersedia kecuali dibatasi secara terpisah oleh moderasi, status akun, atau aturan platform lainnya.
- Jika penggunaan anggota melampaui kuota Free setelah downgrade, pertahankan konten yang sudah ada; blokir hanya tindakan tambahan yang mengonsumsi kuota sampai kapasitas tersedia.
- Tampilan dan kontrol penemuan khusus Premium boleh menjadi tidak aktif setelah downgrade, tetapi tidak boleh menghapus data anggota.
- Kegagalan pembayaran, masa tenggang, pengembalian dana, prorata, dan waktu pembatalan memerlukan kebijakan penagihan terpisah sebelum implementasi pembayaran.

## 9. Urutan keputusan otorisasi

Untuk tindakan yang dilindungi, lakukan pemeriksaan dalam urutan berikut:

1. Autentikasi sesi.
2. Pastikan akun aktif dan memenuhi syarat untuk tindakan tersebut.
3. Pastikan role utama yang bersifat tetap mengizinkan tindakan.
4. Pastikan kepemilikan, status peserta, consent, dan izin pada tingkat sumber daya.
5. Pastikan status moderasi dan keselamatan mengizinkan tindakan.
6. Tentukan paket keanggotaan efektif dan hak fiturnya.
7. Periksa sisa kuota dan lakukan reservasi/konsumsi penggunaan secara atomik.
8. Jalankan tindakan dan catat hasilnya.

Penolakan pada tahap sebelumnya tidak dapat dikesampingkan oleh hak fitur Premium pada tahap berikutnya.

## 10. Pengujian wajib

### Otorisasi role dan sumber daya

- Owner tidak dapat membuat posting pencarian kemitraan khusus Investor.
- Investor tidak dapat membuat posting promosi Owner.
- Owner dan Investor tidak dapat mengakses tindakan tulis khusus role lain dengan mengubah payload request atau rute.
- Pengunjung publik hanya menerima field pratinjau publik yang telah disetujui.
- Investor tidak dapat mengakses field bisnis privat tanpa consent berbagi eksplisit dari Owner.
- Tindakan Admin memerlukan izin yang sesuai dan harus diaudit.

### Keanggotaan dan kuota

- Free dan Premium sama-sama dapat mengajukan koneksi.
- Free dan Premium sama-sama dapat mengirim pesan setelah koneksi disetujui.
- Premium memiliki batas posting dan permintaan koneksi yang dikonfigurasi lebih tinggi daripada Free.
- Free tidak dapat menggunakan filter lanjutan atau analitik khusus Premium.
- Premium tidak dapat melewati pemeriksaan role, kepemilikan, consent, moderasi, atau status akun.
- Permintaan bersamaan yang mengonsumsi kuota tidak dapat melampaui batas yang dikonfigurasi.
- Kuota habis menghasilkan respons yang stabil dan aman bagi pengguna.
- Downgrade mempertahankan posting, koneksi, dan percakapan yang sudah ada.
- Downgrade mencegah tindakan tambahan yang melampaui kuota Free tanpa menghapus konten yang sudah ada.
- Operasi keanggotaan oleh Admin tidak dapat mengubah role utama anggota secara diam-diam.

## 11. Keputusan yang wajib ditetapkan sebelum implementasi

Nilai dan kebijakan berikut masih terbuka dan tidak boleh ditebak dalam kode:

- Kuota numerik posting Free dan Premium untuk setiap role.
- Kuota numerik permintaan koneksi Free dan Premium.
- Periode kuota dan aturan reset (bulan kalender atau periode langganan/penagihan).
- Apakah kuota yang tidak terpakai dapat dibawa ke periode berikutnya (rekomendasi awal: tidak, tetapi tetap memerlukan persetujuan).
- Harga Premium, penyedia penagihan, pajak/pengembalian dana, masa tenggang, dan perilaku pembatalan.
- Field profil dan metrik analitik yang hanya tersedia untuk Premium.
- Rumus pemeringkatan dan label untuk visibilitas prioritas/promosi unggulan.
- Apakah posting yang dihapus karena pelanggaran kebijakan mengonsumsi kuota, serta bagaimana banding memengaruhi penggunaan.
- Apakah kuota dikembalikan untuk kegagalan dari sisi platform atau pengiriman duplikat.

Sebelum keputusan tersebut disetujui, gunakan placeholder yang dapat dikonfigurasi dalam pekerjaan desain dan jangan menampilkan angka rekaan sebagai komitmen produk.
