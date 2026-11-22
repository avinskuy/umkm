# Kontrak Migration Identity Core 0010 — Profil Publik dan Penerimaan Kebijakan

- **Migration:** `20261122190000_add_public_profile_and_policy_acceptance.sql`
- **Status:** tersedia dan diuji melalui PostgreSQL CI.
- **Cakupan:** nama tampilan publik, bukti penerimaan kebijakan wajib, dan consent marketing opsional.
- **Tidak termasuk:** endpoint HTTP, tampilan publik profil, pengubahan nama melalui API, penarikan consent melalui API, atau penerapan migration ke produksi.

## 1. Keputusan profil

- Nama yang diisi saat registrasi disimpan pada `user_profiles.display_name`.
- Field ini adalah nama tampilan publik, bukan nama legal atau bukti identitas.
- Satu akun memiliki paling banyak satu baris profil dasar.
- Nilai wajib berisi 1–120 karakter setelah trim.
- Profil dibuat bersama akun pending dalam transaksi registrasi.
- Kebijakan pembacaan profil tetap harus memeriksa status akun dan aturan otorisasi; tidak ada akses SELECT langsung untuk principal aplikasi.

## 2. Penerimaan kebijakan wajib

Setiap registrasi menyimpan dua baris terpisah pada `policy_acceptance_records`:

- `terms_of_service`
- `privacy_policy`

Setiap catatan menyimpan versi kebijakan dan timestamp server. Versi disediakan oleh konfigurasi backend, bukan dipercaya dari input klien. Perubahan versi menghasilkan catatan penerimaan baru; riwayat lama tidak ditimpa.

Repository saat migration ini dibuat belum memiliki dokumen final Terms of Service yang dipublikasikan. Karena itu, nilai versi aktual dan isi dokumen harus ditetapkan sebelum endpoint registrasi publik diaktifkan. Test menggunakan label versi sintetis dan bukan menyatakan dokumen legal telah final.

## 3. Consent marketing opsional

- Consent marketing disimpan pada `consent_events` sebagai event `granted` atau `withdrawn`.
- Saat registrasi, event `granted` hanya dibuat jika pengguna secara eksplisit memilih opt-in.
- Tidak adanya event grant berarti marketing tidak disetujui.
- Versi pemberitahuan marketing dan timestamp server dicatat.
- Riwayat bersifat append-only. Penarikan di masa depan harus menambahkan event baru, bukan mengubah event grant lama.
- Consent marketing tidak menjadi syarat layanan inti.

## 4. Fungsi dan privilege

`create_client_user_with_profile_and_policy` adalah fungsi `SECURITY DEFINER` dengan `search_path = pg_catalog`, dimiliki principal migrator non-superuser. Fungsi tersebut:

1. Memvalidasi nama tampilan, versi Terms/Privacy, dan kombinasi input consent.
2. Memanggil fungsi internal pembuatan akun pending.
3. Membuat profil publik.
4. Mencatat penerimaan Terms dan Privacy secara terpisah.
5. Mencatat event grant marketing hanya jika opt-in.
6. Menyelesaikan seluruh operasi sebagai satu transaksi database.

Principal aplikasi hanya memperoleh EXECUTE pada fungsi baru tersebut. Hak EXECUTE pada fungsi pembuatan akun internal lama dicabut saat bootstrap grants dijalankan ulang. Tidak ada hak baca/tulis langsung ke tabel profil, penerimaan kebijakan, atau consent.

## 5. Pengujian wajib

- Nama tampilan kosong atau lebih dari batas ditolak.
- Versi Terms/Privacy wajib tersedia dan tervalidasi.
- Opt-in marketing memerlukan versi notice; opt-out tidak membuat event grant.
- Profil dan dua penerimaan wajib dibuat atomik dengan akun.
- Kegagalan satu insert membatalkan seluruh transaksi registrasi.
- Riwayat penerimaan dan consent menolak UPDATE, DELETE, dan TRUNCATE.
- Principal aplikasi hanya dapat menjalankan fungsi registrasi terkontrol, bukan membaca atau memodifikasi tabel secara langsung.
- Alur email dan WhatsApp melalui `VerificationService` menyimpan nama dan consent yang sama dengan input registrasi.

## 6. Batas operasional

Migration ini tidak dijalankan otomatis oleh server dan tidak diterapkan ke database produksi. Bootstrap privilege harus dijalankan sebagai DBA setelah migration 0001–0010 diterapkan pada lingkungan yang disiapkan. Endpoint HTTP tidak boleh diaktifkan sebelum dokumen Terms of Service dan Privacy Policy yang berlaku, versi server-side, serta UI persetujuannya siap.
