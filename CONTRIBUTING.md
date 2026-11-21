# Panduan Kontribusi USahapintar

## Kesepakatan kerja

1. Mulai dari branch `main` yang sudah diperbarui dan gunakan branch khusus untuk setiap pekerjaan.
2. Batasi setiap perubahan pada satu fitur atau satu kebutuhan rekayasa perangkat lunak.
3. Baca dokumentasi arsitektur, standar engineering, dan [Kesiapan Operasional](docs/operations/README.md) yang relevan sebelum mengubah batas tanggung jawab modul atau proses operasional.
4. Tambahkan atau perbarui pengujian setiap kali perilaku sistem berubah.
5. Jalankan pemeriksaan format, lint, tipe, dan pengujian sebelum membuka pull request.
6. Jelaskan pertimbangan teknis serta keterbatasan yang masih diketahui di pull request.
7. Perbarui dokumentasi dan runbook jika perubahan memengaruhi prosedur operasi, pemulihan, monitoring, atau release.
8. Jangan gabungkan perubahan yang gagal dalam pemeriksaan atau belum ditinjau.

## Penamaan branch

- `feat/<scope>` — fitur produk.
- `fix/<scope>` — perbaikan bug.
- `chore/<scope>` — tooling dan pemeliharaan.
- `docs/<scope>` — perubahan dokumentasi saja.
- `test/<scope>` — infrastruktur atau cakupan pengujian.

## Pesan commit

Gunakan format Conventional Commits. Contoh:

- `feat(auth): add owner login flow`
- `fix(api): reject mismatched account roles`
- `chore(ci): add Rust quality checks`
- `docs(architecture): document module boundaries`

Format pesan commit dan identifier teknis tetap mengikuti konvensi aslinya.

## Kriteria Selesai (Definition of Done)

- Cakupan pekerjaan dan kriteria penerimaan sudah terpenuhi.
- Tidak ada perubahan format yang tidak terkait atau file hasil generate yang tidak diperlukan.
- Frontend: pemeriksaan format, lint, typecheck, unit test, dan build berhasil jika relevan.
- Backend: rustfmt, Clippy, unit/integration test, dan build berhasil jika relevan.
- Otorisasi dan validasi input untuk perilaku sensitif terhadap keamanan sudah diuji.
- Dokumentasi, contoh konfigurasi lingkungan, dan runbook yang terdampak sudah diperbarui.
- Tidak ada kredensial, data pribadi, atau data produksi yang ikut di-commit.
- CI berstatus hijau dan seluruh diff sudah ditinjau.
- Perubahan production memiliki persetujuan deployment terpisah dan catatan release yang dapat ditelusuri.

Jangan pernah melakukan commit terhadap rahasia. Gunakan file konfigurasi lingkungan lokal dan pastikan file tersebut tidak masuk version control.
