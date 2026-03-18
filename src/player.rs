use anyhow::Result;
use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

use crate::types::{PlayerState, Track};

/// Shared state between Player and SymphoniaSource
struct DecoderState {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    spec: SignalSpec,
    sample_buf: SampleBuffer<i16>,
    sample_pos: usize,
    seek_to: Option<Duration>,
}

impl DecoderState {
    fn new(path: &Path) -> Result<Self> {
        let file = Box::new(File::open(path)?);
        let mss = MediaSourceStream::new(file, Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let meta_opts = MetadataOptions::default();
        let fmt_opts = FormatOptions::default();

        let probed = symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;

        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| anyhow::anyhow!("No supported audio tracks"))?;

        let track_id = track.id;
        let dec_opts = DecoderOptions::default();
        let decoder = symphonia::default::get_codecs().make(&track.codec_params, &dec_opts)?;

        let spec = track.codec_params.sample_rate.as_ref()
            .and_then(|rate| track.codec_params.channels.as_ref().map(|ch| SignalSpec::new(*rate, *ch)))
            .ok_or_else(|| anyhow::anyhow!("Missing sample rate or channels"))?;

        // Create initial sample buffer with reasonable capacity
        let sample_buf = SampleBuffer::new(8192, spec);

        Ok(Self {
            format,
            decoder,
            track_id,
            spec,
            sample_buf,
            sample_pos: 0,
            seek_to: None,
        })
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        let ts = Time::from(position);
        self.format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: ts,
                track_id: Some(self.track_id),
            },
        )?;
        self.decoder.reset();
        self.sample_pos = 0;
        Ok(())
    }
}

/// Rodio source that reads from Symphonia
struct SymphoniaSource {
    state: Arc<Mutex<DecoderState>>,
}

impl SymphoniaSource {
    fn new(state: Arc<Mutex<DecoderState>>) -> Self {
        Self { state }
    }
}

impl Iterator for SymphoniaSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        let mut state = self.state.lock().unwrap();

        // Handle pending seek
        if let Some(pos) = state.seek_to.take() {
            if let Err(e) = state.seek(pos) {
                eprintln!("Seek error: {}", e);
                return None;
            }
        }

        loop {
            // If we have samples in the buffer, return the next one
            if state.sample_pos < state.sample_buf.len() {
                let sample = state.sample_buf.samples()[state.sample_pos];
                state.sample_pos += 1;
                return Some(sample);
            }

            // Need to decode the next packet
            let packet = match state.format.next_packet() {
                Ok(p) => p,
                Err(_) => return None, // End of stream
            };

            if packet.track_id() != state.track_id {
                continue;
            }

            // Decode and copy the samples
            // SAFETY: We're manually splitting borrows here. The decoder and sample_buf
            // are separate fields, so this is safe. We need this because AudioBufferRef
            // borrows from the decoder, and we need to write to sample_buf at the same time.
            unsafe {
                let state_ptr = &mut *state as *mut DecoderState;
                let decoder = &mut (*state_ptr).decoder;
                let sample_buf = &mut (*state_ptr).sample_buf;

                match decoder.decode(&packet) {
                    Ok(decoded) => {
                        sample_buf.copy_interleaved_ref(decoded);
                        (*state_ptr).sample_pos = 0;
                    }
                    Err(e) => {
                        eprintln!("Decode error: {}", e);
                        return None;
                    }
                }
            }
        }
    }
}

impl Source for SymphoniaSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        let state = self.state.lock().unwrap();
        state.spec.channels.count() as u16
    }

    fn sample_rate(&self) -> u32 {
        let state = self.state.lock().unwrap();
        state.spec.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub struct Player {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Option<Sink>,
    decoder_state: Option<Arc<Mutex<DecoderState>>>,
    pub state: PlayerState,
    pub current_track: Option<Track>,
    pub queue: Vec<Track>,
    pub queue_index: usize,
    pub volume: f32,
    // Time tracking for accurate position after seeks
    playback_start: Option<Instant>,
    paused_elapsed: Duration,
    seek_offset: Duration,
}

impl Player {
    pub fn new() -> Result<Self> {
        let (_stream, handle) = OutputStream::try_default()?;
        Ok(Self {
            _stream,
            handle,
            sink: None,
            decoder_state: None,
            state: PlayerState::Stopped,
            current_track: None,
            queue: Vec::new(),
            queue_index: 0,
            volume: 1.0,
            playback_start: None,
            paused_elapsed: Duration::ZERO,
            seek_offset: Duration::ZERO,
        })
    }

    pub fn play_track(&mut self, track: Track) -> Result<()> {
        // Stop existing sink
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.decoder_state = None;

        let path = Path::new(&track.path);
        if !path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", track.path));
        }

        let decoder_state = Arc::new(Mutex::new(DecoderState::new(path)?));
        let source = SymphoniaSource::new(decoder_state.clone());

        let sink = Sink::try_new(&self.handle)?;
        sink.set_volume(self.volume);
        sink.append(source);

        self.current_track = Some(track);
        self.sink = Some(sink);
        self.decoder_state = Some(decoder_state);
        self.state = PlayerState::Playing;
        self.playback_start = Some(Instant::now());
        self.paused_elapsed = Duration::ZERO;
        self.seek_offset = Duration::ZERO;

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
        self.decoder_state = None;
        self.state = PlayerState::Stopped;
        self.current_track = None;
        self.playback_start = None;
        self.paused_elapsed = Duration::ZERO;
        self.seek_offset = Duration::ZERO;
    }

    pub fn seek(&mut self, position: Duration) -> Result<()> {
        if let Some(ref decoder_state) = self.decoder_state {
            let mut state = decoder_state.lock().unwrap();
            state.seek_to = Some(position);

            // Update time tracking
            self.seek_offset = position;
            self.paused_elapsed = Duration::ZERO;
            self.playback_start = Some(Instant::now());

            Ok(())
        } else {
            Err(anyhow::anyhow!("No track playing"))
        }
    }

    pub fn seek_forward(&mut self, delta: Duration) -> Result<()> {
        if let Some(track) = &self.current_track {
            let current = Duration::from_secs(self.elapsed_secs() as u64);
            let max = Duration::from_secs(track.duration as u64);
            let new_pos = (current + delta).min(max);
            self.seek(new_pos)
        } else {
            Ok(())
        }
    }

    pub fn seek_backward(&mut self, delta: Duration) -> Result<()> {
        let current = Duration::from_secs(self.elapsed_secs() as u64);
        let new_pos = current.saturating_sub(delta);
        self.seek(new_pos)
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
        if self.state == PlayerState::Stopped {
            return 0;
        }

        let from_playback = self
            .playback_start
            .map(|s| s.elapsed())
            .unwrap_or(Duration::ZERO);

        let total = self.seek_offset + self.paused_elapsed + from_playback;
        total.as_secs() as i64
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
