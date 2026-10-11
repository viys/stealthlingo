//! Pronunciation playback. Audio files are downloaded once into the data
//! directory and replayed from disk afterwards, so cached words work offline.

use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use rodio::Source;

/// How long an output stays open after a clip's last sample.
///
/// `Sink::sleep_until_end` returns once that sample has been handed to the
/// audio device, not once it has been heard. Bluetooth headsets keep another
/// 150–300 ms queued past that point, and closing the output discards the
/// queue, which cuts off the end of the word.
pub const OUTPUT_TAIL: Duration = Duration::from_millis(500);

type Clip = rodio::buffer::SamplesBuffer<i16>;

/// An open audio output that clips can play on one after another.
pub struct Output {
    _stream: rodio::OutputStream,
    handle: rodio::OutputStreamHandle,
}

impl Output {
    pub fn open() -> Result<Self> {
        // The full-screen player opens this on a background thread. cpal would
        // otherwise initialize COM as a single-threaded apartment, which needs
        // a message pump that thread never runs.
        ensure_audio_com();
        let (stream, handle) =
            rodio::OutputStream::try_default().context("no audio output device is available")?;
        Ok(Self {
            _stream: stream,
            handle,
        })
    }

    /// Blocks until the clip's last sample has reached the device.
    fn play(&self, clip: Clip) -> Result<()> {
        let sink = rodio::Sink::try_new(&self.handle).context("could not open the audio output")?;
        sink.append(clip);
        sink.sleep_until_end();
        Ok(())
    }
}

#[derive(Clone)]
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

    /// Plays a pronunciation and blocks until it has been heard.
    pub fn play(&self, headword: &str, url: &str) -> Result<()> {
        let mut output = None;
        self.play_on(&mut output, headword, url)?;
        thread::sleep(OUTPUT_TAIL);
        Ok(())
    }

    /// Plays a pronunciation on `output`, opening it first if needed, and
    /// returns once the last sample has reached the device. Keep the output
    /// open for [`OUTPUT_TAIL`] afterwards so the end is heard.
    pub fn play_on(&self, output: &mut Option<Output>, headword: &str, url: &str) -> Result<()> {
        let path = self.ensure_cached(headword, url)?;
        let clip = match decode(&path) {
            Ok(clip) => clip,
            Err(err) => {
                // A corrupt download should not poison the cache.
                if err.downcast_ref::<rodio::decoder::DecoderError>().is_some() {
                    let _ = fs::remove_file(&path);
                }
                return Err(err);
            }
        };
        let open = match output.take() {
            Some(open) => open,
            None => Output::open()?,
        };
        open.play(clip)?;
        *output = Some(open);
        Ok(())
    }
}

/// Decodes the whole clip up front, which keeps decoding off the audio
/// callback; pronunciations are only a second or two long.
fn decode(path: &Path) -> Result<Clip> {
    let file = File::open(path).with_context(|| format!("could not open {}", path.display()))?;
    let decoder = rodio::Decoder::new(BufReader::new(file))?;
    let channels = decoder.channels();
    let sample_rate = decoder.sample_rate();
    if channels == 0 || sample_rate == 0 {
        bail!("the pronunciation file has no playable audio");
    }
    let samples: Vec<i16> = decoder.collect();
    if samples.is_empty() {
        bail!("the pronunciation file is empty");
    }
    Ok(Clip::new(channels, sample_rate, samples))
}

/// Makes COM safe to use from a thread that does not pump window messages.
fn ensure_audio_com() {
    #[cfg(windows)]
    {
        thread_local! {
            static STARTED: () = unsafe {
                #[link(name = "ole32")]
                extern "system" {
                    fn CoInitializeEx(reserved: *mut core::ffi::c_void, coinit: u32) -> i32;
                }
                // COINIT_MULTITHREADED.
                let _hr = CoInitializeEx(core::ptr::null_mut(), 0);
            };
        }
        STARTED.with(|_| {});
    }
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
    use std::io::Write;

    use super::{cache_file_name, decode};
    use rodio::Source;

    /// Writes a quiet mono PCM WAV.
    fn write_pcm_wav(path: &std::path::Path, sample_rate: u32, samples: &[i16]) {
        let data_len = (samples.len() * 2) as u32;
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&sample_rate.to_le_bytes()).unwrap();
        file.write_all(&(sample_rate * 2).to_le_bytes()).unwrap();
        file.write_all(&2u16.to_le_bytes()).unwrap();
        file.write_all(&16u16.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_len.to_le_bytes()).unwrap();
        for sample in samples {
            file.write_all(&sample.to_le_bytes()).unwrap();
        }
    }

    #[test]
    fn decoding_keeps_every_sample() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tone.wav");
        let sample_rate = 48_000u32;
        let clip: Vec<i16> = (0..480)
            .map(|i| if i % 2 == 0 { 1000 } else { -1000 })
            .collect();
        write_pcm_wav(&path, sample_rate, &clip);

        let decoded = decode(&path).unwrap();
        assert_eq!(decoded.channels(), 1);
        assert_eq!(decoded.sample_rate(), sample_rate);
        assert_eq!(decoded.collect::<Vec<i16>>(), clip);
    }

    #[test]
    fn empty_recordings_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.wav");
        write_pcm_wav(&path, 48_000, &[]);
        assert!(decode(&path).is_err());
    }

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
