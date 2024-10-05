use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{self, Stream};
use dasp::{signal, Signal};
use std::io::{stdin, stdout, Write};
use std::sync::{mpsc, Mutex};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

fn main() -> anyhow::Result<()> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config()?;

    match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into())?,
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into())?,
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into())?,
    }

    Ok(())
}

enum Chord {
    Major,
    Minor,
    Maj7,
    Min7,
}

fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig) -> Result<(), anyhow::Error>
where
    T: cpal::Sample,
{
    // Create a signal chain to play back 1 second of each oscillator at A4.
    let one_sec = config.sample_rate.0 as usize * 5;
    let rate = config.sample_rate.0 as f64;

    // let mut synth = hz
    //     .clone()
    //     .sine()
    //     .take(one_sec)
    //     .chain(hz.clone().saw().take(one_sec))
    //     .chain(hz.clone().square().take(one_sec))
    //     .chain(hz.clone().noise_simplex().take(one_sec))
    //     .chain(signal::noise(0).take(one_sec))
    //     .map(|s| s.to_sample::<f32>() * 0.1);

    // A channel for indicating when playback has completed.

    // Create and run the stream.
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);
    let channels = config.channels as usize;

    let octave = Mutex::new(4);
    let stream: Mutex<Option<Stream>> = Mutex::new(None);

    let play = |freq, chord| {
        let freq = { freq * 2_i32.pow(*octave.lock().unwrap()) as f32 };
        let freqs = match chord {
            Chord::Major => [freq, freq * 1.25, freq * 1.5, 0.0],
            Chord::Minor => [freq, freq * 1.20, freq * 1.5, 0.0],
            Chord::Maj7 => [freq, freq * 1.25, freq * 1.5, freq * 1.875],
            Chord::Min7 => [freq, freq * 1.20, freq * 1.5, freq * 1.80],
        };

        let mut synth = freqs
            .iter()
            .map(move |hz| signal::rate(rate).const_hz(*hz as f64))
            .map(move |r| r.sine().take(one_sec));

        let mut synth = synth
            .next()
            .unwrap()
            .zip(synth.next().unwrap())
            .zip(synth.next().unwrap())
            .zip(synth.next().unwrap())
            .map(|(((a, b), c), d)| (a + b + c + d) as f32 / 4.0);

        if let Some(stream) = stream.lock().unwrap().as_ref() {
            stream.pause().unwrap();
        }
        *stream.lock().unwrap() = Some(
            device
                .build_output_stream(
                    config,
                    move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                        write_data(data, channels, &mut synth)
                    },
                    err_fn,
                )
                .unwrap(),
        );
        if let Some(stream) = &stream.lock().unwrap().as_ref() {
            stream.play().unwrap();
        }
    };

    let stdin = stdin();
    let mut stdout = stdout().into_raw_mode().unwrap();
    for c in stdin.keys() {
        match c.unwrap() {
            Key::Char(x) | Key::Alt(x) | Key::Ctrl(x) if ('1'..='9').contains(&x) => {
                *octave.lock().unwrap() = x.to_digit(10).unwrap() as u32
            }
            Key::Ctrl('c') => break,
            Key::Char('q') | Key::Char('Q') => play(16.35, Chord::Major),
            Key::Char('w') | Key::Char('W') => play(18.35, Chord::Major),
            Key::Char('e') | Key::Char('E') => play(20.60, Chord::Major),
            Key::Char('r') | Key::Char('R') => play(21.83, Chord::Major),
            Key::Char('t') | Key::Char('T') => play(24.50, Chord::Major),
            Key::Char('y') | Key::Char('Y') => play(27.50, Chord::Major),
            Key::Char('u') | Key::Char('U') => play(30.87, Chord::Major),
            Key::Ctrl('q') => play(16.35, Chord::Minor),
            Key::Ctrl('w') => play(18.35, Chord::Minor),
            Key::Ctrl('e') => play(20.60, Chord::Minor),
            Key::Ctrl('r') => play(21.83, Chord::Minor),
            Key::Ctrl('t') => play(24.50, Chord::Minor),
            Key::Ctrl('y') => play(27.50, Chord::Minor),
            Key::Ctrl('u') => play(30.87, Chord::Minor),
            Key::Alt('q') => play(16.35, Chord::Maj7),
            Key::Alt('w') => play(18.35, Chord::Maj7),
            Key::Alt('e') => play(20.60, Chord::Maj7),
            Key::Alt('r') => play(21.83, Chord::Maj7),
            Key::Alt('t') => play(24.50, Chord::Maj7),
            Key::Alt('y') => play(27.50, Chord::Maj7),
            Key::Alt('u') => play(30.87, Chord::Maj7),
            Key::Alt('\u{11}') => play(16.35, Chord::Min7),
            Key::Alt('\u{17}') => play(18.35, Chord::Min7),
            Key::Alt('\u{5}') => play(20.60, Chord::Min7),
            Key::Alt('\u{12}') => play(21.83, Chord::Min7),
            Key::Alt('\u{14}') => play(24.50, Chord::Min7),
            Key::Alt('\u{19}') => play(27.50, Chord::Min7),
            Key::Alt('\u{15}') => play(30.87, Chord::Min7),
            Key::Backspace => {
                if let Some(stream) = &stream.lock().unwrap().as_ref() {
                    stream.pause().unwrap();
                }
            }
            x => print!("{:?}", x),
        };
        stdout.flush().unwrap();
    }

    // Wait for playback to complete.
    if let Some(stream) = &stream.lock().unwrap().as_ref() {
        stream.pause()?;
    }

    Ok(())
}

fn write_data<T>(output: &mut [T], channels: usize, signal: &mut dyn Iterator<Item = f32>)
where
    T: cpal::Sample,
{
    for frame in output.chunks_mut(channels) {
        let sample = match signal.next() {
            None => 0.0,
            Some(sample) => sample,
        };
        let value: T = cpal::Sample::from::<f32>(&sample);
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}
