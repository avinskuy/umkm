# Standar Observabilitas & Pemantauan

- **Status:** Disetujui sebagai baseline.
- **Cakupan:** Frontend, API, backend, database, dependency, infrastruktur, dan alur operasional.

## 1. Tujuan dan prinsip

Observability membantu tim memahami kondisi internal sistem melalui telemetry yang relevan. Monitoring harus mendeteksi masalah yang berdampak pada pengguna, membantu diagnosis, dan mendukung keputusan operasional.

Gunakan tiga sinyal utama:

- **Logs:** catatan peristiwa diskret.
- **Metrics:** pengukuran numerik dari waktu ke waktu.
- **Traces:** perjalanan request lintas komponen jika tracing terdistribusi digunakan.

Terapkan prinsip:

- Ukur pengalaman dan dampak pengguna, bukan hanya status proses.
- Mulai dari kebutuhan diagnosis yang nyata; hindari mengumpulkan telemetry tanpa tujuan.
- Minimalkan data dan batasi akses.
- Gunakan threshold dan alert yang dapat ditindaklanjuti.
- Pisahkan telemetry test dan production.
- Jangan menganggap dashboard atau alert sebagai pengganti audit trail.

## 2. Cakupan pengamatan

### Layanan aplikasi

Pantau ketersediaan endpoint, request rate, latency, error rate, status health check, kegagalan background task bila ada, dan hasil integrasi penting.

### Database

Pantau konektivitas, latency query penting, connection pool, transaksi gagal, deadlock, kapasitas storage, penggunaan resource, backup, dan hasil restore test.

### Infrastruktur

Pantau CPU, memory, disk, network, kapasitas, restart proses, status service, TLS/certificate expiry, dan kegagalan platform hosting.

### Keamanan

Hubungkan monitoring dengan peristiwa seperti lonjakan login gagal, pola akses ditolak, aktivitas privilege, perubahan konfigurasi sensitif, penggunaan secret yang tidak wajar, dan indikasi akses lintas akun.

### Pengalaman pengguna

Jika pengukuran tersedia dan privasi dapat dijaga, pantau error frontend, kegagalan alur penting, waktu respons yang dirasakan, dan hambatan aksesibilitas/usability. Jangan merekam isi formulir, dokumen, percakapan, atau data sensitif untuk tujuan analytics umum.

## 3. SLI, SLO, dan error budget

Gunakan konsep berikut ketika layanan telah memiliki baseline pengukuran:

- **SLI:** indikator kuantitatif kualitas layanan, misalnya availability, latency, atau error rate.
- **SLO:** target internal untuk SLI dalam periode evaluasi yang ditentukan.
- **SLA:** komitmen layanan kepada pihak eksternal jika memang disepakati.

SLO dan SLA numerik belum ditetapkan. Tentukan setelah baseline trafik, kebutuhan pengguna, biaya, dan kemampuan infrastruktur diukur. Jangan menyebut target internal sebagai jaminan eksternal.

Error budget dapat digunakan untuk menyeimbangkan pengembangan fitur dan pekerjaan reliabilitas setelah SLO disetujui.

## 4. Health check dan kesiapan layanan

Bedakan:

- **Liveness:** proses masih berjalan dan tidak macet.
- **Readiness:** layanan siap menerima traffic sesuai dependency yang diperlukan.
- **Dependency health:** status database atau layanan eksternal.

Health check harus ringan, tidak membocorkan konfigurasi/secret, dan tidak menghasilkan beban berlebihan. Status dependency yang tidak wajib tidak boleh selalu menjatuhkan seluruh layanan jika degradasi terbatas dapat ditangani dengan aman.

## 5. Logging, privasi, dan korelasi

Ikuti [Logging, Monitoring & Audit Trail Policy](../security/logging-monitoring-audit.md):

- Gunakan timestamp konsisten dan correlation/request ID yang tidak mengandung data pribadi.
- Catat status dan konteks minimum yang diperlukan untuk diagnosis.
- Jangan log password, OTP, token, secret, isi private chat, dokumen rahasia, atau bukti recovery mentah.
- Masking atau hilangkan identifier sensitif.
- Batasi akses, retensi, ekspor, dan penghapusan telemetry.
- Pisahkan audit event yang harus memiliki integritas dari log diagnostik biasa.

## 6. Alert dan eskalasi

Setiap alert penting harus memiliki:

- Kondisi pemicu dan severity.
- Layanan atau aset terdampak.
- Pemilik respons atau jalur eskalasi.
- Langkah diagnosis awal atau link ke runbook.
- Kriteria kapan alert dianggap pulih.
- Mekanisme untuk mengurangi duplikasi dan noise.

Prioritaskan alert untuk availability, error/latency yang berdampak, kapasitas kritis, backup gagal, integritas data, dan insiden keamanan. Alert tidak boleh berisi secret atau data sensitif.

## 7. Dashboard dan review

Dashboard sebaiknya menjawab pertanyaan operasional: apakah layanan tersedia, apakah pengguna terdampak, komponen mana yang gagal, dan apakah kapasitas mendekati batas.

Lakukan review terhadap pola error, tren kapasitas, false positive, alert yang tidak ditindaklanjuti, dan kebutuhan instrumentasi. Frekuensi review mengikuti tingkat kritikal layanan dan kapasitas tim.

## 8. Integrasi dengan penanganan insiden

Alert yang kredibel harus dapat ditautkan ke proses [Incident Response](incident-response.md). Monitoring tidak menggantikan validasi insiden, penetapan severity, atau keputusan komunikasi.

Setiap insiden yang berulang harus menghasilkan evaluasi apakah perlu penambahan metric, log, trace, alert, atau perbaikan runbook.

## 9. Pengujian minimum

- Health check tidak membocorkan detail internal.
- Metric dan log penting tersedia untuk komponen kritis.
- Telemetry tidak memuat secret atau data sensitif yang dilarang.
- Alert penting memiliki owner dan prosedur respons.
- Alert diuji dengan skenario aman dan tidak mengganggu pengguna nyata.
- Timestamp dan correlation ID mendukung penelusuran lintas komponen.
- Retensi serta akses telemetry dibatasi.

## 10. Tahapan penerapan

Mulai dari health check, log terstruktur, metric dasar API/database/infrastruktur, alert availability/error/backup, dan dashboard operasional sederhana. Tambahkan tracing, SLO/error budget, synthetic monitoring, dan observability lanjutan ketika kompleksitas serta kebutuhan membenarkannya.

Vendor, stack telemetry, threshold, retensi, dan jadwal review ditetapkan saat implementasi. Dokumen ini tidak mengasumsikan penyedia tertentu.
