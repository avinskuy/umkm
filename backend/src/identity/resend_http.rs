//! Resend Email API adapter for registration and account-recovery OTP delivery.
//!
//! API credentials are injected by the runtime and never logged.

use std::{future::Future, pin::Pin, time::Duration};

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::delivery::{
    DeliveryChannel, DispatchStatus, OtpDeliveryMessage, OtpDeliveryProvider,
};

const API_URL: &str = "https://api.resend.com/emails";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RESPONSE_BYTES: usize = 16 * 1024;
const OTP_LENGTH: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResendHttpConfigError {
    EmptyApiKey,
    InvalidSender,
    InvalidClient,
}

impl std::fmt::Display for ResendHttpConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EmptyApiKey => "Resend API key is empty or invalid",
            Self::InvalidSender => "Resend sender address is empty or invalid",
            Self::InvalidClient => "Resend HTTP client could not be initialized",
        })
    }
}

impl std::error::Error for ResendHttpConfigError {}

pub struct ResendHttpProvider {
    client: Client,
    api_key: Zeroizing<String>,
    sender: String,
    endpoint: String,
}

#[derive(Serialize)]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: [&'a str; 1],
    subject: &'static str,
    text: String,
    html: String,
}

#[derive(Deserialize)]
struct SendEmailResponse {
    id: String,
}

impl ResendHttpProvider {
    pub fn new(
        api_key: Zeroizing<String>,
        sender: String,
    ) -> Result<Self, ResendHttpConfigError> {
        Self::with_endpoint(api_key, sender, API_URL)
    }

    fn with_endpoint(
        api_key: Zeroizing<String>,
        sender: String,
        endpoint: &str,
    ) -> Result<Self, ResendHttpConfigError> {
        if api_key.trim().is_empty() || api_key.len() > 4096 {
            return Err(ResendHttpConfigError::EmptyApiKey);
        }
        if sender.trim().is_empty()
            || sender.len() > 254
            || sender.chars().any(|character| matches!(character, '\r' | '\n'))
        {
            return Err(ResendHttpConfigError::InvalidSender);
        }

        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("usahapintar-api/0.1")
            .build()
            .map_err(|_| ResendHttpConfigError::InvalidClient)?;

        Ok(Self {
            client,
            api_key,
            sender,
            endpoint: endpoint.to_owned(),
        })
    }

    fn valid_message(message: &OtpDeliveryMessage) -> bool {
        message.channel == DeliveryChannel::Email
            && message.destination.len() <= 254
            && !message.destination.trim().is_empty()
            && !message.destination.chars().any(|character| matches!(character, '\r' | '\n'))
            && message.code.len() == OTP_LENGTH
            && message.code.bytes().all(|byte| byte.is_ascii_digit())
            && !message.idempotency_key.trim().is_empty()
            && message.idempotency_key.len() <= 256
            && !message.idempotency_key.chars().any(|character| matches!(character, '\r' | '\n'))
    }

    fn payload<'a>(&'a self, message: &'a OtpDeliveryMessage) -> SendEmailRequest<'a> {
        let text = format!(
            "Kode verifikasi USahapintar Anda: {}\nKode berlaku selama 10 menit. Jangan bagikan kode ini kepada siapa pun.",
            message.code.as_str()
        );
        let html = format!(
            "<p>Kode verifikasi USahapintar Anda:</p><p><strong>{}</strong></p><p>Kode berlaku selama 10 menit. Jangan bagikan kode ini kepada siapa pun.</p>",
            message.code.as_str()
        );

        SendEmailRequest {
            from: &self.sender,
            to: [message.destination.as_str()],
            subject: "Kode verifikasi USahapintar",
            text,
            html,
        }
    }

    async fn send(&self, message: OtpDeliveryMessage) -> DispatchStatus {
        if !Self::valid_message(&message) {
            return DispatchStatus::Failed;
        }

        let payload = self.payload(&message);
        let response = match self
            .client
            .post(&self.endpoint)
            .bearer_auth(self.api_key.as_str())
            .header("Idempotency-Key", message.idempotency_key.as_str())
            .json(&payload)
            .send()
            .await
        {
            Ok(response) => response,
            Err(_) => return DispatchStatus::Unknown,
        };

        let status = response.status();
        let body = match Self::read_bounded_response(response).await {
            Ok(body) => body,
            Err(()) => return DispatchStatus::Unknown,
        };

        if status.is_success() {
            return match serde_json::from_slice::<SendEmailResponse>(&body) {
                Ok(result) if !result.id.trim().is_empty() => DispatchStatus::Accepted,
                _ => DispatchStatus::Unknown,
            };
        }

        if status.is_client_error()
            && !matches!(
                status,
                StatusCode::REQUEST_TIMEOUT
                    | StatusCode::CONFLICT
                    | StatusCode::TOO_EARLY
                    | StatusCode::TOO_MANY_REQUESTS
            )
        {
            DispatchStatus::Failed
        } else {
            DispatchStatus::Unknown
        }
    }

    async fn read_bounded_response(response: reqwest::Response) -> Result<Vec<u8>, ()> {
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(());
        }

        let mut response = response;
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| ())? {
            if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(());
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }
}

impl OtpDeliveryProvider for ResendHttpProvider {
    fn dispatch<'a>(
        &'a self,
        message: OtpDeliveryMessage,
    ) -> Pin<Box<dyn Future<Output = DispatchStatus> + Send + 'a>> {
        Box::pin(self.send(message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_rejects_empty_credentials_and_invalid_sender() {
        assert_eq!(
            ResendHttpProvider::with_endpoint(
                Zeroizing::new(String::new()),
                "USahapintar <noreply@example.test>".to_owned(),
                "https://api.resend.test/emails",
            )
            .err(),
            Some(ResendHttpConfigError::EmptyApiKey)
        );

        assert_eq!(
            ResendHttpProvider::with_endpoint(
                Zeroizing::new("re_test_key".to_owned()),
                "noreply@example.test\r\nBcc: attacker@example.test".to_owned(),
                "https://api.resend.test/emails",
            )
            .err(),
            Some(ResendHttpConfigError::InvalidSender)
        );
    }

    #[test]
    fn message_validation_accepts_only_six_digit_email_codes() {
        let valid = OtpDeliveryMessage {
            idempotency_key: "otp-registration-123".to_owned(),
            destination: Zeroizing::new("user@example.test".to_owned()),
            channel: DeliveryChannel::Email,
            code: Zeroizing::new("012345".to_owned()),
        };
        assert!(ResendHttpProvider::valid_message(&valid));

        let invalid_channel = OtpDeliveryMessage {
            channel: DeliveryChannel::WhatsApp,
            ..valid
        };
        assert!(!ResendHttpProvider::valid_message(&invalid_channel));
    }

    #[test]
    fn message_validation_rejects_line_breaks_in_idempotency_key() {
        let message = OtpDeliveryMessage {
            idempotency_key: "otp-1\r\nX-Injected: yes".to_owned(),
            destination: Zeroizing::new("user@example.test".to_owned()),
            channel: DeliveryChannel::Email,
            code: Zeroizing::new("123456".to_owned()),
        };
        assert!(!ResendHttpProvider::valid_message(&message));
    }
}
