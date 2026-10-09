//! Pronunciation playback. Audio files are downloaded once into the data
//! directory and replayed from disk afterwards, so cached words work offline.

use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;

pub struct AudioPlayer {
    cache_dir: PathBuf,
    http: Client,
}

impl AudioPlayer {
    pub fn new(cache_dir: PathBuf, timeout: Duration) -> Result<Self> {
        let http = Client::builder()
            .timeout(timeout)
            .user_agent(crate::dictionary::client::USER_AGENT)
            .build()
            .context("could not set up the HTTP client")?;
        Ok(Self { cache_dir, http })
    }

    pub fn cached_path(&self, headword: &str, url: &str) -> PathBuf {
        self.cache_dir.join(cache_file_name(headword, url))
    }

    /// Returns the local audio file, downloading it first if necessary.
    pub fn ensure_cached(&self, headword: &str, url: &str) -> Result<PathBuf> {
        let path = self.cached_path(headword, url);
        if path.is_file() {
            return Ok(path);
        }
        fs::create_dir_all(&self.cache_dir)
            .with_context(|| format!("could not create {}", self.cache_dir.display()))?;
        let response = self
            .http
            .get(url)
            .send()
            .context("could not download the pronunciation (are you offline?)")?;
        if !response.status().is_success() {
            bail!(
                "the pronunciation file is unavailable (HTTP {})",
                response.status().as_u16()
            );
        }
        let bytes = response
            .bytes()
            .context("pronunciation download was interrupted")?;
        if bytes.is_empty() {
            bail!("the pronunciation file is empty");
        }
        let tmp = path.with_extension("part");
        fs::write(&tmp, &bytes).with_context(|| format!("could not write {}", tmp.display()))?;
        fs::rename(&tmp, &path).with_context(|| format!("could not write {}", path.display()))?;
        Ok(path)
    }

    pub fn play(&self, headword: &str, url: &str) -> Result<()> {
        let path = self.ensure_cached(headword, url)?;
        if let Err(err) = play_file(&path) {
            // A corrupt download should not poison the cache.
            if err.downcast_ref::<rodio::decoder::DecoderError>().is_some() {
                let _ = fs::remove_file(&path);
            }
            return Err(err);
        }
        Ok(())
    }
}

/// Plays an audio file and blocks until playback finishes.
pub fn play_file(path: &Path) -> Result<()> {
    let (_stream, handle) =
        rodio::OutputStream::try_default().context("no audio output device is available")?;
    let sink = rodio::Sink::try_new(&handle).context("could not open the audio output")?;
    let file = File::open(path).with_context(|| format!("could not open {}", path.display()))?;
    let source = rodio::Decoder::new(BufReader::new(file))?;
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}

/// 64-bit FNV-1a. Used instead of `DefaultHasher`, whose output may change
/// between Rust releases and would orphan cached files.
fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// File-system safe cache name: the readable headword, a hash of the URL (so a
/// new recording or a colliding headword never replays the wrong file) and the
/// URL's extension.
pub fn cache_file_name(headword: &str, url: &str) -> String {
    let stem: String = headword
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let stem = if stem.is_empty() {
        "word".to_string()
    } else {
        stem
    };
    let extension = url
        .rsplit('/')
        .next()
        .and_then(|name| name.split(['?', '#']).next())
        .and_then(|name| {
            name.rsplit_once('.')
                .map(|(_, ext)| ext.to_ascii_lowercase())
        })
        .filter(|ext| {
            !ext.is_empty() && ext.len() <= 4 && ext.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .unwrap_or_else(|| "mp3".to_string());
    format!("{stem}-{:08x}.{extension}", fnv1a(url) as u32)
}

#[cfg(test)]
mod tests {
    use super::cache_file_name;

    fn parts(name: &str) -> (&str, &str, &str) {
        let (rest, ext) = name.rsplit_once('.').unwrap();
        let (stem, hash) = rest.rsplit_once('-').unwrap();
        assert_eq!(hash.len(), 8, "{name}");
        (stem, hash, ext)
    }

    #[test]
    fn cache_names_are_safe() {
        let name = cache_file_name("o'clock", "https://x/sounds/oclock-us.mp3");
        assert_eq!((parts(&name).0, parts(&name).2), ("o_clock", "mp3"));
        let name = cache_file_name("ice cream", "https://x/a.ogg?v=1");
        assert_eq!((parts(&name).0, parts(&name).2), ("ice_cream", "ogg"));
        let name = cache_file_name("hello", "https://x/noext");
        assert_eq!((parts(&name).0, parts(&name).2), ("hello", "mp3"));
    }

    #[test]
    fn cache_names_depend_on_the_url() {
        let a = cache_file_name("hello", "https://x/hello-uk.mp3");
        assert_eq!(a, cache_file_name("hello", "https://x/hello-uk.mp3"));
        assert_ne!(a, cache_file_name("hello", "https://y/hello.mp3"));
        assert_ne!(
            cache_file_name("co-op", "https://x/co-op.mp3"),
            cache_file_name("co op", "https://x/co_op.mp3")
        );
    }
}
