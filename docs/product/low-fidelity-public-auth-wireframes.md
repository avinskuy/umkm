# Wireframe Low-Fidelity — Situs Publik & Autentikasi

> Status: spesifikasi wireframe struktural  
> Dokumen terkait:
> - [Peta Situs & Rancangan UI/UX](./sitemap-and-ux-blueprint.md)
> - [Spesifikasi UI/UX Situs Publik & Autentikasi](./public-and-auth-uiux-spec.md)
>
> Wireframe ini menetapkan hierarki, penempatan, dan interaksi—bukan warna, gambar, tipografi, atau ukuran akhir yang presisi hingga tingkat piksel.

## 1. Konvensi wireframe

- `[ ... ]` mewakili kontrol, kartu, atau area konten.
- `( ... )` mewakili teks pendukung atau tindakan sekunder.
- `{ ... }` mewakili status atau konten bersyarat.
- Tata letak desktop menggunakan container konten terpusat dengan grid pemasaran yang lebih lebar.
- Tata letak mobile menggunakan satu kolom dengan tindakan yang disusun vertikal.
- Semua tindakan utama harus tetap terlihat tanpa perlu menggulir secara horizontal.

## 2. Halaman beranda

### Tampilan desktop

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [USahapintar]     Cara Kerja   Fitur   Keanggotaan   Tentang   [Masuk] [Daftar]│
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  [PENGANTAR PLATFORM]                           ┌───────────────────────────┐  │
│  Bangun koneksi.                            │                           │  │
│  Kembangkan peluang usaha.                  │     VISUAL HERO           │  │
│                                             │  Kolaborasi bisnis  │  │
│  Deskripsi nilai singkat                    │  / penemuan peluang │  │
│  dalam 2–3 baris yang mudah dibaca.                     │                           │  │
│                                             └───────────────────────────┘  │
│  [Daftar sebagai Pemilik Usaha]                                           │
│  [Daftar sebagai Investor]                                                │
│  (Pelajari cara kerja)                                                    │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│                       DUA JALUR UTAMA                                      │
│  ┌────────────────────────────┐      ┌────────────────────────────┐       │
│  │ PEMILIK USAHA              │      │ INVESTOR                   │       │
│  │ Promosikan usaha           │      │ Temukan peluang     │       │
│  │ Bangun profil usaha     │      │ Tinjau informasi yang dibagikan  │       │
│  │ Temukan mitra yang relevan     │      │ Terhubung dengan pemilik usaha        │       │
│  │ [Mulai sebagai Owner]      │      │ [Mulai sebagai Investor]   │       │
│  └────────────────────────────┘      └────────────────────────────┘       │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│  CARA KERJA                                                                │
│  [1 Buat profil] → [2 Temukan peluang] → [3 Ajukan koneksi] → [4 Terhubung]│
├────────────────────────────────────────────────────────────────────────────┤
│  FITUR UTAMA                                                               │
│  [Promosi usaha] [Discovery] [Koneksi & chat] [AI Intelligence]            │
├────────────────────────────────────────────────────────────────────────────┤
│  MEMBERSHIP                                                                │
│  Free to Connect, Premium to Grow                                          │
│  [Ringkasan Free]                    [Ringkasan Premium]                   │
│  (Lihat detail membership)                                                 │
├────────────────────────────────────────────────────────────────────────────┤
│  KEPERCAYAAN & TANGGUNG JAWAB                                                    │
│  Perkenalan bisnis, data yang disetujui pemilik, tanpa jaminan pendanaan atau keuntungan.    │
├────────────────────────────────────────────────────────────────────────────┤
│  CTA AKHIR: [Daftar sebagai Pemilik Usaha] [Daftar sebagai Investor]      │
├────────────────────────────────────────────────────────────────────────────┤
│  Footer: Platform | Bantuan | Legal | Kontak                              │
└────────────────────────────────────────────────────────────────────────────┘
```

### Tampilan mobile

```text
┌─────────────────────────────┐
│ [USahapintar]          [≡]  │
├─────────────────────────────┤
│ PLATFORM INTRO              │
│                             │
│ Bangun koneksi.             │
│ Kembangkan peluang usaha.   │
│                             │
│ Deskripsi nilai singkat     │
│                             │
│ [Daftar sebagai Pemilik]    │
│ [Daftar sebagai Investor]   │
│ (Pelajari cara kerja)       │
│                             │
│ ┌─────────────────────────┐ │
│ │       VISUAL HERO       │ │
│ └─────────────────────────┘ │
├─────────────────────────────┤
│ PEMILIK USAHA               │
│ Manfaat dalam poin singkat   │
│ [Mulai sebagai Owner]       │
├─────────────────────────────┤
│ INVESTOR                    │
│ Manfaat dalam poin singkat   │
│ [Mulai sebagai Investor]    │
├─────────────────────────────┤
│ CARA KERJA                  │
│ 1. Buat profil              │
│ 2. Temukan peluang          │
│ 3. Ajukan koneksi           │
│ 4. Mulai terhubung          │
├─────────────────────────────┤
│ FITUR UTAMA                 │
│ [Baris fitur]               │
│ [Baris fitur]               │
│ [Baris fitur]               │
├─────────────────────────────┤
│ MEMBERSHIP                  │
│ Free to Connect...          │
│ [Lihat Keanggotaan]          │
├─────────────────────────────┤
│ KEPERCAYAAN & TANGGUNG JAWAB      │
├─────────────────────────────┤
│ FINAL CTA                   │
│ [Daftar sebagai Pemilik]    │
│ [Daftar sebagai Investor]   │
├─────────────────────────────┤
│ Kelompok footer               │
└─────────────────────────────┘
```

### Interaksi halaman beranda

- Header Daftar membuka halaman pemilihan role.
- Header Masuk membuka pilihan Owner/Investor dalam tampilan ringkas.
- Setiap CTA role langsung mengarah ke halaman pendaftaran yang sesuai.
- Tautan fitur dan keanggotaan menggulir halaman atau membuka bagian/halaman publik yang sesuai.
- Saat menu mobile terbuka, fokus keyboard tetap berada di dalam menu; tombol Escape menutup menu dan fokus kembali ke pemicunya.
- Media hero memiliki tampilan pengganti yang netral; teks dan CTA tetap berfungsi tanpa menunggu media dimuat.

## 3. Pemilihan role pendaftaran — `/daftar`

### Tampilan desktop

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [USahapintar]                                      (Kembali ke Beranda)     │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│                       Mulai bersama USahapintar                            │
│             Pilih bagaimana kamu akan menggunakan platform ini.           │
│                                                                            │
│      ┌────────────────────────────────┐  ┌────────────────────────────────┐│
│      │ PEMILIK USAHA                  │  │ INVESTOR                       ││
│      │                                │  │                                ││
│      │ Promote your business, share   │  │ Temukan peluang and     ││
│      │ opportunities, find partners. │  │ connect with business owners. ││
│      │                                │  │                                ││
│      │ [Daftar sebagai Pemilik]       │  │ [Daftar sebagai Investor]      ││
│      └────────────────────────────────┘  └────────────────────────────────┘│
│                                                                            │
│                Sudah punya akun? (Masuk sebagai Owner / Investor)           │
└────────────────────────────────────────────────────────────────────────────┘
```

### Tampilan mobile

```text
┌─────────────────────────────┐
│ [USahapintar]               │
├─────────────────────────────┤
│ Mulai bersama USahapintar   │
│ Pilih role utama akunmu.   │
│                             │
│ ┌─────────────────────────┐ │
│ │ PEMILIK USAHA           │ │
│ │ Deskripsi singkat role  │ │
│ │ [Daftar sebagai Pemilik]│ │
│ └─────────────────────────┘ │
│                             │
│ ┌─────────────────────────┐ │
│ │ INVESTOR                │ │
│ │ Deskripsi singkat role  │ │
│ │ [Daftar sebagai Investor]││
│ └─────────────────────────┘ │
│                             │
│ Sudah punya akun?           │
│ [Masuk sebagai Owner]       │
│ [Masuk sebagai Investor]    │
└─────────────────────────────┘
```

Interaksi: pemilihan role mengarahkan pengguna ke jalur pendaftaran khusus role tersebut. Role bersifat tetap untuk akun dan tidak dapat diubah melalui pengaturan profil, bantuan, tindakan Admin, atau migrasi akun. Pengguna yang ingin memakai role klien lainnya harus mendaftar akun terpisah dengan identitas yang memenuhi syarat dan belum terhubung ke akun lain.

## 4. Login Owner dan Investor

Kedua halaman login menggunakan tata letak yang sama. Perbedaannya hanya pada judul sesuai role, teks bantuan, indikator role, dan tujuan tautan ketika role tidak sesuai.

### Tampilan desktop

```text
┌────────────────────────────────────────────────────────────────────────────┐
│ [USahapintar]                                      (Kembali ke Beranda)     │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│   ┌───────────────────────────────┐    ┌────────────────────────────────┐  │
│   │ KONTEKS AUTENTIKASI           │    │ MASUK SEBAGAI                  │  │
│   │                               │    │ [Pemilik Usaha / Investor]     │  │
│   │ Pesan singkat sesuai role     │    │                                │  │
│   │ dan ilustrasi sederhana       │    │ [Email atau nomor telepon]     │  │
│   │                               │    │ [Kata sandi             (eye)]  │  │
│   │                               │    │ (Lupa kata sandi?)             │  │
│   │                               │    │ [Masuk]                        │  │
│   │                               │    │                                │  │
│   │                               │    │ Belum punya akun? [Daftar]     │  │
│   └───────────────────────────────┘    └────────────────────────────────┘  │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### Tampilan mobile

```text
┌─────────────────────────────┐
│ [USahapintar]               │
├─────────────────────────────┤
│ (Kembali ke Beranda)        │
│                             │
│ MASUK SEBAGAI               │
│ Pemilik Usaha / Investor    │
│ Deskripsi singkat sesuai role│
│                             │
│ [Email atau nomor telepon]  │
│ [Kata sandi           (eye)]│
│ (Lupa kata sandi?)          │
│                             │
│ [Masuk]                     │
│                             │
│ Belum punya akun? [Daftar]  │
│                             │
│ [Privasi] [Ketentuan] [Bantuan] │
└─────────────────────────────┘
```

### Status login

```text
DEFAULT
  Field aktif; tombol kirim tersedia saat input valid.

SEDANG DIKIRIM
  Tombol kirim dinonaktifkan; indikator proses ditampilkan; permintaan ganda dicegah.

KREDENSIAL TIDAK VALID
  Tampilkan pemberitahuan netral; jangan ungkap apakah email/nomor telepon terdaftar.

ROLE TIDAK SESUAI
  Jelaskan bahwa akun ini menggunakan portal role lain dan berikan tautan ke portal yang benar.

DIBATASI SEMENTARA
  Tampilkan pesan percobaan ulang yang aman dan sisa waktu tunggu hanya jika diberikan server.

GANGGUAN JARINGAN
  Pertahankan field yang tidak sensitif dan sediakan opsi coba lagi.

OTP BELUM VALID
  Akun masih pending; pengguna dapat memasukkan kode atau meminta resend setelah cooldown.

OTP VALID
  Backend mengaktifkan akun dan mengarahkan pengguna ke onboarding.
```

## 5. Pendaftaran Owner

Pendaftaran dilakukan secara bertahap. Akun dibuat sebelum pengguna diminta mengisi informasi usaha yang lebih lengkap.

### Langkah 1 — Akun

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                               Langkah 1 dari 3 │
│                                                              │
│ Buat akun Pemilik Usaha                                      │
│                                                              │
│ Nama lengkap          [________________________________]     │
│ Metode kontak awal (pilih satu):                             │
│ ( ) Email   ( ) Telepon / WhatsApp                           │
│ Kontak yang dipilih    [______________________________]     │
│ Kata sandi             [______________________________]     │
│ Konfirmasi kata sandi  [______________________________]     │
│                                                              │
│ [ ] Saya menyetujui Ketentuan dan Kebijakan Privasi           │
│ [ ] Saya ingin menerima kabar produk (opsional)               │
│                                                              │
│ [Kirim kode OTP]                                               │
│ Sudah punya akun? [Masuk]                                     │
└──────────────────────────────────────────────────────────────┘
```

### Konfirmasi — Akun dibuat

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                         Pendaftaran berhasil │
│                                                              │
│ Akun berhasil dibuat                                         │
│ Kamu dapat masuk menggunakan kontak dan kata sandi.          │
│                                                              │
│ Status kontak: Belum diverifikasi                             │
│ Kontak perlu dibuktikan sebelum dipakai untuk pemulihan atau  │
│ tindakan sensitif.                                           │
│                                                              │
│ [Lanjutkan ke profil]                                        │
└──────────────────────────────────────────────────────────────┘
```

OTP tidak diminta pada registrasi maupun login tahap pertama. Verifikasi kontak dan OTP ditunda ke tahap berikutnya; UI tidak boleh memberi kesan bahwa kontak yang baru dimasukkan sudah terverifikasi.

### Langkah 2 — Tahap usaha

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                               Langkah 2 dari 3 │
│                                                              │
│ Sejauh mana perjalanan usahamu?                              │
│                                                              │
│ ( ) Saya masih punya ide usaha                               │
│ ( ) Saya sedang mempersiapkan usaha                           │
│ ( ) Usaha saya sudah berjalan                                │
│ ( ) Saya ingin mengembangkan usaha                           │
│                                                              │
│ [Kembali]                                      [Lanjutkan]    │
└──────────────────────────────────────────────────────────────┘
```

### Langkah 3 — Profil awal usaha

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                               Langkah 3 dari 3 │
│                                                              │
│ Ceritakan sedikit tentang usahamu                            │
│                                                              │
│ Nama usaha/ide         [______________________________]      │
│ Bidang usaha           [Pilih kategori               v]      │
│ Wilayah                [Pilih wilayah                 v]      │
│ Deskripsi singkat      [______________________________]      │
│                        [______________________________]      │
│                                                              │
│ (Kamu bisa melengkapi profil setelah masuk.)                 │
│ [Kembali]                                      [Selesaikan]   │
└──────────────────────────────────────────────────────────────┘
```

Jika tahap usaha masih berupa ide, beri label nama usaha sebagai nama ide/proyek dan jangan mewajibkan metrik operasional.

## 6. Pendaftaran Investor

### Langkah 1 dan 2

Gunakan kembali tata letak akun dan konfirmasi pembuatan akun tanpa OTP dari pendaftaran Owner; cukup ubah teks yang menunjukkan role.

### Langkah 3 — Preferensi Investor

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                               Langkah 3 dari 3 │
│                                                              │
│ Atur preferensi jelajah                                      │
│ (Informasi ini membantu menyaring peluang; bisa diubah.)     │
│                                                              │
│ Bidang yang diminati   [Pilih satu atau beberapa       v]    │
│ Wilayah target         [Pilih wilayah                 v]     │
│ Rentang indikatif      [Opsional                      v]     │
│ Horizon waktu          [Opsional                      v]     │
│ Preferensi risiko      [Opsional                      v]     │
│                                                              │
│ [ ] Saya memahami bahwa preferensi bukan rekomendasi         │
│     investasi atau jaminan hasil.                            │
│                                                              │
│ [Kembali]                                      [Selesaikan]   │
└──────────────────────────────────────────────────────────────┘
```

Rentang keuangan dan preferensi risiko tetap opsional pada onboarding awal, kecuali tinjauan produk/legal kemudian menetapkan alasan yang kuat untuk mewajibkannya.

## 7. Pemulihan dan pengaturan ulang kata sandi

### Permintaan pengaturan ulang

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                                                 │
│                                                              │
│ Lupa kata sandi?                                             │
│ Masukkan email atau nomor telepon yang terkait dengan akun. │
│                                                              │
│ [Email atau nomor telepon____________________________]       │
│ [Kirim instruksi]                                            │
│                                                              │
│ {Pesan konfirmasi netral ditampilkan terlepas dari keberadaan akun}  │
│ (Kembali ke halaman masuk)                                   │
└──────────────────────────────────────────────────────────────┘
```

### Atur kata sandi baru

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar]                                                 │
│                                                              │
│ Buat kata sandi baru                                         │
│ [Kata sandi baru______________________________ (eye)]        │
│ [Ulangi kata sandi____________________________ (eye)]        │
│ [Simpan kata sandi]                                          │
│                                                              │
│ {Token kedaluwarsa: sediakan tindakan untuk meminta tautan baru}                │
└──────────────────────────────────────────────────────────────┘
```

## 8. Login Admin

```text
┌──────────────────────────────────────────────────────────────┐
│ [USahapintar Admin]                         Akses internal  │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│ Akses Administrator                                         │
│                                                              │
│ Email admin             [____________________________]       │
│ Kata sandi              [____________________________]       │
│ [Masuk]                                                     │
│                                                              │
│ {Tantangan MFA setelah autentikasi utama}                 │
│ Tantangan authenticator/passkey                            │
│                                                              │
│ (Hubungi super-admin jika akses bermasalah.)                 │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

Tidak ada pendaftaran Admin publik atau pemilih role Admin. MFA Admin wajib diterapkan sebelum penggunaan produksi; metode/penyedia MFA yang tepat masih menjadi keputusan implementasi. Kebijakan pemulihan akun ditetapkan dalam keputusan Identity Core yang telah disetujui; penyedia layanan, penyimpanan bukti, dan detail operasional tetap harus ditinjau saat implementasi.

## 9. Status error, pemuatan, dan kosong

| Status | Penempatan tampilan | Perilaku |
| --- | --- | --- |
| Validasi field | Tepat di bawah field | Jelaskan perbaikannya dan hubungkan dengan input |
| Error formulir | Di atas tombol kirim | Penjelasan netral tanpa data sensitif |
| Memuat | Tombol kirim | Cegah pengiriman ganda dan pertahankan tata letak |
| Berhasil | Konfirmasi inline atau halaman berikutnya | Jelaskan tindakan selanjutnya |
| Gangguan jaringan | Pemberitahuan di formulir | Coba lagi tanpa menghapus input yang aman |
| Batas percobaan | Pemberitahuan di formulir | Jelaskan waktu tunggu jika diketahui |
| Kontak belum diverifikasi | Status akun/kontak | Jelaskan batas pemakaian kontak tanpa menghalangi login normal |
| Permintaan autentikasi gagal | Pemberitahuan inline | Izinkan pengguna mencoba lagi setelah memeriksa koneksi |

Jangan gunakan warna sebagai satu-satunya penanda status. Buat pesan error singkat dan menjelaskan tindakan yang dapat dilakukan.

## 10. Daftar komponen untuk tahap implementasi pertama

| Komponen | Tanggung jawab | Status awal |
| --- | --- | --- |
| PublicHeader | Navigasi publik dan pilihan role | Desktop, menu mobile terbuka/tertutup |
| PublicFooter | Tautan publik, legal, dan bantuan | Kelompok responsif |
| RoleChoiceCard | Pilihan pendaftaran Owner/Investor | Normal, fokus, dipilih |
| AuthLayout | Tata letak login bersama per role | Desktop, mobile |
| TextField | Input teks berlabel | Normal, fokus, error, nonaktif |
| PasswordField | Input kata sandi dengan kontrol tampil/sembunyi | Tersembunyi, terlihat, error |
| PhoneField | Input nomor telepon internasional | Normal, tidak valid |
| ContactVerificationNotice | Status verifikasi kontak | Belum diverifikasi, terverifikasi, perlu tindakan |
| StepIndicator | Progres multi-langkah | Saat ini, selesai, berikutnya |
| FormNotice | Pesan status formulir | Info, error, berhasil |
| LoadingButton | Tombol kirim asinkron | Diam, memuat, nonaktif |

Tempatkan style komponen dekat fitur terkait atau dalam CSS Modules. Jangan memusatkan seluruh style halaman ke dalam satu stylesheet besar.

## 11. Daftar pemeriksaan

- [ ] Hierarki desktop dan mobile jelas pada setiap layar.
- [ ] Owner dan Investor dapat dibedakan tanpa membuat dua design system yang tidak saling terkait.
- [ ] Admin terpisah dengan jelas dari alur pengguna publik.
- [ ] Tidak ada pemilih role pada profil pengguna yang sudah login seolah-olah role dapat diganti kapan saja.
- [ ] Semua formulir memiliki label yang terlihat, validasi inline, dan status error dari server.
- [ ] Registrasi dan login tahap pertama tidak meminta OTP; kontak belum terverifikasi diberi batas penggunaan yang jelas.
- [ ] Tata letak mobile tidak overflow secara horizontal dan tetap nyaman digunakan saat keyboard layar terbuka.
- [ ] Keanggotaan copy preserves free approved-connection chat.
- [ ] Tidak ada UI yang menyiratkan jaminan pendanaan, hasil investasi, atau metrik terverifikasi tanpa bukti.
- [ ] Setiap tindakan memiliki tujuan atau transisi status yang terdokumentasi.

## 12. Serah terima ke implementasi

Setelah wireframe disetujui:

1. Tetapkan teks final, field wajib/opsional, dan perilaku ketika role Owner/Investor tidak sesuai.
2. Commit lockfile frontend dan stabilkan instalasi dependency.
3. Tetapkan kontrak API autentikasi dan migration database.
4. Tinjau model ancaman autentikasi (penyalahgunaan OTP, session fixation, reset kata sandi, enumerasi akun, CSRF/CORS, dan rate limit).
5. Implementasikan komponen dasar formulir bersama serta kerangka situs publik/autentikasi.
6. Tambahkan pengujian komponen, integrasi, dan browser yang terarah sebelum menghubungkan penyedia autentikasi nyata.

Implementasi autentikasi tidak boleh dimulai hanya berdasarkan wireframe ini; kontrak API dan rancangan keamanan harus tersedia lebih dahulu.
