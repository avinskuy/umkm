use std::{future::Future, pin::Pin};
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryChannel {
    Email,
    WhatsApp,
}

pub struct OtpDeliveryMessage {
    pub destination: Zeroizing<String>,
    pub channel: DeliveryChannel,
    pub code: Zeroizing<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchStatus {
    Accepted,
    Failed,
    Unknown,
}

impl DispatchStatus {
    pub(crate) fn as_db_value(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Failed => "failed",
            Self::Unknown => "unknown",
        }
    }
}

pub trait OtpDeliveryProvider: Send + Sync {
    fn dispatch<'a>(
        &'a self,
        message: OtpDeliveryMessage,
    ) -> Pin<Box<dyn Future<Output = DispatchStatus> + Send + 'a>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeProvider {
        result: DispatchStatus,
    }

    impl OtpDeliveryProvider for FakeProvider {
        fn dispatch<'a>(
            &'a self,
            _message: OtpDeliveryMessage,
        ) -> Pin<Box<dyn Future<Output = DispatchStatus> + Send + 'a>> {
            Box::pin(async move { self.result })
        }
    }

    #[tokio::test]
    async fn fake_provider_returns_explicit_dispatch_outcomes() {
        for status in [
            DispatchStatus::Accepted,
            DispatchStatus::Failed,
            DispatchStatus::Unknown,
        ] {
            let provider = FakeProvider { result: status };
            let result = provider
                .dispatch(OtpDeliveryMessage {
                    destination: Zeroizing::new("recipient@example.test".to_owned()),
                    channel: DeliveryChannel::Email,
                    code: Zeroizing::new("123456".to_owned()),
                })
                .await;
            assert_eq!(result, status);
        }
    }
}
