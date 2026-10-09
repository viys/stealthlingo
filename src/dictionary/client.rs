use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use reqwest::Url;

use super::models::Entry;
use super::{free_dictionary, merriam_webster, wiktionary, Source};
use crate::error::DictionaryError;

pub const USER_AGENT: &str = concat!(
    "stealthlingo/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/viys/stealthlingo)"
);

/// A successful lookup: the parsed entry plus the raw primary response.
#[derive(Debug, Clone)]
pub struct Fetched {
    pub entry: Entry,
    pub raw_json: String,
    pub source: Source,
}

#[derive(Debug, Clone)]
pub struct SourceSettings {
    pub source: Source,
    pub free_dictionary_url: String,
    pub merriam_webster_key: Option<String>,
}

pub struct DictionaryClient {
    http: Client,
    settings: SourceSettings,
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
    pub fn new(settings: SourceSettings, timeout: Duration) -> Result<Self, DictionaryError> {
        let http = Client::builder()
            .timeout(timeout)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| DictionaryError::Network(e.to_string()))?;
        Ok(Self {
            http,
            settings,
            timeout,
        })
    }

    pub fn source(&self) -> Source {
        self.settings.source
    }

    /// URL of the primary request for `word` with the configured source.
    pub fn entry_url(&self, word: &str) -> Result<Url, DictionaryError> {
        let word = word.trim();
        match self.settings.source {
            Source::FreeDictionary => with_segment(&self.settings.free_dictionary_url, word),
            Source::Wiktionary => {
                with_segment(wiktionary::REST_BASE, &wiktionary::page_title(word))
            }
            Source::MerriamWebster => {
                let key = self
                    .settings
                    .merriam_webster_key
                    .as_deref()
                    .map(str::trim)
                    .filter(|k| !k.is_empty())
                    .ok_or_else(merriam_webster::missing_key_error)?;
                Ok(merriam_webster::entry_url(word, key))
            }
        }
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

    /// Looks `word` up. The whole lookup, including any follow-up requests,
    /// shares one `http_timeout_secs` budget.
    pub fn fetch(&self, word: &str) -> Result<Fetched, DictionaryError> {
        let source = self.settings.source;
        let deadline = Instant::now() + self.timeout;
        let (entry, raw_json) = match source {
            Source::FreeDictionary => {
                let (status, body) = self.get(self.entry_url(word)?, deadline)?;
                (free_dictionary::interpret(word, status, &body)?, body)
            }
            Source::MerriamWebster => {
                let (status, body) = self.get(self.entry_url(word)?, deadline)?;
                (merriam_webster::interpret(word, status, &body)?, body)
            }
            Source::Wiktionary => self.fetch_wiktionary(word, deadline)?,
        };
        Ok(Fetched {
            entry,
            raw_json,
            source,
        })
    }

    /// Wiktionary titles are case-sensitive, so "Ephemeral" falls back to
    /// "ephemeral". Pronunciation data is best-effort: if the second request
    /// fails or runs out of time the definitions are still returned.
    fn fetch_wiktionary(
        &self,
        word: &str,
        deadline: Instant,
    ) -> Result<(Entry, String), DictionaryError> {
        let typed = word.trim().to_string();
        let mut candidates = vec![typed.clone()];
        let lower = typed.to_lowercase();
        if lower != typed {
            candidates.push(lower);
        }
        for candidate in &candidates {
            let (status, body) = self.get(self.entry_url(candidate)?, deadline)?;
            match wiktionary::interpret_definitions(candidate, status, &body) {
                Ok(mut entry) => {
                    if let Ok((200, page)) = self.get(wiktionary::wikitext_url(candidate), deadline)
                    {
                        if let Some(wikitext) = wiktionary::extract_wikitext(&page) {
                            wiktionary::apply_extras(
                                &mut entry,
                                wiktionary::parse_wikitext(&wikitext),
                            );
                        }
                    }
                    return Ok((entry, body));
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
        // The URL may carry an API key (Merriam-Webster), so it is never shown.
        DictionaryError::Network(err.without_url().to_string())
    }
}
