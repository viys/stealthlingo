use thiserror::Error;

/// Failures when talking to the remote dictionary service.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DictionaryError {
    #[error("\"{0}\" was not found in the dictionary; check the spelling")]
    NotFound(String),
    /// Connected, but no answer arrived within the timeout (seconds).
    #[error(
        "the dictionary service did not answer within {0}s; it is slow or down right now \
         (a problem on its side, not your connection)"
    )]
    Timeout(u64),
    /// Could not establish a connection at all.
    #[error("could not connect to the dictionary service ({0}); check your internet connection or proxy")]
    Unreachable(String),
    #[error("network error while contacting the dictionary service: {0}")]
    Network(String),
    #[error("{}", describe_status(*.0))]
    Server(u16),
    #[error("could not understand the dictionary response: {0}")]
    Parse(String),
    /// The dictionary endpoint is misconfigured (e.g. an invalid URL).
    #[error("{0}")]
    Config(String),
}

fn describe_status(status: u16) -> String {
    match status {
        // CDNs such as Cloudflare report origin failures as 52x.
        520..=527 | 530 => format!(
            "the dictionary service is down right now (HTTP {status}: its server is not responding); \
             this is a problem on its side, not your connection"
        ),
        429 => "the dictionary service is rate-limiting requests (HTTP 429); wait a minute and try again"
            .to_string(),
        500..=599 => format!(
            "the dictionary service had an internal error (HTTP {status}); this is a problem on its side"
        ),
        other => format!("the dictionary service returned an unexpected HTTP {other} response"),
    }
}

impl DictionaryError {
    /// Temporary failures where retrying later or falling back to cached data makes sense.
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::Timeout(_) | Self::Unreachable(_) | Self::Network(_) | Self::Server(_)
        )
    }

    /// True when the dictionary service itself is failing, as opposed to the local network.
    pub fn is_service_side(&self) -> bool {
        matches!(self, Self::Timeout(_) | Self::Server(_))
    }
}
