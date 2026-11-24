//! Kontrak provider OTP yang membuat dan memverifikasi kode di sisi penyedia.
//!
//! FlowKirim Verify memiliki lifecycle OTP sendiri. Kode tidak pernah dibuat,
//! di-hash, atau disimpan oleh backend untuk challenge provider-managed ini.

use std::{future::Future, pin::Pin};

use zeroize::Zeroizing;

/// Hasil transport/provider dibedakan agar timeout tidak dianggap sebagai penolakan.
/// Rejected berarti provider memproses operasi tetapi menolaknya (untuk check: kode
/// tidak cocok); Unavailable berarti hasil operasi mungkin belum diketahui dan perlu
/// ditangani secara idempoten sebelum mengulang permintaan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowKirimProviderOutcome<T> {
    Accepted(T),
    Rejected,
    Unavailable,
}

/// Integrasi FlowKirim Verify untuk OTP WhatsApp registrasi.
///
/// Implementasi HTTP wajib:
/// - memakai HTTPS dan timeout terbatas;
/// - mengirim Idempotency-Key stabil untuk operasi start;
/// - tidak mencatat token, nomor telepon, atau kode OTP ke log;
/// - memvalidasi struktur dan ukuran respons sebelum menggunakannya.
pub trait FlowKirimVerifyProvider: Send + Sync {
    /// Meminta FlowKirim mengirim OTP dan mengembalikan request_id.
    fn start<'a>(
        &'a self,
        destination: Zeroizing<String>,
        idempotency_key: String,
    ) -> Pin<Box<dyn Future<Output = FlowKirimProviderOutcome<String>> + Send + 'a>>;

    /// Meminta FlowKirim memeriksa kode terhadap request_id.
    fn check<'a>(
        &'a self,
        request_id: String,
        code: Zeroizing<String>,
    ) -> Pin<Box<dyn Future<Output = FlowKirimProviderOutcome<bool>> + Send + 'a>>;
}
