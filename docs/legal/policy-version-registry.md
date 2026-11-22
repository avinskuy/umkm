# Registry Versi Kebijakan Publik

Dokumen ini menjadi satu sumber rujukan untuk versi dokumen yang ditampilkan kepada pengguna dan versi yang dicatat oleh backend. Registry ini bukan pengganti dokumen kebijakan.

## Status saat ini

| Dokumen | Versi kerja | Status | Versi penerimaan aktif |
|---|---|---|---|
| Ketentuan Layanan | `draft-terms-0.1` | Draf; belum disetujui | Tidak ada |
| Kebijakan Privasi | `draft-privacy-0.1` | Draf; belum disetujui | Tidak ada |
| Pemberitahuan Marketing | `draft-marketing-0.1` | Draf; belum disetujui | Tidak ada |

**Penting:** Backend belum boleh mengaktifkan registrasi publik dengan label versi draf. Fungsi database migration 0010 menerima versi sebagai parameter dari backend; versi final harus dipilih dari konfigurasi server-side, tidak dari input browser.

## Gerbang untuk menetapkan versi aktif

Versi baru hanya boleh menjadi aktif setelah:

1. Isi dokumen selesai dan konsisten dengan fitur yang benar-benar aktif.
2. Identitas pengelola, kontak, wilayah pemrosesan, dan keputusan hukum yang relevan telah diisi.
3. Pemilik produk menyetujui isi dan tanggal berlaku.
4. Tinjauan hukum/privasi yang diperlukan selesai.
5. UI menampilkan dokumen lengkap dan versi yang sama dengan yang akan dicatat backend.
6. Backend memegang versi aktif dari konfigurasi server-side dan tidak mempercayai versi yang dikirim klien.
7. Tes memastikan dua penerimaan wajib tersimpan terpisah; consent marketing hanya tercatat jika pengguna opt-in.
8. Perubahan versi menghasilkan catatan penerimaan baru tanpa menimpa histori lama.

## Aturan penamaan versi

Gunakan label immutable yang ringkas dan unik, misalnya `terms-2026-12-01-v1` dan `privacy-2026-12-01-v1`, setelah tanggal berlaku final ditentukan. Label ini hanya contoh format, bukan versi yang sudah disetujui.

Jangan mengubah isi dokumen setelah versi aktif diterbitkan. Perubahan substantif harus menghasilkan versi baru dan mengikuti proses pemberitahuan/penerimaan ulang yang telah ditentukan.

## Jejak rujukan implementasi

- Skema dan kontrak persistence: [Migration 0010 — Profil Publik dan Penerimaan Kebijakan](../architecture/identity-core-migration-0010-contract.md).
- Baseline consent: [Kebijakan Persetujuan & Pemrosesan Data](../security/consent-data-processing.md).
- Draf pilihan marketing: [Pemberitahuan Persetujuan Marketing](marketing-consent-notice-draft.md).
- Baseline privasi: [Kebijakan Privasi & Kendali Data](../security/privacy-data-control.md).
- Keputusan registrasi: [Keputusan Autentikasi Identity Core](../architecture/identity-core-authentication-stage-one-decision.md).

## Catatan operasional

Tidak ada versi penerimaan aktif sampai seluruh gerbang di atas selesai. Jangan mengubah status registry menjadi aktif hanya karena file draf sudah berada di repository.
