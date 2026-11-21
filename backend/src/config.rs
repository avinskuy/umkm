use std::{
    error::Error,
    fmt,
    net::{IpAddr, SocketAddr},
};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 5000;

#[derive(Debug, PartialEq, Eq)]
pub enum ServerConfigError {
    InvalidHost(String),
    InvalidPort(String),
    ZeroPort,
}

impl fmt::Display for ServerConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHost(host) => {
                write!(formatter, "HOST must be a valid IP address, got '{host}'")
            }
            Self::InvalidPort(port) => {
                write!(
                    formatter,
                    "PORT must be an integer between 1 and 65535, got '{port}'"
                )
            }
            Self::ZeroPort => write!(formatter, "PORT must be greater than zero"),
        }
    }
}

impl Error for ServerConfigError {}

pub fn server_address(
    host: Option<&str>,
    port: Option<&str>,
) -> Result<SocketAddr, ServerConfigError> {
    let host = host.unwrap_or(DEFAULT_HOST);
    let ip = host
        .parse::<IpAddr>()
        .map_err(|_| ServerConfigError::InvalidHost(host.to_owned()))?;

    let port = match port {
        Some(port) => port
            .parse::<u16>()
            .map_err(|_| ServerConfigError::InvalidPort(port.to_owned()))?,
        None => DEFAULT_PORT,
    };

    if port == 0 {
        return Err(ServerConfigError::ZeroPort);
    }

    Ok(SocketAddr::new(ip, port))
}

#[cfg(test)]
mod tests {
    use super::{server_address, ServerConfigError};
    use std::net::SocketAddr;

    #[test]
    fn uses_safe_defaults_when_environment_values_are_missing() {
        assert_eq!(
            server_address(None, None),
            Ok("127.0.0.1:5000".parse::<SocketAddr>().unwrap())
        );
    }

    #[test]
    fn accepts_custom_ipv4_host_and_port() {
        assert_eq!(
            server_address(Some("0.0.0.0"), Some("8080")),
            Ok("0.0.0.0:8080".parse::<SocketAddr>().unwrap())
        );
    }

    #[test]
    fn accepts_ipv6_host() {
        assert_eq!(
            server_address(Some("::1"), Some("5001")),
            Ok("[::1]:5001".parse::<SocketAddr>().unwrap())
        );
    }

    #[test]
    fn rejects_invalid_host() {
        assert_eq!(
            server_address(Some("localhost"), Some("5000")),
            Err(ServerConfigError::InvalidHost("localhost".to_owned()))
        );
    }

    #[test]
    fn rejects_non_numeric_port() {
        assert_eq!(
            server_address(Some("127.0.0.1"), Some("abc")),
            Err(ServerConfigError::InvalidPort("abc".to_owned()))
        );
    }

    #[test]
    fn rejects_out_of_range_port() {
        assert_eq!(
            server_address(Some("127.0.0.1"), Some("65536")),
            Err(ServerConfigError::InvalidPort("65536".to_owned()))
        );
    }

    #[test]
    fn rejects_zero_port() {
        assert_eq!(
            server_address(Some("127.0.0.1"), Some("0")),
            Err(ServerConfigError::ZeroPort)
        );
    }
}
