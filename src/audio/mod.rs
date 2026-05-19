use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

/// Volumes for each noise colour + master, all in [0.0, 1.0].
#[derive(Debug, Clone)]
pub struct NoiseMix {
    pub white: f32,
    pub pink: f32,
    pub brown: f32,
    pub master: f32,
}

impl Default for NoiseMix {
    fn default() -> Self {
        NoiseMix { white: 0.0, pink: 0.0, brown: 0.0, master: 0.5 }
    }
}

/// Owns the cpal stream. Dropping this stops audio.
pub struct NoiseEngine {
    _stream: Option<cpal::Stream>,
    pub mix: Arc<Mutex<NoiseMix>>,
    pub _available: bool,
}

impl NoiseEngine {
    pub fn new() -> Self {
        let mix = Arc::new(Mutex::new(NoiseMix::default()));

        match Self::try_build(Arc::clone(&mix)) {
            Ok(stream) => NoiseEngine { _stream: Some(stream), mix, _available: true },
            Err(e) => {
                eprintln!("[blossom] audio unavailable: {e}");
                NoiseEngine { _stream: None, mix, _available: false }
            }
        }
    }

    fn try_build(mix: Arc<Mutex<NoiseMix>>) -> anyhow::Result<cpal::Stream> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("no output device"))?;
        let config = device.default_output_config()?;
        let channels = config.channels() as usize;

        // Pink-noise state (Paul Kellett's refinement of Voss-McCartney)
        let mut b = [0f64; 7];
        // Brown-noise state
        let mut brown: f64 = 0.0;

        let mix_clone = Arc::clone(&mix);

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _| {
                    let m = mix_clone.lock().unwrap().clone();
                    let active = m.white + m.pink + m.brown;
                    if active < 1e-4 || m.master < 1e-4 {
                        data.fill(0.0);
                        return;
                    }
                    for frame in data.chunks_mut(channels) {
                        let white: f64 = (rand::random::<f64>() - 0.5) * 2.0;

                        // Pink noise
                        b[0] = 0.99886 * b[0] + white * 0.0555179;
                        b[1] = 0.99332 * b[1] + white * 0.0750759;
                        b[2] = 0.96900 * b[2] + white * 0.1538520;
                        b[3] = 0.86650 * b[3] + white * 0.3104856;
                        b[4] = 0.55000 * b[4] + white * 0.5329522;
                        b[5] = -0.7616 * b[5] - white * 0.0168980;
                        let pink =
                            (b[0]+b[1]+b[2]+b[3]+b[4]+b[5]+b[6]+white*0.5362)*0.11;
                        b[6] = white * 0.115926;

                        // Brown noise (random walk)
                        brown = (brown + 0.02 * white) / 1.02;
                        let brown_s = brown.clamp(-1.0, 1.0) * 3.5;

                        let sample = (white  * m.white  as f64
                                    + pink   * m.pink   as f64
                                    + brown_s* m.brown  as f64)
                                    * m.master as f64
                                    * 0.5; // headroom scale

                        let s = sample.clamp(-1.0, 1.0) as f32;
                        for ch in frame.iter_mut() { *ch = s; }
                    }
                },
                |e| eprintln!("[blossom] audio stream error: {e}"),
                None,
            )?,
            // Fallback: convert through f32 using cpal's Sample trait
            fmt => {
                return Err(anyhow::anyhow!("unsupported sample format: {fmt:?}"));
            }
        };
        stream.play()?;
        Ok(stream)
    }

    pub fn set_mix(&self, mix: NoiseMix) {
        *self.mix.lock().unwrap() = mix;
    }

    pub fn _get_mix(&self) -> NoiseMix {
        self.mix.lock().unwrap().clone()
    }
}
