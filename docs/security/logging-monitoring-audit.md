# Logging, Monitoring & Audit Trail Policy

- **Status:** Disetujui sebagai baseline
- **Tanggal:** 21 November 2026
- **Cakupan:** Log aplikasi, keamanan, database, infrastruktur, dan tindakan Admin

## 1. Tujuan

Log dan audit trail membantu mendeteksi penyalahgunaan, menyelidiki gangguan, membuktikan tindakan penting, dan meningkatkan keandalan. Pencatatan harus berguna tanpa menjadi saluran kebocoran data.

## 2. Peristiwa yang perlu dicatat

Sesuai kebutuhan dan risiko, catat:

- Login berhasil/gagal dan peristiwa autentikasi penting.
- Perubahan password, kontak utama, kontak pemulihan, serta status akun.
- Pembuatan, penggunaan, dan pencabutan sesi.
- Penolakan otorisasi dan pola penyalahgunaan yang relevan.
- Permintaan dan hasil pemulihan akun.
- Perubahan privilege, konfigurasi keamanan, dan schema database.
- Tindakan Admin: aktor, tindakan, target, waktu, alasan minimum, dan hasil.
- Peristiwa deployment, perubahan release, dan kegagalan layanan penting.

## 3. Data yang dilarang atau harus diminimalkan

Jangan mencatat:

- Password, OTP, session token, reset token, API key, private key, atau secret plaintext.
- Isi percakapan privat atau dokumen rahasia ke log umum.
- Bukti recovery mentah atau data pembayaran sensitif.
- Data L3/L4 yang tidak diperlukan untuk tujuan pencatatan.

Gunakan identifier yang sesuai, masking, agregasi, atau referensi audit yang tidak mengungkap isi sensitif. Jangan memasukkan data pribadi ke pesan error atau telemetry tanpa kebutuhan yang jelas.

## 4. Integritas dan akses log

- Batasi akses log berdasarkan kebutuhan kerja.
- Lindungi audit trail dari perubahan atau penghapusan tanpa izin.
- Pisahkan hak membaca log dari hak mengubah konfigurasi aplikasi bila memungkinkan.
- Sinkronkan waktu sistem agar kronologi dapat dipercaya.
- Tetapkan retensi sesuai tujuan, klasifikasi, kebutuhan keamanan, dan kewajiban hukum.
- Simpan bukti insiden dengan kontrol akses dan rantai penguasaan yang sesuai.

## 5. Monitoring dan alert

Pantau indikator yang dapat ditindaklanjuti, seperti lonjakan kegagalan autentikasi, penolakan akses yang tidak biasa, penggunaan privilege, error rate, saturasi resource, kegagalan dependency, dan perubahan konfigurasi kritis.

Alert harus memiliki tingkat prioritas, pemilik respons, serta tindakan awal yang jelas. Hindari alert berlebihan yang tidak dapat ditindaklanjuti.

## 6. Hubungan dengan operasi

Log dan alert mendukung proses [Incident Response](../operations/incident-response.md) dan [Observability & Monitoring](../operations/observability-monitoring.md). Setiap investigasi harus mengikuti prinsip minimisasi data dan pembatasan akses.

## 7. Pengujian

- Verifikasi peristiwa penting tercatat dengan konteks minimum.
- Pastikan secret dan data sensitif tidak masuk log.
- Uji pembatasan akses dan perlindungan integritas log.
- Uji alert untuk skenario penting dan pastikan jalur eskalasi tersedia.
- Tinjau retensi dan penghapusan log secara berkala.

## Referensi

- NIST SP 800-53, Audit and Accountability.
- OWASP Logging Cheat Sheet.
- NIST SP 800-92, Guide to Computer Security Log Management.
