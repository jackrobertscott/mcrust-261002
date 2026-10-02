//! Audio: an AudioQueue (AudioToolbox) output stream fed by a software mixer,
//! with all sound effects and music synthesized procedurally at startup.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use crate::math::Vec3;
use crate::noise::Random;

const RATE: f32 = 44100.0;

#[repr(C)]
struct AudioStreamBasicDescription {
    sample_rate: f64,
    format_id: u32,
    format_flags: u32,
    bytes_per_packet: u32,
    frames_per_packet: u32,
    bytes_per_frame: u32,
    channels_per_frame: u32,
    bits_per_channel: u32,
    reserved: u32,
}

#[repr(C)]
struct AudioQueueBuffer {
    capacity: u32,
    data: *mut c_void,
    size: u32,
    user: *mut c_void,
    pd_capacity: u32,
    pd: *mut c_void,
    pd_count: u32,
}

type AudioQueueRef = *mut c_void;
type Callback = extern "C" fn(*mut c_void, AudioQueueRef, *mut AudioQueueBuffer);

#[link(name = "AudioToolbox", kind = "framework")]
unsafe extern "C" {
    fn AudioQueueNewOutput(fmt: *const AudioStreamBasicDescription, cb: Callback, user: *mut c_void, rl: *const c_void, mode: *const c_void, flags: u32, out: *mut AudioQueueRef) -> i32;
    fn AudioQueueAllocateBuffer(q: AudioQueueRef, size: u32, out: *mut *mut AudioQueueBuffer) -> i32;
    fn AudioQueueEnqueueBuffer(q: AudioQueueRef, b: *mut AudioQueueBuffer, n: u32, pd: *const c_void) -> i32;
    fn AudioQueueStart(q: AudioQueueRef, t: *const c_void) -> i32;
}

struct Voice {
    samples: Arc<Vec<f32>>,
    pos: f32,
    pitch: f32,
    vol_l: f32,
    vol_r: f32,
    music: bool,
}

struct Mixer {
    voices: Vec<Voice>,
    master: f32,
    music_vol: f32,
}

extern "C" fn callback(user: *mut c_void, q: AudioQueueRef, buf: *mut AudioQueueBuffer) {
    unsafe {
        let mixer = &*(user as *const Mutex<Mixer>);
        let b = &mut *buf;
        let frames = (b.capacity / 8) as usize;
        let out = std::slice::from_raw_parts_mut(b.data as *mut f32, frames * 2);
        out.iter_mut().for_each(|s| *s = 0.0);
        if let Ok(mut m) = mixer.lock() {
            let master = m.master;
            let music_vol = m.music_vol;
            for v in m.voices.iter_mut() {
                let gain = if v.music { music_vol } else { 1.0 } * master;
                for f in 0..frames {
                    let i = v.pos as usize;
                    if i + 1 >= v.samples.len() {
                        v.pos = v.samples.len() as f32;
                        break;
                    }
                    let t = v.pos - i as f32;
                    let s = v.samples[i] * (1.0 - t) + v.samples[i + 1] * t;
                    out[f * 2] += s * v.vol_l * gain;
                    out[f * 2 + 1] += s * v.vol_r * gain;
                    v.pos += v.pitch;
                }
            }
            m.voices.retain(|v| (v.pos as usize) + 1 < v.samples.len());
        }
        for s in out.iter_mut() {
            *s = s.clamp(-1.0, 1.0);
        }
        b.size = (frames * 8) as u32;
        AudioQueueEnqueueBuffer(q, buf, 0, std::ptr::null());
    }
}

pub struct Audio {
    mixer: Option<&'static Mutex<Mixer>>,
    sounds: HashMap<&'static str, Vec<Arc<Vec<f32>>>>,
    rng: Random,
    music_timer: f32,
    pub listener: Vec3,
    pub listener_yaw: f32,
}

impl Audio {
    pub fn new() -> Audio {
        let mut a = Audio { mixer: None, sounds: HashMap::new(), rng: Random::new(99), music_timer: 30.0, listener: Vec3::ZERO, listener_yaw: 0.0 };
        a.build_sounds();
        let mixer: &'static Mutex<Mixer> = Box::leak(Box::new(Mutex::new(Mixer { voices: Vec::new(), master: 0.8, music_vol: 0.5 })));
        unsafe {
            let fmt = AudioStreamBasicDescription {
                sample_rate: RATE as f64,
                format_id: 0x6C70636D, // 'lpcm'
                format_flags: 1 | 8,   // float | packed
                bytes_per_packet: 8,
                frames_per_packet: 1,
                bytes_per_frame: 8,
                channels_per_frame: 2,
                bits_per_channel: 32,
                reserved: 0,
            };
            let mut q: AudioQueueRef = std::ptr::null_mut();
            let st = AudioQueueNewOutput(&fmt, callback, mixer as *const _ as *mut c_void, std::ptr::null(), std::ptr::null(), 0, &mut q);
            if st == 0 && !q.is_null() {
                for _ in 0..3 {
                    let mut b: *mut AudioQueueBuffer = std::ptr::null_mut();
                    if AudioQueueAllocateBuffer(q, 2048 * 8, &mut b) == 0 && !b.is_null() {
                        callback(mixer as *const _ as *mut c_void, q, b);
                    }
                }
                if AudioQueueStart(q, std::ptr::null()) == 0 {
                    a.mixer = Some(mixer);
                }
            }
            if a.mixer.is_none() {
                eprintln!("audio: could not start output (status {st}); continuing without sound");
            } else if std::env::var("MCRUST_AUDIO_DEBUG").is_ok() {
                eprintln!("audio: output started");
            }
        }
        a
    }

    pub fn set_volume(&self, master: f32, music: f32) {
        if let Some(m) = self.mixer {
            if let Ok(mut m) = m.lock() {
                m.master = master;
                m.music_vol = music;
            }
        }
    }

    /// Play a sound; `pos` = None for non-positional (UI) sounds.
    pub fn play(&mut self, name: &str, pos: Option<Vec3>, volume: f32, pitch: f32) {
        let Some(m) = self.mixer else { return };
        let Some(list) = self.sounds.get(name) else { return };
        if list.is_empty() {
            return;
        }
        let s = list[self.rng.range(list.len() as i32) as usize].clone();
        let (mut l, mut r) = (volume, volume);
        if let Some(p) = pos {
            let d = p - self.listener;
            let dist = d.len();
            let att = (1.0 - dist / 16.0).clamp(0.0, 1.0);
            if att <= 0.0 {
                return;
            }
            // pan: listener right vector for yaw (vanilla convention)
            let yr = self.listener_yaw.to_radians();
            let right = Vec3 { x: -yr.cos(), y: 0.0, z: -yr.sin() };
            let pan = if dist > 0.01 { (d.dot(right) / dist).clamp(-1.0, 1.0) } else { 0.0 };
            l = volume * att * (1.0 - pan.max(0.0) * 0.6);
            r = volume * att * (1.0 + pan.min(0.0) * 0.6);
        }
        if let Ok(mut mx) = m.lock() {
            if mx.voices.len() < 48 {
                mx.voices.push(Voice { samples: s, pos: 0.0, pitch, vol_l: l, vol_r: r, music: false });
            }
        }
    }

    fn music_playing(&self) -> bool {
        self.mixer.and_then(|m| m.lock().ok().map(|m| m.voices.iter().any(|v| v.music))).unwrap_or(false)
    }

    /// Call every frame; occasionally starts a generated piano piece.
    pub fn update_music(&mut self, dt: f32, enabled: bool) {
        if !enabled || self.mixer.is_none() {
            return;
        }
        self.music_timer -= dt;
        if self.music_timer <= 0.0 && !self.music_playing() {
            self.music_timer = 180.0 + self.rng.next_f32() * 240.0;
            let seed = self.rng.next_u64();
            let piece = Arc::new(compose_piece(seed));
            if let Some(m) = self.mixer {
                if let Ok(mut mx) = m.lock() {
                    mx.voices.push(Voice { samples: piece, pos: 0.0, pitch: 1.0, vol_l: 0.5, vol_r: 0.5, music: true });
                }
            }
        }
    }

    pub fn start_music_soon(&mut self) {
        self.music_timer = self.music_timer.min(3.0);
    }

    fn add(&mut self, name: &'static str, variants: Vec<Vec<f32>>) {
        self.sounds.insert(name, variants.into_iter().map(Arc::new).collect());
    }

    fn build_sounds(&mut self) {
        let mut r = Random::new(1234);
        // Block dig / step sounds per material
        for (mat, kind) in [("stone", 0), ("wood", 1), ("grass", 2), ("gravel", 3), ("sand", 4), ("snow", 5), ("cloth", 6)] {
            let dig: Vec<Vec<f32>> = (0..4).map(|_| material_hit(&mut r, kind, 0.22, 1.0)).collect();
            let step: Vec<Vec<f32>> = (0..4).map(|_| material_hit(&mut r, kind, 0.12, 0.45)).collect();
            let name_dig: &'static str = Box::leak(format!("dig.{mat}").into_boxed_str());
            let name_step: &'static str = Box::leak(format!("step.{mat}").into_boxed_str());
            self.add(name_dig, dig);
            self.add(name_step, step);
        }
        self.add("glass", (0..3).map(|_| glass_break(&mut r)).collect());
        self.add("pop", vec![sweep(600.0, 1300.0, 0.07, 0.35)]);
        self.add("click", vec![click()]);
        self.add("hurt", (0..2).map(|i| oof(160.0 + i as f32 * 15.0)).collect());
        self.add("explode", (0..2).map(|_| explosion(&mut r)).collect());
        self.add("eat", (0..3).map(|_| crunch(&mut r)).collect());
        self.add("burp", vec![burp()]);
        self.add("bow", vec![twang()]);
        self.add("fizz", vec![hiss(&mut r, 0.6, 0.3)]);
        self.add("fuse", vec![hiss(&mut r, 1.5, 0.35)]);
        self.add("splash", vec![splash(&mut r)]);
        self.add("pig", (0..3).map(|i| oink(220.0 + i as f32 * 20.0)).collect());
        self.add("cow", (0..2).map(|i| moo(105.0 + i as f32 * 8.0)).collect());
        self.add("sheep", (0..2).map(|i| baa(330.0 + i as f32 * 25.0)).collect());
        self.add("chicken", (0..3).map(|i| cluck(&mut r, 900.0 + i as f32 * 120.0)).collect());
        self.add("zombie", (0..3).map(|i| groan(&mut r, 85.0 + i as f32 * 10.0)).collect());
        self.add("skeleton", (0..2).map(|_| rattle(&mut r)).collect());
        self.add("spider", (0..2).map(|_| hiss(&mut r, 0.5, 0.25)).collect());
        self.add("mobhurt", (0..2).map(|i| oof(260.0 + i as f32 * 30.0)).collect());
        self.add("rain", (0..3).map(|_| rain_noise(&mut r)).collect());
        self.add("till", (0..2).map(|_| material_hit(&mut r, 3, 0.18, 0.8)).collect());
    }
}

// ---------------- synthesis helpers ----------------

fn env(i: usize, n: usize, attack: f32) -> f32 {
    let t = i as f32 / n as f32;
    let a = (t / attack.max(1e-4)).min(1.0);
    a * (1.0 - t).powf(2.0)
}

/// One-pole low-pass filter in place.
fn lowpass(s: &mut [f32], cutoff: f32) {
    let rc = 1.0 / (cutoff * std::f32::consts::TAU);
    let dt = 1.0 / RATE;
    let a = dt / (rc + dt);
    let mut y = 0.0;
    for v in s.iter_mut() {
        y += a * (*v - y);
        *v = y;
    }
}
fn highpass(s: &mut [f32], cutoff: f32) {
    let rc = 1.0 / (cutoff * std::f32::consts::TAU);
    let dt = 1.0 / RATE;
    let a = rc / (rc + dt);
    let (mut y, mut px) = (0.0, 0.0);
    for v in s.iter_mut() {
        let x = *v;
        y = a * (y + x - px);
        px = x;
        *v = y;
    }
}
/// Simple resonant band-pass (state variable filter).
fn bandpass(s: &mut [f32], freq: f32, q: f32) {
    let f = 2.0 * (std::f32::consts::PI * freq / RATE).sin();
    let (mut low, mut band) = (0.0f32, 0.0f32);
    for v in s.iter_mut() {
        let high = *v - low - band / q;
        band += f * high;
        low += f * band;
        *v = band;
    }
}
fn normalize(s: &mut [f32], peak: f32) {
    let m = s.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
    if m > 0.0 {
        for v in s.iter_mut() {
            *v *= peak / m;
        }
    }
}

fn material_hit(r: &mut Random, kind: i32, dur: f32, vol: f32) -> Vec<f32> {
    let n = (RATE * dur) as usize;
    let mut s: Vec<f32> = (0..n).map(|_| r.uniform(-1.0, 1.0)).collect();
    match kind {
        0 => {
            // stone: sharp, bright click with some body
            bandpass(&mut s, 900.0 + r.next_f32() * 500.0, 1.2);
            for (i, v) in s.iter_mut().enumerate() {
                *v *= (-(i as f32) / (RATE * 0.03)).exp();
            }
        }
        1 => {
            // wood: hollow knock
            bandpass(&mut s, 350.0 + r.next_f32() * 120.0, 6.0);
            for (i, v) in s.iter_mut().enumerate() {
                *v *= (-(i as f32) / (RATE * 0.05)).exp();
            }
        }
        2 => {
            // grass: soft rustle
            highpass(&mut s, 1500.0);
            lowpass(&mut s, 5000.0);
            for (i, v) in s.iter_mut().enumerate() {
                *v *= env(i, n, 0.15) * (1.0 + (i as f32 * 0.003).sin() * 0.3);
            }
        }
        3 => {
            // gravel/dirt: crunchy granular
            let mut g = vec![0.0; n];
            for _ in 0..40 {
                let at = r.range(n as i32 * 3 / 4) as usize;
                let amp = r.uniform(0.3, 1.0);
                for k in 0..200.min(n - at) {
                    g[at + k] += r.uniform(-1.0, 1.0) * amp * (-(k as f32) / 40.0).exp();
                }
            }
            s = g;
            lowpass(&mut s, 2500.0);
            for (i, v) in s.iter_mut().enumerate() {
                *v *= env(i, n, 0.05);
            }
        }
        4 => {
            // sand: hiss
            highpass(&mut s, 2500.0);
            for (i, v) in s.iter_mut().enumerate() {
                *v *= env(i, n, 0.2) * 0.8;
            }
        }
        5 => {
            // snow: muffled crunch
            lowpass(&mut s, 1800.0);
            for (i, v) in s.iter_mut().enumerate() {
                *v *= env(i, n, 0.2);
            }
        }
        _ => {
            // cloth
            lowpass(&mut s, 900.0);
            for (i, v) in s.iter_mut().enumerate() {
                *v *= env(i, n, 0.1);
            }
        }
    }
    normalize(&mut s, 0.5 * vol);
    s
}

fn glass_break(r: &mut Random) -> Vec<f32> {
    let n = (RATE * 0.5) as usize;
    let mut s = vec![0.0f32; n];
    for _ in 0..25 {
        let f = r.uniform(2500.0, 7000.0);
        let at = r.range(n as i32 / 3) as usize;
        let d = r.uniform(0.02, 0.15);
        for k in 0..(RATE * d) as usize {
            if at + k >= n {
                break;
            }
            let t = k as f32 / RATE;
            s[at + k] += (t * f * std::f32::consts::TAU).sin() * (-t / (d * 0.3)).exp() * 0.3;
        }
    }
    let mut noise: Vec<f32> = (0..n).map(|i| r.uniform(-1.0, 1.0) * (-(i as f32) / (RATE * 0.03)).exp()).collect();
    highpass(&mut noise, 3000.0);
    for i in 0..n {
        s[i] += noise[i] * 0.5;
    }
    normalize(&mut s, 0.45);
    s
}

fn sweep(f0: f32, f1: f32, dur: f32, vol: f32) -> Vec<f32> {
    let n = (RATE * dur) as usize;
    let mut ph = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let f = f0 + (f1 - f0) * t;
            ph += f / RATE;
            (ph * std::f32::consts::TAU).sin() * vol * (1.0 - t) * (t * 20.0).min(1.0)
        })
        .collect()
}

fn click() -> Vec<f32> {
    let n = (RATE * 0.06) as usize;
    let mut s: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            ((t * 2200.0 * std::f32::consts::TAU).sin() * 0.6 + (t * 3300.0 * std::f32::consts::TAU).sin() * 0.3) * (-t / 0.008).exp()
        })
        .collect();
    normalize(&mut s, 0.3);
    s
}

/// Vowel-ish voiced sound via a sawtooth through formant filters.
fn voiced(f0: impl Fn(f32) -> f32, dur: f32, formants: &[(f32, f32)], noise: f32, r: Option<&mut Random>) -> Vec<f32> {
    let n = (RATE * dur) as usize;
    let mut ph = 0.0f32;
    let mut rr = r;
    let mut src: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            ph += f0(t / dur) / RATE;
            let saw = 2.0 * (ph - ph.floor()) - 1.0;
            let nz = match rr.as_deref_mut() {
                Some(r) => r.uniform(-1.0, 1.0) * noise,
                None => 0.0,
            };
            saw + nz
        })
        .collect();
    let mut out = vec![0.0; n];
    for &(f, g) in formants {
        let mut b = src.clone();
        bandpass(&mut b, f, 5.0);
        for i in 0..n {
            out[i] += b[i] * g;
        }
    }
    src.clear();
    for (i, v) in out.iter_mut().enumerate() {
        *v *= env(i, n, 0.08);
    }
    normalize(&mut out, 0.5);
    out
}

fn oof(f: f32) -> Vec<f32> {
    voiced(move |t| f * (1.0 - t * 0.35), 0.22, &[(500.0, 1.0), (900.0, 0.5), (2400.0, 0.15)], 0.0, None)
}
fn oink(f: f32) -> Vec<f32> {
    voiced(move |t| f * (1.2 - t * 0.5), 0.25, &[(700.0, 1.0), (1200.0, 0.6), (2600.0, 0.2)], 0.0, None)
}
fn moo(f: f32) -> Vec<f32> {
    voiced(move |t| f * (1.0 + (t * 30.0).sin() * 0.02 - t * 0.15), 0.9, &[(350.0, 1.0), (750.0, 0.5), (2200.0, 0.1)], 0.0, None)
}
fn baa(f: f32) -> Vec<f32> {
    voiced(move |t| f * (1.0 + (t * 70.0).sin() * 0.06), 0.6, &[(800.0, 1.0), (1300.0, 0.6), (2600.0, 0.3)], 0.0, None)
}
fn groan(r: &mut Random, f: f32) -> Vec<f32> {
    voiced(move |t| f * (1.0 + (t * 9.0).sin() * 0.08 - t * 0.2), 1.0, &[(400.0, 1.0), (700.0, 0.7), (1100.0, 0.3)], 0.6, Some(r))
}
fn burp() -> Vec<f32> {
    voiced(|t| 90.0 * (1.0 + (t * 25.0).sin() * 0.1), 0.35, &[(300.0, 1.0), (600.0, 0.5)], 0.0, None)
}

fn cluck(r: &mut Random, f: f32) -> Vec<f32> {
    let n = (RATE * 0.3) as usize;
    let mut s = vec![0.0f32; n];
    let mut at = 0usize;
    for _ in 0..2 + r.range(2) {
        let len = (RATE * 0.05) as usize;
        for k in 0..len {
            if at + k >= n {
                break;
            }
            let t = k as f32 / RATE;
            s[at + k] += (t * f * (1.0 - t * 4.0) * std::f32::consts::TAU).sin() * (1.0 - k as f32 / len as f32);
        }
        at += (RATE * 0.09) as usize;
    }
    normalize(&mut s, 0.35);
    s
}

fn rattle(r: &mut Random) -> Vec<f32> {
    let n = (RATE * 0.4) as usize;
    let mut s = vec![0.0f32; n];
    for _ in 0..14 {
        let at = r.range(n as i32 - 600) as usize;
        let f = r.uniform(1200.0, 2500.0);
        for k in 0..600 {
            let t = k as f32 / RATE;
            s[at + k] += (t * f * std::f32::consts::TAU).sin() * (-t / 0.004).exp();
        }
    }
    normalize(&mut s, 0.4);
    s
}

fn hiss(r: &mut Random, dur: f32, vol: f32) -> Vec<f32> {
    let n = (RATE * dur) as usize;
    let mut s: Vec<f32> = (0..n).map(|_| r.uniform(-1.0, 1.0)).collect();
    highpass(&mut s, 3000.0);
    for (i, v) in s.iter_mut().enumerate() {
        *v *= env(i, n, 0.3);
    }
    normalize(&mut s, vol);
    s
}

fn splash(r: &mut Random) -> Vec<f32> {
    let n = (RATE * 0.6) as usize;
    let mut s: Vec<f32> = (0..n).map(|_| r.uniform(-1.0, 1.0)).collect();
    lowpass(&mut s, 3000.0);
    highpass(&mut s, 300.0);
    for (i, v) in s.iter_mut().enumerate() {
        *v *= env(i, n, 0.02);
    }
    normalize(&mut s, 0.4);
    s
}

fn explosion(r: &mut Random) -> Vec<f32> {
    let n = (RATE * 2.0) as usize;
    let mut s: Vec<f32> = (0..n).map(|_| r.uniform(-1.0, 1.0)).collect();
    lowpass(&mut s, 400.0);
    lowpass(&mut s, 600.0);
    for (i, v) in s.iter_mut().enumerate() {
        let t = i as f32 / RATE;
        *v *= (-t / 0.45).exp() * (t * 200.0).min(1.0);
    }
    normalize(&mut s, 0.9);
    s
}

fn rain_noise(r: &mut Random) -> Vec<f32> {
    let n = (RATE * 1.6) as usize;
    let mut s: Vec<f32> = (0..n).map(|_| r.uniform(-1.0, 1.0)).collect();
    lowpass(&mut s, 2500.0);
    highpass(&mut s, 400.0);
    // droplets
    for _ in 0..60 {
        let at = r.range(n as i32 - 300) as usize;
        let a = r.uniform(0.5, 1.5);
        for k in 0..300 {
            s[at + k] += r.uniform(-1.0, 1.0) * a * (-(k as f32) / 30.0).exp();
        }
    }
    // fade in/out so overlapping copies blend into a continuous wash
    for (i, v) in s.iter_mut().enumerate() {
        let t = i as f32 / n as f32;
        *v *= (t * std::f32::consts::PI).sin();
    }
    normalize(&mut s, 0.5);
    s
}

fn crunch(r: &mut Random) -> Vec<f32> {
    material_hit(r, 3, 0.15, 0.6)
}

fn twang() -> Vec<f32> {
    let n = (RATE * 0.35) as usize;
    let mut s: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / RATE;
            let f = 180.0 * (1.0 + (-t * 30.0).exp());
            ((t * f * std::f32::consts::TAU).sin() + 0.4 * (t * f * 2.0 * std::f32::consts::TAU).sin()) * (-t / 0.08).exp()
        })
        .collect();
    normalize(&mut s, 0.4);
    s
}

/// A short, calm generative piano piece (in the spirit of the vanilla soundtrack).
fn compose_piece(seed: u64) -> Vec<f32> {
    let mut r = Random::new(seed);
    // C major pentatonic-ish scale over two octaves, with chord roots
    let roots = [48, 53, 55, 57, 52, 50]; // C F G A E D (midi)
    let scale = [0, 2, 4, 7, 9, 12, 14, 16, 19, 21];
    let beat = 0.62 + r.next_f32() * 0.25;
    let bars = 10 + r.range(6) as usize;
    let total = beat * 4.0 * bars as f32 + 4.0;
    let n = (RATE * total) as usize;
    let mut out = vec![0.0f32; n];
    let add_note = |midi: i32, start: f32, vel: f32, out: &mut Vec<f32>| {
        let f = 440.0 * 2f32.powf((midi as f32 - 69.0) / 12.0);
        let st = (start * RATE) as usize;
        let dur = (RATE * 3.5) as usize;
        for k in 0..dur {
            if st + k >= out.len() {
                break;
            }
            let t = k as f32 / RATE;
            let e = (-t * 1.6).exp() * (t * 400.0).min(1.0);
            let w = t * f * std::f32::consts::TAU;
            let tone = w.sin() + 0.35 * (2.0 * w).sin() * (-t * 2.5).exp() + 0.12 * (3.0 * w).sin() * (-t * 4.0).exp();
            out[st + k] += tone * e * vel;
        }
    };
    for b in 0..bars {
        let root = roots[if b == 0 || b == bars - 1 { 0 } else { r.range(roots.len() as i32) as usize }];
        let t0 = b as f32 * beat * 4.0 + 0.5;
        // left hand: root + fifth
        add_note(root - 12, t0, 0.18, &mut out);
        add_note(root - 5, t0 + beat * 2.0, 0.12, &mut out);
        // right hand: sparse melody
        let mut t = t0;
        while t < t0 + beat * 4.0 - 0.01 {
            if r.chance(0.55) {
                let deg = scale[r.range(scale.len() as i32) as usize];
                add_note(60 + deg + if r.chance(0.2) { 12 } else { 0 }, t, 0.1 + r.next_f32() * 0.06, &mut out);
            }
            t += beat * if r.chance(0.7) { 1.0 } else { 2.0 };
        }
    }
    // gentle reverb-ish echo
    let d = (RATE * 0.23) as usize;
    for i in d..n {
        out[i] += out[i - d] * 0.25;
    }
    normalize(&mut out, 0.35);
    out
}

/// Sound material name for a block (dig/step sounds).
pub fn material(b: u8) -> &'static str {
    use crate::block::*;
    match b {
        OAK_PLANKS..=ACACIA_PLANKS | OAK_LOG..=ACACIA_LOG | CRAFTING_TABLE | CHEST | BOOKSHELF | PUMPKIN => "wood",
        GRASS | OAK_LEAVES..=ACACIA_LEAVES | SHORT_GRASS | FERN | DEAD_BUSH | DANDELION | POPPY | BLUE_ORCHID | WHEAT | SUGAR_CANE | TNT | CACTUS => "grass",
        DIRT | GRAVEL | FARMLAND | CLAY => "gravel",
        SAND => "sand",
        SNOW_LAYER | SNOW_BLOCK => "snow",
        WOOL => "cloth",
        _ => "stone",
    }
}

pub fn dig_sound(b: u8) -> &'static str {
    if b == crate::block::GLASS || b == crate::block::ICE {
        return "glass";
    }
    match material(b) {
        "wood" => "dig.wood",
        "grass" => "dig.grass",
        "gravel" => "dig.gravel",
        "sand" => "dig.sand",
        "snow" => "dig.snow",
        "cloth" => "dig.cloth",
        _ => "dig.stone",
    }
}

pub fn step_sound(b: u8) -> &'static str {
    match material(b) {
        "wood" => "step.wood",
        "grass" => "step.grass",
        "gravel" => "step.gravel",
        "sand" => "step.sand",
        "snow" => "step.snow",
        "cloth" => "step.cloth",
        _ => "step.stone",
    }
}

/// Set by UI code when a button is clicked (played by the main loop).
pub static UI_CLICK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sounds_are_valid() {
        let mut a = Audio { mixer: None, sounds: HashMap::new(), rng: Random::new(1), music_timer: 0.0, listener: Vec3::ZERO, listener_yaw: 0.0 };
        a.build_sounds();
        assert!(a.sounds.len() > 20);
        for (name, vs) in &a.sounds {
            for v in vs {
                assert!(!v.is_empty(), "{name} empty");
                assert!(v.iter().all(|s| s.is_finite()), "{name} has NaN");
                let peak = v.iter().fold(0.0f32, |m, s| m.max(s.abs()));
                assert!(peak > 0.01 && peak <= 1.0, "{name} peak {peak}");
            }
        }
        let m = compose_piece(5);
        assert!(m.iter().all(|s| s.is_finite()) && m.len() > 44100 * 10);
    }
}
