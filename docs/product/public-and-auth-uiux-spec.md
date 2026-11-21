# Spesifikasi UI/UX Situs Publik & Autentikasi

> Status: spesifikasi desain  
> Rancangan induk: [Peta Situs & Rancangan UI/UX](./sitemap-and-ux-blueprint.md)  
> Cakupan: halaman publik, pemilihan role, autentikasi Owner/Investor, awal onboarding, dan login Admin  
> Status implementasi: belum diimplementasikan

Dokumen ini menerjemahkan peta situs menjadi kebutuhan interaksi pada tingkat halaman. Rancangan dibuat netral terhadap implementasi agar desain visual dan pengerjaan frontend dapat ditinjau secara terpisah.

## 1. Tujuan pengalaman

1. Pengunjung baru memahami dalam beberapa detik bahwa USahapintar menghubungkan pemilik usaha dengan investor.
2. Pengunjung dapat membedakan alur Owner dan Investor tanpa harus membaca penjelasan panjang.
3. Pendaftaran dipandu dan hanya meminta informasi yang diperlukan pada langkah tersebut.
4. Jalur login Owner dan Investor terpisah, tetapi memakai pola interaksi bersama yang konsisten.
5. Pesan kesalahan autentikasi mudah dipahami tanpa membocorkan apakah suatu akun terdaftar.
6. Pengalaman mobile dirancang sebagai tampilan utama, bukan sekadar versi desktop yang diperkecil.
7. Klaim kepercayaan, metrik bisnis, dan kemampuan AI dijelaskan secara hati-hati; tidak menjanjikan pendanaan, keuntungan, atau hasil investasi.

## 2. Fondasi visual global

### Tata letak dan permukaan

- Latar utama halaman: Soft White `#F8FAF7`.
- Permukaan konten utama: putih dengan border tipis dan bayangan lembut hanya jika diperlukan untuk membedakan area.
- Permukaan aksen: Pale Sage `#E8EDE5`.
- Tindakan utama: Forest Green `#344E41`.
- Aksen sekunder: Sage Green `#A3B18A`.
- Teks utama: Deep Charcoal `#29332D`.
- Aksen netral hangat: `#D8CDBB`.
- Lebar konten harus nyaman dibaca. Bagian pemasaran boleh memakai container lebih lebar, sedangkan halaman formulir tetap cukup sempit agar mudah dipindai.
- Gunakan token jarak yang konsisten dan skala tipografi yang jelas. Hindari grid kartu yang terlalu padat dan container membulat secara berlebihan.

### Komponen bersama

Tahap awal design system perlu menetapkan komponen berikut:

- `PublicHeader` dan `PublicFooter`
- `PrimaryButton`, `SecondaryButton`, `TextButton`
- `TextField`, `PasswordField`, `PhoneField`, `SelectField`
- `CheckboxField`, `RadioCard`
- `FormFieldError`, `InlineNotice`, `Toast`
- `PageContainer`, `SectionHeading`, `AuthLayout`
- `LoadingButton`, `PasswordStrengthHint`, `OtpCodeInput`
- `StepIndicator`, `ProgressSummary`

Komponen harus menyediakan label dan status yang aksesibel, bukan hanya mengandalkan placeholder. Gunakan semantic HTML, indikator fokus keyboard yang terlihat, dan penempatan pesan error yang konsisten.

## 3. Halaman beranda publik — `/`

### Tujuan halaman

Menyampaikan nilai utama platform dan memberi dua tindakan lanjutan yang jelas: menjelajahi platform atau mendaftar sebagai Owner/Investor.

### Struktur desktop

1. **Header**
   - Kiri: simbol brand dan nama USahapintar.
   - Tengah: Cara Kerja, Fitur, Keanggotaan, Tentang.
   - Kanan: Masuk dan CTA Daftar yang menonjol.
   - Tampilan header tetap tenang; jangan menjadikan setiap item navigasi sebagai tombol.

2. **Hero**
   - Komposisi editorial dua kolom.
   - Kiri: eyebrow singkat, judul yang jelas, deskripsi ringkas, dan dua CTA sesuai role.
   - Kanan: satu gambar/ilustrasi kuat yang mewakili kolaborasi bisnis dan penemuan peluang.
   - Usulan judul: **Bangun koneksi. Kembangkan peluang usaha.**
   - Usulan deskripsi: **USahapintar mempertemukan pemilik usaha dan investor untuk menemukan peluang, membangun kemitraan, dan mengambil keputusan dengan informasi yang lebih baik.**
   - CTA utama: **Daftar sebagai Pemilik Usaha**
   - CTA sekunder: **Daftar sebagai Investor**
   - Tautan pendukung: **Pelajari cara kerja USahapintar**

3. **Penjelasan role**
   - Dua bagian/kartu seimbang: Pemilik Usaha dan Investor.
   - Owner: mempromosikan usaha, merapikan informasi bisnis, menemukan mitra yang relevan, dan menggunakan Growth Advisor.
   - Investor: menemukan peluang bisnis, meninjau informasi usaha yang disetujui, terhubung dengan pemilik usaha, dan menggunakan Investment Intelligence.
   - Setiap bagian memiliki CTA khusus menuju jalur pendaftaran yang sesuai.

4. **Cara kerja**
   - Tampilkan tiga atau empat langkah dengan label singkat:
     - Buat profil
     - Temukan peluang
     - Ajukan koneksi
     - Bangun komunikasi
   - Jelaskan bahwa fitur pesan terbuka setelah permintaan koneksi disetujui.

5. **Kemampuan platform**
   - Promosi bisnis dan penemuan peluang.
   - Permintaan koneksi dan pesan setelah koneksi disetujui.
   - Growth Advisor dan Investment Intelligence.
   - Verifikasi dan label data yang transparan.
   - Jangan memberi kesan bahwa seluruh kemampuan sudah tersedia pada rilis pertama.

6. **Pratinjau keanggotaan**
   - Jelaskan prinsip **Free to Connect, Premium to Grow**.
   - Kedua paket dapat menemukan peluang, mengirim/menerima permintaan koneksi, dan mengirim pesan setelah disetujui.
   - Premium berfokus pada alat pengembangan usaha, kuota lebih tinggi, visibilitas, dan wawasan yang lebih lengkap.
   - CTA: **Lihat Keanggotaan**.
   - Jangan menampilkan harga final atau angka kuota sebelum disetujui sebagai keputusan produk.

7. **Kepercayaan dan tanggung jawab**
   - Jelaskan bahwa USahapintar memfasilitasi perkenalan dan jejaring bisnis.
   - Tegaskan bahwa platform tidak menjamin pendanaan atau keuntungan dan tidak menjalankan transaksi investasi pada MVP.
   - Investor hanya dapat melihat informasi yang disetujui pemilik usaha untuk dibagikan.

8. **CTA akhir dan footer**
   - Ulangi dua pilihan role.
   - Kelompok tautan footer: Platform, Bantuan, Legal, Kontak.
   - Sertakan tautan kebijakan privasi dan ketentuan.

### Struktur mobile

- Header ringkas dengan logo dan pemicu menu.
- Tampilkan judul serta deskripsi hero terlebih dahulu, lalu CTA role, kemudian gambar/ilustrasi.
- CTA role menggunakan lebar penuh atau ditumpuk jika ruang terbatas.
- Langkah cara kerja ditampilkan sebagai urutan vertikal.
- Bagian fitur ditumpuk; hindari kartu teks yang harus digeser secara horizontal.
- Tautan footer hanya dikelompokkan dalam bagian yang dapat dibuka/tutup jika interaksinya tetap aksesibel.

### Status halaman beranda

- Konten normal dapat ditampilkan secara statis dan tidak bergantung pada pemanggilan API yang memerlukan autentikasi.
- Jika media hero gagal dimuat, tampilkan fallback netral yang tetap memperlihatkan judul dan CTA secara utuh.
- Menu navigasi dapat dibuka/ditutup dengan keyboard dan menampilkan status expanded.
- Navigasi CTA harus mempertahankan jalur role yang dipilih.

## 4. Navigasi publik dan awal pendaftaran

### Tindakan header

- **Masuk** membuka pilihan role sederhana: Pemilik Usaha atau Investor. Jangan arahkan kedua role ke halaman login klien umum yang ambigu.
- **Daftar** membuka pemilihan role di `/daftar`.
- Tautan langsung ke `/masuk/pemilik` dan `/masuk/investor` tetap tersedia.
- Login Admin tidak ditampilkan di header publik atau pilihan role pendaftaran.

### Pemilihan role pendaftaran — `/daftar`

Tujuan: membantu pengguna baru memilih role utama akun sebelum masuk ke formulir khusus role.

Konten:

- Judul: **Mulai bersama USahapintar**
- Teks pendukung: **Pilih bagaimana kamu akan menggunakan platform ini.**
- Dua panel role yang dapat dipilih:
  - **Saya Pemilik Usaha** — promosikan usaha, bagikan peluang, dan temukan mitra.
  - **Saya Investor** — temukan peluang bisnis dan terhubung dengan pemilik usaha.
- CTA pada setiap panel mengarah ke jalur pendaftaran yang sesuai.
- Tautan kecil untuk pengguna yang sudah memiliki akun mengarah ke pilihan login berdasarkan role.
- Tidak ada pilihan Admin.
- Pilihan role harus disimpan sebagai bagian dari alur pendaftaran dan divalidasi backend; pengguna tidak dapat memilih Admin sendiri.
- Role klien yang dipilih bersifat permanen untuk akun tersebut. Sebelum pengiriman formulir, jelaskan bahwa akun tidak dapat berpindah antara Business Owner dan Investor.
- Jika pengguna ingin memakai role lain, ia harus membuat akun baru dengan identitas login yang belum terhubung ke akun lain. Jangan menyediakan kontrol pindah role atau jalur migrasi role melalui support.

## 5. Login Owner — `/masuk/pemilik`

### Urutan konten

1. Brand dan tautan singkat untuk kembali ke situs publik.
2. Judul: **Masuk sebagai Pemilik Usaha**
3. Teks pendukung: **Kelola profil usaha, bagikan peluang, dan kembangkan koneksi.**
4. Formulir autentikasi menggunakan identifier/metode login yang telah disetujui.
5. Tindakan login Google untuk identitas yang sudah terhubung dan pendaftaran baru yang terikat role.
6. Tautan pendaftaran untuk Owner baru.
7. Tautan privasi/ketentuan dan bantuan.

### Field formulir dan perilaku

- Login menerima email atau nomor telepon yang terdaftar beserta kata sandi akun. Kode email/WhatsApp hanya untuk aktivasi akun dan bukan kredensial login.
- Field kata sandi dengan kontrol tampil/sembunyi.
- Tautan lupa kata sandi.
- Opsi ingat perangkat/sesi hanya boleh ditampilkan jika kebijakan sesi backend mendukungnya dengan aman; jangan memberi kesan sesi berlaku permanen.
- Tombol kirim: **Masuk**.
- Validasi sisi klien memeriksa input kosong atau format yang salah, tetapi validasi server tetap menjadi acuan utama.
- Nonaktifkan pengiriman berulang saat permintaan masih diproses.
- Pertahankan nilai field yang aman dan tidak sensitif setelah error yang dapat dipulihkan; jangan pernah menampilkan kembali kata sandi.

### Perilaku role tidak sesuai

Role akun ditetapkan saat pendaftaran. Jika Investor mencoba masuk melalui portal Owner, tampilkan pesan netral seperti:

**Akun ini tidak dapat masuk melalui halaman Pemilik Usaha. Coba halaman masuk Investor.**

Sediakan tautan ke halaman login Investor. Server wajib menegakkan kecocokan role; menyembunyikan rute di frontend bukan otorisasi. Jangan menawarkan konversi akun menjadi Owner. Pengguna yang menginginkan akun Owner harus menyelesaikan pendaftaran terpisah dengan identitas login yang memenuhi syarat dan belum terhubung ke akun lain.

### Status error dan pemulihan

- Kredensial tidak valid: **Email/nomor atau kata sandi belum sesuai. Periksa kembali lalu coba lagi.**
- Rate limit: jelaskan bahwa percobaan masuk dibatasi sementara dan kapan pengguna dapat mencoba lagi jika waktunya diketahui.
- Gangguan jaringan/server: **Belum bisa terhubung. Periksa koneksi lalu coba lagi.**
- Kontak belum diverifikasi: arahkan ke alur verifikasi yang sesuai.
- Jangan mengungkap apakah email atau nomor telepon terdaftar.
- Sediakan jalur bantuan untuk masalah akses akun yang berulang.

## 6. Login Investor — `/masuk/investor`

Gunakan tata letak dan komponen yang sama dengan login Owner, dengan teks sesuai role:

- Judul: **Masuk sebagai Investor**
- Teks pendukung: **Jelajahi peluang usaha dan bangun koneksi dengan pemilik bisnis.**
- Tautan jika role tidak sesuai mengarah ke login Owner.
- Validasi role oleh backend wajib dilakukan.

Pertahankan tata letak visual yang konsisten agar kedua halaman terasa sebagai satu produk. Bedakan melalui teks dan indikator role yang sederhana, bukan desain yang sepenuhnya berbeda.

## 7. Pendaftaran Owner — `/daftar/pemilik`

### Prinsip pendaftaran

- Pisahkan pembuatan akun dari pengisian profil usaha yang lebih panjang.
- Gunakan onboarding bertahap; jangan meminta data keuangan usaha lengkap sebelum akun dibuat.
- Jelaskan alasan setiap field wajib.
- Simpan progres dengan aman jika didukung; jangan menghapus input secara diam-diam.
- Kumpulkan hanya data pribadi yang diperlukan.

### Langkah yang disarankan

**Langkah 1 — Akun**

- Nama lengkap.
- Pilih satu metode kontak: alamat email ATAU nomor telepon.
- Jika memilih email, kirim kode verifikasi ke alamat email tersebut.
- Jika memilih telepon, kirim kode verifikasi melalui WhatsApp.
- Pendaftaran dengan email/telepon wajib memakai kata sandi dan konfirmasi. Pendaftaran Google tidak meminta kata sandi lokal terpisah.
- Persetujuan Ketentuan dan Kebijakan Privasi wajib.
- Persetujuan pembaruan produk bersifat opsional dan tidak dicentang secara default.

**Langkah 2 — Verifikasi kontak**

- Jika memilih Email: masukkan kode sekali pakai yang dikirim ke alamat email tersebut.
- Jika memilih Telepon: masukkan kode sekali pakai yang dikirim melalui WhatsApp.
- Hanya kontak yang dipilih yang wajib diverifikasi untuk aktivasi awal. Kontak lainnya dapat ditambahkan dan diverifikasi kemudian.
- Tampilkan tujuan pengiriman dalam bentuk tersamarkan.
- Sediakan tindakan kirim ulang dengan jeda dan batas percobaan yang terlihat.
- Masa berlaku OTP dan status percobaan ulang harus jelas.
- Jangan mencatat atau menampilkan nilai OTP dalam pesan error yang terlihat oleh klien.

**Langkah 3 — Tahap usaha**

- Baru berupa ide.
- Sedang bersiap memulai.
- Sudah beroperasi.
- Sedang mengembangkan usaha yang berjalan.

**Langkah 4 — Profil awal usaha**

Minta hanya field yang relevan dengan tahap yang dipilih:

- Nama usaha atau ide.
- Sektor/kategori.
- Wilayah operasi atau target utama.
- Deskripsi singkat.
- Konfirmasi tahap usaha saat ini.
- Pertanyaan tambahan sesuai tahap dapat ditunda ke ruang kerja Owner.

**Penyelesaian**

- Halaman konfirmasi dengan langkah berikutnya.
- CTA menuju dashboard Owner.
- Checklist kelengkapan profil yang dapat diselesaikan kemudian.

## 8. Pendaftaran Investor — `/daftar/investor`

### Langkah yang disarankan

**Langkah 1 — Akun**

- Nama lengkap.
- Pilih satu metode kontak: alamat email ATAU nomor telepon.
- Jika memilih email, kirim kode verifikasi ke alamat email tersebut.
- Jika memilih telepon, kirim kode verifikasi melalui WhatsApp.
- Pendaftaran dengan email/telepon wajib memakai kata sandi dan field konfirmasi.
- Persetujuan Ketentuan dan Kebijakan Privasi wajib.
- Persetujuan pembaruan produk bersifat opsional dan tidak dicentang secara default.

**Langkah 2 — Verifikasi kontak**

- Gunakan pilihan Email atau Telepon/WhatsApp yang saling eksklusif dan pola verifikasi yang sama dengan pendaftaran Owner. Kata sandi dan konfirmasi wajib untuk jalur pendaftaran email/telepon ini.

**Langkah 3 — Profil Investor**

- Jenis/deskripsi profil Investor dengan kategori non-sensitif yang telah disetujui.
- Sektor pilihan.
- Wilayah target.
- Kisaran investasi indikatif, opsional dan dijelaskan sebagai preferensi penemuan.
- Horizon waktu pilihan, opsional.
- Preferensi risiko, opsional dan dinyatakan sebagai preferensi pribadi, bukan penilaian kesesuaian investasi.

**Penyelesaian**

- Halaman konfirmasi.
- CTA menuju dashboard Investor dan penemuan peluang.
- Preferensi dapat diubah kemudian.

## 9. Perilaku Google OAuth

- Google OAuth tetap tersedia untuk login dan pendaftaran.
- Sebelum memulai pendaftaran Google, pengguna harus memilih Owner atau Investor secara eksplisit; pilihan tersebut diikat ke state OAuth di server. Pendaftaran Google tidak meminta kata sandi lokal terpisah.
- Identitas Google baru tetap harus menyelesaikan verifikasi kontak yang dipilih: kode email ATAU kode telepon yang dikirim melalui WhatsApp. Email yang dinyatakan Google saja tidak memenuhi persyaratan ini.
- Identitas Google yang sudah terhubung hanya dapat masuk ke akun yang sudah ada. Role utama akun tetap dan menentukan portal yang benar.
- Jangan menghubungkan identitas secara otomatis hanya karena email cocok. Pengaitan memerlukan bukti kendali terbaru atas kedua identitas.
- Pendaftaran Admin tidak pernah tersedia melalui Google OAuth publik.
- Validasi state, nonce, issuer, audience, signature, expiry, redirect URI, dan perlindungan replay.

## 10. Pemulihan kata sandi

### Halaman permintaan — `/lupa-kata-sandi`

- Terima email atau nomor telepon akun yang sudah diverifikasi sebagai tujuan pemulihan, dengan challenge pemulihan terpisah. Kode aktivasi pendaftaran tidak boleh digunakan ulang.
- Gunakan pesan konfirmasi netral, baik akun ada maupun tidak.
- Terapkan rate limit dan pemantauan penyalahgunaan.
- Jangan mengungkap keberadaan akun.

### Halaman atur kata sandi — `/atur-kata-sandi`

- Token divalidasi di server dan memiliki masa berlaku.
- Sediakan field kata sandi baru dan konfirmasi.
- Tampilkan panduan kebijakan kata sandi sebelum pengiriman.
- Setelah berhasil, cabut sesi yang ada sesuai kebijakan keamanan dan arahkan pengguna ke halaman login role yang sesuai.
- Jika token tidak valid/kedaluwarsa, sediakan cara aman untuk meminta tautan/kode baru.

## 11. Login Admin — `/admin/login`

Autentikasi Admin merupakan pengalaman internal yang terpisah.

- Tidak ada pendaftaran Admin publik.
- Identitas Admin disediakan oleh administrator yang berwenang.
- Wajib menggunakan MFA; utamakan metode yang tahan phishing jika didukung.
- Terapkan masa sesi yang ketat dan autentikasi ulang untuk tindakan sensitif.
- Terapkan rate limit dan pemantauan keamanan yang lebih ketat daripada login klien biasa.
- Jangan menampilkan detail akun atau izin dalam error yang belum terautentikasi.
- Lindungi rute dan API Admin di sisi server dengan izin eksplisit.
- Catat keberhasilan/kegagalan autentikasi dan tindakan administratif sensitif ke log audit tanpa menyimpan kata sandi, OTP, atau secret sesi.

## 12. Perilaku responsif

### Perilaku breakpoint

Gunakan konvensi breakpoint proyek yang sudah ada setelah dikonfirmasi melalui token/konfigurasi frontend. Jangan menambahkan breakpoint satu kali yang arbitrer pada halaman tertentu.

- Desktop lebar: navigasi publik lengkap dan komposisi hero/form dua kolom jika sesuai.
- Tablet: kurangi kepadatan navigasi; susun hero secara vertikal saat kolom mulai terlalu sempit.
- Mobile: konten satu kolom, CTA bertumpuk, kontrol formulir selebar area konten, tanpa overflow horizontal.
- Formulir autentikasi harus tetap nyaman digunakan di layar sempit dan saat keyboard layar terbuka.
- Gunakan tipe input dan atribut autocomplete yang tepat untuk email, telepon, current-password, dan new-password.

### Aksesibilitas

- Semua kontrol dapat digunakan dengan keyboard.
- Setelah navigasi/pengiriman, fokus berpindah ke judul halaman atau field pertama yang tidak valid sesuai konteks.
- Pesan validasi terhubung secara programatis ke field terkait.
- Kontrol tampil/sembunyi kata sandi memiliki accessible name dan status yang bermakna.
- Input OTP mendukung paste dan teknologi bantu; jangan memaksa pengguna berpindah di antara enam input terpisah jika satu input semantik dapat digunakan.
- Kontras warna memenuhi target WCAG AA.
- Hormati reduced motion dan zoom browser.

## 13. Analitik dan peristiwa privasi

Kumpulkan hanya peristiwa yang diperlukan untuk memahami kegunaan produk dan keandalan operasional, misalnya:

- CTA publik yang dipilih (pilihan role).
- Alur pendaftaran dimulai/diselesaikan/ditinggalkan pada tiap langkah.
- Kategori keberhasilan/kegagalan verifikasi.
- Kategori keberhasilan/kegagalan login.
- Permintaan/penyelesaian pemulihan kata sandi.

Jangan mengirim kata sandi, nilai OTP, nomor telepon/email mentah, token, atau isi formulir sensitif ke analitik. Gunakan analitik atau tracking pihak ketiga hanya untuk tujuan yang terdokumentasi dan dasar pemrosesan yang sesuai. Consent pemasaran/analitik opsional harus terpisah dari ketentuan layanan inti. Data yang dikirim ke analitik harus diminimalkan dan tidak boleh berisi isi formulir sensitif.

## 14. Checklist penerimaan sebelum implementasi

### Halaman beranda publik

- [ ] Nilai utama platform jelas tanpa login.
- [ ] Jalur Owner dan Investor terlihat berbeda dan sama-sama mudah dipahami.
- [ ] Semua navigasi publik dan CTA memiliki tujuan yang ditetapkan.
- [ ] Pesan keanggotaan tidak memberi kesan bahwa chat hanya tersedia untuk Premium.
- [ ] Batasan platform dan pernyataan tanpa jaminan dijelaskan dengan jelas.
- [ ] Fallback hero tetap mempertahankan konten utama jika media gagal dimuat.

### Login

- [ ] Owner dan Investor memiliki halaman masuk terpisah.
- [ ] Perilaku formulir bersama tetap konsisten.
- [ ] Ketidaksesuaian role ditangani backend dan memberikan langkah berikutnya yang aman.
- [ ] Kontrol kata sandi, lupa kata sandi, loading, error, dan rate limit dirancang.
- [ ] Google OAuth mempertahankan pemilihan role wajib sebelum pendaftaran baru, verifikasi kontak terpisah, tanpa kewajiban kata sandi lokal terpisah, dan pengaitan identitas yang aman.

### Pendaftaran dan verifikasi

- [ ] Hanya Owner dan Investor yang dapat mendaftar secara publik.
- [ ] Pembuatan akun dipisahkan dari onboarding yang lebih panjang.
- [ ] Pilihan pendaftaran Email ATAU Telepon/WhatsApp terlihat jelas; verifikasi kanal terpilih, jeda kirim ulang, expiry, dan batas percobaan sudah ditetapkan.
- [ ] Consent opsional tidak dicentang secara default.
- [ ] Progres dan perilaku pemulihan alur sudah ditetapkan.

### Admin

- [ ] Admin tidak ditampilkan dalam pendaftaran publik.
- [ ] Portal Admin terpisah dan mewajibkan MFA.
- [ ] Pemeriksaan izin di server dan kebutuhan audit terdokumentasi.

### Kesiapan engineering

- [ ] Props dan status komponen UI bersama ditetapkan sebelum implementasi.
- [ ] Aturan validasi formulir didokumentasikan dan diuji.
- [ ] Pemeriksaan keyboard, mobile, dan aksesibilitas disertakan.
- [ ] Lockfile dependency frontend sudah tersedia sebelum pengerjaan fitur frontend yang substansial.
- [ ] Kontrak API autentikasi, model database, dan threat model ditinjau sebelum autentikasi nyata diimplementasikan.

## 15. Titik desain berikutnya

Setelah spesifikasi ini ditinjau, buat wireframe low-fidelity untuk:

1. Halaman beranda versi desktop dan mobile.
2. Pemilihan role pendaftaran.
3. Login Owner dan Investor.
4. Langkah pendaftaran Owner dan Investor.
5. Status verifikasi email/telepon dan pemulihan.
6. Login Admin.

Implementasi baru dimulai setelah wireframe dan kontrak autentikasi disepakati.
