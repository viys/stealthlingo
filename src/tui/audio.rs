//! Plays pronunciations on a background thread so the screen stays responsive
//! while a recording downloads or plays.

use std::collections::VecDeque;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use crate::audio::AudioPlayer;

/// Recordings playing or waiting: the current one and at most one more, so
/// pressing the audio key twice compares two accents without piling up.
const MAX_PENDING: usize = 2;

pub struct AudioWorker {
    requests: Sender<(String, String)>,
    results: Receiver<Result<(), String>>,
    /// Accent labels of the recordings sent and not finished yet.
    pending: VecDeque<Option<String>>,
}

impl AudioWorker {
    pub fn new(player: AudioPlayer) -> Self {
        let (requests, inbox) = channel::<(String, String)>();
        let (outbox, results) = channel();
        thread::spawn(move || {
            for (headword, url) in inbox {
                let result = player.play(&headword, &url).map_err(|e| format!("{e:#}"));
                if outbox.send(result).is_err() {
                    break;
                }
            }
        });
        Self {
            requests,
            results,
            pending: VecDeque::new(),
        }
    }

    /// Accent label of the recording playing now, if one is.
    pub fn playing(&self) -> Option<Option<String>> {
        self.pending.front().cloned()
    }

    pub fn play(&mut self, headword: String, url: String, accent: Option<String>) {
        if self.pending.len() < MAX_PENDING && self.requests.send((headword, url)).is_ok() {
            self.pending.push_back(accent);
        }
    }

    /// Error message of a finished playback that failed.
    pub fn poll(&mut self) -> Option<String> {
        let mut error = None;
        while let Ok(result) = self.results.try_recv() {
            self.pending.pop_front();
            if let Err(err) = result {
                error = Some(err);
            }
        }
        error
    }
}
