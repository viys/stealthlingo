use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use reqwest::Url;

use super::models::{clean_word, Entry};
use super::wiktionary;
use crate::error::DictionaryError;

pub const USER_AGENT: &str = concat!(
    "stealthlingo/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/viys/stealthlingo)"
);

/// A successful lookup: the parsed entry plus the raw definition response.
#[derive(Debug, Clone)]
pub struct Fetched {
    pub entry: Entry,
    pub raw_json: String,
    /// False when the page wikitext (pronunciations, audio, synonyms) could
    /// not be fetched, so the entry only has definitions.
    pub complete: bool,
}

/// Wiktionary API base URLs. Tests point these at a closed local port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoints {
    pub rest_base: String,
    pub action_api: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            rest_base: wiktionary::REST_BASE.to_string(),
            action_api: wiktionary::ACTION_API.to_string(),
        }
    }
}

pub struct DictionaryClient {
    http: Client,
    endpoints: Endpoints,
    timeout: Duration,
}

/// Appends `segment` (percent-encoded) to the path of `base`.
fn with_segment(base: &str, segment: &str) -> Result<Url, DictionaryError> {
    let mut url = Url::parse(base)
        .map_err(|e| DictionaryError::Config(format!("invalid dictionary URL {base}: {e}")))?;
    url.path_segments_mut()
        .map_err(|_| DictionaryError::Config(format!("invalid dictionary URL {base}")))?
        .pop_if_empty()
        .push(segment);
    Ok(url)
}

impl DictionaryClient {
    pub fn new(endpoints: Endpoints, timeout: Duration) -> Result<Self, DictionaryError> {
        let http = Client::builder()
            .timeout(timeout)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| DictionaryError::Network(e.to_string()))?;
        Ok(Self {
            http,
            endpoints,
            timeout,
        })
    }

    /// URL of the definition request for `word`.
    pub fn entry_url(&self, word: &str) -> Result<Url, DictionaryError> {
        with_segment(
            &self.endpoints.rest_base,
            &wiktionary::page_title(&clean_word(word)),
        )
    }

    /// Sends a GET request that must finish before `deadline`.
    fn get(&self, url: Url, deadline: Instant) -> Result<(u16, String), DictionaryError> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(DictionaryError::Timeout(self.timeout.as_secs()));
        }
        let describe = |err| describe_request_error(err, self.timeout);
        let response = self
            .http
            .get(url)
            .timeout(remaining)
            .send()
            .map_err(describe)?;
        let status = response.status().as_u16();
        let body = response.text().map_err(describe)?;
        Ok((status, body))
    }

    /// Looks `word` up. The whole lookup, including the pronunciation request,
    /// shares one `http_timeout_secs` budget.
    ///
    /// Wiktionary titles are case-sensitive, so "Ephemeral" falls back to
    /// "ephemeral". Pronunciation data is best-effort: if the second request
    /// fails or runs out of time the definitions are still returned.
    pub fn fetch(&self, word: &str) -> Result<Fetched, DictionaryError> {
        let deadline = Instant::now() + self.timeout;
        let typed = clean_word(word);
        let mut candidates = vec![typed.clone()];
        let lower = typed.to_lowercase();
        if lower != typed {
            candidates.push(lower);
        }
        for candidate in &candidates {
            let (status, body) = self.get(self.entry_url(candidate)?, deadline)?;
            match wiktionary::interpret_definitions(candidate, status, &body) {
                Ok(mut entry) => {
                    let page_url = wiktionary::wikitext_url(&self.endpoints.action_api, candidate)?;
                    let wikitext = match self.get(page_url, deadline) {
                        Ok((200, page)) => wiktionary::extract_wikitext(&page),
                        _ => None,
                    };
                    let complete = wikitext.is_some();
                    if let Some(wikitext) = wikitext {
                        wiktionary::apply_extras(&mut entry, wiktionary::parse_wikitext(&wikitext));
                    }
                    return Ok(Fetched {
                        entry,
                        raw_json: body,
                        complete,
                    });
                }
                Err(DictionaryError::NotFound(_)) => continue,
                Err(err) => return Err(err),
            }
        }
        Err(DictionaryError::NotFound(typed))
    }
}

fn describe_request_error(err: reqwest::Error, timeout: Duration) -> DictionaryError {
    if err.is_connect() {
        let reason = if err.is_timeout() {
            "connection timed out"
        } else {
            "connection refused or no network"
        };
        DictionaryError::Unreachable(reason.to_string())
    } else if err.is_timeout() {
        DictionaryError::Timeout(timeout.as_secs())
    } else {
        DictionaryError::Network(err.without_url().to_string())
    }
}
