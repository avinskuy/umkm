# Kebijakan Berbagi Data Pihak Ketiga & Perlindungan Vendor

- **Status:** Disetujui sebagai baseline keamanan dan tata kelola data.
- **Cakupan:** Penyedia infrastruktur, vendor operasional, mitra transaksi, subprocessor, integrasi yang diaktifkan pengguna, dan pengungkapan data kepada pihak eksternal.
- **Hubungan:** Melengkapi Dependency & Third-Party Component Security serta Kebijakan Persetujuan & Pemrosesan Data.

## 1. Prinsip utama

- Bagikan data sesedikit mungkin untuk tujuan yang telah ditentukan.
- Tentukan dasar pemrosesan, penerima, ruang lingkup, dan masa penggunaan sebelum berbagi.
- Terapkan klasifikasi L1–L4 dan kontrol sesuai sensitivitas.
- Gunakan akses least privilege dan batasi akses vendor berdasarkan tugas.
- USahapintar tetap bertanggung jawab atas pemilihan, pengawasan, dan penggunaan vendor.
- Jangan menjual data pengguna atau mengizinkan vendor menggunakan data untuk marketing/tujuan sendiri yang tidak disetujui.
- Jangan menganggap adanya kontrak vendor sebagai pengganti otorisasi pemilik atau dasar pemrosesan yang sesuai.

## 2. Kategori pihak ketiga

Kelompokkan penerima data setidaknya menjadi:

1. **Penyedia infrastruktur:** hosting, database terkelola, penyimpanan file, jaringan, backup, dan layanan cloud.
2. **Layanan operasional:** email, OTP/WhatsApp, monitoring, analitik operasional, dukungan pelanggan, dan keamanan.
3. **Mitra transaksi:** penyedia pembayaran atau layanan yang diperlukan untuk transaksi jika fitur tersebut diaktifkan.
4. **Integrasi yang diizinkan pengguna:** aplikasi atau layanan eksternal yang dihubungkan secara sadar oleh pengguna.
5. **Otoritas pemerintah/penegak hukum:** ditangani sebagai kategori pengungkapan terpisah berdasarkan proses dan dasar hukum yang sesuai.

Setiap kategori harus memiliki inventaris penerima, fungsi, kategori data, lokasi pemrosesan bila diketahui, dan penanggung jawab internal.

## 3. Persyaratan sebelum berbagi

Sebelum mengaktifkan vendor atau integrasi:

- Tetapkan tujuan dan kebutuhan data.
- Pastikan dasar pemrosesan dan pemberitahuan yang sesuai.
- Bagikan data minimum yang diperlukan.
- Tentukan klasifikasi data dan kontrol keamanan.
- Evaluasi reputasi, praktik keamanan, akses, lokasi pemrosesan, subprocessor, dan kemampuan respons insiden.
- Tinjau risiko ketergantungan dan rencana penggantian atau penghentian layanan penting.
- Dapatkan persetujuan pemilik jika pembagian memerlukan otorisasi pengguna.
- Dokumentasikan keputusan dan pemilik hubungan vendor.

Jangan mengirim dokumen bisnis privat, percakapan, kontak pribadi, bukti recovery, atau data L4 ke layanan eksternal kecuali benar-benar diperlukan, memiliki dasar dan perlindungan yang memadai, serta secara eksplisit diizinkan oleh kebijakan terkait.

## 4. Perjanjian dan kewajiban vendor

Perjanjian vendor yang memproses data pengguna harus mengatur, sesuai peran dan ketentuan yang berlaku:

- Kerahasiaan dan personel yang berwenang.
- Tujuan, instruksi, ruang lingkup, dan durasi pemrosesan.
- Kategori data dan subjek data.
- Standar keamanan minimum dan kontrol akses.
- Penggunaan serta persetujuan subprocessor.
- Pemberitahuan dan kerja sama penanganan insiden.
- Bantuan untuk permintaan pengguna dan kewajiban hukum.
- Pengembalian atau penghapusan data setelah layanan berakhir, dengan pengecualian retensi yang sah.
- Transparansi lokasi pemrosesan dan transfer lintas negara.
- Hak evaluasi atau audit yang proporsional dengan risiko.

Detail kontrak dan kewajiban hukum perlu ditinjau sebelum penggunaan vendor dalam produksi.

## 5. Batas penggunaan data

Dilarang:

- Menjual data pengguna.
- Menggunakan data privat atau rahasia untuk marketing vendor.
- Membagikan dokumen bisnis privat tanpa izin pemilik yang diperlukan.
- Memberi vendor akses produksi yang lebih luas daripada kebutuhan.
- Menggunakan data setelah tujuan layanan berakhir tanpa dasar yang sah.
- Mengizinkan subprocessor yang tidak dievaluasi atau tidak tercakup dalam kontrol vendor.
- Memindahkan data ke negara atau lokasi pemrosesan lain tanpa evaluasi persyaratan yang berlaku.
- Menyertakan secret, password, OTP, token, atau data L4 dalam tiket dukungan biasa atau analitik vendor.

## 6. Integrasi yang diaktifkan pengguna

- Jelaskan jenis data dan izin yang diminta sebelum pengguna mengaktifkan integrasi.
- Minta scope minimum.
- Sediakan cara untuk memutus integrasi dan mencabut kredensial.
- Validasi respons eksternal, gunakan timeout, dan tangani kegagalan secara aman.
- Jangan mempercayai data eksternal sebagai bukti identitas, kepemilikan, atau otorisasi tanpa validasi.
- Catat aktivitas integrasi yang relevan tanpa menyimpan secret.
- Hormati pencabutan izin untuk akses dan pengiriman berikutnya.

## 7. Pengungkapan kepada otoritas

Permintaan dari pemerintah atau penegak hukum ditangani melalui proses terpisah:

- Verifikasi keaslian dan kewenangan permintaan.
- Pastikan dasar hukum dan yurisdiksi yang sesuai.
- Batasi data pada ruang lingkup yang diwajibkan.
- Eskalasi untuk tinjauan hukum jika diperlukan.
- Catat keputusan, data yang diungkap, waktu, dan pihak yang menyetujui.
- Beri tahu pengguna jika diizinkan dan tidak mengganggu proses yang sah.

Jangan menjanjikan pemberitahuan kepada pengguna jika hukum melarang atau terdapat pembatasan yang sah.

## 8. Pengawasan dan penghentian vendor

- Tinjau akses, scope, subprocessor, dan kebutuhan vendor secara berkala serta ketika terjadi perubahan material.
- Cabut kredensial dan akses yang tidak lagi diperlukan.
- Ketika kontrak atau tujuan berakhir, hentikan akses, kembalikan atau hapus data sesuai ketentuan, dan minta bukti bila sesuai.
- Pastikan salinan backup vendor mengikuti retensi dan prosedur penghapusan yang disepakati.
- Evaluasi dampak dan alternatif jika vendor penting mengalami gangguan atau kompromi.

## 9. Pengujian dan bukti

- Pastikan hanya field yang diperlukan yang dikirim ke vendor.
- Uji scope integrasi dan pencabutan akses.
- Uji bahwa dokumen privat tidak dibagikan tanpa izin.
- Uji bahwa secret dan data L4 tidak masuk log atau tiket dukungan biasa.
- Verifikasi autentikasi webhook dan perlindungan replay jika digunakan.
- Simpan inventaris vendor, hasil evaluasi, perjanjian, perubahan scope, dan catatan insiden.

## 10. Referensi

- NIST Privacy Framework.
- NIST SP 800-53.
- ISO/IEC 27001 dan ISO/IEC 27701 sebagai referensi pengelolaan keamanan serta privasi.
- OWASP API Security Top 10.
- OWASP Third-Party JavaScript Management Cheat Sheet bila relevan untuk integrasi web.

Penerapan kebijakan ini tidak menyatakan sertifikasi atau kepatuhan hukum otomatis. Persyaratan kontrak, transfer data, dan pengungkapan harus ditinjau sesuai yurisdiksi serta layanan yang benar-benar digunakan.
