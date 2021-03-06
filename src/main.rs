/* This example expose parameter to pass generator of sample.
Good starting point for integration of cpal into your application.
*/

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn main() -> anyhow::Result<()> {
    let stream = stream_setup_for(sample_next)?;
    stream.play()?;
    std::thread::sleep(std::time::Duration::from_millis(3000));
    Ok(())
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

fn sample_next(o: &mut SampleRequestOptions) -> f32 {
    o.tick();

    let c_freq = 440.;

    let c_maj = Interval::major_triad_freqs(c_freq);

    let c_maj: f32 = c_maj.iter().map(|f| o.tone(*f)).sum();

    c_maj
    // o.tone(Interval::MajorSeventh.by_interval(c_freq))

    // o.tone(Interval::MajorSixth.by_interval(c_freq))
}

pub struct SampleRequestOptions {
    pub sample_rate: f32,
    pub sample_clock: f32,
    pub nchannels: usize,
}

impl SampleRequestOptions {
    fn tone(&self, freq: f32) -> f32 {
        (self.sample_clock * freq * 2.0 * std::f32::consts::PI / self.sample_rate).sin()
    }
    fn tick(&mut self) {
        self.sample_clock = (self.sample_clock + 1.0) % self.sample_rate;
    }
}

pub fn stream_setup_for<F>(on_sample: F) -> Result<cpal::Stream, anyhow::Error>
where
    F: FnMut(&mut SampleRequestOptions) -> f32 + std::marker::Send + 'static + Copy,
{
    let (_host, device, config) = host_device_setup()?;

    match config.sample_format() {
        cpal::SampleFormat::F32 => stream_make::<f32, _>(&device, &config.into(), on_sample),
        cpal::SampleFormat::I16 => stream_make::<i16, _>(&device, &config.into(), on_sample),
        cpal::SampleFormat::U16 => stream_make::<u16, _>(&device, &config.into(), on_sample),
    }
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

pub fn stream_make<T, F>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    on_sample: F,
) -> Result<cpal::Stream, anyhow::Error>
where
    T: cpal::Sample,
    F: FnMut(&mut SampleRequestOptions) -> f32 + std::marker::Send + 'static + Copy,
{
    let sample_rate = config.sample_rate.0 as f32;
    let sample_clock = 0f32;
    let nchannels = config.channels as usize;
    let mut request = SampleRequestOptions {
        sample_rate,
        sample_clock,
        nchannels,
    };
    let err_fn = |err| eprintln!("Error building output sound stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
            on_window(output, &mut request, on_sample)
        },
        err_fn,
    )?;

    Ok(stream)
}

fn on_window<T, F>(output: &mut [T], request: &mut SampleRequestOptions, mut on_sample: F)
where
    T: cpal::Sample,
    F: FnMut(&mut SampleRequestOptions) -> f32 + std::marker::Send + 'static,
{
    for frame in output.chunks_mut(request.nchannels) {
        let value: T = cpal::Sample::from::<f32>(&on_sample(request));
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}
