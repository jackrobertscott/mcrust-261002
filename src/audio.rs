//! Audio: an AudioQueue (AudioToolbox) output stream fed by a software mixer,
//! with all sound effects and music synthesized procedurally at startup.
//!
//! Sounds are built from a small DSP kit: biquad filters, band-limited voice
//! sources pushed through formant filters, struck-resonator "modes" and
//! filtered noise grains. Every sound is loudness-matched with a
//! frequency-weighted meter, so bright sounds come out no louder than dull
//! ones, and the mixer ends in a smooth peak limiter instead of a hard clip.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::math::Vec3;
use crate::noise::Random;

const RATE: f32 = 44100.0;
const TAU: f32 = std::f32::consts::TAU;

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
    /// Samples are interleaved L/R frames (music) rather than mono.
    stereo: bool,
}

impl Voice {
    fn frames(&self) -> usize {
        if self.stereo { self.samples.len() / 2 } else { self.samples.len() }
    }
}

struct Mixer {
    voices: Vec<Voice>,
    master: f32,
    music_vol: f32,
    /// Current limiter gain (1 = no reduction).
    limiter: f32,
}

/// Limiter ceiling (about -1 dBFS) and release time.
const LIMIT: f32 = 0.89;
const LIMIT_RELEASE: f32 = 0.25;

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
                let n = v.frames();
                let (gl, gr) = (v.vol_l * gain, v.vol_r * gain);
                for f in 0..frames {
                    let i = v.pos as usize;
                    if i + 1 >= n {
                        v.pos = n as f32;
                        break;
                    }
                    let t = v.pos - i as f32;
                    if v.stereo {
                        let s = &v.samples;
                        out[f * 2] += (s[i * 2] * (1.0 - t) + s[i * 2 + 2] * t) * gl;
                        out[f * 2 + 1] += (s[i * 2 + 1] * (1.0 - t) + s[i * 2 + 3] * t) * gr;
                    } else {
                        let s = v.samples[i] * (1.0 - t) + v.samples[i + 1] * t;
                        out[f * 2] += s * gl;
                        out[f * 2 + 1] += s * gr;
                    }
                    v.pos += v.pitch;
                }
            }
            m.voices.retain(|v| (v.pos as usize) + 1 < v.frames());
            // Peak limiter: instant attack, smooth release, so loud moments
            // (explosions, many sounds at once) duck instead of distorting.
            let rel = 1.0 - (-1.0 / (LIMIT_RELEASE * RATE)).exp();
            let mut g = m.limiter;
            for f in 0..frames {
                let peak = out[f * 2].abs().max(out[f * 2 + 1].abs());
                let target = if peak > LIMIT { LIMIT / peak } else { 1.0 };
                if target < g {
                    g = target;
                } else {
                    g += (target - g) * rel;
                }
                out[f * 2] *= g;
                out[f * 2 + 1] *= g;
            }
            m.limiter = g;
        }
        for s in out.iter_mut() {
            *s = s.clamp(-1.0, 1.0);
        }
        b.size = (frames * 8) as u32;
        AudioQueueEnqueueBuffer(q, buf, 0, std::ptr::null());
    }
}

/// Set while a music piece is being composed on a background thread.
static MUSIC_PENDING: AtomicBool = AtomicBool::new(false);

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
        let t0 = std::time::Instant::now();
        a.build_sounds();
        let build_ms = t0.elapsed().as_millis();
        let mixer: &'static Mutex<Mixer> = Box::leak(Box::new(Mutex::new(Mixer { voices: Vec::new(), master: 0.8, music_vol: 0.5, limiter: 1.0 })));
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
                eprintln!("audio: output started ({} sounds synthesized in {build_ms} ms)", a.sounds.len());
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
            let att = (1.0 - dist / (16.0 * volume.max(1.0))).clamp(0.0, 1.0);
            if att <= 0.0 {
                return;
            }
            // pan: listener right vector for yaw (vanilla convention); equal-power law
            let yr = self.listener_yaw.to_radians();
            let right = Vec3 { x: -yr.cos(), y: 0.0, z: -yr.sin() };
            let pan = if dist > 0.01 { (d.dot(right) / dist).clamp(-1.0, 1.0) } else { 0.0 } * 0.7;
            let a = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
            let g = volume * att * std::f32::consts::SQRT_2;
            l = g * a.cos();
            r = g * a.sin();
        }
        if let Ok(mut mx) = m.lock() {
            if mx.voices.len() < 48 {
                mx.voices.push(Voice { samples: s, pos: 0.0, pitch, vol_l: l, vol_r: r, music: false, stereo: false });
            }
        }
    }

    fn music_playing(&self) -> bool {
        MUSIC_PENDING.load(Ordering::Relaxed) || self.mixer.and_then(|m| m.lock().ok().map(|m| m.voices.iter().any(|v| v.music))).unwrap_or(false)
    }

    /// Call every frame; occasionally composes and starts a generated piano piece.
    pub fn update_music(&mut self, dt: f32, enabled: bool) {
        if !enabled || self.mixer.is_none() {
            return;
        }
        self.music_timer -= dt;
        if self.music_timer <= 0.0 && !self.music_playing() {
            self.music_timer = 180.0 + self.rng.next_f32() * 240.0;
            let seed = self.rng.next_u64();
            let Some(mixer) = self.mixer else { return };
            // composing takes a moment, so keep it off the render thread
            MUSIC_PENDING.store(true, Ordering::Relaxed);
            std::thread::spawn(move || {
                let piece = Arc::new(compose_piece(seed));
                if let Ok(mut mx) = mixer.lock() {
                    mx.voices.push(Voice { samples: piece, pos: 0.0, pitch: 1.0, vol_l: 0.6, vol_r: 0.6, music: true, stereo: true });
                }
                MUSIC_PENDING.store(false, Ordering::Relaxed);
            });
        }
    }

    pub fn start_music_soon(&mut self) {
        self.music_timer = self.music_timer.min(3.0);
    }

    fn build_sounds(&mut self) {
        for (name, variants) in synth_all() {
            self.sounds.insert(name, variants.into_iter().map(Arc::new).collect());
        }
    }
}

// ---------------- sound table ----------------

type Gen = fn(&mut Random, usize) -> Vec<f32>;

/// Every sound: name, number of variants, generator. Each variant gets its own
/// seeded RNG, so generation can be spread over threads deterministically.
fn sound_table() -> Vec<(&'static str, usize, Gen)> {
    vec![
        ("dig.stone", 4, |r, _| stone(r, 1.0)),
        ("step.stone", 6, |r, _| stone(r, 0.4)),
        ("dig.metal", 3, |r, _| metal(r, 1.0)),
        ("step.metal", 4, |r, _| metal(r, 0.4)),
        ("dig.wood", 4, |r, _| wood(r, 1.0)),
        ("step.wood", 6, |r, _| wood(r, 0.4)),
        ("dig.grass", 4, |r, _| grass(r, 1.0)),
        ("step.grass", 6, |r, _| grass(r, 0.4)),
        ("dig.gravel", 4, |r, _| gravel(r, 1.0)),
        ("step.gravel", 6, |r, _| gravel(r, 0.4)),
        ("dig.sand", 4, |r, _| sand(r, 1.0)),
        ("step.sand", 6, |r, _| sand(r, 0.4)),
        ("dig.snow", 4, |r, _| snow(r, 1.0)),
        ("step.snow", 6, |r, _| snow(r, 0.4)),
        ("dig.cloth", 4, |r, _| cloth(r, 1.0)),
        ("step.cloth", 6, |r, _| cloth(r, 0.4)),
        ("glass", 3, |r, _| glass_break(r)),
        ("pop", 1, |r, _| pop(r)),
        ("click", 1, |r, _| ui_click(r)),
        ("hurt", 3, |r, i| player_hurt(r, i)),
        ("explode", 3, |r, _| explosion(r)),
        ("eat", 3, |r, _| chomp(r)),
        ("burp", 1, |r, _| burp(r)),
        ("bow", 2, |r, _| bow(r)),
        ("arrow.hit", 3, |r, _| arrow_hit(r)),
        ("fizz", 2, |r, _| sizzle(r, 0.7, false)),
        ("fuse", 1, |r, _| sizzle(r, 1.6, true)),
        ("splash", 2, |r, _| splash(r)),
        ("rain", 4, |r, _| rain(r)),
        ("till", 3, |r, _| gravel(r, 0.7)),
        ("door.open", 2, |r, _| door(r, true)),
        ("door.close", 2, |r, _| door(r, false)),
        ("chest.open", 1, |r, _| chest(r, true)),
        ("chest.close", 1, |r, _| chest(r, false)),
        ("bucket.fill", 2, |r, _| bucket(r, true, false)),
        ("bucket.empty", 2, |r, _| bucket(r, false, false)),
        ("bucket.fill_lava", 1, |r, _| bucket(r, true, true)),
        ("bucket.empty_lava", 1, |r, _| bucket(r, false, true)),
        ("shear", 2, |r, _| shear(r)),
        ("pig", 3, |r, i| pig_say(r, i)),
        ("pig.hurt", 2, |r, i| pig_hurt(r, i)),
        ("cow", 3, |r, i| cow_say(r, i)),
        ("cow.hurt", 2, |r, i| cow_hurt(r, i)),
        ("sheep", 3, |r, i| sheep_say(r, i)),
        ("sheep.hurt", 2, |r, i| sheep_hurt(r, i)),
        ("chicken", 3, |r, i| chicken_say(r, i)),
        ("chicken.hurt", 2, |r, i| chicken_hurt(r, i)),
        ("zombie", 3, |r, i| zombie_say(r, i)),
        ("zombie.hurt", 2, |r, i| zombie_hurt(r, i)),
        ("skeleton", 3, |r, _| skeleton_say(r)),
        ("skeleton.hurt", 2, |r, _| skeleton_hurt(r)),
        ("spider", 3, |r, _| spider_say(r)),
        ("spider.hurt", 2, |r, _| spider_hurt(r)),
        ("creeper.hurt", 2, |r, _| creeper_hurt(r)),
    ]
}

fn seed_for(name: &str, i: usize) -> u64 {
    name.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3)) ^ (i as u64).wrapping_mul(0x9E3779B97F4A7C15)
}

/// Synthesize every sound, spread across a few threads.
fn synth_all() -> Vec<(&'static str, Vec<Vec<f32>>)> {
    let table = sound_table();
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 8);
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let table = &table;
                s.spawn(move || {
                    table
                        .iter()
                        .skip(t)
                        .step_by(threads)
                        .map(|&(name, count, g)| (name, (0..count).map(|i| g(&mut Random::new(seed_for(name, i)), i)).collect()))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
    })
}

// ---------------- DSP kit ----------------

fn secs(t: f32) -> usize {
    (t.max(0.0) * RATE) as usize
}

/// RBJ biquad filter (transposed direct form II).
#[derive(Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn from(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Biquad {
        Biquad { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: a1 / a0, a2: a2 / a0, z1: 0.0, z2: 0.0 }
    }
    fn sc(f: f32) -> (f32, f32) {
        (TAU * f.clamp(10.0, RATE * 0.45) / RATE).sin_cos()
    }
    fn lowpass(f: f32, q: f32) -> Biquad {
        let (s, c) = Self::sc(f);
        let a = s / (2.0 * q);
        Self::from((1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0, 1.0 + a, -2.0 * c, 1.0 - a)
    }
    fn highpass(f: f32, q: f32) -> Biquad {
        let (s, c) = Self::sc(f);
        let a = s / (2.0 * q);
        Self::from((1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0, 1.0 + a, -2.0 * c, 1.0 - a)
    }
    /// Band-pass with 0 dB gain at the centre frequency.
    fn bandpass(f: f32, q: f32) -> Biquad {
        let (s, c) = Self::sc(f);
        let a = s / (2.0 * q);
        Self::from(a, 0.0, -a, 1.0 + a, -2.0 * c, 1.0 - a)
    }
    fn high_shelf(f: f32, db: f32) -> Biquad {
        let (s, c) = Self::sc(f);
        let a = 10f32.powf(db / 40.0);
        let sa = 2.0 * a.sqrt() * (s / 2.0 * std::f32::consts::SQRT_2);
        Self::from(
            a * ((a + 1.0) + (a - 1.0) * c + sa),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * c),
            a * ((a + 1.0) + (a - 1.0) * c - sa),
            (a + 1.0) - (a - 1.0) * c + sa,
            2.0 * ((a - 1.0) - (a + 1.0) * c),
            (a + 1.0) - (a - 1.0) * c - sa,
        )
    }
    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
    fn run(mut self, s: &mut [f32]) {
        for v in s.iter_mut() {
            *v = self.tick(*v);
        }
    }
    /// Change coefficients but keep the filter state (time-varying filters).
    fn retune(&mut self, to: Biquad) {
        let (z1, z2) = (self.z1, self.z2);
        *self = to;
        self.z1 = z1;
        self.z2 = z2;
    }
}

fn lp(s: &mut [f32], f: f32) {
    Biquad::lowpass(f, std::f32::consts::FRAC_1_SQRT_2).run(s)
}
fn hp(s: &mut [f32], f: f32) {
    Biquad::highpass(f, std::f32::consts::FRAC_1_SQRT_2).run(s)
}
fn bp(s: &mut [f32], f: f32, q: f32) {
    Biquad::bandpass(f, q).run(s)
}

fn white(r: &mut Random, n: usize) -> Vec<f32> {
    (0..n).map(|_| r.uniform(-1.0, 1.0)).collect()
}

/// Pink (1/f) noise, Paul Kellet's filter.
fn pink(r: &mut Random, n: usize) -> Vec<f32> {
    let mut b = [0.0f32; 7];
    (0..n)
        .map(|_| {
            let w = r.uniform(-1.0, 1.0);
            b[0] = 0.99886 * b[0] + w * 0.0555179;
            b[1] = 0.99332 * b[1] + w * 0.0750759;
            b[2] = 0.969 * b[2] + w * 0.153852;
            b[3] = 0.86650 * b[3] + w * 0.3104856;
            b[4] = 0.55000 * b[4] + w * 0.5329522;
            b[5] = -0.7616 * b[5] - w * 0.0168980;
            let y = b[0] + b[1] + b[2] + b[3] + b[4] + b[5] + b[6] + w * 0.5362;
            b[6] = w * 0.115926;
            y * 0.2
        })
        .collect()
}

/// Brown (1/f^2) noise: deep rumble.
fn brown(r: &mut Random, n: usize) -> Vec<f32> {
    let mut y = 0.0f32;
    (0..n)
        .map(|_| {
            y = (y + 0.02 * r.uniform(-1.0, 1.0)) / 1.02;
            y * 3.5
        })
        .collect()
}

/// Linear attack then exponential decay.
fn ad(t: f32, attack: f32, decay: f32) -> f32 {
    if t < attack { t / attack } else { (-(t - attack) / decay).exp() }
}

/// Smooth attack / sustain / release over a sound of length `dur`.
fn asr(t: f32, dur: f32, attack: f32, release: f32) -> f32 {
    let a = (t / attack).min(1.0);
    let r = ((dur - t) / release).clamp(0.0, 1.0);
    let s = |x: f32| x * x * (3.0 - 2.0 * x);
    s(a) * s(r)
}

/// Add `src * gain` into `out` starting at sample `at`.
fn mix(out: &mut [f32], src: &[f32], at: usize, gain: f32) {
    for (k, &v) in src.iter().enumerate() {
        if let Some(o) = out.get_mut(at + k) {
            *o += v * gain;
        }
    }
}

/// A short burst of band-passed noise: the building block of crunches,
/// rustles and crackles.
fn grain(out: &mut [f32], seed: u64, at: f32, len: f32, freq: f32, q: f32, amp: f32) {
    let mut r = Random::new(seed);
    let st = secs(at);
    let n = secs(len).max(16);
    let mut f = Biquad::bandpass(freq, q);
    let attack = (len * 0.1).max(0.0004);
    for k in 0..n {
        let Some(o) = out.get_mut(st + k) else { break };
        let t = k as f32 / RATE;
        *o += f.tick(r.uniform(-1.0, 1.0)) * ad(t, attack, len * 0.3) * amp;
    }
}

/// Exponentially decaying sinusoids (freq Hz, decay seconds, amplitude):
/// a struck resonant object such as a plank, a bone or a glass shard.
fn modes(out: &mut [f32], at: f32, ms: &[(f32, f32, f32)]) {
    let st = secs(at);
    let attack = secs(0.0006).max(1) as f32;
    for &(f, decay, amp) in ms {
        if f >= RATE * 0.45 {
            continue;
        }
        let (s, c) = (TAU * f / RATE).sin_cos();
        let k_dec = (-1.0 / (decay * RATE)).exp();
        let (mut x, mut y, mut e) = (1.0f32, 0.0f32, amp);
        for k in 0..secs(decay * 7.0) {
            let Some(o) = out.get_mut(st + k) else { break };
            *o += y * e * (k as f32 / attack).min(1.0);
            let nx = x * c - y * s;
            y = x * s + y * c;
            x = nx;
            e *= k_dec;
        }
    }
}

/// Band-limited sawtooth correction (polyBLEP).
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

/// Voice parameters: pitch wobble, breathiness, brightness and roughness.
#[derive(Clone, Copy)]
struct Vp {
    /// Random pitch wander, as a fraction of f0.
    jitter: f32,
    /// Aspiration noise mixed into the source (shaped by the formants too).
    breath: f32,
    /// Source low-pass (Hz): lower is a softer, darker voice.
    tilt: f32,
    /// Period-to-period amplitude irregularity: growl / vocal fry.
    rough: f32,
}

/// Formant voice synthesis: a band-limited glottal source through parallel
/// formant band-passes. Formants are (closed Hz, open Hz, Q, gain) and glide
/// between closed and open following `open(t)`; all contours take t in 0..1.
fn voice(r: &mut Random, dur: f32, f0: impl Fn(f32) -> f32, amp: impl Fn(f32) -> f32, open: impl Fn(f32) -> f32, formants: &[(f32, f32, f32, f32)], p: Vp) -> Vec<f32> {
    let n = secs(dur);
    let mut out = vec![0.0f32; n];
    let mut filters: Vec<Biquad> = formants.iter().map(|&(f, _, q, _)| Biquad::bandpass(f, q)).collect();
    let (mut ph, mut jit, mut jit_to, mut pamp, mut odd) = (0.0f32, 0.0f32, 0.0f32, 1.0f32, false);
    let mut src_lp = Biquad::lowpass(p.tilt, 0.6);
    for (i, o) in out.iter_mut().enumerate() {
        let t = i as f32 / n as f32;
        if i % 32 == 0 {
            let op = open(t).clamp(0.0, 1.0);
            for (fl, &(fc, fo, q, _)) in filters.iter_mut().zip(formants) {
                fl.retune(Biquad::bandpass(fc + (fo - fc) * op, q));
            }
            jit_to = r.uniform(-1.0, 1.0);
        }
        jit += (jit_to - jit) * 0.003;
        let f = (f0(t) * (1.0 + jit * p.jitter)).max(20.0);
        let dt = f / RATE;
        ph += dt;
        if ph >= 1.0 {
            ph -= 1.0;
            odd = !odd;
            pamp = 1.0 - p.rough * if odd { 0.4 + 0.6 * r.next_f32() } else { 0.25 * r.next_f32() };
        }
        let saw = 2.0 * ph - 1.0 - poly_blep(ph, dt);
        let src = src_lp.tick(saw * pamp) + r.uniform(-1.0, 1.0) * p.breath;
        let mut y = 0.0;
        for (fl, fm) in filters.iter_mut().zip(formants) {
            y += fl.tick(src) * fm.3;
        }
        *o = y * amp(t);
    }
    out
}

/// Frequency-weighted short-term loudness: K-weighting (high shelf + high
/// pass, as in broadcast loudness meters), then the loudest 30 ms window.
/// The ear is most sensitive around 2-5 kHz, so bright sounds measure louder
/// and get turned down more.
fn loudness(s: &[f32]) -> f32 {
    let mut k = s.to_vec();
    Biquad::high_shelf(1500.0, 4.0).run(&mut k);
    Biquad::highpass(60.0, 0.5).run(&mut k);
    let win = secs(0.03).min(k.len()).max(1);
    let mut acc: f64 = k[..win].iter().map(|v| (*v as f64).powi(2)).sum();
    let mut best = acc;
    for i in win..k.len() {
        acc += (k[i] as f64).powi(2) - (k[i - win] as f64).powi(2);
        best = best.max(acc);
    }
    ((best.max(0.0) / win as f64).sqrt()) as f32
}

/// Final pass for every effect: remove DC, soften the extreme top end, fade the
/// edges so nothing clicks, and set the loudness to `db` (dBFS, weighted).
fn finish(mut s: Vec<f32>, db: f32) -> Vec<f32> {
    if s.is_empty() {
        return s;
    }
    hp(&mut s, 30.0);
    lp(&mut s, 11000.0);
    let n = s.len();
    let fi = 24.min(n);
    for (i, v) in s[..fi].iter_mut().enumerate() {
        *v *= i as f32 / fi as f32;
    }
    let fo = secs(0.015).min(n);
    for i in 0..fo {
        let x = i as f32 / fo as f32;
        s[n - 1 - i] *= x * x * (3.0 - 2.0 * x);
    }
    let l = loudness(&s);
    if l > 1e-9 {
        let g = 10f32.powf(db / 20.0) / l;
        s.iter_mut().for_each(|v| *v *= g);
    }
    let peak = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    if peak > 0.9 {
        s.iter_mut().for_each(|v| *v *= 0.9 / peak);
    }
    s
}

// ---------------- block materials ----------------
// `s` is strength: 1.0 for breaking/placing, ~0.4 for footsteps.

fn stone(r: &mut Random, s: f32) -> Vec<f32> {
    let dur = 0.1 + 0.2 * s;
    let mut out = vec![0.0; secs(dur + 0.05)];
    // dull body of the impact
    let mut thump = white(r, secs(0.05));
    lp(&mut thump, 320.0);
    lp(&mut thump, 320.0);
    for (k, v) in thump.iter_mut().enumerate() {
        *v *= ad(k as f32 / RATE, 0.001, 0.011);
    }
    mix(&mut out, &thump, 0, 2.2 * s.sqrt());
    // gritty crunch: scattered rock grains, densest at the start
    let count = (5.0 + 14.0 * s) as usize;
    for _ in 0..count {
        let at = r.next_f32().powf(1.8) * dur * 0.65;
        let fade = 1.0 - at / dur;
        grain(&mut out, r.next_u64(), at, r.uniform(0.005, 0.02), r.uniform(800.0, 3000.0), r.uniform(1.2, 2.5), r.uniform(0.4, 1.0) * fade);
    }
    // a little rocky click resonance
    let f = r.uniform(600.0, 900.0);
    modes(&mut out, 0.0, &[(f, 0.012, 0.25), (f * 2.13, 0.007, 0.12)]);
    lp(&mut out, 6500.0);
    finish(out, if s > 0.7 { -15.0 } else { -17.0 })
}

fn metal(r: &mut Random, s: f32) -> Vec<f32> {
    // a stone-like impact with a short inharmonic metallic ring
    let mut out = stone(r, s);
    out.resize(secs(0.45), 0.0);
    let f = r.uniform(900.0, 1300.0);
    modes(&mut out, 0.0, &[(f, 0.09, 0.05), (f * 2.76, 0.05, 0.03), (f * 5.40, 0.03, 0.015), (f * 1.51, 0.07, 0.03)]);
    lp(&mut out, 7000.0);
    finish(out, if s > 0.7 { -15.0 } else { -17.0 })
}

fn wood(r: &mut Random, s: f32) -> Vec<f32> {
    let dur = 0.12 + 0.18 * s;
    let mut out = vec![0.0; secs(dur + 0.05)];
    // hollow knock: plank modes excited by a short noise tap
    let f = r.uniform(150.0, 230.0);
    modes(&mut out, 0.0, &[(f, 0.05, 1.0), (f * 2.41, 0.03, 0.55), (f * 3.93, 0.018, 0.3), (f * 5.62, 0.01, 0.15)]);
    let mut tap = white(r, secs(0.004));
    bp(&mut tap, 1300.0, 0.8);
    mix(&mut out, &tap, 0, 0.5);
    // splintery detail when breaking
    for _ in 0..(1.0 + 6.0 * s) as usize {
        grain(&mut out, r.next_u64(), r.next_f32() * dur * 0.5, r.uniform(0.004, 0.012), r.uniform(1200.0, 2800.0), 1.8, r.uniform(0.04, 0.12) * s);
    }
    lp(&mut out, 6000.0);
    finish(out, if s > 0.7 { -15.0 } else { -17.0 })
}

fn grass(r: &mut Random, s: f32) -> Vec<f32> {
    let dur = 0.12 + 0.2 * s;
    let n = secs(dur);
    // soft swish under many tiny leaf-rustle grains
    let mut out = pink(r, n);
    bp(&mut out, 1800.0, 0.6);
    for (k, v) in out.iter_mut().enumerate() {
        let t = k as f32 / RATE;
        *v *= asr(t, dur, 0.012, dur * 0.8) * 0.5;
    }
    for _ in 0..(10.0 + 30.0 * s) as usize {
        let at = r.next_f32().powf(1.3) * dur * 0.85;
        grain(&mut out, r.next_u64(), at, r.uniform(0.003, 0.01), r.uniform(1500.0, 4500.0), 1.0, r.uniform(0.1, 0.5) * (1.0 - at / dur));
    }
    lp(&mut out, 6000.0);
    finish(out, if s > 0.7 { -17.0 } else { -19.0 })
}

fn gravel(r: &mut Random, s: f32) -> Vec<f32> {
    let dur = 0.12 + 0.2 * s;
    let mut out = vec![0.0; secs(dur + 0.03)];
    for _ in 0..(18.0 + 45.0 * s) as usize {
        let at = r.next_f32().powf(1.5) * dur * 0.85;
        grain(&mut out, r.next_u64(), at, r.uniform(0.004, 0.014), r.uniform(350.0, 2200.0), r.uniform(1.2, 2.5), r.uniform(0.3, 1.0) * (1.0 - at / dur).sqrt());
    }
    let mut thump = white(r, secs(0.05));
    lp(&mut thump, 260.0);
    lp(&mut thump, 260.0);
    for (k, v) in thump.iter_mut().enumerate() {
        *v *= ad(k as f32 / RATE, 0.002, 0.016);
    }
    mix(&mut out, &thump, 0, 1.6 * s);
    lp(&mut out, 5000.0);
    finish(out, if s > 0.7 { -15.0 } else { -17.0 })
}

fn sand(r: &mut Random, s: f32) -> Vec<f32> {
    let dur = 0.14 + 0.22 * s;
    let n = secs(dur);
    let mut a = pink(r, n);
    bp(&mut a, 1600.0, 0.5);
    let mut b = pink(r, n);
    bp(&mut b, 3600.0, 0.9);
    // granular flutter: slowly varying random amplitude
    let (mut g, mut g_to) = (0.5f32, 0.5f32);
    let mut out = vec![0.0; n];
    for k in 0..n {
        if k % 220 == 0 {
            g_to = r.uniform(0.3, 1.0);
        }
        g += (g_to - g) * 0.02;
        let t = k as f32 / RATE;
        out[k] = (a[k] + b[k] * 0.3) * g * asr(t, dur, 0.02, dur * 0.75);
    }
    lp(&mut out, 5000.0);
    finish(out, if s > 0.7 { -18.0 } else { -20.0 })
}

fn snow(r: &mut Random, s: f32) -> Vec<f32> {
    let dur = 0.14 + 0.2 * s;
    let n = secs(dur);
    let mut out = pink(r, n);
    lp(&mut out, 1200.0);
    for (k, v) in out.iter_mut().enumerate() {
        *v *= asr(k as f32 / RATE, dur, 0.02, dur * 0.7) * 0.6;
    }
    // compacting crunch
    for _ in 0..(14.0 + 30.0 * s) as usize {
        let at = r.next_f32() * dur * 0.8;
        grain(&mut out, r.next_u64(), at, r.uniform(0.008, 0.02), r.uniform(600.0, 1800.0), 2.0, r.uniform(0.2, 0.6));
    }
    lp(&mut out, 4000.0);
    finish(out, if s > 0.7 { -17.0 } else { -19.0 })
}

fn cloth(r: &mut Random, s: f32) -> Vec<f32> {
    let dur = 0.14 + 0.16 * s;
    let n = secs(dur);
    let mut out = pink(r, n);
    lp(&mut out, 900.0);
    bp(&mut out, 450.0, 0.5);
    for (k, v) in out.iter_mut().enumerate() {
        *v *= asr(k as f32 / RATE, dur, 0.02, dur * 0.8);
    }
    finish(out, if s > 0.7 { -18.0 } else { -20.0 })
}

fn glass_break(r: &mut Random) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.7)];
    // the crack
    let mut crack = white(r, secs(0.006));
    bp(&mut crack, 2400.0, 0.8);
    mix(&mut out, &crack, 0, 1.0);
    let mut body = white(r, secs(0.05));
    lp(&mut body, 600.0);
    for (k, v) in body.iter_mut().enumerate() {
        *v *= ad(k as f32 / RATE, 0.001, 0.012);
    }
    mix(&mut out, &body, 0, 1.2);
    // falling shards: small inharmonic tinkles
    for _ in 0..(16 + r.range(10)) {
        let at = r.next_f32().powf(1.8) * 0.4;
        let f = r.uniform(1800.0, 4800.0);
        let d = r.uniform(0.02, 0.06);
        let a = r.uniform(0.08, 0.3) * (1.0 - at / 0.45);
        modes(&mut out, at, &[(f, d, a), (f * r.uniform(1.4, 1.6), d * 0.7, a * 0.5), (f * r.uniform(2.0, 2.3), d * 0.5, a * 0.25)]);
    }
    for _ in 0..8 {
        grain(&mut out, r.next_u64(), r.next_f32() * 0.2, 0.004, r.uniform(3000.0, 6000.0), 1.0, 0.15);
    }
    lp(&mut out, 8500.0);
    finish(out, -18.0)
}

// ---------------- interface and items ----------------

/// Item pickup: a soft rising "bloop".
fn pop(_r: &mut Random) -> Vec<f32> {
    let n = secs(0.1);
    let mut ph = 0.0f32;
    let out = (0..n)
        .map(|k| {
            let t = k as f32 / RATE;
            let f = 340.0 + 620.0 * (1.0 - (-t / 0.014).exp());
            ph += f / RATE;
            let w = ph * TAU;
            (w.sin() + 0.12 * (2.0 * w).sin()) * ad(t, 0.002, 0.024)
        })
        .collect();
    finish(out, -20.0)
}

/// Menu button: a short, rounded mechanical tick.
fn ui_click(r: &mut Random) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.07)];
    modes(&mut out, 0.0, &[(1500.0, 0.006, 0.5), (2600.0, 0.004, 0.25), (620.0, 0.012, 0.35)]);
    let mut tap = white(r, secs(0.0015));
    bp(&mut tap, 2500.0, 1.0);
    mix(&mut out, &tap, 0, 0.3);
    lp(&mut out, 6000.0);
    finish(out, -21.0)
}

fn player_hurt(r: &mut Random, i: usize) -> Vec<f32> {
    let f = 175.0 + i as f32 * 12.0;
    let dur = 0.24;
    let out = voice(
        r,
        dur,
        |t| f * (1.08 - 0.32 * t),
        |t| asr(t * dur, dur, 0.012, 0.14),
        |t| 1.0 - t * 0.5,
        &[(450.0, 620.0, 5.0, 1.0), (900.0, 1150.0, 6.0, 0.5), (2300.0, 2500.0, 8.0, 0.15)],
        Vp { jitter: 0.015, breath: 0.12, tilt: 2000.0, rough: 0.15 },
    );
    finish(out, -16.0)
}

fn chomp(r: &mut Random) -> Vec<f32> {
    let dur = 0.16;
    let mut out = vec![0.0; secs(dur + 0.04)];
    for _ in 0..18 {
        let at = r.next_f32().powf(1.4) * dur * 0.7;
        grain(&mut out, r.next_u64(), at, r.uniform(0.004, 0.016), r.uniform(500.0, 2200.0), 1.6, r.uniform(0.3, 1.0) * (1.0 - at / dur));
    }
    let mut thump = white(r, secs(0.04));
    lp(&mut thump, 300.0);
    lp(&mut thump, 300.0);
    for (k, v) in thump.iter_mut().enumerate() {
        *v *= ad(k as f32 / RATE, 0.002, 0.012);
    }
    mix(&mut out, &thump, 0, 1.5);
    lp(&mut out, 5000.0);
    finish(out, -18.0)
}

fn burp(r: &mut Random) -> Vec<f32> {
    let dur = 0.42;
    let out = voice(
        r,
        dur,
        |t| 88.0 * (1.0 + 0.1 * (t * 14.0).sin() - 0.15 * t),
        |t| asr(t * dur, dur, 0.03, 0.15),
        |t| (t * 3.0).min(1.0),
        &[(350.0, 560.0, 4.0, 1.0), (800.0, 950.0, 5.0, 0.5), (2200.0, 2300.0, 6.0, 0.08)],
        Vp { jitter: 0.05, breath: 0.08, tilt: 900.0, rough: 0.6 },
    );
    finish(out, -18.0)
}

/// Bow release: a damped string thrum plus a falling whoosh.
fn bow(r: &mut Random) -> Vec<f32> {
    let dur = 0.4;
    let n = secs(dur);
    let mut out = vec![0.0; n];
    let f = r.uniform(150.0, 175.0);
    modes(&mut out, 0.0, &[(f, 0.08, 1.0), (f * 2.0, 0.05, 0.5), (f * 3.01, 0.035, 0.3), (f * 4.02, 0.025, 0.15)]);
    let noise = pink(r, n);
    let mut f_bp = Biquad::bandpass(2000.0, 1.2);
    for k in 0..n {
        let t = k as f32 / RATE;
        if k % 32 == 0 {
            f_bp.retune(Biquad::bandpass(2200.0 * (-t / 0.12).exp() + 500.0, 1.2));
        }
        out[k] += f_bp.tick(noise[k]) * ad(t, 0.01, 0.07) * 1.2;
    }
    lp(&mut out, 5000.0);
    finish(out, -17.0)
}

/// Arrow striking something: a short woody thunk.
fn arrow_hit(r: &mut Random) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.2)];
    let f = r.uniform(240.0, 300.0);
    modes(&mut out, 0.0, &[(f, 0.03, 1.0), (f * 2.6, 0.015, 0.4), (f * 4.3, 0.008, 0.2)]);
    let mut tap = white(r, secs(0.006));
    bp(&mut tap, 900.0, 0.8);
    mix(&mut out, &tap, 0, 0.6);
    lp(&mut out, 5000.0);
    finish(out, -17.0)
}

/// Lava hissing in water (or a lit fuse): soft crackling sizzle.
fn sizzle(r: &mut Random, dur: f32, sustained: bool) -> Vec<f32> {
    let n = secs(dur);
    let mut out = white(r, n);
    hp(&mut out, 1500.0);
    lp(&mut out, 6000.0);
    let (mut g, mut g_to) = (0.5f32, 0.5f32);
    for (k, v) in out.iter_mut().enumerate() {
        if k % 180 == 0 {
            g_to = if r.chance(0.2) { 1.0 } else { r.uniform(0.25, 0.6) };
        }
        g += (g_to - g) * 0.05;
        let t = k as f32 / RATE;
        let env = if sustained { asr(t, dur, 0.03, 0.2) } else { ad(t, 0.01, dur * 0.35) };
        *v *= g * env * 0.6;
    }
    // crackling sparks
    for _ in 0..(dur * 25.0) as usize {
        grain(&mut out, r.next_u64(), r.next_f32() * dur * 0.9, 0.002, r.uniform(1500.0, 4000.0), 1.2, r.uniform(0.2, 0.6));
    }
    finish(out, -23.0)
}

#[allow(clippy::too_many_arguments)]
fn bubbles(out: &mut [f32], r: &mut Random, count: usize, start: f32, spread: f32, fmin: f32, fmax: f32, amp: f32) {
    for _ in 0..count {
        let at = start + r.next_f32().powf(1.3) * spread;
        let st = secs(at);
        let len = r.uniform(0.02, 0.05);
        let f0 = r.uniform(fmin, fmax);
        let a = r.uniform(0.3, 1.0) * amp;
        let mut ph = 0.0f32;
        for k in 0..secs(len) {
            let Some(o) = out.get_mut(st + k) else { break };
            let t = k as f32 / RATE;
            ph += f0 * (1.0 + 1.5 * t / len) / RATE;
            *o += (ph * TAU).sin() * ad(t, 0.002, len * 0.3) * a;
        }
    }
}

fn splash(r: &mut Random) -> Vec<f32> {
    let dur = 0.9;
    let n = secs(dur);
    let mut out = pink(r, n);
    lp(&mut out, 2400.0);
    hp(&mut out, 150.0);
    for (k, v) in out.iter_mut().enumerate() {
        *v *= ad(k as f32 / RATE, 0.004, 0.13);
    }
    let mut spray = white(r, n);
    bp(&mut spray, 3000.0, 0.7);
    for (k, v) in spray.iter_mut().enumerate() {
        *v *= ad(k as f32 / RATE, 0.01, 0.18) * 0.15;
    }
    mix(&mut out, &spray, 0, 1.0);
    bubbles(&mut out, r, 22, 0.03, 0.55, 450.0, 1400.0, 0.12);
    lp(&mut out, 6500.0);
    finish(out, -16.0)
}

/// Rain: a long, softly windowed wash with patter, played overlapping as a loop.
fn rain(r: &mut Random) -> Vec<f32> {
    let dur = 4.0;
    let n = secs(dur);
    let mut out = pink(r, n);
    lp(&mut out, 4500.0);
    hp(&mut out, 300.0);
    out.iter_mut().for_each(|v| *v *= 0.5);
    for _ in 0..700 {
        grain(&mut out, r.next_u64(), r.next_f32() * (dur - 0.01), r.uniform(0.0015, 0.004), r.uniform(1500.0, 5000.0), 1.0, r.uniform(0.05, 0.3));
    }
    for _ in 0..60 {
        let f = r.uniform(900.0, 2500.0);
        modes(&mut out, r.next_f32() * (dur - 0.05), &[(f, r.uniform(0.003, 0.008), 0.08)]);
    }
    let fade = 0.8;
    for (k, v) in out.iter_mut().enumerate() {
        let t = k as f32 / RATE;
        let w = (t / fade).min((dur - t) / fade).clamp(0.0, 1.0);
        *v *= (w * std::f32::consts::FRAC_PI_2).sin().powi(2);
    }
    lp(&mut out, 7000.0);
    finish(out, -22.0)
}

/// Wooden creak: a stick-slip "voice" with very irregular pitch.
fn creak(r: &mut Random, dur: f32, f: f32) -> Vec<f32> {
    voice(
        r,
        dur,
        move |t| f * (1.0 + 0.35 * (t * 9.0).sin() + 0.2 * t),
        |t| (t * std::f32::consts::PI).sin().powf(1.5),
        |_| 0.5,
        &[(800.0, 800.0, 4.0, 1.0), (1700.0, 1700.0, 5.0, 0.5), (2900.0, 2900.0, 6.0, 0.15)],
        Vp { jitter: 0.12, breath: 0.03, tilt: 2200.0, rough: 0.7 },
    )
}

fn latch(out: &mut [f32], r: &mut Random, at: f32, amp: f32) {
    modes(out, at, &[(r.uniform(1200.0, 1500.0), 0.01, amp), (r.uniform(2400.0, 2800.0), 0.006, amp * 0.5)]);
}

fn door(r: &mut Random, open: bool) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.5)];
    if open {
        latch(&mut out, r, 0.0, 0.25);
        let f = r.uniform(160.0, 220.0);
        let c = creak(r, 0.32, f);
        mix(&mut out, &c, secs(0.04), 0.12);
        let f = r.uniform(170.0, 210.0);
        modes(&mut out, 0.0, &[(f, 0.03, 0.4), (f * 2.4, 0.02, 0.2)]);
    } else {
        // the door swinging shut against its frame, then the latch
        let f = r.uniform(120.0, 150.0);
        modes(&mut out, 0.0, &[(f, 0.06, 1.0), (f * 2.41, 0.035, 0.5), (f * 3.93, 0.02, 0.25), (f * 5.6, 0.012, 0.12)]);
        let mut tap = white(r, secs(0.006));
        bp(&mut tap, 900.0, 0.8);
        mix(&mut out, &tap, 0, 0.5);
        latch(&mut out, r, 0.025, 0.2);
    }
    lp(&mut out, 6000.0);
    finish(out, -17.0)
}

fn chest(r: &mut Random, open: bool) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.6)];
    if open {
        let f = r.uniform(110.0, 140.0);
        let c = creak(r, 0.45, f);
        mix(&mut out, &c, 0, 0.15);
        let f = r.uniform(140.0, 170.0);
        modes(&mut out, 0.0, &[(f, 0.03, 0.3), (f * 2.4, 0.02, 0.15)]);
    } else {
        let f = r.uniform(110.0, 135.0);
        modes(&mut out, 0.0, &[(f, 0.07, 1.0), (f * 2.41, 0.04, 0.5), (f * 3.93, 0.022, 0.25)]);
        let mut tap = white(r, secs(0.008));
        bp(&mut tap, 700.0, 0.8);
        mix(&mut out, &tap, 0, 0.5);
    }
    lp(&mut out, 5500.0);
    finish(out, -18.0)
}

/// Bucket scooping or pouring a liquid: a filtered slosh sweeping up (fill) or
/// down (empty), with bubbles. Lava is thicker and lower.
fn bucket(r: &mut Random, fill: bool, lava: bool) -> Vec<f32> {
    let dur = if lava { 0.75 } else { 0.6 };
    let n = secs(dur);
    let noise = pink(r, n);
    let mut out = vec![0.0; n];
    let (lo, hi) = if lava { (250.0, 700.0) } else { (500.0, 1500.0) };
    let mut f = Biquad::bandpass(lo, 1.5);
    for k in 0..n {
        let t = k as f32 / n as f32;
        if k % 32 == 0 {
            let x = if fill { t } else { 1.0 - t };
            f.retune(Biquad::bandpass(lo + (hi - lo) * x, 1.5));
        }
        out[k] = f.tick(noise[k]) * asr(t * dur, dur, 0.03, dur * 0.6);
    }
    bubbles(&mut out, r, 14, 0.02, dur * 0.7, if lava { 180.0 } else { 400.0 }, if lava { 500.0 } else { 1200.0 }, 0.15);
    lp(&mut out, if lava { 2500.0 } else { 5000.0 });
    finish(out, -18.0)
}

/// Shears: two quick metallic snips.
fn shear(r: &mut Random) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.3)];
    for at in [0.0, r.uniform(0.09, 0.12)] {
        let f = r.uniform(2300.0, 2700.0);
        modes(&mut out, at, &[(f, 0.012, 0.4), (f * 1.48, 0.008, 0.25), (f * 0.72, 0.015, 0.3)]);
        grain(&mut out, r.next_u64(), at, 0.006, 4500.0, 1.0, 0.4);
    }
    lp(&mut out, 7500.0);
    finish(out, -20.0)
}

fn explosion(r: &mut Random) -> Vec<f32> {
    let dur = 2.6;
    let n = secs(dur);
    let mut out = vec![0.0; n];
    // sub-bass thump with a falling pitch
    let mut ph = 0.0f32;
    for (k, o) in out.iter_mut().enumerate() {
        let t = k as f32 / RATE;
        ph += (32.0 + 45.0 * (-t / 0.07).exp()) / RATE;
        *o += (ph * TAU).sin() * ad(t, 0.003, 0.32) * 1.0;
    }
    // the blast body
    let mut body = brown(r, n);
    lp(&mut body, 700.0);
    for (k, v) in body.iter_mut().enumerate() {
        *v *= ad(k as f32 / RATE, 0.004, 0.45);
    }
    mix(&mut out, &body, 0, 1.6);
    // rolling rumble tail
    let mut rumble = brown(r, n);
    lp(&mut rumble, 160.0);
    for (k, v) in rumble.iter_mut().enumerate() {
        *v *= ad(k as f32 / RATE, 0.06, 0.9);
    }
    mix(&mut out, &rumble, 0, 1.4);
    // debris crackle
    for _ in 0..45 {
        let at = r.next_f32().powf(1.4) * 1.6;
        grain(&mut out, r.next_u64(), at, r.uniform(0.005, 0.025), r.uniform(500.0, 2500.0), 1.5, r.uniform(0.1, 0.35) * (1.0 - at / 1.8));
    }
    // gentle saturation for weight
    for v in out.iter_mut() {
        *v = (*v * 1.4).tanh() / 1.4;
    }
    lp(&mut out, 6000.0);
    finish(out, -10.0)
}

// ---------------- mobs ----------------

fn pig_say(r: &mut Random, i: usize) -> Vec<f32> {
    let f = 112.0 + i as f32 * 9.0;
    let dur = 0.34 + i as f32 * 0.03;
    let mut out = voice(
        r,
        dur,
        |t| f * (1.1 - 0.25 * t),
        |t| {
            // a snorty double pulse: "hrr-onk"
            let a = (-(t - 0.18).powi(2) / 0.006).exp() * 0.6;
            let b = (-(t - 0.55).powi(2) / 0.035).exp();
            (a + b) * asr(t, 1.0, 0.03, 0.15)
        },
        |t| t,
        &[(280.0, 420.0, 4.0, 1.0), (900.0, 1100.0, 5.0, 0.55), (2200.0, 2400.0, 6.0, 0.2)],
        Vp { jitter: 0.04, breath: 0.25, tilt: 1600.0, rough: 0.55 },
    );
    lp(&mut out, 4500.0);
    finish(out, -17.0)
}

fn pig_hurt(r: &mut Random, i: usize) -> Vec<f32> {
    let f = 620.0 + i as f32 * 60.0;
    let dur = 0.36;
    let mut out = voice(
        r,
        dur,
        |t| f * (1.0 + 0.4 * (t * std::f32::consts::PI).sin() - 0.2 * t),
        |t| asr(t * dur, dur, 0.015, 0.12),
        |t| (t * 4.0).min(1.0),
        &[(800.0, 1000.0, 5.0, 1.0), (1800.0, 2100.0, 6.0, 0.5), (3000.0, 3200.0, 8.0, 0.15)],
        Vp { jitter: 0.03, breath: 0.1, tilt: 2600.0, rough: 0.2 },
    );
    lp(&mut out, 5500.0);
    finish(out, -18.0)
}

fn moo(r: &mut Random, dur: f32, f: f32, rise: f32) -> Vec<f32> {
    voice(
        r,
        dur,
        move |t| f * (1.0 + rise * (t * 2.2).min(1.0) * (1.0 - t) * 2.0 - 0.15 * t + 0.01 * (t * 40.0).sin()),
        move |t| asr(t * dur, dur, 0.12, 0.3),
        |t| (t * 2.5).min(1.0) * (1.0 - 0.4 * t),
        &[(250.0, 520.0, 4.0, 1.0), (700.0, 1000.0, 5.0, 0.5), (2300.0, 2400.0, 7.0, 0.1)],
        Vp { jitter: 0.012, breath: 0.12, tilt: 1300.0, rough: 0.15 },
    )
}

fn cow_say(r: &mut Random, i: usize) -> Vec<f32> {
    let out = moo(r, 1.1 + i as f32 * 0.12, 100.0 + i as f32 * 8.0, 0.18);
    finish(out, -16.0)
}

fn cow_hurt(r: &mut Random, i: usize) -> Vec<f32> {
    let out = moo(r, 0.5, 135.0 + i as f32 * 10.0, 0.08);
    finish(out, -16.0)
}

fn bleat(r: &mut Random, dur: f32, f: f32) -> Vec<f32> {
    voice(
        r,
        dur,
        move |t| f * (1.0 + 0.06 * (t * dur * 7.5 * TAU).sin() - 0.08 * t),
        move |t| asr(t * dur, dur, 0.03, 0.18) * (0.65 + 0.35 * (t * dur * 7.5 * TAU).sin()),
        |t| (t * 3.0).min(1.0),
        &[(600.0, 780.0, 6.0, 1.0), (1500.0, 1750.0, 8.0, 0.55), (2600.0, 2800.0, 8.0, 0.2)],
        Vp { jitter: 0.02, breath: 0.08, tilt: 2400.0, rough: 0.1 },
    )
}

fn sheep_say(r: &mut Random, i: usize) -> Vec<f32> {
    let mut out = bleat(r, 0.7 + i as f32 * 0.08, 265.0 + i as f32 * 18.0);
    lp(&mut out, 5000.0);
    finish(out, -18.0)
}

fn sheep_hurt(r: &mut Random, i: usize) -> Vec<f32> {
    let mut out = bleat(r, 0.35, 330.0 + i as f32 * 20.0);
    lp(&mut out, 5000.0);
    finish(out, -18.0)
}

fn cluck(r: &mut Random, dur: f32, f: f32, bright: f32) -> Vec<f32> {
    voice(
        r,
        dur,
        move |t| f * (1.1 - 0.3 * t),
        move |t| asr(t * dur, dur, 0.006, dur * 0.6),
        |t| 1.0 - t,
        &[(900.0, 1100.0, 5.0, 1.0), (1900.0, 2200.0, 6.0, 0.5 * bright), (3000.0, 3200.0, 7.0, 0.15 * bright)],
        Vp { jitter: 0.03, breath: 0.15, tilt: 2400.0 * bright, rough: 0.2 },
    )
}

fn chicken_say(r: &mut Random, i: usize) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.55)];
    let mut at = 0.0;
    for _ in 0..2 + r.range(3) {
        let (d, f) = (r.uniform(0.05, 0.08), r.uniform(420.0, 540.0) + i as f32 * 20.0);
        let c = cluck(r, d, f, 0.8);
        mix(&mut out, &c, secs(at), r.uniform(0.6, 1.0));
        at += r.uniform(0.1, 0.15);
    }
    lp(&mut out, 5000.0);
    finish(out, -20.0)
}

fn chicken_hurt(r: &mut Random, i: usize) -> Vec<f32> {
    let mut out = cluck(r, 0.24, 640.0 + i as f32 * 50.0, 1.0);
    lp(&mut out, 5500.0);
    finish(out, -19.0)
}

fn groan(r: &mut Random, dur: f32, f: f32) -> Vec<f32> {
    voice(
        r,
        dur,
        move |t| f * (1.0 + 0.18 * (t * std::f32::consts::PI).sin() - 0.12 * t),
        move |t| asr(t * dur, dur, 0.12, dur * 0.4) * (0.75 + 0.25 * (t * dur * 4.0 * TAU).sin()),
        |t| (t * std::f32::consts::PI).sin(),
        &[(380.0, 600.0, 4.0, 1.0), (750.0, 1000.0, 5.0, 0.6), (2300.0, 2400.0, 6.0, 0.12)],
        Vp { jitter: 0.05, breath: 0.3, tilt: 1100.0, rough: 0.65 },
    )
}

fn zombie_say(r: &mut Random, i: usize) -> Vec<f32> {
    let mut out = groan(r, 1.1 + i as f32 * 0.15, 78.0 + i as f32 * 7.0);
    lp(&mut out, 4000.0);
    finish(out, -16.0)
}

fn zombie_hurt(r: &mut Random, i: usize) -> Vec<f32> {
    let mut out = groan(r, 0.4, 105.0 + i as f32 * 10.0);
    lp(&mut out, 4000.0);
    finish(out, -16.0)
}

/// Bones clacking together.
fn clacks(out: &mut [f32], r: &mut Random, count: usize, spread: f32) {
    let mut at = 0.0;
    for _ in 0..count {
        let f = r.uniform(900.0, 1700.0);
        let a = r.uniform(0.5, 1.0);
        modes(out, at, &[(f, 0.009, a), (f * 1.62, 0.006, a * 0.5), (f * 2.3, 0.004, a * 0.25)]);
        grain(out, r.next_u64(), at, 0.002, 2500.0, 1.0, a * 0.3);
        at += r.uniform(0.2, 1.0) * spread;
    }
}

fn skeleton_say(r: &mut Random) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.5)];
    let n = 5 + r.range(4) as usize;
    clacks(&mut out, r, n, 0.055);
    lp(&mut out, 6000.0);
    finish(out, -19.0)
}

fn skeleton_hurt(r: &mut Random) -> Vec<f32> {
    let mut out = vec![0.0; secs(0.35)];
    clacks(&mut out, r, 6, 0.03);
    let mut thump = white(r, secs(0.04));
    lp(&mut thump, 400.0);
    for (k, v) in thump.iter_mut().enumerate() {
        *v *= ad(k as f32 / RATE, 0.001, 0.01);
    }
    mix(&mut out, &thump, 0, 1.0);
    lp(&mut out, 6000.0);
    finish(out, -17.0)
}

/// Spider: a breathy, chittering hiss.
fn chitter(r: &mut Random, dur: f32, rate: f32) -> Vec<f32> {
    let n = secs(dur);
    let mut a = pink(r, n);
    bp(&mut a, 2000.0, 1.2);
    let mut b = white(r, n);
    bp(&mut b, 3400.0, 2.0);
    let mut out: Vec<f32> = (0..n)
        .map(|k| {
            let t = k as f32 / RATE;
            let trem = 0.4 + 0.6 * (0.5 + 0.5 * (t * rate * TAU).sin()).powi(2);
            (a[k] + b[k] * 0.4) * trem * asr(t, dur, 0.04, dur * 0.5)
        })
        .collect();
    for _ in 0..6 {
        let f = r.uniform(1500.0, 2200.0);
        modes(&mut out, r.next_f32() * dur * 0.8, &[(f, 0.005, 0.15)]);
    }
    lp(&mut out, 5500.0);
    out
}

fn spider_say(r: &mut Random) -> Vec<f32> {
    let dur = r.uniform(0.5, 0.7);
    let rate = r.uniform(24.0, 32.0);
    let out = chitter(r, dur, rate);
    finish(out, -21.0)
}

fn spider_hurt(r: &mut Random) -> Vec<f32> {
    let out = chitter(r, 0.3, 38.0);
    finish(out, -19.0)
}

/// Creepers rustle like dry leaves when hit.
fn creeper_hurt(r: &mut Random) -> Vec<f32> {
    let dur = 0.28;
    let mut out = vec![0.0; secs(dur + 0.03)];
    for _ in 0..40 {
        let at = r.next_f32().powf(1.3) * dur * 0.85;
        grain(&mut out, r.next_u64(), at, r.uniform(0.004, 0.012), r.uniform(700.0, 2600.0), 1.6, r.uniform(0.3, 1.0) * (1.0 - at / dur));
    }
    lp(&mut out, 5000.0);
    finish(out, -17.0)
}

// ---------------- music ----------------

/// Freeverb-style comb filter with damping.
struct Comb {
    buf: Vec<f32>,
    i: usize,
    store: f32,
}
impl Comb {
    fn tick(&mut self, x: f32, fb: f32, damp: f32) -> f32 {
        let y = self.buf[self.i];
        self.store = y * (1.0 - damp) + self.store * damp;
        self.buf[self.i] = x + self.store * fb;
        self.i = (self.i + 1) % self.buf.len();
        y
    }
}
struct Allpass {
    buf: Vec<f32>,
    i: usize,
}
impl Allpass {
    fn tick(&mut self, x: f32) -> f32 {
        let b = self.buf[self.i];
        let y = b - x;
        self.buf[self.i] = x + b * 0.5;
        self.i = (self.i + 1) % self.buf.len();
        y
    }
}

/// Stereo room reverb (Freeverb tunings) applied in place to interleaved audio.
fn reverb(st: &mut [f32], wet: f32) {
    let combs = [1116usize, 1188, 1277, 1356, 1422, 1491];
    let aps = [556usize, 441, 341];
    for ch in 0..2 {
        let spread = ch * 23;
        let mut cs: Vec<Comb> = combs.iter().map(|&l| Comb { buf: vec![0.0; l + spread], i: 0, store: 0.0 }).collect();
        let mut ap: Vec<Allpass> = aps.iter().map(|&l| Allpass { buf: vec![0.0; l + spread], i: 0 }).collect();
        let frames = st.len() / 2;
        for f in 0..frames {
            let x = (st[f * 2] + st[f * 2 + 1]) * 0.5 * 0.05;
            let mut y: f32 = cs.iter_mut().map(|c| c.tick(x, 0.84, 0.35)).sum();
            for a in ap.iter_mut() {
                y = a.tick(y);
            }
            st[f * 2 + ch] += y * wet;
        }
    }
}

/// A felt-piano note: inharmonic partials with two slightly detuned strings,
/// two-stage decay, a soft hammer attack and a damper release.
fn piano_note(out: &mut [f32], midi: i32, start: f32, vel: f32, hold: f32, pan: f32) {
    let f = 440.0 * 2f32.powf((midi as f32 - 69.0) / 12.0);
    let st = secs(start);
    let release = 0.35;
    let len = secs(hold + release * 4.0);
    let tau = (3.0 * (262.0 / f).powf(0.6)).clamp(0.8, 6.0);
    let b_inh = 0.00025;
    let np = ((7000.0 / f) as usize).clamp(1, 10);
    let mut note = vec![0.0f32; len];
    for k in 1..=np {
        let kf = k as f32;
        let fk = kf * f * (1.0 + b_inh * kf * kf).sqrt();
        let ak = vel * kf.powf(-(1.7 - 0.8 * vel));
        let tk = tau / (1.0 + 0.55 * (kf - 1.0));
        let strings: &[f32] = if k <= 4 { &[0.9993, 1.0007] } else { &[1.0] };
        for &det in strings {
            let (s, c) = (TAU * fk * det / RATE).sin_cos();
            let (mut x, mut y) = (1.0f32, 0.0f32);
            let gain = ak / strings.len() as f32;
            for (j, o) in note.iter_mut().enumerate() {
                if j % 4096 == 0 {
                    let m = (x * x + y * y).sqrt();
                    x /= m;
                    y /= m;
                }
                let t = j as f32 / RATE;
                let env = 0.65 * (-t / (tk * 0.22)).exp() + 0.35 * (-t / tk).exp();
                *o += y * env * gain;
                let nx = x * c - y * s;
                y = x * s + y * c;
                x = nx;
            }
        }
    }
    // soft hammer onset and damper
    let atk = secs(0.004).max(1);
    for (j, v) in note.iter_mut().enumerate() {
        let t = j as f32 / RATE;
        let a = ((j as f32 / atk as f32).min(1.0) * std::f32::consts::FRAC_PI_2).sin();
        let d = if t > hold { (-(t - hold) / release).exp() } else { 1.0 };
        *v *= a * d;
    }
    lp(&mut note, 2500.0 + 4000.0 * vel);
    let a = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    let (gl, gr) = (a.cos(), a.sin());
    for (j, &v) in note.iter().enumerate() {
        let i = (st + j) * 2;
        if i + 1 >= out.len() {
            break;
        }
        out[i] += v * gl;
        out[i + 1] += v * gr;
    }
}

/// A short, calm generative piano piece in the spirit of the vanilla
/// soundtrack: a slow diatonic chord progression with broken-chord left hand
/// and a sparse, mostly stepwise melody. Returns interleaved stereo.
fn compose_piece(seed: u64) -> Vec<f32> {
    let mut r = Random::new(seed);
    let major = [0, 2, 4, 5, 7, 9, 11];
    let key = [0, 2, 5, 7, -3][r.range(5) as usize];
    // scale degree (0-based, may exceed 6) -> midi note around middle C
    let note = |deg: i32, base: i32| -> i32 {
        let o = deg.div_euclid(7);
        base + key + major[deg.rem_euclid(7) as usize] + 12 * o
    };
    let progressions: [[i32; 4]; 6] = [[0, 5, 3, 4], [0, 3, 0, 4], [0, 4, 5, 3], [3, 4, 0, 5], [5, 3, 0, 4], [0, 2, 3, 3]];
    let prog = progressions[r.range(progressions.len() as i32) as usize];
    let beat = 0.72 + r.next_f32() * 0.2;
    let bars = 12 + r.range(7) as usize;
    let total = beat * 4.0 * bars as f32 + 6.0;
    let mut out = vec![0.0f32; secs(total) * 2];
    let human = |r: &mut Random| r.uniform(-0.012, 0.012);
    // a one-bar rhythmic motif (in eighth notes) reused with variation
    let motif: Vec<usize> = (0..8).filter(|&e| if e % 2 == 0 { r.chance(0.55) } else { r.chance(0.15) }).collect();
    let mut mel = 7 + 2; // scale degree of the melody, starting on the third above middle C
    for b in 0..bars {
        let last = b == bars - 1;
        let chord = if last { 0 } else { prog[b % 4] };
        let t0 = 0.5 + b as f32 * beat * 4.0;
        let seventh = r.chance(0.35);
        // left hand: broken chord
        let lh_vel = 0.34 + r.next_f32() * 0.06;
        let root = note(chord, 36);
        piano_note(&mut out, root, t0 + human(&mut r), lh_vel * 0.8, beat * 4.0, -0.25);
        if !last {
            piano_note(&mut out, note(chord + 4, 48), t0 + beat + human(&mut r), lh_vel * 0.8, beat * 3.0, -0.15);
            piano_note(&mut out, note(chord + if seventh { 6 } else { 2 }, 48 + 12), t0 + beat * 2.0 + human(&mut r), lh_vel * 0.75, beat * 2.0, -0.05);
            if r.chance(0.5) {
                piano_note(&mut out, note(chord + 4, 48), t0 + beat * 3.0 + human(&mut r), lh_vel * 0.7, beat, -0.15);
            }
        } else {
            piano_note(&mut out, note(chord + 4, 48), t0 + 0.03, lh_vel * 0.8, beat * 6.0, -0.15);
            piano_note(&mut out, note(chord + 2, 60), t0 + 0.06, lh_vel * 0.7, beat * 6.0, 0.05);
            piano_note(&mut out, note(7, 60), t0 + beat + 0.04, 0.4, beat * 6.0, 0.25);
            break;
        }
        // right hand: sparse melody; some bars rest
        if b == 0 || r.chance(0.2) {
            continue;
        }
        let vary = r.chance(0.35);
        for e in 0..8usize {
            let play = if vary { r.chance(0.3) && e % 2 == 0 } else { motif.contains(&e) };
            if !play {
                continue;
            }
            let strong = e % 4 == 0;
            // step to a neighbouring degree, preferring chord tones on strong beats
            let mut cand = mel + [-2, -1, -1, 0, 1, 1, 2][r.range(7) as usize];
            if strong {
                let tones = [chord, chord + 2, chord + 4, chord + 7, chord + 9, chord + 11];
                cand = *tones.iter().min_by_key(|&&d| (d - cand).abs()).unwrap_or(&cand);
            }
            mel = cand.clamp(4, 14);
            let vel = 0.32 + r.next_f32() * 0.12 + if strong { 0.05 } else { 0.0 };
            let midi = note(mel, 60);
            piano_note(&mut out, midi, t0 + e as f32 * beat * 0.5 + human(&mut r), vel, beat * 1.5, (midi - 60) as f32 / 30.0);
        }
    }
    reverb(&mut out, 0.32);
    // normalize on the mono sum
    let mono: Vec<f32> = out.chunks(2).map(|c| (c[0] + c[1]) * 0.5).collect();
    let l = loudness(&mono);
    if l > 1e-9 {
        let g = 10f32.powf(-21.0 / 20.0) / l;
        out.iter_mut().for_each(|v| *v *= g);
    }
    let peak = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    if peak > 0.8 {
        out.iter_mut().for_each(|v| *v *= 0.8 / peak);
    }
    // fade the tail
    let fo = secs(1.5).min(out.len() / 2);
    let n = out.len() / 2;
    for i in 0..fo {
        let g = i as f32 / fo as f32;
        out[(n - 1 - i) * 2] *= g;
        out[(n - 1 - i) * 2 + 1] *= g;
    }
    out
}

// ---------------- block -> sound mapping ----------------

/// Sound material name for a block (dig/step sounds).
pub fn material(b: u8) -> &'static str {
    use crate::block::*;
    match b {
        OAK_PLANKS..=ACACIA_PLANKS | OAK_LOG..=ACACIA_LOG | CRAFTING_TABLE | CHEST | BOOKSHELF | PUMPKIN | TORCH | OAK_DOOR | LADDER | OAK_FENCE | BED => "wood",
        GRASS | OAK_LEAVES..=ACACIA_LEAVES | SHORT_GRASS | FERN | DEAD_BUSH | DANDELION | POPPY | BLUE_ORCHID | WHEAT | SUGAR_CANE | TNT | OAK_SAPLING..=ACACIA_SAPLING => "grass",
        DIRT | GRAVEL | FARMLAND | CLAY => "gravel",
        SAND => "sand",
        SNOW_LAYER | SNOW_BLOCK => "snow",
        WOOL | CACTUS => "cloth",
        GOLD_BLOCK | IRON_BLOCK | DIAMOND_BLOCK => "metal",
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
        "metal" => "dig.metal",
        _ => "dig.stone",
    }
}

/// Sound for placing a block (glass places with a stone sound, as in vanilla).
pub fn place_sound(b: u8) -> &'static str {
    if b == crate::block::GLASS || b == crate::block::ICE {
        return "dig.stone";
    }
    dig_sound(b)
}

pub fn step_sound(b: u8) -> &'static str {
    match material(b) {
        "wood" => "step.wood",
        "grass" => "step.grass",
        "gravel" => "step.gravel",
        "sand" => "step.sand",
        "snow" => "step.snow",
        "cloth" => "step.cloth",
        "metal" => "step.metal",
        _ => "step.stone",
    }
}

/// Set by UI code when a button is clicked (played by the main loop).
pub static UI_CLICK: AtomicBool = AtomicBool::new(false);

// ---------------- debugging ----------------

fn wav_bytes(samples: &[f32], channels: u16) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&channels.to_le_bytes());
    v.extend_from_slice(&(RATE as u32).to_le_bytes());
    v.extend_from_slice(&(RATE as u32 * channels as u32 * 2).to_le_bytes());
    v.extend_from_slice(&(channels * 2).to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        v.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    v
}

/// `--dump-sounds DIR`: write every sound (and one music piece) as WAV files,
/// and print each one's peak and weighted loudness.
pub fn dump_sounds(dir: &str) {
    let _ = std::fs::create_dir_all(dir);
    let mut all = synth_all();
    all.sort_by_key(|(n, _)| *n);
    for (name, vs) in &all {
        for (i, s) in vs.iter().enumerate() {
            let peak = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            println!("{name:18} #{i}  {:5.2}s  peak {:6.1} dB  loud {:6.1} dB", s.len() as f32 / RATE, 20.0 * peak.log10(), 20.0 * loudness(s).log10());
            let _ = std::fs::write(format!("{dir}/{name}_{i}.wav"), wav_bytes(s, 1));
        }
    }
    let m = compose_piece(7);
    let _ = std::fs::write(format!("{dir}/music.wav"), wav_bytes(&m, 2));
    println!("wrote sounds to {dir}");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sounds_are_valid() {
        let all = synth_all();
        assert!(all.len() > 40);
        for (name, vs) in &all {
            for v in vs {
                assert!(!v.is_empty(), "{name} empty");
                assert!(v.iter().all(|s| s.is_finite()), "{name} has NaN");
                let peak = v.iter().fold(0.0f32, |m, s| m.max(s.abs()));
                assert!(peak > 0.01 && peak <= 0.9001, "{name} peak {peak}");
                // edges are faded, so nothing starts or stops with a click
                assert!(v[0].abs() < 1e-3 && v[v.len() - 1].abs() < 1e-3, "{name} edge click");
            }
        }
    }
    #[test]
    fn music_is_valid() {
        let m = compose_piece(5);
        assert!(m.iter().all(|s| s.is_finite()) && m.len() > 44100 * 2 * 10);
        assert!(m.iter().fold(0.0f32, |a, s| a.max(s.abs())) <= 0.8001);
    }
    #[test]
    fn biquad_lowpass_attenuates_highs() {
        let mut lo: Vec<f32> = (0..4410).map(|i| (i as f32 * TAU * 200.0 / RATE).sin()).collect();
        let mut hi: Vec<f32> = (0..4410).map(|i| (i as f32 * TAU * 8000.0 / RATE).sin()).collect();
        lp(&mut lo, 1000.0);
        lp(&mut hi, 1000.0);
        let rms = |s: &[f32]| (s[1000..].iter().map(|v| v * v).sum::<f32>() / (s.len() - 1000) as f32).sqrt();
        assert!(rms(&lo) > 0.6 && rms(&hi) < 0.03);
    }
}
