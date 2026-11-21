# Standar Pondasi Kriptografi Identity Core

> **Catatan nama produk:** Nama produk belum final. Dokumen ini menggunakan istilah netral dan tidak mengunci nama merek.

- Status: keputusan teknis tahap Cryptographic Foundation
- Cakupan: password hashing, AEAD, HMAC, CSPRNG, key separation, key versioning
- Tidak mencakup: provider KMS produksi, endpoint publik, pengiriman OTP nyata, atau pengelolaan credential produksi

## 1. Prinsip adopsi praktik industri

Desain mengadopsi pola publik yang dapat diverifikasi dari ekosistem Google dan panduan keamanan akun Meta, lalu menerapkannya menggunakan standar terbuka dan library Rust yang sesuai.

Yang diadopsi:
- API kriptografi tingkat tinggi dan primitive standar, bukan algoritma buatan sendiri.
- Keyset/versioned keys: satu versi aktif untuk operasi baru, versi lama tetap tersedia untuk dekripsi/verifikasi sampai migrasi data selesai.
- Pemisahan key berdasarkan tujuan.
- Envelope encryption sebagai arah integrasi KMS mendatang.
- Least privilege, rotasi terencana, audit akses key, dan fail-closed saat key tidak tersedia.
- Penggunaan password hashing adaptif dan perlindungan autentikasi berlapis.

Tidak ada klaim bahwa algoritma atau konfigurasi internal Google/Meta disalin. Detail internal yang tidak dipublikasikan tidak dijadikan asumsi.

## 2. Keputusan primitive

| Kebutuhan | Keputusan awal | Batas |
|---|---|---|
| Password | Argon2id v=19, PHC encoded hash | 19 MiB, 2 iterasi, paralelisme 1; benchmark sebelum produksi |
| Enkripsi data yang perlu dipulihkan | AES-256-GCM melalui `ring` | Nonce 96-bit acak dari CSPRNG; key version dan AAD wajib |
| Fingerprint kontak | HMAC-SHA-256 | Key khusus fingerprint; kanal dan kontak dinormalisasi masuk domain-separated input |
| Digest OTP | HMAC-SHA-256 | Key khusus OTP; digest terikat challenge, sesi, purpose, kanal, dan kode |
| Randomness | `ring::rand::SystemRandom` | Gagal tertutup bila OS CSPRNG gagal |
| Pembersihan memori | `zeroize` | Secret key dan plaintext hasil dekripsi dibersihkan saat dilepas |

### 2.1 Password

Argon2id dipilih sebagai password hashing adaptif, bukan enkripsi. Setiap password memperoleh salt acak unik; output disimpan sebagai PHC string yang membawa algoritma, versi, parameter, salt, dan hash. Verifikasi membatasi panjang PHC string serta parameter untuk menghindari hash tersimpan yang dimanipulasi meminta resource berlebihan.

Parameter awal mengikuti baseline minimum OWASP Password Storage Cheat Sheet. Sebelum dipakai untuk autentikasi publik, lakukan benchmark dengan concurrency realistis pada target runtime dan batasi concurrency hashing agar tidak membuka jalur resource exhaustion. Tingkatkan parameter bila kapasitas memungkinkan; jangan menurunkannya di bawah baseline.

### 2.2 Enkripsi terautentikasi

AES-256-GCM menyediakan confidentiality dan integrity sekaligus. Associated data harus mengikat ciphertext ke konteks pemakaian, misalnya session ID, channel, purpose, dan format version. Mengubah AAD atau ciphertext harus menyebabkan dekripsi gagal.

Nonce 96-bit dibuat menggunakan OS CSPRNG dan disimpan bersama ciphertext. GCM memiliki batas penggunaan per key; sebelum produksi, key provider wajib menerapkan kuota penggunaan dan rotasi versi berdasarkan batas standar/volume. Jangan memakai satu key tanpa batas pada banyak instance. Modul tahap ini belum menyediakan provider produksi atau penghitung penggunaan terdistribusi, sehingga belum boleh dipakai untuk data produksi.

### 2.3 HMAC dan OTP

Fingerprint kontak bukan hash biasa: HMAC-SHA-256 memakai key rahasia khusus sehingga kebocoran database tidak langsung memungkinkan enumerasi kontak secara offline. Input menggunakan domain separation dan field length-prefix agar encoding tidak ambigu.

OTP tidak disimpan plaintext. Digest HMAC terikat pada challenge ID, session ID, purpose, kanal, dan kode. Verifikasi memakai fungsi constant-time dari library. Key fingerprint dan key OTP tidak boleh sama dengan key enkripsi atau satu sama lain.

Generator kode OTP dan jumlah digit belum diputuskan. `random_bytes` hanya menyediakan CSPRNG; kebijakan format dan rejection sampling harus ditetapkan saat OTP lifecycle dibangun.

## 3. Key management

`KeyProvider` adalah boundary yang harus dipenuhi implementasi runtime:

- `active_key(purpose)` mengambil versi aktif untuk enkripsi/MAC baru.
- `key(purpose, version)` mengambil versi lama yang masih diizinkan untuk dekripsi/verifikasi.
- Purpose terpisah: data encryption, contact fingerprint, OTP verification.
- Key material dibungkus tipe yang tidak dapat dicetak sebagai Debug dan dibersihkan dengan zeroize.
- Key tidak disimpan di repository, database aplikasi, image container, frontend, atau log.
- Tidak ada default key, hardcoded key, fallback key, atau auto-generate key production.
- Provider yang gagal, key tidak dikenal, atau key version yang hilang harus menyebabkan operasi gagal tertutup.

**Batas interface:** `KeyProvider` pada tahap ini hanya mengembalikan raw key material yang memang boleh berada sementara di memori proses. Interface sinkron tersebut bukan adapter universal untuk remote KMS atau key non-exportable. Integrasi remote harus memisahkan I/O async dari primitive kriptografi dan mengikuti [Desain Integrasi Key Management Identity Core](key-management-runtime-design.md). Format `EncryptedPayload` saat ini juga belum memuat wrapped DEK sehingga belum mewakili envelope encryption.

Tahap berikutnya dapat menerapkan envelope encryption: DEK untuk data dienkripsi di aplikasi, sedangkan KEK berada di KMS dan tidak meninggalkan boundary KMS. Secret manager dapat digunakan untuk distribusi secret/key material lain sesuai model kepercayaan, tetapi tidak otomatis menggantikan fungsi KMS. Pemilihan vendor belum dilakukan. Jangan membuat asumsi Google Cloud KMS hanya karena Google menjadi salah satu referensi desain.

Rotasi mengikuti pola keyset:
1. Tambahkan key version baru sebagai non-primary.
2. Sebarkan keyset yang berisi key baru dan versi lama ke seluruh instance.
3. Jadikan versi baru primary untuk operasi baru.
4. Pertahankan versi lama untuk dekripsi/verifikasi data lama.
5. Re-encrypt/migrasikan data sesuai kebutuhan dan bukti bahwa versi lama tidak lagi dipakai.
6. Nonaktifkan/hapus key lama hanya setelah backup, restore, dan seluruh data aktif tidak lagi bergantung padanya.

Rotasi tidak otomatis mengubah ciphertext lama. Penghapusan key yang masih diperlukan dapat menyebabkan kehilangan data permanen.

## 4. Dependency dan kompatibilitas

- `argon2 0.5.3`: RustCrypto Argon2id, PHC integration; kompatibel dengan MSRV repository saat ini.
- `ring 0.17.14`: AEAD AES-256-GCM, HMAC-SHA-256, dan OS-backed secure randomness; sudah ada di dependency graph dan ditambahkan sebagai dependency langsung.
- `zeroize 1.9.2`: pembersihan secret material; sudah ada di dependency graph dan ditambahkan sebagai dependency langsung.

Versi dikunci di `Cargo.lock`. Pembaruan dependency harus melewati audit advisories, license review, changelog review, CI, dan lockfile diff. Tidak menambahkan dependency kriptografi yang tidak dipakai.

## 5. Pengujian dan batas pemakaian

Tes unit mencakup:
- Password hash salted, PHC format, verifikasi benar/salah, dan penolakan parameter berlebihan.
- AEAD round-trip dan kegagalan autentikasi saat AAD/ciphertext berubah.
- Fingerprint domain separation.
- OTP digest binding terhadap challenge dan sesi.
- Error generik yang tidak mengungkap key, plaintext, OTP, atau nilai rahasia.

Belum tersedia:
- Provider KMS/secret manager production.
- Key usage quota terdistribusi dan rotasi otomatis.
- OTP code generator/format final.
- Persistence/serialization format encrypted payload.
- Password hashing concurrency control dan benchmark pada target runtime.
- Integrasi database, provider OTP, HTTP API, dan startup validation.

Karena batas tersebut, modul ini adalah pondasi library internal yang diuji di CI, bukan izin untuk mengaktifkan registrasi atau menyimpan data produksi. Jangan menaruh key produksi pada fixture, chat, atau repository.

## 6. Referensi

- OWASP Password Storage Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
- RFC 9106 — Argon2: https://www.rfc-editor.org/rfc/rfc9106
- Google Tink — Keysets and rotation: https://developers.google.com/tink/design/keysets
- Google Tink — Choosing cryptographic primitives: https://developers.google.com/tink/choose-primitive
- Google Cloud KMS — Envelope encryption: https://docs.cloud.google.com/kms/docs/envelope-encryption
- Google Cloud KMS — Key rotation: https://docs.cloud.google.com/kms/docs/key-rotation
- Android security — Keystore and hardware-backed keys: https://source.android.com/docs/security/features
- Meta account security guidance: https://dev.meta.ai/help/accounts-and-login/account-security
- RustCrypto Argon2: https://docs.rs/argon2/0.5.3/argon2/
- ring: https://docs.rs/ring/0.17.14/ring/
- zeroize: https://docs.rs/zeroize/1.9.2/zeroize/
