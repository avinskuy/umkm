//! HTTP adapter FlowKirim Verify.
//!
//! API key hanya diberikan melalui constructor (nantinya dari secret runtime),
//! tidak pernah dibaca dari frontend atau dicatat ke log.

use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::flowkirim::{FlowKirimProviderOutcome, FlowKirimVerifyProvider};

const API_BASE: &str = "https://scan.flowkirim.com/api/v1/verify";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RESPONSE_BYTES: usize = 16 * 1024;
const OTP_CODE_LENGTH: u8 = 6;
const OTP_TTL_SECONDS: u16 = 600;

#[derive(Debug)]
pub enum FlowKirimHttpConfigError {
    EmptyApiKey,
    InvalidClient,
}

impl std::fmt::Display for FlowKirimHttpConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EmptyApiKey => "FlowKirim API key is empty",
            Self::InvalidClient => "FlowKirim HTTP client could not be initialized",
        })
    }
}

impl std::error::Error for FlowKirimHttpConfigError {}

pub struct FlowKirimHttpProvider {
    client: Client,
    api_key: Zeroizing<String>,
}

impl FlowKirimHttpProvider {
    pub fn new(api_key: Zeroizing<String>) -> Result<Self, FlowKirimHttpConfigError> {
        if api_key.trim().is_empty() || api_key.len() > 4096 {
            return Err(FlowKirimHttpConfigError::EmptyApiKey);
        }

        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("usahapintar-api/0.1")
            .build()
            .map_err(|_| FlowKirimHttpConfigError::InvalidClient)?;

        Ok(Self { client, api_key })
    }

    async fn read_bounded_response(
        response: reqwest::Response,
    ) -> Result<(StatusCode, Vec<u8>), ()> {
        let status = response.status();
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(());
        }

        let mut body = Vec::new();
        let mut response = response;
        while let Some(chunk) = response.chunk().await.map_err(|_| ())? {
            if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(());
            }
            body.extend_from_slice(&chunk);
        }
        Ok((status, body))
    }

    fn classify_start_status(status: StatusCode) -> FlowKirimProviderOutcome<String> {
        if status.is_client_error()
            && !matches!(
                status,
                StatusCode::UNAUTHORIZED
                    | StatusCode::FORBIDDEN
                    | StatusCode::TOO_MANY_REQUESTS
            )
        {
            FlowKirimProviderOutcome::Rejected
        } else {
            FlowKirimProviderOutcome::Unavailable
        }
    }
}

#[derive(Serialize)]
struct StartRequest<'a> {
    phone: &'a str,
    channel: &'static str,
    code_length: u8,
    ttl: u16,
}

#[derive(Deserialize)]
struct StartResponse {
    request_id: String,
}

#[derive(Serialize)]
struct CheckRequest<'a> {
    request_id: &'a str,
    code: &'a str,
}

#[derive(Deserialize)]
struct CheckResponse {
    status: String,
}

impl FlowKirimVerifyProvider for FlowKirimHttpProvider {
    fn start<'a>(
        &'a self,
        destination: Zeroizing<String>,
        idempotency_key: String,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = FlowKirimProviderOutcome<String>> + Send + 'a,
        >,
    > {
        Box::pin(async move {
            if idempotency_key.trim().is_empty() || idempotency_key.len() > 256 {
                return FlowKirimProviderOutcome::Rejected;
            }

            let payload = StartRequest {
                phone: destination.as_str(),
                channel: "whatsapp",
                code_length: OTP_CODE_LENGTH,
                ttl: OTP_TTL_SECONDS,
            };

            let response = match self
                .client
                .post(format!("{API_BASE}/start"))
                .bearer_auth(self.api_key.as_str())
                .header("Idempotency-Key", idempotency_key)
                .json(&payload)
                .send()
                .await
            {
                Ok(response) => response,
                Err(_) => return FlowKirimProviderOutcome::Unavailable,
            };

            let (status, body) = match Self::read_bounded_response(response).await {
                Ok(result) => result,
                Err(()) => return FlowKirimProviderOutcome::Unavailable,
            };
            if !status.is_success() {
                return Self::classify_start_status(status);
            }

            match serde_json::from_slice::<StartResponse>(&body) {
                Ok(parsed)
                    if !parsed.request_id.trim().is_empty()
                        && parsed.request_id.len() <= 256 =>
                {
                    FlowKirimProviderOutcome::Accepted(parsed.request_id)
                }
                _ => FlowKirimProviderOutcome::Unavailable,
            }
        })
    }

    fn check<'a>(
        &'a self,
        request_id: String,
        code: Zeroizing<String>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = FlowKirimProviderOutcome<bool>> + Send + 'a,
        >,
    > {
        Box::pin(async move {
            if request_id.trim().is_empty()
                || request_id.len() > 256
                || code.len() != OTP_CODE_LENGTH as usize
                || !code.bytes().all(|byte| byte.is_ascii_digit())
            {
                return FlowKirimProviderOutcome::Rejected;
            }

            let payload = CheckRequest {
                request_id: &request_id,
                code: code.as_str(),
            };
            let response = match self
                .client
                .post(format!("{API_BASE}/check"))
                .bearer_auth(self.api_key.as_str())
                .json(&payload)
                .send()
                .await
            {
                Ok(response) => response,
                Err(_) => return FlowKirimProviderOutcome::Unavailable,
            };

            let (status, body) = match Self::read_bounded_response(response).await {
                Ok(result) => result,
                Err(()) => return FlowKirimProviderOutcome::Unavailable,
            };
            if !status.is_success() {
                return if status.is_client_error()
                    && !matches!(
                        status,
                        StatusCode::UNAUTHORIZED
                            | StatusCode::FORBIDDEN
                            | StatusCode::TOO_MANY_REQUESTS
                    )
                {
                    FlowKirimProviderOutcome::Rejected
                } else {
                    FlowKirimProviderOutcome::Unavailable
                };
            }

            match serde_json::from_slice::<CheckResponse>(&body) {
                Ok(parsed) if parsed.status == "verified" => {
                    FlowKirimProviderOutcome::Accepted(true)
                }
                Ok(parsed) if !parsed.status.trim().is_empty() && parsed.status.len() <= 64 => {
                    FlowKirimProviderOutcome::Accepted(false)
                }
                _ => FlowKirimProviderOutcome::Unavailable,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_a_valid_start_request_id() {
        let parsed: StartResponse =
            serde_json::from_str(r#"{"request_id":"verify_123"}"#).unwrap();
        assert_eq!(parsed.request_id, "verify_123");
        assert!(serde_json::from_str::<StartResponse>(r#"{"request_id":""}"#).is_ok());
        assert!(serde_json::from_str::<StartResponse>(r#"{"id":"verify_123"}"#).is_err());
    }

    #[test]
    fn check_accepts_only_documented_verified_status() {
        let verified: CheckResponse =
            serde_json::from_str(r#"{"status":"verified"}"#).unwrap();
        let not_verified: CheckResponse =
            serde_json::from_str(r#"{"status":"invalid"}"#).unwrap();
        assert_eq!(verified.status, "verified");
        assert_ne!(not_verified.status, "verified");
        assert!(serde_json::from_str::<CheckResponse>(r#"{"verified":true}"#).is_err());
    }

    #[test]
    fn request_defaults_match_registration_policy() {
        assert_eq!(OTP_CODE_LENGTH, 6);
        assert_eq!(OTP_TTL_SECONDS, 600);
        assert_eq!(MAX_RESPONSE_BYTES, 16 * 1024);
    }
}
