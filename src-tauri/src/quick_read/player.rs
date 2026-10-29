//! Bounded in-memory PCM output. A dedicated thread owns the non-Send cpal stream.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

pub const PREFETCH: usize = 2;
pub const MAX_SAMPLES: usize = 24_000 * 120;
#[derive(Clone, Default, Serialize)]
pub struct Snapshot {
    pub generation: u64,
    pub phase: String,
    pub current: usize,
    pub total: usize,
    pub paused: bool,
    pub speed: f64,
    pub error: Option<String>,
}
struct Clip {
    index: usize,
    samples: Vec<i16>,
    position: f64,
}
pub struct Buffer {
    pub snapshot: Snapshot,
    queue: VecDeque<Clip>,
    pub complete: bool,
}
impl Default for Buffer {
    fn default() -> Self {
        Self {
            snapshot: Snapshot {
                phase: "idle".into(),
                speed: 1.0,
                ..Default::default()
            },
            queue: VecDeque::new(),
            complete: false,
        }
    }
}
impl Buffer {
    pub fn reset(&mut self, generation: u64) {
        let speed = self.snapshot.speed;
        *self = Self::default();
        self.snapshot.generation = generation;
        self.snapshot.speed = speed;
    }
    pub fn room(&self) -> bool {
        self.queue.len() < PREFETCH
    }
    pub fn push(&mut self, index: usize, samples: Vec<i16>) -> Result<(), String> {
        if !self.room() || samples.len() > MAX_SAMPLES {
            return Err("PCM queue limit".into());
        }
        if !samples.is_empty() {
            self.queue.push_back(Clip {
                index,
                samples,
                position: 0.0,
            });
        }
        Ok(())
    }
    fn sample(&mut self, rate: f64) -> f32 {
        if self.snapshot.paused || self.snapshot.phase == "idle" || self.snapshot.error.is_some() {
            return 0.0;
        }
        loop {
            let Some(clip) = self.queue.front_mut() else {
                if self.complete {
                    self.snapshot.phase = "idle".into();
                }
                return 0.0;
            };
            if clip.position >= clip.samples.len() as f64 {
                self.queue.pop_front();
                continue;
            }
            self.snapshot.phase = "playing".into();
            self.snapshot.current = clip.index;
            let index = clip.position as usize;
            let next = (index + 1).min(clip.samples.len() - 1);
            let fraction = (clip.position - index as f64) as f32;
            let sample = (clip.samples[index] as f32 * (1.0 - fraction)
                + clip.samples[next] as f32 * fraction)
                / 32768.0;
            clip.position += rate * self.snapshot.speed;
            return sample;
        }
    }
}

/// Lazily open output only for Quick Read; stop flag joins the owner at app exit.
pub fn run(shared: Arc<Mutex<Buffer>>, stop: Arc<AtomicBool>) {
    let mut stream = None;
    while !stop.load(Ordering::Relaxed) {
        let active = {
            let b = shared.lock().unwrap();
            b.snapshot.phase != "idle" && b.snapshot.error.is_none()
        };
        if active && stream.is_none() {
            match open(shared.clone()) {
                Ok(s) => stream = Some(s),
                Err(_) => {
                    let mut b = shared.lock().unwrap();
                    b.snapshot.error = Some(super::message(
                        "Audio output is unavailable.",
                        "Аудиоустройство недоступно.",
                    ));
                    b.queue.clear();
                }
            }
        } else if !active {
            stream = None;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn open(shared: Arc<Mutex<Buffer>>) -> Result<cpal::Stream, String> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("no output")?;
    let config = device.default_output_config().map_err(|e| e.to_string())?;
    let channels = config.channels() as usize;
    let ratio = 24000.0 / config.sample_rate() as f64;
    let errors = shared.clone();
    let on_error = move |_| {
        let mut b = errors.lock().unwrap();
        b.snapshot.error = Some(super::message(
            "Audio output stopped.",
            "Аудиоустройство отключилось.",
        ));
        b.queue.clear();
    };
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config.config(),
            move |out: &mut [f32], _| fill(out, &shared, channels, ratio, |v| v),
            on_error,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_output_stream(
            &config.config(),
            move |out: &mut [i16], _| fill(out, &shared, channels, ratio, |v| (v * 32767.0) as i16),
            on_error,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_output_stream(
            &config.config(),
            move |out: &mut [u16], _| {
                fill(out, &shared, channels, ratio, |v| {
                    ((v + 1.0) * 32767.5) as u16
                })
            },
            on_error,
            None,
        ),
        _ => return Err("unsupported output format".into()),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}
fn fill<T: Copy>(
    output: &mut [T],
    shared: &Mutex<Buffer>,
    channels: usize,
    ratio: f64,
    convert: impl Fn(f32) -> T,
) {
    let mut b = shared.lock().unwrap();
    for frame in output.chunks_mut(channels) {
        let value = convert(b.sample(ratio));
        frame.fill(value);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_pause_resume_replacement_and_order() {
        let mut b = Buffer::default();
        b.reset(1);
        b.snapshot.phase = "preparing".into();
        b.push(0, vec![100, 200]).unwrap();
        b.push(1, vec![300]).unwrap();
        assert!(b.push(2, vec![400]).is_err());
        assert_eq!(b.sample(1.0), 100.0 / 32768.0);
        b.snapshot.paused = true;
        assert_eq!(b.sample(1.0), 0.0);
        b.snapshot.paused = false;
        assert_eq!(b.sample(1.0), 200.0 / 32768.0);
        assert_eq!(b.sample(1.0), 300.0 / 32768.0);
        assert_eq!(b.snapshot.current, 1);
        b.reset(2);
        assert_eq!(b.sample(1.0), 0.0);
        assert!(b.room());
    }
    #[test]
    fn complete_drains_and_speed_preserves_pause_position() {
        let mut b = Buffer::default();
        b.snapshot.phase = "preparing".into();
        b.snapshot.speed = 2.0;
        b.complete = true;
        b.push(0, vec![100, 200, 300]).unwrap();
        assert_eq!(b.sample(1.0), 100.0 / 32768.0);
        assert_eq!(b.sample(1.0), 300.0 / 32768.0);
        assert_eq!(b.sample(1.0), 0.0);
        assert_eq!(b.snapshot.phase, "idle");
        assert!(b.push(0, vec![0; MAX_SAMPLES + 1]).is_err());
    }
}
