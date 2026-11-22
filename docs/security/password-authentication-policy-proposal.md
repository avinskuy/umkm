# Usulan Kebijakan Password Pengguna

- **Status:** Usulan untuk persetujuan produk dan keamanan; belum menjadi konfigurasi aktif.
- **Cakupan:** Password akun Business Owner dan Investor.
- **Tidak termasuk:** MFA Admin, token reset, OTP, atau sesi.
- **Acuan:** NIST SP 800-63B-4 (Juli 2025) dan OWASP Authentication Cheat Sheet.

## 1. Konteks autentikasi

Login rutin USahapintar menggunakan email atau nomor telepon + password tanpa OTP setiap kali login. Karena password menjadi satu-satunya faktor login rutin pengguna klien, usulan ini memakai baseline password single-factor dari NIST SP 800-63B-4.

OTP registrasi hanya membuktikan penguasaan kontak awal pada saat pendaftaran. OTP tersebut tidak menjadikan login rutin sebagai autentikasi multifaktor.

## 2. Usulan aturan password

| Kontrol | Usulan |
|---|---|
| Panjang minimum | 15 karakter |
| Panjang maksimum | 128 karakter |
| Penghitungan | Unicode code point setelah normalisasi NFC |
| Karakter | Izinkan spasi dan karakter yang dapat dicetak; dukung Unicode |
| Komposisi | Tidak mewajibkan huruf besar, huruf kecil, angka, atau simbol tertentu |
| Blocklist | Tolak password umum, mudah ditebak, kontekstual, atau diketahui pernah bocor |
| Pemeriksaan blocklist | Bandingkan keseluruhan password, bukan potongan/substrat |
| Rotasi berkala | Tidak diwajibkan |
| Perubahan wajib | Hanya jika ada bukti kompromi atau tindakan keamanan yang sah |
| Pemotongan input | Dilarang; password harus diverifikasi secara utuh |

Batas maksimum 128 karakter merupakan pilihan produk untuk menerima passphrase panjang sekaligus memberi batas input yang jelas. Nilai ini memenuhi rekomendasi NIST agar verifier mengizinkan setidaknya 64 karakter.

## 3. Pengalaman pengguna

- Izinkan paste dan autofill dari password manager.
- Tampilkan aturan panjang secara jelas sebelum pengguna mengirim formulir.
- Berikan pesan yang dapat ditindaklanjuti jika password masuk blocklist.
- Jangan memaksa aturan komposisi karakter yang tidak disetujui.
- Password confirmation hanya untuk pemeriksaan formulir; jangan disimpan.
- Jangan menyediakan password hint atau pertanyaan keamanan berbasis pengetahuan pribadi.
- Jangan pernah menampilkan password atau password hash kepada Admin, log, telemetry, atau respons API.

Password strength meter boleh dipakai sebagai bantuan UX, tetapi tidak menggantikan validasi server dan blocklist.

## 4. Penyimpanan dan pemrosesan

- Gunakan Argon2id dengan salt unik.
- Hashing dilakukan di backend; password plaintext tidak boleh masuk database, log, audit metadata, atau pesan OTP.
- Jalankan hashing pada blocking worker agar tidak menghambat async runtime.
- Tetapkan parameter Argon2id berdasarkan benchmark pada lingkungan deployment dan tinjau ulang saat kapasitas berubah.
- Jangan mengirim password atau hash password ke layanan pihak ketiga untuk pemeriksaan kebocoran.
- Jika memakai sumber password bocor eksternal, prioritaskan blocklist lokal atau mekanisme yang tidak mengungkap password; lakukan tinjauan privasi dan keamanan terlebih dahulu.
- Terapkan rate limit dan pemantauan pada login serta pemulihan akun. Rate limit harus ditegakkan server-side dan tahan terhadap request bersamaan.

Implementasi saat ini telah memiliki fungsi Argon2id dengan parameter awal 19 MiB memory, 2 iterasi, dan parallelism 1. Parameter tersebut belum dianggap final sebelum benchmark dan tinjauan kapasitas.

## 5. Batas implementasi saat ini

- `RegistrationPasswordPolicy` pada HTTP router masih berupa dependency yang diinjeksi.
- Belum ada implementasi validator password produksi atau blocklist.
- Belum ada keputusan final mengenai sumber blocklist dan jadwal pembaruannya.
- Router HTTP registrasi belum dipasang pada aplikasi utama.
- Dokumen ini tidak mengaktifkan kebijakan password dan tidak mengubah kode runtime.

## 6. Keputusan yang diminta

Mohon persetujuan terhadap baseline berikut sebelum validator dibuat:

1. Minimum 15 dan maksimum 128 Unicode code point.
2. Normalisasi NFC sebelum hashing dan verifikasi.
3. Tidak ada aturan komposisi karakter.
4. Blocklist untuk password umum, mudah ditebak, kontekstual, dan diketahui bocor.
5. Tidak ada rotasi berkala; perubahan wajib hanya saat kompromi.
6. Paste/autofill dan password manager didukung.

Jika disetujui, keputusan ini akan dipindahkan ke kebijakan autentikasi aktif dan diterapkan pada validator server serta pengujian.

## 7. Referensi

- NIST, [SP 800-63B-4 — Authentication and Authenticator Management](https://pages.nist.gov/800-63-4/sp800-63b.html), khususnya bagian Password Authenticators dan Password Verifiers.
- NIST, [SP 800-63-4 Implementation Resources — FAQ](https://pages.nist.gov/800-63-4-Implementation-Resources/faqs/).
- OWASP, [Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html).

Dokumen ini merupakan usulan engineering, bukan pernyataan sertifikasi atau kepatuhan formal terhadap NIST/OWASP.
