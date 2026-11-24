//! Opt-in smoke test for the real FlowKirim Verify API.
//!
//! This test is ignored by default because it sends a real WhatsApp OTP.
//! Run manually with:
//! FLOWKIRIM_API_KEY set in the environment,
//! FLOWKIRIM_TEST_PHONE set to a phone number you control in E.164 format,
//! cargo test --locked -p usahapintar-api --test flowkirim_smoke -- --ignored --nocapture

use std::{
    env,
    io::{self, Write},
    time::{SystemTime, UNIX_EPOCH},
};

use usahapintar_api::identity::{
    flowkirim::FlowKirimVerifyProvider,
    flowkirim_http::FlowKirimHttpProvider,
    FlowKirimProviderOutcome,
};
use zeroize::Zeroizing;

#[tokio::test]
#[ignore = "sends a real WhatsApp OTP through FlowKirim"]
async fn sends_and_verifies_a_real_whatsapp_otp() {
    let api_key = env::var("FLOWKIRIM_API_KEY")
        .expect("FLOWKIRIM_API_KEY must be set for this manual smoke test");
    let phone = env::var("FLOWKIRIM_TEST_PHONE")
        .expect("FLOWKIRIM_TEST_PHONE must be set to a number you control");

    assert!(
        phone.starts_with("+") && phone.len() >= 8 && phone.len() <= 16,
        "FLOWKIRIM_TEST_PHONE must use E.164 format, for example +628123456789"
    );

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after UNIX epoch")
        .as_nanos();
    let idempotency_key = format!("manual-flowkirim-smoke-{nonce}");

    let provider = FlowKirimHttpProvider::new(Zeroizing::new(api_key))
        .expect("FlowKirim provider configuration should be valid");

    let request_id = match provider
        .start(Zeroizing::new(phone), idempotency_key)
        .await
    {
        FlowKirimProviderOutcome::Accepted(request_id) => request_id,
        FlowKirimProviderOutcome::Rejected => {
            panic!("FlowKirim rejected the OTP request; check account/API configuration")
        }
        FlowKirimProviderOutcome::Unavailable => {
            panic!("FlowKirim is unavailable; check connectivity and provider status")
        }
    };

    print!("OTP sent. Enter the code received on your WhatsApp: ");
    io::stdout().flush().expect("prompt should flush");

    let mut code = String::new();
    io::stdin()
        .read_line(&mut code)
        .expect("OTP should be readable from stdin");
    let code = code.trim().to_owned();

    assert!(
        code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit()),
        "OTP must contain exactly six digits"
    );

    match provider
        .check(request_id, Zeroizing::new(code))
        .await
    {
        FlowKirimProviderOutcome::Accepted(true) => {
            println!("PASS: FlowKirim accepted the real WhatsApp OTP.");
        }
        FlowKirimProviderOutcome::Accepted(false) | FlowKirimProviderOutcome::Rejected => {
            panic!("FlowKirim did not verify the supplied OTP");
        }
        FlowKirimProviderOutcome::Unavailable => {
            panic!("FlowKirim verification endpoint is unavailable");
        }
    }
}
