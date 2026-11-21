# Penanganan Insiden & Manajemen Insiden Keamanan

- **Status:** Disetujui sebagai baseline kebijakan dan prosedur.
- **Acuan utama:** NIST Cybersecurity Framework (CSF) 2.0 dan NIST SP 800-61 Rev. 3.
- **Acuan pelengkap:** CISA Incident Response Playbooks, AWS Well-Architected Security Pillar, dan Microsoft Cloud Security Benchmark.
- **Cakupan:** Insiden keamanan, privasi, ketersediaan, integritas data, dan gangguan layanan.

## 1. Tujuan dan prinsip

Proses respons insiden harus membatasi dampak, melindungi pengguna dan data, memulihkan layanan secara aman, serta menghasilkan perbaikan yang dapat ditindaklanjuti.

Gunakan enam fungsi NIST CSF 2.0 sebagai kerangka menyeluruh:

- **Govern:** peran, kewenangan, kebijakan, dan komunikasi.
- **Identify:** aset, layanan, ancaman, dan dampak.
- **Protect:** kontrol pencegahan dan kesiapan.
- **Detect:** monitoring, laporan, dan validasi indikasi.
- **Respond:** analisis, containment, mitigasi, dan komunikasi.
- **Recover:** pemulihan, verifikasi, dan peningkatan.

Insiden tidak boleh dinyatakan sebagai kebocoran data hanya berdasarkan alert yang belum divalidasi. Sebaliknya, indikasi yang kredibel harus segera ditangani tanpa menunggu kepastian sempurna.

## 2. Ruang lingkup dan contoh insiden

Contoh insiden meliputi:

- Akun atau kredensial Admin/layanan diduga diambil alih.
- Akses lintas akun atau eskalasi privilege.
- Paparan data pribadi, dokumen privat, atau data L3/L4.
- Kerentanan aktif atau dependency berbahaya.
- Perubahan database atau data tanpa otorisasi.
- Gangguan API, database, autentikasi, atau layanan inti.
- Kehilangan integritas data, backup, atau artefak release.
- Penyalahgunaan integrasi pihak ketiga atau secret.
- Pelanggaran vendor yang berdampak pada data/layanan USahapintar.

Laporan dapat berasal dari pengguna, Admin, tim teknis, monitoring, penyedia layanan, atau pihak eksternal.

## 3. Tingkat keparahan dan eskalasi

Gunakan empat tingkat awal. Penetapan akhir mempertimbangkan dampak aktual, paparan, sensitivitas data, jumlah pengguna terdampak, serta urgensi.

| Tingkat | Kriteria umum | Respons |
| --- | --- | --- |
| **SEV-1 Critical** | Kompromi aktif yang luas, paparan L4 atau data sensitif berskala besar, kehilangan integritas kritis, atau layanan inti tidak tersedia dengan dampak berat | Eskalasi segera; containment dan koordinasi pimpinan |
| **SEV-2 High** | Kompromi atau gangguan signifikan dengan ruang lingkup terbatas, akses tidak sah yang kredibel, atau degradasi layanan penting | Prioritas tinggi; pemilik insiden ditetapkan dan tindakan mitigasi dipantau |
| **SEV-3 Medium** | Dampak terbatas, gangguan sebagian, atau indikasi yang memerlukan investigasi terarah | Ditangani melalui proses respons terjadwal dengan pemantauan |
| **SEV-4 Low** | Anomali berdampak rendah atau kejadian yang tidak menunjukkan kompromi/kerusakan berarti | Dicatat, dianalisis, dan ditangani sesuai prioritas |

Tingkat dapat dinaikkan atau diturunkan berdasarkan bukti. Jangan menetapkan SLA numerik sebelum kapasitas dan kebutuhan layanan ditinjau.

## 4. Peran dan tanggung jawab

Untuk setiap insiden, tetapkan:

- **Incident Owner:** bertanggung jawab atas koordinasi dan keputusan operasional dalam kewenangannya.
- **Technical Lead:** memimpin investigasi teknis, containment, dan pemulihan.
- **Security/Privacy Reviewer:** menilai paparan keamanan, data, dan kebutuhan eskalasi.
- **Communications Owner:** mengoordinasikan komunikasi kepada pengguna, mitra, dan pihak terkait.
- **Decision Authority:** memberi persetujuan untuk tindakan berdampak besar sesuai tata kelola.

Pada tahap tim kecil, satu orang dapat memegang lebih dari satu peran. Pemisahan peran harus diperkuat ketika kapasitas tim memungkinkan. Tindakan darurat tetap dicatat dan ditinjau setelahnya.

## 5. Siklus penanganan

### 5.1 Terima dan catat

- Terima laporan melalui kanal yang tersedia.
- Berikan ID insiden unik.
- Catat waktu pertama diketahui, sumber laporan, layanan/asset yang diduga terdampak, dan ringkasan indikasi.
- Simpan bukti awal secara aman; jangan menyalin secret atau data pribadi berlebihan ke tiket umum.

### 5.2 Validasi dan klasifikasi

- Periksa apakah indikasi dapat diverifikasi.
- Identifikasi aset, akun, environment, dependency, dan data yang mungkin terdampak.
- Tentukan severity awal dan pemilik insiden.
- Catat asumsi serta hal yang belum diketahui.
- Eskalasi jika dampak atau paparan meningkat.

### 5.3 Pembatasan dampak

Pilih tindakan yang proporsional, misalnya:

- Cabut sesi, token, key, atau kredensial yang diduga kompromi.
- Nonaktifkan integrasi atau endpoint yang terdampak.
- Batasi akses atau isolasi komponen yang terpengaruh.
- Hentikan release atau proses yang memperluas dampak.
- Terapkan pembatasan sementara pada fitur terkait.

Jaga bukti yang diperlukan dan hindari tindakan yang dapat memperburuk kehilangan data. Catat siapa yang memutuskan dan menjalankan containment.

### 5.4 Eradikasi dan perbaikan

- Temukan akar penyebab sejauh bukti memungkinkan.
- Hapus persistence atau akses tidak sah.
- Patch aplikasi, dependency, konfigurasi, atau infrastruktur.
- Rotasi/revoke secret dan credential yang terdampak.
- Validasi bahwa jalur penyebab telah ditutup.
- Uji perbaikan sebelum membuka kembali layanan.

### 5.5 Komunikasi

- Berikan informasi yang akurat, relevan, dan tidak spekulatif.
- Tentukan pihak yang perlu diberi tahu berdasarkan dampak, kewajiban, dan kebijakan.
- Jika data pengguna terdampak, koordinasikan pemberitahuan sesuai hasil investigasi dan persyaratan hukum yang berlaku.
- Jangan mengungkap detail teknis yang meningkatkan risiko, data pengguna lain, atau bukti investigasi.
- Koordinasikan insiden vendor dengan penyedia terkait.

### 5.6 Pemulihan

- Pulihkan layanan dari kondisi yang diketahui aman.
- Verifikasi autentikasi, otorisasi, integritas database, konfigurasi, dan dependency.
- Periksa bahwa data yang seharusnya dihapus tidak aktif kembali setelah restore.
- Pantau layanan setelah pemulihan dan tetapkan kondisi untuk eskalasi ulang.
- Catat keputusan membuka kembali layanan.

## 6. Bukti dan catatan insiden

Setiap insiden minimal memiliki:

- ID dan severity.
- Incident Owner dan pihak yang terlibat.
- Kronologi dengan timestamp tersinkronisasi.
- Layanan, aset, environment, dan data yang terdampak.
- Dampak yang diketahui serta ketidakpastian.
- Bukti dan lokasi penyimpanan bukti dengan akses terbatas.
- Keputusan, tindakan, pelaksana, dan hasil.
- Komunikasi dan pemberitahuan.
- Status pemulihan, akar penyebab, dan tindak lanjut.

Lindungi bukti dari perubahan tanpa izin. Jangan mencatat password, OTP, token, private key, atau isi data sensitif ke log/tiket umum. Terapkan prinsip chain of custody sesuai kebutuhan investigasi.

## 7. Penutupan dan tinjauan pascainsiden

Insiden dapat ditutup setelah dampak terkendali, layanan diverifikasi, tindakan sementara memiliki rencana, dan pemilik tindak lanjut ditetapkan.

Post-incident review harus menilai:

- Kronologi dan penyebab utama/kontributor.
- Apa yang berjalan baik dan apa yang gagal.
- Dampak terhadap pengguna, data, dan layanan.
- Kualitas deteksi, eskalasi, komunikasi, dan pemulihan.
- Kontrol pencegahan dan pengujian yang perlu ditingkatkan.
- Tindakan korektif, pemilik, prioritas, dan status.

Fokus pada perbaikan sistem, bukan menyalahkan individu.

## 8. Latihan dan kesiapan

- Sediakan kontak eskalasi dan kewenangan tindakan darurat.
- Latih skenario yang relevan, seperti kebocoran secret, akses Admin tidak sah, kerusakan database, dan gangguan penyedia.
- Tinjau prosedur setelah insiden atau perubahan arsitektur besar.
- Catat hasil latihan, kesenjangan, dan tindak lanjut.

Frekuensi latihan dan target waktu respons ditetapkan setelah kapasitas operasional dievaluasi.

## 9. Pengujian minimum

- Setiap insiden dapat diberi ID, owner, severity, dan kronologi.
- Jalur eskalasi untuk SEV-1/SEV-2 jelas.
- Tindakan revoke credential/session dapat dilakukan oleh pihak berwenang.
- Bukti dan log terlindungi dari akses tidak sah.
- Pemulihan memverifikasi integritas data dan konfigurasi.
- Pemberitahuan tidak mengungkap data sensitif atau informasi investigasi.
- Tindak lanjut post-incident dapat ditelusuri hingga selesai.

## Referensi

- NIST Cybersecurity Framework 2.0.
- NIST SP 800-61 Rev. 3, Incident Response Recommendations and Considerations for Cybersecurity Risk Management.
- CISA Incident Response Playbooks.
- AWS Well-Architected Framework — Security Pillar.
- Microsoft Cloud Security Benchmark.
