# USahapintar — Peta Situs & Rancangan UI/UX

> Status: baseline rancangan produk  
> Cakupan: aplikasi web responsif dan PWA  
> Arah desain: Editorial Minimalis × Marketplace Modern × SaaS  
> Palet utama: putih lembut, hijau sage, hijau hutan, dan aksen warna netral hangat

Dokumen ini menetapkan struktur informasi dan pengalaman pengguna sebelum fitur dikembangkan. Dokumen ini merupakan rancangan produk, bukan pernyataan bahwa seluruh rute atau fitur sudah tersedia.

## 1. Model produk

USahapintar adalah platform promosi bisnis, intelijen, dan jejaring berbasis keanggotaan. Platform ini membantu pemilik usaha menampilkan peluang bisnis dan membantu investor menemukan bisnis serta peluang kemitraan.

USahapintar **bukan** marketplace barang atau platform transaksi investasi. MVP berfokus pada penemuan peluang, perkenalan, koneksi yang disetujui, dan komunikasi. Platform tidak menampung dana, menjalankan transaksi investasi, atau menjamin pendanaan maupun keuntungan.

### Peran pengguna

- **Business Owner** — membuat dan mengelola profil usaha, menerbitkan posting promosi/peluang bisnis, menggunakan Growth Advisor, dan membangun koneksi.
- **Investor** — menemukan peluang bisnis, membuat dan mengelola posting pencarian kemitraan dengan format khusus Investor, menggunakan Investment Intelligence, dan membangun koneksi.
- **Admin** — peran operasional internal untuk verifikasi, moderasi, keanggotaan, laporan, dan tata kelola platform. Akun Admin disediakan melalui proses internal dan tidak tersedia pada pendaftaran publik.

Setiap akun klien hanya memiliki satu role utama yang tetap: `business_owner` atau `investor`. Role dipilih saat pendaftaran dan tidak dapat diubah melalui pengaturan profil, permintaan bantuan, migrasi akun, OAuth, atau proses administratif lainnya. Pengguna yang ingin memakai role klien lain harus membuat akun baru dengan identitas login yang belum terhubung ke akun lain. Owner dan Investor memiliki jalur login terpisah serta otorisasi sesuai role. Backend menjadi sumber kebenaran untuk role dan akses sumber daya. Admin merupakan role internal yang disediakan secara terpisah, bukan pilihan pendaftaran klien.

### Model keanggotaan

USahapintar memiliki paket keanggotaan Free dan Premium yang berlaku terpisah dari role utama akun. Kedua paket tetap dapat menggunakan fitur inti berupa penemuan peluang, permintaan koneksi, persetujuan koneksi, dan pesan setelah koneksi diterima. Premium menambahkan kuota posting dan permintaan koneksi yang lebih tinggi, prioritas visibilitas/promosi unggulan, analitik yang lebih lengkap, filter penemuan lanjutan, dan fitur profil tambahan. Keanggotaan tidak memberikan izin milik role lain atau akses ke informasi privat yang belum dibagikan secara eksplisit.

Baseline akses role, entitlement, aturan kuota, perilaku downgrade, dan pengujian wajib dijelaskan dalam [Kebijakan Keanggotaan & Hak Fitur](membership-and-entitlements.md). Angka kuota dan ketentuan komersial tetap menjadi keputusan produk; jangan mengarang nilainya di UI atau implementasi.

## 2. Struktur informasi global

### Situs publik (tanpa login)

| Rute | Halaman | Tujuan |
| --- | --- | --- |
| `/` | Beranda | Menjelaskan platform dan mengarahkan pengunjung untuk mendaftar atau masuk |
| `/cara-kerja` | Cara kerja | Menjelaskan penemuan peluang, persetujuan koneksi, dan jejaring |
| `/fitur` | Fitur | Menampilkan kemampuan Owner, Investor, dan fitur bersama |
| `/membership` | Keanggotaan | Membandingkan manfaat dan kuota Free serta Premium |
| `/tentang` | Tentang | Menjelaskan misi, cakupan, dan prinsip platform |
| `/pusat-bantuan` | Pusat bantuan | FAQ dan panduan dasar |
| `/kontak` | Kontak | Kanal dukungan dan pertanyaan resmi |
| `/kebijakan/privasi` | Kebijakan privasi | Menjelaskan pengumpulan, penggunaan, consent, dan retensi data |
| `/ketentuan` | Ketentuan | Menjelaskan aturan platform, batasan, dan tanggung jawab pengguna |

Pengunjung publik dapat memahami layanan dan model keanggotaan. Feed penemuan internal, profil privat, koneksi, pesan, dan ruang kerja AI memerlukan autentikasi.

### Autentikasi

| Rute | Halaman | Tujuan |
| --- | --- | --- |
| `/masuk/pemilik` | Login Owner | Jalur masuk khusus Owner |
| `/masuk/investor` | Login Investor | Jalur masuk khusus Investor |
| `/daftar` | Pilihan role pendaftaran | Memilih Owner atau Investor; tanpa pilihan Admin publik |
| `/daftar/pemilik` | Pendaftaran Owner | Membuat akun Owner dan memulai onboarding usaha |
| `/daftar/investor` | Pendaftaran Investor | Membuat akun Investor dan memulai onboarding investor |
| `/verifikasi/email` | Verifikasi email (tahap berikutnya) | Memverifikasi alamat email setelah fitur verifikasi tersedia |
| `/verifikasi/telepon` | Verifikasi OTP telepon (tahap berikutnya) | Memverifikasi nomor telepon setelah fitur verifikasi tersedia |
| `/lupa-kata-sandi` | Pemulihan kata sandi | Meminta pengaturan ulang kata sandi |
| `/atur-kata-sandi` | Atur kata sandi baru | Menyelesaikan pemulihan kata sandi |

Pendaftaran tahap pertama menggunakan email atau nomor telepon + password, atau Google OAuth sebagai alternatif. OTP tidak diminta saat registrasi maupun login. Kontak yang dimasukkan melalui formulir password tetap berstatus belum terverifikasi sampai fitur verifikasi kontak tersedia; jangan gunakan kontak tersebut untuk pemulihan atau tindakan sensitif. Pendaftar baru melalui Google wajib memilih Owner atau Investor sebelum OAuth, tidak perlu membuat kata sandi lokal terpisah, dan tidak memerlukan OTP tambahan. Klaim email Google hanya dianggap terverifikasi jika token tervalidasi menyatakan `email_verified=true`; jangan menggabungkan akun hanya karena email cocok. Jalur login berdasarkan role tetap terpisah dan backend harus menolak ketidaksesuaian role tanpa membocorkan informasi akun. Sediakan kontrol tampil/sembunyi kata sandi, status loading/error, tautan ketentuan/privasi, dan validasi yang aksesibel.

Admin menggunakan jalur internal terpisah di `/admin/login`, dengan kontrol sesi yang lebih ketat dan MFA. Jalur ini tidak ditampilkan sebagai pilihan pendaftaran publik.

## 3. Kerangka aplikasi setelah login

Setelah login, pengguna masuk ke kerangka aplikasi sesuai role utamanya. Kerangka aplikasi dapat memakai elemen visual bersama, tetapi navigasi, isi dashboard, dan izin tetap mengikuti role masing-masing.

### Rute bersama setelah login

| Rute | Halaman | Akses |
| --- | --- | --- |
| `/app/notifikasi` | Notifikasi | Owner dan Investor; notifikasi keamanan tidak dapat dimatikan melalui preferensi pemasaran |
| `/app/koneksi` | Koneksi | Owner dan Investor |
| `/app/pesan` | Pesan | Owner dan Investor; hanya untuk koneksi yang disetujui |
| `/app/pesan/:conversationId` | Percakapan | Hanya peserta percakapan |
| `/app/profil` | Profil saya | Owner dan Investor |
| `/app/profil/:userId` | Profil anggota yang dapat dilihat publik | Pengguna terautentikasi, mengikuti visibilitas tiap field |
| `/app/pengaturan` | Pengaturan akun | Owner dan Investor |
| `/app/keanggotaan` | Keanggotaan dan penggunaan | Owner dan Investor |
| `/app/riwayat-aktivitas` | Riwayat aktivitas akun | Hanya akun sendiri; mencakup tindakan moderasi serta status/hasil banding |
| `/app/riwayat-aktivitas/:actionId/banding` | Ajukan banding moderasi | Hanya akun terdampak; maksimal satu banding untuk setiap tindakan moderasi |

Permintaan koneksi harus disetujui sebelum percakapan dapat dimulai. Pesan langsung bukan fitur khusus Premium. Pengguna Free maupun Premium dapat terhubung dan mengirim pesan setelah persetujuan, dengan tetap mengikuti batas penggunaan wajar dan keselamatan.

## 4. Ruang kerja Business Owner

Awalan rute yang disarankan: `/owner`.

| Rute | Halaman | Isi dan tindakan utama |
| --- | --- | --- |
| `/owner/dashboard` | Dashboard Owner | Ringkasan usaha, kelengkapan profil, performa posting, aktivitas koneksi, penggunaan keanggotaan, dan tindakan berikutnya |
| `/owner/bisnis` | Bisnis saya | Melihat dan mengelola profil usaha |
| `/owner/bisnis/baru` | Buat profil usaha | Identitas usaha, sektor, lokasi, tahap usaha, deskripsi, dan informasi pendukung |
| `/owner/bisnis/:businessId` | Ringkasan usaha | Informasi usaha, status verifikasi, metrik yang disetujui, dan posting terbit |
| `/owner/postingan` | Posting saya | Posting promosi/peluang, status, performa, serta tindakan edit/arsip |
| `/owner/postingan/baru` | Buat posting promosi | Menerbitkan promosi atau peluang usaha dengan bukti dan pengaturan visibilitas yang jelas |
| `/owner/postingan/:postId` | Detail dan performa posting | Konten posting, status moderasi, interaksi, dan analitik |
| `/owner/jelajah` | Temukan investor dan peluang | Mencari dan memfilter profil serta posting kemitraan yang relevan |
| `/owner/koneksi` | Pengelolaan koneksi | Permintaan masuk/keluar dan koneksi yang diterima |
| `/owner/pesan` | Pesan | Percakapan dengan koneksi yang disetujui |
| `/owner/growth-advisor` | Beranda Growth Advisor | Analisis usaha terpandu dan laporan tersimpan |
| `/owner/growth-advisor/validasi-ide` | Validasi ide | Mengevaluasi ide usaha tahap awal |
| `/owner/growth-advisor/perencanaan` | Perencanaan bisnis | Menyusun rencana bisnis terstruktur |
| `/owner/growth-advisor/keuangan` | Perencanaan keuangan | Lembar kerja HPP, harga, break-even, dan perencanaan modal |
| `/owner/growth-advisor/pasar` | Riset pasar | Riset pasar, pelanggan, pesaing, dan wilayah |
| `/owner/growth-advisor/pertumbuhan` | Strategi pertumbuhan | Prioritas dan skenario pertumbuhan yang praktis |
| `/owner/growth-advisor/kesehatan-bisnis` | Pemeriksaan kesehatan bisnis | Meninjau indikator usaha dan kelengkapan data |
| `/owner/analitik` | Analitik bisnis | Tren serta performa posting/profil berdasarkan data yang tersedia |
| `/owner/keanggotaan` | Keanggotaan | Paket, kuota penggunaan, dan informasi peningkatan paket |
| `/owner/pengaturan` | Pengaturan | Preferensi akun, keamanan, privasi, dan notifikasi |

Onboarding Owner menyesuaikan tahap usaha: baru berupa ide, sedang bersiap memulai, sudah beroperasi, atau sedang berkembang. UI hanya meminta informasi yang relevan dengan tahap yang dipilih dan memungkinkan pengguna melengkapi profil secara bertahap.

## 5. Ruang kerja Investor

Awalan rute yang disarankan: `/investor`.

| Rute | Halaman | Isi dan tindakan utama |
| --- | --- | --- |
| `/investor/dashboard` | Dashboard Investor | Peluang tersimpan, sorotan penemuan, aktivitas koneksi, dan ringkasan preferensi |
| `/investor/jelajah` | Penemuan peluang | Mencari dan memfilter peluang usaha |
| `/investor/jelajah/:postId` | Detail peluang | Informasi usaha yang disetujui Owner, label bukti, dan permintaan koneksi |
| `/investor/tersimpan` | Peluang tersimpan | Peluang yang ditandai dan perubahan statusnya |
| `/investor/postingan` | Posting kemitraan saya | Membuat dan mengelola posting pencarian kemitraan dengan format khusus Investor, mengikuti kuota keanggotaan dan moderasi |
| `/investor/postingan/baru` | Buat posting kemitraan | Membuat posting format Investor yang menjelaskan tujuan kemitraan, sektor pilihan, wilayah, dan cakupan kolaborasi |
| `/investor/koneksi` | Pengelolaan koneksi | Permintaan, koneksi yang diterima, dan perkenalan |
| `/investor/pesan` | Pesan | Percakapan dengan koneksi yang disetujui |
| `/investor/investment-intelligence` | Beranda Investment Intelligence | Analisis terpandu dan laporan tersimpan |
| `/investor/investment-intelligence/analisis` | Analisis peluang | Meninjau kesehatan keuangan, profitabilitas, pertumbuhan, pasar, dan risiko |
| `/investor/investment-intelligence/perbandingan` | Perbandingan | Membandingkan peluang menggunakan data yang tersedia dan sebanding |
| `/investor/investment-intelligence/simulasi` | Simulasi skenario | Mengeksplorasi asumsi dan kemungkinan skenario tanpa menjanjikan hasil |
| `/investor/analitik` | Analitik penemuan | Wawasan dari pencarian tersimpan dan aktivitas penemuan |
| `/investor/keanggotaan` | Keanggotaan | Paket, kuota penggunaan, dan informasi peningkatan paket |
| `/investor/pengaturan` | Pengaturan | Preferensi akun, keamanan, privasi, dan notifikasi |

Onboarding Investor mengumpulkan informasi profil dan preferensi penemuan seperti sektor, wilayah target, kisaran indikatif, horizon, dan preferensi risiko. Preferensi ini mendukung penemuan dan analisis; jangan menyajikannya sebagai jaminan kesesuaian investasi atau hasil investasi.

AI untuk Investor hanya boleh menggunakan data usaha yang secara eksplisit disetujui Owner untuk dibagikan. Detail transaksi pada tingkat pelanggan tidak boleh ditampilkan.

## 6. Ruang kerja Admin

Awalan rute yang disarankan: `/admin`. Akses bersifat internal dan wajib ditegakkan di server.

| Rute | Halaman | Isi dan tindakan utama |
| --- | --- | --- |
| `/admin/login` | Login Admin | Login internal, MFA, dan pemberitahuan keamanan |
| `/admin/dashboard` | Ringkasan operasional | Kesehatan platform, antrean tinjauan, laporan, dan metrik operasional |
| `/admin/users` | Pengelolaan pengguna | Pencarian, status, role, verifikasi, dan tindakan akun |
| `/admin/users/:userId` | Detail pengguna | Tinjauan profil, riwayat verifikasi, laporan, dan tindakan moderasi |
| `/admin/businesses` | Verifikasi usaha | Meninjau profil usaha dan bukti yang dikirimkan |
| `/admin/posts` | Moderasi posting | Meninjau, menyetujui, menolak, menyembunyikan, atau meminta perbaikan posting |
| `/admin/reports` | Laporan dan keluhan | Mengelompokkan laporan, menyelidiki, dan mencatat hasil |
| `/admin/banding` | Banding moderasi | Meninjau banding dan mencatat hasil diterima, diubah, atau ditolak |
| `/admin/memberships` | Pengelolaan keanggotaan | Paket, entitlement, kuota, dan status langganan |
| `/admin/connections` | Pengawasan koneksi | Menyelidiki laporan penyalahgunaan dan peristiwa keselamatan terkait; bukan untuk membaca percakapan privat secara rutin |
| `/admin/ai-governance` | Tata kelola AI dan data | Meninjau status sumber data, consent, insiden keluaran, dan metadata audit |
| `/admin/admins` | Akun Admin | Provisioning super-admin dan penetapan izin |
| `/admin/audit-logs` | Log audit | Mencari tindakan administratif yang sensitif terhadap keamanan |
| `/admin/settings` | Pengaturan platform | Konfigurasi dan pengaturan operasional yang terkontrol |

Gunakan izin least privilege, MFA, masa sesi yang ketat, rate limit, dan catatan audit append-only untuk tindakan sensitif. Akses Admin ke konten privat pengguna harus dibatasi berdasarkan tujuan, diperiksa izinnya, dan dapat diaudit.

## 7. Alur utama pengguna

### Alur Owner

1. Beranda publik → pilih pendaftaran Owner.
2. Pilih Business Owner sebagai role utama permanen, lalu buat akun menggunakan email/telepon + password atau Google OAuth. Tidak ada OTP pada tahap pertama; kontak yang belum terverifikasi dibatasi untuk pemulihan dan tindakan sensitif.
3. Selesaikan onboarding usaha sesuai tahap bisnis.
4. Masuk ke dashboard Owner dan lengkapi profil usaha.
5. Terbitkan posting promosi/peluang sesuai batas keanggotaan.
6. Temukan investor yang relevan dan terima permintaan koneksi.
7. Setujui koneksi → mulai percakapan.
8. Gunakan Growth Advisor dan analitik dengan label sumber data yang jelas.

### Alur Investor

1. Beranda publik → pilih pendaftaran Investor.
2. Pilih Investor sebagai role utama permanen, lalu buat akun menggunakan email/telepon + password atau Google OAuth. Tidak ada OTP pada tahap pertama; kontak yang belum terverifikasi dibatasi untuk pemulihan dan tindakan sensitif.
3. Lengkapi profil Investor dan preferensi penemuan.
4. Masuk ke dashboard Investor dan jelajahi peluang.
5. Tinjau peluang beserta label sumber data/verifikasinya.
6. Kirim permintaan koneksi → tunggu persetujuan.
7. Mulai percakapan setelah disetujui dan gunakan Investment Intelligence dengan data yang disetujui Owner.

### Alur Admin

1. Akun Admin disediakan melalui proses internal.
2. Masuk melalui portal Admin terpisah dan selesaikan MFA.
3. Tinjau antrean verifikasi pengguna/usaha/posting.
4. Moderasi laporan dan kelola entitlement keanggotaan.
5. Catat tindakan sensitif di log audit.

## 8. Model navigasi

### Desktop

- Header publik: brand, Cara Kerja, Fitur, Keanggotaan, Bantuan, Masuk, dan CTA utama Daftar.
- Sidebar Owner: Ringkasan, Bisnis Saya, Posting, Jelajah, Koneksi, Pesan, Growth Advisor, Analitik, Keanggotaan, Pengaturan.
- Sidebar Investor: Ringkasan, Jelajah, Tersimpan, Posting Kemitraan, Koneksi, Pesan, Investment Intelligence, Analitik, Keanggotaan, Pengaturan.
- Sidebar Admin: Ringkasan, Pengguna, Tinjauan Usaha, Moderasi Posting, Laporan, Keanggotaan, Tata Kelola AI & Data, Akun Admin, Log Audit, Pengaturan.

### Mobile

Gunakan top bar ringkas dan navigasi bawah yang menyesuaikan role untuk tujuan yang paling sering dipakai. Tempatkan tujuan sekunder di drawer yang dikelompokkan dengan jelas. Jangan sekadar mengecilkan sidebar desktop. Label navigasi utama harus tetap terlihat, status aktif harus jelas, dan area sentuh harus nyaman.

## 9. Pola tata letak halaman

Gunakan beberapa pola yang konsisten, bukan membuat tata letak baru untuk setiap rute:

1. **Halaman publik** — header, hero, bagian bukti/manfaat, konten penjelas, CTA, dan footer.
2. **Dashboard terautentikasi** — judul halaman, tindakan kontekstual, kartu ringkasan, area kerja utama, dan aktivitas terbaru.
3. **Halaman daftar/penemuan** — judul, kontrol pencarian/filter, jumlah hasil, kartu/daftar responsif, serta status kosong/loading/error.
4. **Halaman detail** — identitas/ringkasan, label verifikasi dan sumber data, konten utama, tindakan kontekstual, dan informasi terkait.
5. **Formulir/onboarding** — indikator langkah singkat, field yang dikelompokkan, panduan langsung, validasi, dan tindakan simpan/lanjut.
6. **Ruang kerja AI** — pilihan tugas, input terstruktur, status proses, bagian hasil, asumsi/bukti/batasan, serta simpan/ekspor jika didukung.
7. **Halaman tinjauan Admin** — filter antrean, konteks tinjauan, panel bukti, kontrol keputusan, pencatatan alasan, dan metadata audit.

## 10. Sistem visual dan interaksi

### Arah visual

- Putih lembut menjadi permukaan utama; hijau sage menjadi aksen utama; hijau hutan digunakan untuk tindakan dan penekanan penting.
- Gunakan beige hangat dan sage pucat sebagai permukaan sekunder secara terbatas.
- Tipografi terasa editorial dan mudah dibaca, dengan hierarki jelas serta line-height yang nyaman.
- Utamakan ruang kosong, perataan yang rapi, dan jarak yang konsisten daripada dashboard yang padat.
- Gunakan gambar produk asli atau ilustrasi netral hanya jika membantu menjelaskan kategori usaha atau tujuan halaman. Hindari dekorasi berlebihan dan gradien yang terlalu ramai.

### Token desain inti (awal)

| Token | Nilai | Penggunaan |
| --- | --- | --- |
| Putih lembut | `#F8FAF7` | Latar utama |
| Sage pucat | `#E8EDE5` | Permukaan lembut |
| Hijau sage | `#A3B18A` | Aksen |
| Hijau hutan | `#344E41` | Penekanan utama |
| Charcoal gelap | `#29332D` | Teks utama |
| Beige hangat | `#D8CDBB` | Aksen netral |
| Hijau redup | `#6B9277` | Aksen sekunder |

Nilai ini merupakan token awal dan dapat disempurnakan setelah peninjauan visual halaman pertama.

### Status wajib dan aksesibilitas

Setiap halaman berbasis data harus memiliki status loading, kosong, error, dan berhasil. Formulir memerlukan label yang terlihat, dukungan keyboard, pesan error yang berguna, dan indikator fokus yang jelas. Status tidak boleh hanya dibedakan melalui warna. Grafik harus memiliki ringkasan teks atau alternatif data yang aksesibel. Hormati preferensi reduced motion.

### Kepercayaan dan transparansi data

Tampilkan status data pada setiap bagian yang menyajikan metrik bisnis atau analisis AI:

- **Terverifikasi** — sudah ditinjau melalui proses verifikasi yang jelas.
- **Terintegrasi sistem** — diterima dari sistem yang terhubung.
- **Dilaporkan Owner** — dimasukkan atau dinyatakan oleh pemilik usaha.
- **Estimasi AI** — disimpulkan atau dimodelkan; asumsi harus terlihat.

Tampilkan sumber, periode pelaporan, waktu pembaruan terakhir, consent/visibilitas, dan kelengkapan jika relevan. Estimasi AI tidak boleh terlihat seolah-olah setara dengan data keuangan terverifikasi.

## 11. Pengalaman keanggotaan

Gunakan halaman keanggotaan bersama dengan detail manfaat yang menyesuaikan role. Prinsip utamanya adalah **Free to Connect, Premium to Grow**.

- Pengguna Free dan Premium dapat menemukan peluang yang sesuai, mengirim/menerima permintaan koneksi, dan mengirim pesan setelah koneksi disetujui.
- Premium dapat menawarkan kuota posting dan permintaan koneksi yang lebih tinggi, prioritas visibilitas, promosi unggulan, profil lebih lengkap, filter penemuan lanjutan, serta riwayat analitik/AI yang lebih mendalam.
- Harga, jumlah kuota, dan durasi paket tetap menjadi keputusan produk dan sengaja belum ditetapkan dalam rancangan ini.
- Tampilkan penggunaan kuota, periode reset, dan tindakan saat kuota tercapai dengan jelas. Jangan pernah memblokir percakapan yang sudah disetujui hanya karena pengguna memakai paket Free.

## 12. Urutan implementasi MVP setelah tinjauan desain

1. Finalisasi peta situs, matriks akses role, dan alur utama pengguna.
2. Finalisasi fondasi visual bersama dan kerangka aplikasi responsif.
3. Implementasikan autentikasi dan onboarding (Owner, Investor, serta portal Admin terpisah).
4. Implementasikan profil pengguna dan profil usaha dengan kontrol kepemilikan/visibilitas.
5. Implementasikan posting dan penemuan peluang.
6. Implementasikan permintaan koneksi dan pesan setelah koneksi disetujui.
7. Implementasikan entitlement keanggotaan dan penegakan kuota.
8. Implementasikan Growth Advisor dan Investment Intelligence di atas data usaha yang tata kelolanya jelas.
9. Implementasikan verifikasi Admin, moderasi, operasional keanggotaan, dan alat audit.
10. Tambahkan cakupan end-to-end, tinjauan keamanan, tinjauan aksesibilitas, dan kesiapan deployment.

## 13. Hal yang tidak termasuk dalam MVP

- Checkout barang, keranjang, pengiriman, atau alur e-commerce yang berfokus pada inventaris.
- Menampung atau memindahkan dana investasi.
- Menjalankan kontrak investasi atau menjamin keuntungan.
- Pendaftaran Admin publik.
- Pesan langsung yang hanya tersedia untuk Premium.
- Akses Investor ke data privat Owner tanpa izin eksplisit.
- Klaim keuangan yang dibuat AI tanpa bukti, asumsi, dan batasan.

---

**Titik tinjauan desain:** validasi hierarki halaman, label navigasi, dan konten tiap halaman sebelum beralih ke desain visual tingkat tinggi atau implementasi fitur.
