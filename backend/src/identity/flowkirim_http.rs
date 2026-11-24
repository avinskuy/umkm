//! HTTP adapter FlowKirim Verify.
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
    base_url: String,
}

impl FlowKirimHttpProvider {
    pub fn new(api_key: Zeroizing<String>) -> Result<Self, FlowKirimHttpConfigError> {
        Self::with_base_url(api_key, API_BASE)
    }

    fn with_base_url(
        api_key: Zeroizing<String>,
        base_url: &str,
    ) -> Result<Self, FlowKirimHttpConfigError> {
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

        Ok(Self {
            client,
            api_key,
            base_url: base_url.trim_end_matches('/').to_owned(),
        })
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
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS
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
        Box<dyn std::future::Future<Output = FlowKirimProviderOutcome<String>> + Send + 'a>,
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
                .post(format!("{}/start", self.base_url))
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
                    if !parsed.request_id.trim().is_empty() && parsed.request_id.len() <= 256 =>
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
        Box<dyn std::future::Future<Output = FlowKirimProviderOutcome<bool>> + Send + 'a>,
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
                .post(format!("{}/check", self.base_url))
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
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    fn mock_server(
        status_line: &'static str,
        response_body: String,
    ) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();

            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();

            let mut request = Vec::new();
            let mut buffer = [0_u8; 2048];

            let expected_len = loop {
                let read = stream.read(&mut buffer).unwrap();

                assert!(read > 0, "client closed before sending a complete request");

                request.extend_from_slice(&buffer[..read]);

                if let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..header_end]);

                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;

                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);

                    break header_end + 4 + content_length;
                }
            };

            while request.len() < expected_len {
                let read = stream.read(&mut buffer).unwrap();

                assert!(read > 0, "client closed before sending the full body");

                request.extend_from_slice(&buffer[..read]);
            }

            write!(
                stream,
                "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            )
            .unwrap();

            String::from_utf8(request).unwrap()
        });

        (format!("http://{address}"), handle)
    }

    fn request_body(request: &str) -> serde_json::Value {
        let (_, body) = request.split_once("\r\n\r\n").unwrap();
        serde_json::from_str(body).unwrap()
    }

    #[tokio::test]
    async fn start_sends_authenticated_idempotent_whatsapp_request() {
        let (base_url, server) = mock_server("200 OK", r#"{"request_id":"verify_123"}"#.to_owned());

        let provider = FlowKirimHttpProvider::with_base_url(
            Zeroizing::new("test-secret".to_owned()),
            &base_url,
        )
        .unwrap();

        let outcome = provider
            .start(
                Zeroizing::new("+628123456789".to_owned()),
                "registration-whatsapp-challenge-1".to_owned(),
            )
            .await;

        assert_eq!(
            outcome,
            FlowKirimProviderOutcome::Accepted("verify_123".to_owned())
        );

        let request = server.join().unwrap();

        assert!(request.starts_with("POST /start HTTP/1.1"));

        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer test-secret"));

        assert!(request
            .to_ascii_lowercase()
            .contains("idempotency-key: registration-whatsapp-challenge-1"));

        let body = request_body(&request);

        assert_eq!(body["phone"], "+628123456789");
        assert_eq!(body["channel"], "whatsapp");
        assert_eq!(body["code_length"], OTP_CODE_LENGTH);
        assert_eq!(body["ttl"], OTP_TTL_SECONDS);
    }

    #[tokio::test]
    async fn check_sends_request_id_and_code_and_accepts_verified_response() {
        let (base_url, server) = mock_server("200 OK", r#"{"status":"verified"}"#.to_owned());

        let provider = FlowKirimHttpProvider::with_base_url(
            Zeroizing::new("test-secret".to_owned()),
            &base_url,
        )
        .unwrap();

        let outcome = provider
            .check("verify_123".to_owned(), Zeroizing::new("123456".to_owned()))
            .await;

        assert_eq!(outcome, FlowKirimProviderOutcome::Accepted(true));

        let request = server.join().unwrap();

        assert!(request.starts_with("POST /check HTTP/1.1"));

        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer test-secret"));

        let body = request_body(&request);

        assert_eq!(body["request_id"], "verify_123");
        assert_eq!(body["code"], "123456");
    }

    #[tokio::test]
    async fn check_maps_non_verified_status_to_rejected_code_result() {
        let (base_url, server) = mock_server("200 OK", r#"{"status":"invalid"}"#.to_owned());

        let provider = FlowKirimHttpProvider::with_base_url(
            Zeroizing::new("test-secret".to_owned()),
            &base_url,
        )
        .unwrap();

        let outcome = provider
            .check("verify_123".to_owned(), Zeroizing::new("000000".to_owned()))
            .await;

        assert_eq!(outcome, FlowKirimProviderOutcome::Accepted(false));

        server.join().unwrap();
    }

    #[tokio::test]
    async fn oversized_response_is_unavailable() {
        let oversized = format!(r#"{{"request_id":"{}"}}"#, "x".repeat(MAX_RESPONSE_BYTES));

        let (base_url, server) = mock_server("200 OK", oversized);

        let provider = FlowKirimHttpProvider::with_base_url(
            Zeroizing::new("test-secret".to_owned()),
            &base_url,
        )
        .unwrap();

        let outcome = provider
            .start(
                Zeroizing::new("+628123456789".to_owned()),
                "registration-whatsapp-challenge-2".to_owned(),
            )
            .await;

        assert_eq!(outcome, FlowKirimProviderOutcome::Unavailable);

        server.join().unwrap();
    }

    #[test]
    fn parses_only_a_valid_start_request_id() {
        let parsed: StartResponse = serde_json::from_str(r#"{"request_id":"verify_123"}"#).unwrap();

        assert_eq!(parsed.request_id, "verify_123");

        assert!(serde_json::from_str::<StartResponse>(r#"{"request_id":""}"#).is_ok());

        assert!(serde_json::from_str::<StartResponse>(r#"{"id":"verify_123"}"#).is_err());
    }

    #[test]
    fn check_accepts_only_documented_verified_status() {
        let verified: CheckResponse = serde_json::from_str(r#"{"status":"verified"}"#).unwrap();

        let not_verified: CheckResponse = serde_json::from_str(r#"{"status":"invalid"}"#).unwrap();

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
