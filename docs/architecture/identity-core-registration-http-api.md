# Kontrak HTTP Registrasi Identity Core

- **Status:** implementasi router tersedia dan diuji secara unit; belum dipasang pada aplikasi runtime.
- **Cakupan:** menerima registrasi, resend OTP, dan verifikasi OTP.
- **Tidak termasuk:** login, sesi/cookie, pemulihan, provider nyata, KMS nyata, deployment, atau aktivasi endpoint publik.

## 1. Endpoint

| Method | Path | Fungsi |
|---|---|---|
| POST | `/api/v1/identity/registrations` | Membuat akun pending, profil, penerimaan kebijakan, verification session, dan memulai dispatch OTP |
| POST | `/api/v1/identity/registrations/{verification_id}/resend` | Meminta challenge OTP baru sesuai cooldown dan rate limit |
| POST | `/api/v1/identity/registrations/{verification_id}/verify` | Memverifikasi kode dan mengaktifkan akun secara atomik |

Router disediakan melalui `identity::http::registration_router`. Fungsi `app(pool)` dan `main.rs` tetap hanya memasang health/readiness. Tidak ada endpoint registrasi yang dapat diakses melalui aplikasi default.

## 2. Input registrasi

Body menerima:
- `display_name`
- `role`: `business_owner` atau `investor`
- `contact_channel`: `email` atau `whatsapp`
- Tepat satu dari `email` atau `phone`, sesuai kanal
- `password` dan `password_confirmation`
- `terms_accepted` dan `privacy_accepted`, keduanya wajib `true`
- `marketing_consent`, default `false`

Versi Terms, Privacy, dan pemberitahuan marketing tidak boleh dikirim oleh browser. Nilai berasal dari `RegistrationPolicyVersions` yang diinjeksi server. Opt-in marketing ditolak jika versi pemberitahuan server-side belum tersedia.

## 3. Dependensi wajib sebelum router dapat dibuat

- `KeyProvider` untuk enkripsi tujuan, fingerprint kontak, dan keyed hash OTP. Implementasi produksi belum tersedia.
- `OtpDeliveryProvider` untuk pengiriman email/WhatsApp. Adapter Resend/FlowKirim nyata belum tersedia.
- `RegistrationAbuseGuard` untuk rate limit berbasis IP dan konteks request. Interface tersedia, implementasi terdistribusi/produksi belum tersedia.
- `RegistrationPasswordPolicy` untuk aturan password yang disetujui. Nilai minimum/panjang maksimum dan pemeriksaan password umum/bocor belum diputuskan.
- `RegistrationPolicyVersions` dari konfigurasi server-side yang telah disetujui. Registry saat ini belum memiliki versi aktif.

Router tidak boleh dibuat dengan fake/allow-all guard, fake key provider, atau fake delivery provider pada lingkungan publik.

## 4. Perilaku keamanan

- Password dan konfirmasi dibungkus dengan `Zeroizing`; hashing Argon2id berjalan melalui `spawn_blocking`.
- Validasi format dasar, kanal, role, persetujuan wajib, dan kesesuaian kontak dilakukan sebelum hashing password.
- Batas body request 16 KiB; field JSON yang tidak dikenal ditolak.
- Handler memerlukan `ConnectInfo<SocketAddr>`. Di belakang reverse proxy, alamat klien hanya boleh diteruskan setelah validasi proxy tepercaya; jangan mempercayai header forwarding sembarang.
- Abuse guard dipanggil untuk create, resend, dan verify. Kontrol database tetap menegakkan batas OTP per kontak/sesi.
- Konflik identifier unik menghasilkan respons 202 generik dengan verification ID acak yang tidak terhubung ke akun. Tujuannya mengurangi enumerasi akun; ID palsu tidak dapat diverifikasi.
- Resend memberikan respons generik untuk sesi tidak dikenal/kedaluwarsa agar status sesi tidak menjadi oracle.
- Kode salah/kedaluwarsa memberi pesan generik. Detail database, key, provider, dan stack tidak dikirim ke klien.
- Dispatch `accepted`, `failed`, dan `unknown` tidak dibedakan pada respons publik registrasi.

## 5. Status dan batas respons

- Registrasi valid mengembalikan HTTP 202 dan ID sesi verifikasi.
- Konflik identifier unik juga mengembalikan HTTP 202 dengan bentuk respons yang sama.
- Input tidak valid atau penerimaan wajib yang tidak dicentang mengembalikan HTTP 400.
- Abuse guard dapat mengembalikan HTTP 429; kegagalan guard mengembalikan HTTP 503.
- Resend mengembalikan respons generik HTTP 202 jika sesi masih dapat diproses atau tidak ditemukan.
- OTP valid mengembalikan HTTP 200 setelah aktivasi berhasil.
- OTP salah/kedaluwarsa atau sesi tidak valid mengembalikan HTTP 400 generik.
- Kegagalan internal/kriptografi/provider/database yang tidak dapat diproses mengembalikan HTTP 503 generik.

Belum ada login endpoint atau cookie sesi. Setelah verifikasi, pengguna perlu menggunakan login yang akan dibuat pada tahap autentikasi berikutnya.

## 6. Pengujian

- Validasi label versi kebijakan dan penolakan label kosong/berformat salah.
- Validasi kanal dan pasangan kontak sebelum hashing password.
- Format verification ID acak dan label versi kebijakan.
- Router default tidak memasang endpoint registrasi.
- Integration test PostgreSQL mencakup registrasi email, pengiriman melalui fake provider, verifikasi/aktivasi, serta respons generik untuk identifier duplikat.
- Tes tambahan untuk resend, rate-limit decision, invalid/expired OTP, marketing opt-in, dan kegagalan provider masih menjadi gerbang sebelum router dipasang pada runtime.

## 7. Gerbang aktivasi

Sebelum endpoint dipasang ke `app()` atau `main.rs`:

1. Finalisasi dan setujui Terms of Service, Privacy Notice, dan marketing notice; tetapkan versi aktif pada registry.
2. Tetapkan password policy melalui keputusan produk/keamanan.
3. Implementasikan KeyProvider produksi dengan rotasi dan fail-closed.
4. Implementasikan provider email/WhatsApp yang telah diverifikasi dan dikonfigurasi.
5. Implementasikan abuse guard/rate limiter yang tahan restart dan multi-instance.
6. Tetapkan trusted proxy/origin policy, CORS/CSRF sesuai topologi, observability, dan batas biaya.
7. Tambahkan integration test penuh dengan PostgreSQL dan fake dependency, serta pengujian abuse/security.
8. Tinjau diff dan CI; aktivasi pada server produksi memerlukan persetujuan eksplisit terpisah.

Migration tidak dijalankan otomatis dan deployment produksi tidak termasuk dalam kontrak ini.
