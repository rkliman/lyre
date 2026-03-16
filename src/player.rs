use anyhow::Result;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::types::{PlayerState, Track};

pub struct Player {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    pub state: PlayerState,
    pub current_track: Option<Track>,
    pub queue: Vec<Track>,
    pub queue_index: usize,
    pub volume: f32,
    // We track elapsed manually because rodio's position is unreliable for seek
    playback_start: Option<Instant>,
    paused_elapsed: Duration,
}

impl Player {
    pub fn new() -> Result<Self> {
        let (_stream, handle) = OutputStream::try_default()?;
        Ok(Self {
            _stream,
            handle,
            sink: None,
            state: PlayerState::Stopped,
            current_track: None,
            queue: Vec::new(),
            queue_index: 0,
            volume: 1.0,
            playback_start: None,
            paused_elapsed: Duration::ZERO,
        })
    }

    pub fn play_track(&mut self, track: Track) -> Result<()> {
        // Stop existing sink
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }

        let path = Path::new(&track.path);
        if !path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", track.path));
        }

        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?;

        let sink = Sink::try_new(&self.handle)?;
        sink.set_volume(self.volume);
        sink.append(source);

        self.current_track = Some(track);
        self.sink = Some(sink);
        self.state = PlayerState::Playing;
        self.playback_start = Some(Instant::now());
        self.paused_elapsed = Duration::ZERO;

        Ok(())
    }

    pub fn toggle_pause(&mut self) {
        if let Some(sink) = &self.sink {
            if self.state == PlayerState::Playing {
                sink.pause();
                self.state = PlayerState::Paused;
                // Record how long we've been playing so far
                if let Some(start) = self.playback_start.take() {
                    self.paused_elapsed += start.elapsed();
                }
            } else if self.state == PlayerState::Paused {
                sink.play();
                self.state = PlayerState::Playing;
                self.playback_start = Some(Instant::now());
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.state = PlayerState::Stopped;
        self.current_track = None;
        self.playback_start = None;
        self.paused_elapsed = Duration::ZERO;
    }

    pub fn next(&mut self) -> Result<()> {
        if self.queue.is_empty() {
            return Ok(());
        }
        self.queue_index = (self.queue_index + 1) % self.queue.len();
        let track = self.queue[self.queue_index].clone();
        self.play_track(track)
    }

    pub fn prev(&mut self) -> Result<()> {
        if self.queue.is_empty() {
            return Ok(());
        }
        if self.queue_index == 0 {
            self.queue_index = self.queue.len() - 1;
        } else {
            self.queue_index -= 1;
        }
        let track = self.queue[self.queue_index].clone();
        self.play_track(track)
    }

    pub fn volume_up(&mut self) {
        self.volume = (self.volume + 0.05).min(1.0);
        if let Some(sink) = &self.sink {
            sink.set_volume(self.volume);
        }
    }

    pub fn volume_down(&mut self) {
        self.volume = (self.volume - 0.05).max(0.0);
        if let Some(sink) = &self.sink {
            sink.set_volume(self.volume);
        }
    }

    pub fn elapsed_secs(&self) -> i64 {
        let from_playback = self
            .playback_start
            .map(|s| s.elapsed())
            .unwrap_or(Duration::ZERO);
        (self.paused_elapsed + from_playback).as_secs() as i64
    }

    pub fn progress(&self) -> f64 {
        if let Some(track) = &self.current_track {
            if track.duration > 0 {
                return (self.elapsed_secs() as f64 / track.duration as f64).clamp(0.0, 1.0);
            }
        }
        0.0
    }

    /// Returns true if the sink has finished playing (auto-advance)
    pub fn is_finished(&self) -> bool {
        if self.state != PlayerState::Playing {
            return false;
        }
        self.sink
            .as_ref()
            .map(|s| s.empty())
            .unwrap_or(false)
    }

    pub fn set_queue(&mut self, tracks: Vec<Track>, start_index: usize) {
        self.queue = tracks;
        self.queue_index = start_index;
    }

    pub fn add_to_queue(&mut self, track: Track) {
        self.queue.push(track);
    }

    pub fn remove_from_queue(&mut self, index: usize) {
        if index < self.queue.len() {
            self.queue.remove(index);
            if self.queue_index >= self.queue.len() && !self.queue.is_empty() {
                self.queue_index = self.queue.len() - 1;
            }
        }
    }

    pub fn clear_queue(&mut self) {
        self.queue.clear();
        self.queue_index = 0;
    }
}