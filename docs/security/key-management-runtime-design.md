# Desain Integrasi Key Management Identity Core

> **Status nama produk:** Nama produk belum final. Dokumen ini memakai istilah netral dan tidak mengunci nama merek.

- Status: rancangan integrasi vendor-neutral; belum menjadi konfigurasi produksi
- Cakupan: sumber key, envelope encryption, pemisahan tujuan, versi key, rotasi, kuota penggunaan, cache, startup validation, audit, dan pemulihan
- Prasyarat: Cryptographic Foundation PR #38
- Tidak mencakup: pemilihan vendor final, credential produksi, provisioning KMS, migrasi database produksi, atau deployment

## 1. Ringkasan keputusan

Integrasi key management harus tetap dapat dipindahkan antarpenyedia dan tidak boleh mengikat aplikasi ke satu vendor sebelum infrastruktur hosting ditetapkan.

Keputusan desain tahap ini:

1. **Pisahkan KMS dari secret manager.** KMS menangani operasi kunci tingkat tinggi seperti membuat/membuka data-encryption key (DEK) atau operasi MAC. Secret manager menyimpan dan mendistribusikan secret aplikasi. Keduanya dapat disediakan layanan yang sama, tetapi batas kepercayaannya tetap berbeda.
2. **Gunakan envelope encryption untuk data yang perlu dipulihkan.** DEK dipakai oleh aplikasi untuk AEAD; key-encryption key (KEK) tetap berada di KMS dan hanya digunakan untuk membungkus/membuka DEK.
3. **Jangan menganggap semua KMS dapat mengeluarkan raw key.** Kunci non-exportable/HSM tidak dapat memenuhi interface yang meminta byte key secara langsung. Adapter harus mengikuti kemampuan layanan dan tidak menurunkan perlindungan dengan mengekspor kunci tanpa alasan.
4. **Pisahkan operasi jaringan dari primitive kriptografi.** Pemanggilan KMS/secret manager bersifat I/O dan dapat gagal/timeout; primitive AEAD/HMAC lokal tetap sederhana dan tidak melakukan network request tersembunyi.
5. **Pertahankan key purpose dan key version.** Kunci enkripsi data, fingerprint kontak, dan verifikasi OTP tidak boleh dipakai silang.
6. **Fail closed.** Key hilang, versi tidak dikenal, autentikasi gagal, KMS tidak tersedia, atau konfigurasi ambigu tidak boleh memicu fallback ke key default atau plaintext.
7. **Belum memilih vendor.** Pilihan final dilakukan setelah hosting, wilayah data, biaya, SLA, kemampuan pemulihan, dan model operasional diketahui.

## 2. Batas kemampuan fondasi saat ini

PR #38 menyediakan primitive AES-256-GCM, HMAC-SHA-256, CSPRNG, zeroization, versi key, dan trait `KeyProvider`. Implementasi saat ini menerima `SecretKey` berupa 32 byte yang tersedia di proses aplikasi.

Batas penting:

- `KeyProvider` saat ini adalah abstraksi **resolusi key material lokal**, bukan adapter universal untuk remote KMS.
- Trait sinkron yang mengembalikan raw key tidak cocok untuk layanan KMS yang hanya menyediakan operasi kriptografi remote atau key non-exportable.
- `EncryptedPayload` saat ini menyimpan versi key, nonce, dan ciphertext; format tersebut belum memuat wrapped DEK sehingga belum menjadi format envelope encryption.
- Belum ada batas penggunaan AES-GCM terdistribusi, cache lifecycle, provider produksi, startup validation, atau prosedur pemulihan key.

Karena itu, modul PR #38 tetap dianggap primitive internal untuk pengujian dan pengembangan. Jangan menggunakannya untuk data produksi sampai desain envelope/provider, kuota, dan rotasi diterapkan serta ditinjau.

## 3. Model key hierarchy

Untuk data yang perlu didekripsi kembali:

```text
KMS / HSM boundary
└── KEK (versioned, non-exportable where supported)
    └── wraps / unwraps
        └── DEK (random, scoped, short-lived in application memory)
            └── AES-256-GCM encrypts / decrypts payload
```

Aturan:

- KEK tidak dikirim ke aplikasi dan tidak disimpan di database aplikasi.
- DEK dibuat dengan CSPRNG atau layanan GenerateDataKey yang sesuai; plaintext DEK hanya berada sementara di memori proses.
- Wrapped DEK disimpan bersama payload terenkripsi atau referensi metadata yang diperlukan untuk mengambilnya.
- DEK tidak digunakan lintas tujuan. Scope minimal mencakup purpose, tenant/account boundary bila relevan, dan versi format.
- AAD mengikat ciphertext pada konteks record, misalnya record ID, purpose, dan format version. AAD tidak boleh berasal dari nilai yang dapat berubah tanpa migrasi yang direncanakan.
- DEK dibersihkan setelah penggunaan. Jangan menulis plaintext DEK, KEK, atau wrapped payload sensitif ke log.
- Kegagalan unwrap atau mismatch AAD menyebabkan operasi gagal; jangan mencoba key lain secara acak.

Format payload envelope perlu versioned dan setidaknya mengidentifikasi:
- format version;
- algorithm identifier;
- KMS provider/key reference dan KEK version yang tidak rahasia;
- wrapped DEK;
- nonce;
- ciphertext plus authentication tag;
- metadata AAD yang diperlukan untuk merekonstruksi konteks secara deterministik.

Format final harus ditetapkan sebelum data persisten pertama ditulis. Jangan mengubah format dengan asumsi data lama dapat dibaca otomatis.

## 4. Strategi untuk tiap tujuan key

| Tujuan | Strategi integrasi | Catatan |
| --- | --- | --- |
| Data encryption | Envelope encryption: DEK lokal, KEK di KMS | Raw DEK hanya sementara di memori; wrapped DEK ikut payload |
| Contact fingerprint | Key HMAC terpisah yang diambil dari secret manager, atau operasi MAC KMS bila adapter dan kebutuhan latency mendukung | Rotasi harus mempertahankan pencarian dan rate-limit selama jendela transisi |
| OTP verification | Key HMAC terpisah; versi key dicatat pada challenge/digest | Verifikasi memakai versi yang tercatat; jangan menerima key version dari input publik |
| Password | Argon2id salted hash | Password tidak dienkripsi dan tidak memerlukan KMS key untuk verifikasi |

Jangan menggunakan satu key untuk beberapa tujuan. Nama purpose, versi, dan environment harus menjadi bagian dari identitas key yang divalidasi oleh adapter.

### 4.1 Contact fingerprint dan rotasi

Rotasi fingerprint berbeda dari rotasi enkripsi: fingerprint lama tidak dapat dihitung ulang dari digest saja.

Saat rotasi:
1. Versi baru menjadi primary untuk fingerprint baru.
2. Selama masa transisi, runtime menghitung fingerprint menggunakan versi aktif dan versi lama yang masih dibutuhkan.
3. Query/rate-limit harus memeriksa seluruh versi yang masih valid agar rotasi tidak menghapus batas lintas sesi atau membuka duplikasi kontak.
4. Event lama tetap dapat dievaluasi sampai melewati rolling window dan kebijakan retensinya.
5. Versi lama baru dapat dinonaktifkan setelah data dan jendela kebijakan tidak lagi bergantung padanya.

Strategi dual-read/dual-write, indeks, dan format kolom harus ditinjau bersama schema database sebelum implementasi. Jangan menambah atau mengubah migration hanya untuk menyesuaikan asumsi provider yang belum dipilih.

### 4.2 OTP key version

Challenge/digest harus mengikat key version yang dipakai saat penerbitan. Verifikasi mengambil key berdasarkan versi tersimpan di database, bukan versi dari request. Versi lama hanya boleh digunakan selama challenge terkait masih valid dan belum terminal. Setelah tidak ada challenge aktif yang bergantung pada versi tersebut, key dapat masuk tahap decommission sesuai prosedur.

## 5. Kontrak adapter dan batas async

Implementasi berikutnya harus memisahkan tiga tanggung jawab:

1. **Key management adapter:** komunikasi dengan KMS/secret manager, otorisasi, timeout, retry terkontrol, dan mapping error.
2. **Key material/session boundary:** menyediakan material DEK/HMAC yang sudah diotorisasi untuk operasi lokal, dengan lifetime dan zeroization yang jelas.
3. **Cryptographic primitives:** AEAD/HMAC/CSPRNG yang tidak melakukan network I/O dan tidak mengetahui vendor.

Pemanggilan remote harus berada pada boundary async/service. Jangan menyisipkan network I/O ke fungsi kriptografi sinkron atau memblokir Tokio runtime.

Kontrak minimum adapter:

- Resolve key berdasarkan purpose, environment, dan version yang diizinkan.
- Menghasilkan/membuka wrapped DEK dengan key reference dan version yang eksplisit.
- Menolak purpose/version yang tidak dikenal.
- Memiliki timeout dan klasifikasi error yang aman.
- Tidak melakukan retry otomatis untuk operasi yang tidak idempotent tanpa operation ID.
- Tidak mencatat key material, plaintext DEK, ciphertext sensitif, OTP, atau kontak.
- Mendukung health/readiness check tanpa membocorkan konfigurasi rahasia.
- Mendukung audit akses key tanpa mencatat nilai key.
- Menerapkan least privilege per environment dan operasi.

Interface `KeyProvider` yang ada boleh dipertahankan sebagai adapter lokal untuk key material yang memang boleh berada di memori. Untuk remote KMS, buat interface async terpisah atau generic service boundary yang sesuai; jangan memaksakan KMS non-exportable ke trait yang meminta raw bytes.

## 6. Penggunaan, cache, dan rotasi

### 6.1 Kuota penggunaan

AES-GCM memiliki batas keamanan terkait jumlah invocation, volume data, nonce, dan tag forgery. Kuota tidak boleh dihitung per proses saja karena aplikasi dapat berjalan pada beberapa instance.

Sebelum produksi:
- tetapkan batas per key berdasarkan spesifikasi algoritma, ukuran payload, volume puncak, dan model nonce;
- hitung penggunaan secara global atau gunakan pembagian kuota per instance yang memiliki batas atas terverifikasi;
- rotasi sebelum batas konservatif tercapai, bukan setelah;
- tangani retry agar satu operasi tidak menggandakan konsumsi tanpa pencatatan;
- alert ketika pemakaian mendekati ambang dan hentikan operasi enkripsi jika kuota aman habis.

Angka kuota final belum ditetapkan karena kapasitas, jumlah instance, ukuran payload, dan karakteristik provider belum dipilih. Jangan membuat angka default produksi.

### 6.2 Cache

Cache key hanya boleh dipakai bila model provider dan kebijakan keamanan mengizinkan.

- Cache harus dibatasi berdasarkan purpose, key version, environment, kapasitas, dan TTL.
- Cache tidak boleh mengubah status key yang telah dicabut menjadi aktif.
- Revocation harus memiliki jalur invalidasi yang jelas; TTL bukan satu-satunya kontrol pencabutan.
- Key material yang dikeluarkan dari cache dibersihkan bila memungkinkan.
- Jangan menyimpan plaintext DEK ke persistent cache, disk, database, tracing, atau crash dump.
- Jangan membagikan cache antar-environment atau antar-purpose.

TTL dan kapasitas cache final ditetapkan berdasarkan SLA provider, risiko pencabutan, dan pengukuran runtime.

### 6.3 State lifecycle

Versi key perlu memiliki state operasional yang eksplisit:

- `staged`: tersedia untuk distribusi dan validasi, belum dipakai untuk operasi baru.
- `active`: dipakai untuk operasi baru.
- `decrypt_only`: hanya membuka data/challenge lama yang masih sah.
- `revoked`: tidak boleh dipakai; insiden atau kompromi memerlukan penanganan khusus.
- `destroyed`: material sudah dimusnahkan setelah dependensi dan kewajiban retensi dipastikan selesai.

Transisi harus diaudit dan dikendalikan. Jangan mengaktifkan key baru sebelum seluruh instance yang relevan dapat membacanya. Jangan menghancurkan key lama hanya karena key baru sudah aktif.

## 7. Startup, readiness, dan kegagalan

Startup harus memvalidasi konfigurasi key tanpa mencetak nilai rahasia:

- environment dan provider identifier sesuai deployment;
- key purpose wajib tersedia;
- active version valid dan dapat diakses;
- versi decrypt-only yang masih dibutuhkan tersedia;
- algoritma dan panjang key sesuai kontrak;
- akses KMS/secret manager memakai identity workload dengan least privilege;
- tidak ada key default, fallback kosong, atau key dari environment lain.

Readiness harus gagal bila operasi kriptografi wajib tidak dapat dilakukan dengan aman. Liveness tidak boleh bergantung pada KMS sehingga proses tetap dapat dihentikan/dikelola saat provider bermasalah. Detail error untuk pengguna tetap generik; detail teknis hanya masuk log terkontrol tanpa secret.

Jika provider tidak tersedia:
- jangan menulis ciphertext tanpa key yang valid;
- jangan melewati enkripsi atau fingerprint;
- jangan menggunakan key lama yang sudah revoked;
- jangan mengembalikan keberhasilan palsu pada operasi yang memerlukan kriptografi;
- gunakan retry terbatas hanya untuk kegagalan sementara dan sesuai idempotency;
- catat kegagalan operasional tanpa payload atau key material.

## 8. Audit, backup, dan pemulihan

Audit minimal mencatat:
- identitas key non-rahasia dan version;
- purpose dan environment;
- actor/workload identity;
- operasi (create, wrap, unwrap, MAC, rotate, revoke, restore);
- outcome dan timestamp;
- correlation/operation ID.

Audit tidak mencatat key, plaintext DEK, password, OTP, token, kontak plaintext, atau isi ciphertext.

Backup dan pemulihan:
- backup metadata key reference dan wrapped DEK bersama data yang memerlukannya;
- backup/recovery KEK mengikuti fasilitas dan prosedur resmi provider;
- uji restore dengan key version lama pada lingkungan terisolasi;
- pastikan restore tidak mengaktifkan kembali akun/data yang sudah dihapus atau melewati kebijakan retensi;
- dokumentasikan pemisahan otorisasi antara backup data dan administrasi key;
- prosedur break-glass memerlukan persetujuan, audit, dan tinjauan setelah penggunaan.

Kehilangan KEK atau key material yang masih diperlukan dapat membuat data tidak dapat dipulihkan. Pemulihan key harus diuji sebelum data produksi bergantung padanya.

## 9. Pemilihan vendor

Vendor belum dipilih. Evaluasi harus dilakukan setelah hosting/runtime dan kebutuhan wilayah data ditetapkan. Bandingkan minimal:

- dukungan envelope encryption dan GenerateDataKey/unwrap;
- dukungan HMAC/MAC bila diperlukan;
- non-exportable/HSM dan model boundary;
- workload identity dan least-privilege policy;
- versioning, rotation, revocation, dan recovery;
- audit log dan integrasi monitoring;
- region/data residency, availability, SLA, dan outage behavior;
- latency, kuota API, batas payload, dan biaya;
- SDK Rust atau protokol API yang didukung;
- kemudahan migrasi/portabilitas dan risiko vendor lock-in;
- biaya operasional untuk proyek dengan skala awal.

Jangan memilih vendor hanya berdasarkan popularitas atau contoh dokumentasi. Jangan menambahkan SDK vendor ke dependency sebelum keputusan tercatat.

## 10. Pengujian dan kriteria penerimaan

### Unit/contract tests
- Purpose dan environment tidak dapat tertukar.
- Versi aktif dipakai untuk operasi baru; versi lama hanya untuk read/decrypt yang diizinkan.
- Unknown, revoked, atau destroyed version ditolak.
- Wrapped DEK hanya dapat dibuka dengan KEK/version yang tepat.
- Ciphertext/AAD/wrapped DEK yang berubah ditolak.
- Error dan log tidak mengandung key material atau plaintext.
- Rotasi fingerprint tetap menemukan nilai versi lama selama masa transisi.
- OTP challenge hanya diverifikasi dengan key version yang tercatat.
- Cache menghormati expiry dan revocation.
- Kuota global mencegah pemakaian melewati batas yang ditetapkan.

### Integration tests
- KMS/secret manager sandbox dengan workload identity terbatas.
- Timeout, throttling, permission denied, key disabled, key deleted, dan provider unavailable.
- Retry dan idempotency tidak menggandakan operasi yang tidak aman.
- Multi-instance usage accounting dan rotation.
- Restore backup memakai versi key lama.
- Startup/readiness fail-closed tanpa membocorkan secret.

CI reguler menggunakan fake adapter dan contract tests. Tes provider nyata dilakukan pada akun sandbox terpisah dengan key non-production; tidak ada credential atau key produksi di CI.

### Gate sebelum data produksi
Semua poin berikut wajib selesai:
- vendor dan wilayah dipilih serta ditinjau;
- threat model dan privilege policy disetujui;
- adapter dan payload envelope diuji;
- usage quota dan rotasi lintas instance diterapkan;
- backup/recovery dan incident runbook diuji;
- observability dan alert tanpa PII tersedia;
- staging test dan security review lulus;
- persetujuan terpisah untuk provisioning key produksi, migrasi produksi, dan deployment.

## 11. Urutan implementasi

1. Tetapkan hosting/runtime target dan vendor shortlist berdasarkan kriteria bagian 9.
2. Putuskan apakah key HMAC diambil dari secret manager atau dioperasikan sebagai MAC non-exportable di KMS.
3. Refactor batas async/provider agar remote I/O tidak masuk primitive sinkron.
4. Tetapkan versioned envelope payload dan migration strategy sebelum penyimpanan persisten digunakan.
5. Implementasikan adapter sandbox, workload identity, startup/readiness validation, cache, dan audit.
6. Implementasikan distributed usage accounting, rotasi, revocation, backup, dan recovery.
7. Jalankan integration/security review dan uji staging.
8. Minta persetujuan terpisah sebelum provisioning key produksi, migrasi produksi, atau deployment.

## 12. Referensi

- NIST SP 800-57, Recommendation for Key Management: https://csrc.nist.gov/pubs/sp/800/57/pt1/r5/final
- OWASP Cryptographic Storage Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html
- Google Tink — Keysets and rotation: https://developers.google.com/tink/design/keysets
- Google Cloud KMS — Envelope encryption: https://docs.cloud.google.com/kms/docs/envelope-encryption
- AWS KMS — Envelope encryption: https://docs.aws.amazon.com/kms/latest/APIReference/API_GenerateDataKey.html
- Azure Key Vault — Key concepts: https://learn.microsoft.com/en-us/azure/key-vault/keys/about-keys
- HashiCorp Vault — Transit secrets engine: https://developer.hashicorp.com/vault/docs/secrets/transit
- Rust `ring` documentation: https://docs.rs/ring/0.17.14/ring/
- Rust `zeroize` documentation: https://docs.rs/zeroize/1.9.2/zeroize/
