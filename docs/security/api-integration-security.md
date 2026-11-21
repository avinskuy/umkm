# API Security & Integration Security Policy

- **Status:** Disetujui sebagai baseline
- **Tanggal:** 21 November 2026
- **Cakupan:** API internal/eksternal, endpoint aplikasi, webhook, OAuth, SDK, dan layanan pihak ketiga

## 1. Referensi utama

- OWASP API Security Top 10 (2023).
- OWASP Application Security Verification Standard (ASVS) 5.0.0.
- IETF RFC 9700 — OAuth 2.0 Security Best Current Practice.
- NIST SP 800-63-4 — Digital Identity Guidelines.

Referensi tersebut menjadi dasar pemilihan kontrol sesuai risiko dan tahap implementasi, bukan klaim sertifikasi.

## 2. Authentication & session

- Endpoint privat wajib memerlukan autentikasi.
- Sesi dan token harus memiliki masa berlaku, penyimpanan aman, dan mekanisme pencabutan.
- Tindakan sensitif dapat meminta autentikasi ulang atau verifikasi tambahan berdasarkan risiko.
- Jika OAuth/OIDC digunakan, utamakan Authorization Code Flow dengan PKCE, validasi redirect URI secara ketat, dan konfigurasi sesuai panduan penyedia.
- Jangan gunakan Implicit Grant atau Resource Owner Password Credentials Grant untuk rancangan baru.
- Jangan membocorkan keberadaan akun melalui respons registrasi atau recovery.

## 3. Authorization

- Terapkan RBAC + ABAC pada setiap permintaan backend.
- Periksa akses objek dan properti; jangan mengandalkan pemeriksaan frontend.
- Terapkan least privilege dan deny-by-default.
- Pastikan kegagalan pemeriksaan berakhir dengan penolakan.
- Membership tidak boleh melewati role, kepemilikan, persetujuan, status akun, atau moderasi.

## 4. Input, query, dan response

- Validasi payload berdasarkan schema, tipe, ukuran, format, dan allowlist.
- Cegah mass assignment dengan pemetaan field yang eksplisit.
- Gunakan parameterized query dan akses database yang dibatasi.
- Batasi field respons sesuai hak akses dan klasifikasi data.
- Gunakan format error yang konsisten dan aman; jangan mengirim stack trace, query internal, secret, atau detail infrastruktur kepada pengguna.
- Terapkan batas ukuran request, pagination, dan batas kompleksitas sesuai endpoint.

## 5. Rate limit dan resource protection

- Terapkan rate limit berdasarkan akun, IP/jaringan, endpoint, tujuan, dan risiko sesuai kebutuhan.
- Lindungi endpoint autentikasi, OTP, recovery, pencarian, upload, dan operasi yang mahal.
- Batasi konkurensi, ukuran payload, waktu eksekusi, dan penggunaan resource.
- Gunakan idempotency untuk operasi yang rentan terhadap pengiriman ulang jika sesuai.
- Cegah penyalahgunaan yang dapat menyebabkan resource exhaustion.

## 6. Transport dan konfigurasi

- Gunakan HTTPS/TLS.
- Atur CORS secara ketat sesuai origin yang benar-benar dibutuhkan.
- Pisahkan konfigurasi dan kredensial antar-environment.
- Nonaktifkan debug mode dan respons error terperinci di production.
- Jangan menyimpan secret di source code atau dokumentasi publik.

## 7. API inventory dan versioning

- Gunakan prefix versi API, dengan baseline `/api/v1`.
- Inventarisasikan endpoint, pemilik, data yang diproses, autentikasi, dan dependensinya.
- Perubahan kontrak harus ditinjau dampak kompatibilitasnya.
- Penghentian endpoint dilakukan secara terencana dan didokumentasikan.
- Dokumentasi API publik harus disanitasi dari kredensial, data pribadi, dan detail operasional sensitif.

## 8. Integrasi eksternal

- Minta scope dan data minimum dari penyedia.
- Simpan, rotasi, dan cabut kredensial integrasi secara aman.
- Validasi respons eksternal; jangan mempercayai data atau URL dari pihak luar.
- Tetapkan timeout, batas ukuran, dan penanganan kegagalan.
- Terapkan perlindungan SSRF untuk fitur yang mengambil URL atau resource yang ditentukan pengguna.
- Autentikasi webhook, validasi signature, timestamp, dan perlindungan replay sesuai kemampuan penyedia.
- Evaluasi vendor, subprocessor, serta dampak jika dependency tidak tersedia.

## 9. Pengujian

Uji BOLA, BFLA, BOPLA, bypass autentikasi, injection, resource exhaustion, SSRF, konfigurasi yang salah, kebocoran data, manipulasi parameter, webhook replay, dan kegagalan integrasi.

Prioritaskan Identity Core, profil, posting/moderasi, koneksi, percakapan privat, dokumen, membership, recovery, serta operasi Admin.

## Referensi

- OWASP API Security Top 10 (2023).
- OWASP ASVS 5.0.0.
- IETF RFC 9700.
- NIST SP 800-63-4.
