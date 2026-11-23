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

Role utama Business Owner atau Investor bersifat tetap setelah pendaftaran. Hanya ada satu akun Admin untuk platform, dibuat melalui provisioning internal; tidak ada pendaftaran Admin publik maupun sub-role Admin. Akun Admin menggunakan autentikasi email + password dan TOTP authenticator 6 digit. Kebijakan teknis Admin dijelaskan dalam [Autentikasi Admin dan MFA](admin-authentication-and-mfa.md).

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

## 5. Akun Admin tunggal

- Platform hanya memiliki satu akun Admin aktif yang diprovisikan secara internal.
- Tidak ada Super Admin, Verification Admin, sub-role, atau pendaftaran Admin publik.
- Akun Admin memiliki izin operasional yang ditetapkan secara eksplisit, termasuk peninjauan pemulihan akun.
- Semua tindakan sensitif wajib memiliki audit trail; akun Admin tidak boleh melihat password, OTP, secret TOTP plaintext, atau token reset.
- Login Admin wajib menggunakan email + password dan TOTP authenticator 6 digit. Kode TOTP hanya dapat digunakan sekali per time-step.
- Jika akun Admin terkunci atau authenticator hilang, pemulihan dilakukan melalui prosedur operator internal yang tercatat, bukan endpoint publik atau bypass MFA.

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
