# Kebijakan Kontrol Akses & Otorisasi

- **Status:** Disetujui sebagai baseline
- **Tanggal:** 21 November 2026
- **Cakupan:** Pengguna, resource, API, data privat, dan operasi Admin

## 1. Model otorisasi

USahapintar menggunakan gabungan **Role-Based Access Control (RBAC)** dan **Attribute-Based Access Control (ABAC)**.

RBAC menetapkan kemampuan dasar berdasarkan role kanonis:

- `business_owner`
- `investor`
- `admin`

ABAC mengevaluasi konteks setiap permintaan, termasuk identitas peminta, status akun, kepemilikan resource, status publikasi, status koneksi, persetujuan berbagi dokumen, klasifikasi data, tindakan yang diminta, dan tujuan akses.

Role utama Business Owner atau Investor bersifat tetap setelah pendaftaran. Admin disediakan melalui proses internal; tidak ada pendaftaran Admin publik. Kebijakan role dijelaskan lebih lanjut dalam dokumen Identity Core dan keputusan produk.

## 2. Prinsip wajib

- Terapkan **least privilege** dan **deny-by-default**.
- Backend menjadi sumber kebenaran; kondisi UI atau klaim dari client tidak memberikan hak akses.
- Lakukan pemeriksaan autentikasi dan otorisasi pada setiap permintaan backend yang dilindungi.
- Periksa akses pada tingkat objek/resource dan properti/field, bukan hanya endpoint atau menu.
- Gagal secara tertutup (*fail closed*) jika konteks atau keputusan otorisasi tidak dapat dipastikan.
- Pisahkan autentikasi, status akun, role, izin resource, persetujuan, status moderasi, dan entitlement membership.
- Catat akses dan perubahan sensitif sesuai kebijakan audit tanpa merekam isi rahasia yang tidak diperlukan.

## 3. Urutan keputusan akses

Untuk setiap tindakan yang dilindungi, backend setidaknya memeriksa:

1. Sesi valid, belum kedaluwarsa, dan belum dicabut.
2. Status akun mengizinkan tindakan.
3. Role utama mengizinkan kemampuan dasar.
4. Peminta memiliki resource atau merupakan pihak yang berhak mengaksesnya.
5. Status resource dan konteks mengizinkan tindakan.
6. Persetujuan pemilik tersedia jika data atau dokumen dibagikan secara privat.
7. Batasan keselamatan/moderasi dipenuhi.
8. Entitlement dan kuota membership diperiksa jika relevan.

Membership Premium tidak boleh melewati role, kepemilikan, persetujuan, status akun, atau pembatasan moderasi.

## 4. Akses Admin

Admin memiliki akses operasional yang luas sesuai kebutuhan tahap proyek, tetapi bukan akses tanpa batas.

- Berikan izin berdasarkan fungsi dan kebutuhan operasional.
- Batasi akses ke percakapan privat hanya untuk laporan pengguna, permintaan dukungan yang relevan, atau investigasi keamanan yang sah; batasi pada percakapan terkait dan catat aksesnya.
- Lindungi kredensial, OTP, token, bukti recovery, data pembayaran sensitif, serta dokumen privat.
- Password dan OTP plaintext tidak boleh terlihat oleh Admin.
- Akses ke data sensitif harus memiliki tujuan yang sah, cakupan minimum, dan audit trail.
- Admin tidak dapat mengubah role utama yang immutable atau melakukan penghapusan permanen akun secara sepihak.
- Perubahan password dan kontak utama harus melalui proses keamanan yang semestinya, bukan pengeditan langsung oleh Admin.

## 5. Pemisahan tugas dan peninjauan

Pisahkan tanggung jawab untuk tindakan sensitif jika layak, misalnya persetujuan akses istimewa, penanganan recovery berisiko tinggi, dan perubahan konfigurasi production. Tinjau izin secara berkala dan cabut hak yang tidak lagi diperlukan.

## 6. Pengujian wajib

- Akses sah dan penolakan akses untuk setiap role.
- BOLA/IDOR dan akses lintas akun.
- BFLA dan eskalasi privilege.
- BOPLA serta kebocoran field.
- Kepemilikan resource, status koneksi, dan persetujuan dokumen.
- Pembatasan akses Admin dan pencatatan audit.
- Perubahan status akun dan pencabutan sesi.
- Kegagalan pemeriksaan otorisasi harus berakhir dengan penolakan, bukan akses terbuka.

## Referensi

- OWASP API Security Top 10 (2023).
- OWASP ASVS 5.0.0.
- NIST SP 800-53, khususnya kontrol Access Control dan Audit and Accountability.
