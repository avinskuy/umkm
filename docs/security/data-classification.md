# Kebijakan Klasifikasi & Sensitivitas Data

- **Status:** Disetujui sebagai baseline
- **Tanggal:** 21 November 2026
- **Cakupan:** Seluruh data dan aset informasi USahapintar

## 1. Tujuan

Klasifikasi menentukan tingkat perlindungan berdasarkan dampak jika data diakses, diubah, hilang, atau dibagikan tanpa izin. Klasifikasi berlaku di seluruh siklus hidup data, bukan hanya pada database.

Kontrol mengikuti sensitivitas data meskipun data berada di API, log, file attachment, cache, backup, perangkat pengembangan, atau penyedia pihak ketiga.

## 2. Tingkat klasifikasi

| Tingkat | Nama | Contoh |
| --- | --- | --- |
| L1 | Publik | Profil publik, posting yang diterbitkan, informasi fitur yang memang tersedia untuk umum |
| L2 | Internal | Dokumen internal proyek, laporan operasional, konfigurasi non-rahasia |
| L3 | Rahasia | Kontak pribadi, percakapan, dokumen bisnis privat, informasi transaksi, data akun yang tidak dipublikasikan |
| L4 | Sangat Rahasia | Kredensial autentikasi, password hash, session token, secret, OTP aktif, bukti recovery, catatan investigasi keamanan |

Contoh bukan daftar tertutup. Pemilik data dan tim bertanggung jawab menetapkan klasifikasi yang sesuai ketika jenis data baru diperkenalkan.

## 3. Aturan perlindungan

- L1 dapat dipublikasikan hanya jika memang ditujukan untuk publik dan tidak mengandung data sensitif yang tersisip.
- L2 dibatasi untuk kebutuhan internal dan tidak boleh dianggap aman untuk dipublikasikan hanya karena tidak mengandung password.
- L3 memerlukan kontrol akses berbasis akun, tujuan, kepemilikan, dan persetujuan yang sesuai; enkripsi dan pembatasan ekspor diterapkan sesuai risiko.
- L4 mendapat perlindungan paling ketat. Batasi akses manusia dan layanan, jangan tampilkan secret atau OTP plaintext, dan jangan masukkan nilainya ke log biasa.
- Akses Admin tetap harus memiliki tujuan dan izin yang sesuai; role Admin tidak otomatis memberikan akses ke semua data.
- Data attachment memiliki klasifikasi sendiri. Attachment tidak otomatis menjadi publik hanya karena dipasang pada posting publik.
- Salinan, cache, export, log, dan backup mewarisi kebutuhan perlindungan dari data sumbernya atau klasifikasi yang lebih ketat jika digabungkan.

## 4. Siklus hidup

Klasifikasikan data saat dirancang atau dikumpulkan. Tinjau ulang jika tujuan, konteks penggunaan, atau dampak kebocoran berubah. Simpan hanya selama diperlukan, batasi pemakaian sekunder, dan hapus atau anonimisasi sesuai kebijakan retensi dan penghapusan.

## 5. Hubungan dengan kebijakan lain

- [Access Control & Authorization](access-control-authorization.md)
- [Encryption & Key Management](encryption-key-management.md)
- [Logging, Monitoring & Audit Trail](logging-monitoring-audit.md)
- [Database Security & Data Integrity](database-security-data-integrity.md)

## Referensi

- NIST SP 800-53.
- NIST Privacy Framework.
- OWASP Database Security Cheat Sheet.
