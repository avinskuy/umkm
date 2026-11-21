# Kebijakan Kinerja, Skalabilitas & Keandalan

- **Status:** Disetujui sebagai baseline
- **Tanggal:** 21 November 2026
- **Cakupan:** Frontend, API, backend, database, dependency, dan layanan production

## 1. Tujuan dan referensi

USahapintar harus menjaga pengalaman pengguna, integritas data, dan ketersediaan layanan dengan kompleksitas serta biaya yang proporsional.

Acuan:

- Google SRE Workbook dan praktik Site Reliability Engineering.
- AWS Well-Architected Framework, khususnya Reliability dan Performance Efficiency.
- Google Cloud Well-Architected Framework, khususnya Reliability dan Performance Optimization.

## 2. Pengukuran dan indikator

- Tentukan Service Level Indicators (SLI) dan Service Level Objectives (SLO) untuk alur dan layanan penting sebelum membuat klaim ketersediaan.
- Pantau latency distribution dan percentile, traffic/throughput, error rate, saturation, resource utilization, dan hasil yang terlihat oleh pengguna.
- Ukur pengalaman sisi client dan backend jika relevan.
- Buat workload baseline serta catat kondisi pengukuran, environment, dan versi aplikasi.
- Jangan menyatakan angka uptime, response time, atau kapasitas yang belum dibuktikan dengan pengukuran.

Target numerik ditetapkan setelah kebutuhan produk, risiko, dan kapasitas aktual dipahami.

## 3. Prinsip performa

- Optimalkan berdasarkan pengukuran, bukan asumsi.
- Gunakan query database yang efisien, index berdasarkan pola akses nyata, pagination, dan payload yang dibatasi.
- Gunakan connection pooling dengan batas yang sesuai kapasitas database.
- Hindari operasi blocking atau pekerjaan mahal pada jalur request tanpa alasan.
- Terapkan caching hanya jika aman terhadap privasi, otorisasi, freshness, dan konsistensi data.
- Pantau penggunaan CPU, memory, storage, koneksi database, dan bandwidth.
- Hindari optimasi prematur dan dependency yang tidak memberi manfaat terukur.

## 4. Skalabilitas

- Pertahankan modular monolith sebagai baseline sampai ada bukti kebutuhan untuk pemisahan layanan.
- Evaluasi vertical dan horizontal scaling berdasarkan bottleneck yang terukur.
- Pastikan batas resource, concurrency, dan connection pool tidak membuat satu komponen menghabiskan kapasitas seluruh sistem.
- Rancang operasi penting agar aman saat request diproses bersamaan.
- Pertimbangkan kapasitas dependency eksternal dan batas layanan penyedia.
- Evaluasi biaya sebelum menambah instance, storage, cache, queue, atau layanan terkelola.

Autoscaling, multi-region, dan microservices bukan kewajiban awal. Pilih hanya jika dampak, kompleksitas, dan biaya dapat dibenarkan.

## 5. Reliability dan ketahanan terhadap kegagalan

- Gunakan timeout pada komunikasi internal dan eksternal.
- Retry harus dibatasi dan menggunakan backoff/jitter untuk mencegah retry storm.
- Terapkan idempotency pada operasi yang aman dan membutuhkan perlindungan terhadap pengiriman ulang.
- Batasi concurrency, ukuran request, waktu eksekusi, dan penggunaan resource.
- Sediakan health check yang merefleksikan kondisi layanan secara tepat.
- Isolasi kegagalan dependency agar tidak menjalar ke seluruh aplikasi.
- Jaga transaksi dan integritas data saat terjadi timeout atau kegagalan parsial.
- Sediakan mekanisme graceful shutdown dan pemulihan sesuai runtime.

## 6. Graceful degradation

Ketika sebagian layanan terganggu:

- Fitur yang tidak bergantung pada layanan gagal tetap dapat digunakan jika aman.
- Beri status yang jelas kepada pengguna.
- Jangan menyajikan data lama seolah-olah data terbaru.
- Jangan mengonfirmasi transaksi sebelum hasilnya benar-benar diketahui.
- Lindungi data privat dan aturan otorisasi dalam mode degradasi.
- Jangan melakukan retry tanpa batas atau membuat duplikasi operasi.

## 7. Pengujian dan pemantauan

- Lakukan load, stress, dan soak testing pada environment non-production yang aman.
- Uji timeout, dependency unavailable, koneksi database terputus, dan kegagalan parsial.
- Pantau tren latency, error, saturation, dan kapasitas.
- Buat alert untuk gejala yang dapat ditindaklanjuti serta tetapkan pemilik respons.
- Tinjau insiden dan temuan performa untuk menentukan perbaikan.

## 8. Kapasitas dan pengelolaan biaya

Perkirakan kebutuhan CPU, RAM, database, storage, bandwidth, dan dependency berdasarkan workload yang terukur. Catat asumsi, batas, dan biaya. Rencana kapasitas harus diperbarui saat pola penggunaan atau arsitektur berubah.

## 9. Keputusan yang ditunda

Target SLO/SLA, batas latency, kapasitas pengguna, autoscaling policy, multi-region, dan arsitektur distribusi final ditentukan setelah pengukuran, kebutuhan kompetisi/produk, dan anggaran tersedia. Jangan menciptakan target numerik tanpa dasar.

## Referensi

- Google SRE Workbook.
- AWS Well-Architected Framework.
- Google Cloud Well-Architected Framework.
