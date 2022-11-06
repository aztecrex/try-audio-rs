use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn main() -> anyhow::Result<()> {
    let (_host, device, config) = host_device_setup()?;

    let fundamental = 440.;
    let third = Interval::MajorThird.by_interval(fundamental);
    let fifth = Interval::Fifth.by_interval(fundamental);

    let fundamental = SineWave::new(fundamental, config.sample_rate().0);
    let third = SineWave::new(third, config.sample_rate().0);
    let fifth = SineWave::new(fifth, config.sample_rate().0);

    let parts = [fundamental, third, fifth];
    let synth = Mixer::new(parts);
    let stream = make_stream2(device, config, synth)?;

    stream.play()?;
    std::thread::sleep(std::time::Duration::from_millis(3000));
    Ok(())
}

pub struct Mixer(Vec<SineWave>);

impl Mixer {
    pub fn new<I>(oscillators: I) -> Self
    where
        I: IntoIterator<Item = SineWave>,
    {
        Self(oscillators.into_iter().collect())
    }
}

impl Oscillator for Mixer {
    fn next(&mut self) -> f32 {
        let amp = 1. / self.0.len() as f32;
        self.0.iter_mut().map(|o| o.next() * amp).sum()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interval {
    Unison,
    MinorSecond,
    MajorSecond,
    MinorThird,
    MajorThird,
    Fourth,
    DiminishedFifth,
    Fifth,
    MinorSixth,
    MajorSixth,
    MinorSeventh,
    MajorSeventh,
    Octave,
}

pub trait Oscillator: Sized {
    fn next(&mut self) -> f32;
}

pub struct SineWave {
    period: u32,
    clock: u32,
}

impl SineWave {
    pub fn new(freq: f32, sample_rate: u32) -> Self {
        Self {
            period: (sample_rate as f32 / freq) as u32,
            clock: 0,
        }
    }
}

impl Oscillator for SineWave {
    fn next(&mut self) -> f32 {
        self.clock = (self.clock + 1) % self.period;
        (self.clock as f32 / self.period as f32 * std::f32::consts::PI * 2.0).sin()
    }
}

impl Interval {
    pub fn equal_temperament_ratio(&self) -> f32 {
        use Interval::*;

        let half_step_factor = 2_f32.powf(1. / 12.);

        match self {
            Unison => 1.0,
            MinorSecond => half_step_factor,
            MajorSecond => half_step_factor.powf(2.),
            MinorThird => half_step_factor.powf(3.),
            MajorThird => half_step_factor.powf(4.),
            Fourth => half_step_factor.powf(5.),
            DiminishedFifth => half_step_factor.powf(6.),
            Fifth => half_step_factor.powf(7.),
            MinorSixth => half_step_factor.powf(8.),
            MajorSixth => half_step_factor.powf(9.),
            MinorSeventh => half_step_factor.powf(10.),
            MajorSeventh => half_step_factor.powf(11.),
            Octave => 2.,
        }
    }

    pub fn major_triad_freqs(fundamental: f32) -> [f32; 3] {
        [
            fundamental,
            fundamental * Interval::MajorThird.equal_temperament_ratio(),
            fundamental * Interval::Fifth.equal_temperament_ratio(),
        ]
    }

    pub fn by_interval(self, from: f32) -> f32 {
        from * self.equal_temperament_ratio()
    }
}

fn make_stream2(
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    synth: impl Oscillator + Send + 'static,
) -> Result<cpal::Stream, anyhow::Error> {
    let err_fn = |err| eprintln!("Error in stream: {}", err);
    let channels = config.channels();
    let format = config.sample_format();
    let config: cpal::StreamConfig = config.into();
    let stream = match format {
        cpal::SampleFormat::F32 => {
            device.build_output_stream(&config, make_on_data::<f32>(synth, channels), err_fn)
        }
        cpal::SampleFormat::I16 => {
            device.build_output_stream(&config.into(), make_on_data::<i16>(synth, channels), err_fn)
        }
        cpal::SampleFormat::U16 => {
            device.build_output_stream(&config.into(), make_on_data::<u16>(synth, channels), err_fn)
        }
    }?;

    Ok(stream)
}

pub fn host_device_setup(
) -> Result<(cpal::Host, cpal::Device, cpal::SupportedStreamConfig), anyhow::Error> {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::Error::msg("Default output device is not available"))?;
    println!("Output device : {}", device.name()?);

    let config = device.default_output_config()?;
    println!("Default output config : {:?}", config);

    Ok((host, device, config))
}

fn make_on_data<Sam>(
    mut osc: impl Oscillator + Send,
    num_channels: u16,
) -> impl FnMut(&mut [Sam], &cpal::OutputCallbackInfo) + Send
where
    Sam: cpal::Sample,
{
    let num_channels = num_channels as usize;
    move |output: &mut [Sam], _: &cpal::OutputCallbackInfo| {
        for frame in output.chunks_mut(num_channels) {
            let osc_sample = cpal::Sample::from::<f32>(&osc.next());
            for out in frame.iter_mut() {
                *out = osc_sample;
            }
        }
    }
}
