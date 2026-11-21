# Manajemen Rilis & Pengendalian Perubahan

- **Status:** Disetujui sebagai baseline.
- **Cakupan:** Perubahan kode, konfigurasi, schema, dependency, workflow, artifact, dan deployment.

## 1. Tujuan dan prinsip

Manajemen release memastikan perubahan dapat ditinjau, diuji, ditelusuri, disetujui, dan dipulihkan jika terjadi masalah.

Terapkan prinsip:

- Perubahan kecil dan memiliki tujuan jelas.
- Review dan pengujian berdasarkan risiko.
- Build menggunakan lockfile dan sumber yang dapat ditelusuri.
- Pisahkan persetujuan merge dari persetujuan deployment production.
- Tidak ada deployment production tanpa persetujuan eksplisit pihak berwenang.
- Catat pengecualian, perubahan darurat, dan hasil pascadeployment.

## 2. Kategori perubahan

Klasifikasikan perubahan sebagai:

- **Standard:** risiko rendah, pola berulang, prosedur dan tes telah diketahui.
- **Normal:** membutuhkan review, penilaian dampak, dan persetujuan sesuai kewenangan.
- **Emergency:** diperlukan untuk mengatasi insiden atau risiko mendesak; tetap harus dicatat, dibatasi, dan ditinjau setelah kondisi terkendali.

Perubahan database, autentikasi, otorisasi, privasi, secret, infrastruktur, CI/CD, dan dependency berisiko tinggi memerlukan perhatian khusus.

## 3. Pengajuan dan penilaian perubahan

Setiap perubahan harus menjelaskan:

- Tujuan dan ruang lingkup.
- Komponen, API, data, dan pengguna yang terdampak.
- Risiko keamanan, privasi, kompatibilitas, dan ketersediaan.
- Dependency dan kebutuhan konfigurasi.
- Migration atau perubahan data jika ada.
- Rencana pengujian dan bukti hasilnya.
- Rencana release, health check, dan rollback/pemulihan.
- Dokumentasi yang perlu diperbarui.

Perubahan yang tidak terkait sebaiknya dipisahkan agar review dan rollback lebih mudah.

## 4. Tinjauan dan gerbang kualitas

Sebelum merge atau release sesuai tahapnya:

- Review diff, desain, otorisasi, validasi, dan data lifecycle.
- Jalankan format, lint, typecheck, tests, build, serta scan yang relevan.
- Periksa dependency dan lockfile.
- Uji migration pada database uji dan pertimbangkan kompatibilitas versi.
- Pastikan tidak ada secret, data production, atau artefak lokal yang tidak semestinya.
- Pastikan dokumentasi, konfigurasi contoh, dan runbook diperbarui.
- Semua CI wajib lulus sebelum merge.
- Temuan Critical harus diselesaikan; temuan High menahan release kecuali ada keputusan risiko eksplisit dan terdokumentasi oleh pihak berwenang.

Pengecualian harus menyebut alasan, dampak, mitigasi, pemilik risiko, dan masa berlaku. Jangan mengabaikan pemeriksaan hanya untuk mempercepat release.

## 5. Persetujuan dan pemisahan kewenangan

- Persetujuan pull request/merge tidak otomatis berarti deployment disetujui.
- Deployment production memerlukan persetujuan eksplisit terpisah.
- Perubahan berisiko tinggi sebaiknya ditinjau oleh pihak selain pembuat jika kapasitas memungkinkan.
- Batasi akses deployment dan secret sesuai least privilege.
- Gunakan approval dan environment protection pada platform CI/CD jika tersedia.
- Catat aktor, waktu, commit, artifact, environment, dan hasil release.

## 6. Build dan artefak

- Build dari commit yang telah ditinjau dan diuji.
- Catat commit SHA, versi, dependency/lockfile, hasil CI, dan artifact.
- Jangan memasukkan secret atau data production ke artifact.
- Verifikasi artifact yang akan dipromosikan.
- Gunakan artifact yang sama antar-environment bila proses build mendukungnya.
- SBOM, provenance, dan signing diterapkan sesuai kesiapan supply chain dan kebijakan dependency.

## 7. Deployment dan verifikasi

Sebelum deployment:

- Pastikan persetujuan production tersedia.
- Pastikan backup dan prosedur pemulihan relevan siap, terutama untuk perubahan database.
- Periksa kompatibilitas konfigurasi dan dependency.
- Siapkan runbook serta kontak eskalasi.
- Tentukan indikator keberhasilan dan kondisi rollback.

Sesudah deployment:

- Verifikasi health check dan alur penting.
- Pantau error, latency, resource, dan sinyal keamanan.
- Pastikan migration dan data integrity sesuai harapan.
- Catat hasil dan setiap tindakan korektif.
- Jika terjadi dampak tidak dapat diterima, jalankan rollback atau pemulihan yang telah dinilai aman.

## 8. Rollback dan perubahan database

Rollback aplikasi tidak selalu berarti rollback database. Migration harus dirancang dengan mempertimbangkan kompatibilitas, backup, dan risiko kehilangan data.

- Utamakan migration bertahap dan kompatibel bila memungkinkan.
- Jangan mengedit migration yang sudah diterapkan.
- Migration destruktif memerlukan rencana transisi dan pemulihan yang eksplisit.
- Jangan menjalankan rollback otomatis yang berpotensi menghapus data tanpa validasi.
- Verifikasi integritas setelah rollback atau forward-fix.
- Catat kondisi ketika rollback tidak aman dan alternatif pemulihan yang dipilih.

## 9. Perubahan darurat

Perubahan darurat harus memiliki alasan, pemilik, cakupan minimum, bukti pengujian yang memungkinkan, dan catatan tindakan. Setelah risiko terkendali, lakukan review retrospektif, lengkapi dokumentasi, dan tambahkan regression test.

## 10. Catatan rilis

Catat sekurang-kurangnya:

- Versi/identitas release dan commit SHA.
- Ringkasan perubahan.
- Hasil CI dan review.
- Environment dan waktu release.
- Pemberi persetujuan dan pelaksana.
- Migration/configuration yang diterapkan.
- Hasil health check dan monitoring.
- Insiden, rollback, atau tindak lanjut.

## 11. Pengujian minimum

- Release dapat ditelusuri ke commit dan hasil CI.
- Deployment production diblokir tanpa approval yang sesuai.
- Perubahan berisiko tinggi memiliki review dan test relevan.
- Migration dan pemulihan diuji pada environment aman.
- Rollback tidak menyebabkan kehilangan data yang tidak direncanakan.
- Perubahan darurat tetap memiliki catatan dan review pascakejadian.

## Referensi

- NIST SSDF.
- OWASP CI/CD Security Cheat Sheet.
- Google SRE — Release Engineering.
- DORA — Software Delivery Performance dan praktik perubahan yang aman.
