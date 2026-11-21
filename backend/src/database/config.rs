use std::{env, error::Error, fmt, str::FromStr};

use sqlx::postgres::PgConnectOptions;

const DEFAULT_MAX_CONNECTIONS: u32 = 10;
const MAX_ALLOWED_CONNECTIONS: u32 = 100;

#[derive(Clone)]
pub struct DatabaseConfig {
    connect_options: PgConnectOptions,
    max_connections: u32,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, DatabaseConfigError> {
        let database_url =
            env::var("DATABASE_URL").map_err(|_| DatabaseConfigError::MissingDatabaseUrl)?;
        let max_connections = env::var("DATABASE_MAX_CONNECTIONS").ok();

        Self::from_values(&database_url, max_connections.as_deref())
    }

    fn from_values(
        database_url: &str,
        max_connections: Option<&str>,
    ) -> Result<Self, DatabaseConfigError> {
        let connect_options = PgConnectOptions::from_str(database_url)
            .map_err(|_| DatabaseConfigError::InvalidDatabaseUrl)?;

        let max_connections = match max_connections {
            Some(value) => value
                .parse::<u32>()
                .map_err(|_| DatabaseConfigError::InvalidMaxConnections)?,
            None => DEFAULT_MAX_CONNECTIONS,
        };

        if !(1..=MAX_ALLOWED_CONNECTIONS).contains(&max_connections) {
            return Err(DatabaseConfigError::InvalidMaxConnections);
        }

        Ok(Self {
            connect_options,
            max_connections,
        })
    }

    pub(super) fn connect_options(&self) -> PgConnectOptions {
        self.connect_options.clone()
    }

    pub(super) fn max_connections(&self) -> u32 {
        self.max_connections
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseConfigError {
    MissingDatabaseUrl,
    InvalidDatabaseUrl,
    InvalidMaxConnections,
}

impl fmt::Display for DatabaseConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDatabaseUrl => {
                write!(formatter, "DATABASE_URL environment variable is required")
            }
            Self::InvalidDatabaseUrl => write!(formatter, "DATABASE_URL is invalid"),
            Self::InvalidMaxConnections => write!(
                formatter,
                "DATABASE_MAX_CONNECTIONS must be an integer between 1 and 100"
            ),
        }
    }
}

impl Error for DatabaseConfigError {}

#[cfg(test)]
mod tests {
    use super::{DatabaseConfig, DatabaseConfigError};

    const VALID_URL: &str = "postgres://app_user:secret@localhost/usahapintar_test";

    #[test]
    fn applies_default_connection_limit() {
        let config = DatabaseConfig::from_values(VALID_URL, None).unwrap();
        assert_eq!(config.max_connections(), 10);
    }

    #[test]
    fn accepts_custom_connection_limit() {
        let config = DatabaseConfig::from_values(VALID_URL, Some("24")).unwrap();
        assert_eq!(config.max_connections(), 24);
    }

    #[test]
    fn rejects_invalid_database_url_without_echoing_it() {
        let error = match DatabaseConfig::from_values("not-a-database-url", None) {
            Err(error) => error,
            Ok(_) => panic!("invalid URL must be rejected"),
        };
        assert_eq!(error, DatabaseConfigError::InvalidDatabaseUrl);
        assert!(!error.to_string().contains("not-a-database-url"));
    }

    #[test]
    fn rejects_non_numeric_connection_limit() {
        assert!(matches!(
            DatabaseConfig::from_values(VALID_URL, Some("many")),
            Err(DatabaseConfigError::InvalidMaxConnections)
        ));
    }

    #[test]
    fn rejects_zero_connection_limit() {
        assert!(matches!(
            DatabaseConfig::from_values(VALID_URL, Some("0")),
            Err(DatabaseConfigError::InvalidMaxConnections)
        ));
    }

    #[test]
    fn rejects_connection_limit_above_safety_cap() {
        assert!(matches!(
            DatabaseConfig::from_values(VALID_URL, Some("101")),
            Err(DatabaseConfigError::InvalidMaxConnections)
        ));
    }
}
