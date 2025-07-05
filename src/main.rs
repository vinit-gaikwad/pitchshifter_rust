use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::io::{self, Write};

/// Naive pitch shifter using linear interpolation
fn pitch_shift(samples: &[f32], pitch_factor: f32) -> Vec<f32> {
    let input_len = samples.len() as f32;
    let output_len = (input_len / pitch_factor) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_index = i as f32 * pitch_factor;
        let idx = src_index.floor() as usize;
        let frac = src_index.fract();

        let s1 = samples.get(idx).copied().unwrap_or(0.0);
        let s2 = samples.get(idx + 1).copied().unwrap_or(0.0);

        output.push(s1 + frac * (s2 - s1));
    }

    output
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();

    let input_device = host
        .default_input_device()
        .expect("No input device available");
    let output_device = host
        .default_output_device()
        .expect("No output device available");

    let input_config = input_device.default_input_config()?.config();
    let output_config = output_device.default_output_config()?.config();

    println!("  Input config: {:?}", input_config);
    println!(" Output config: {:?}", output_config);

    let buffer = Arc::new(Mutex::new(vec![0.0_f32; 1024]));
    let pitch_factor = Arc::new(Mutex::new(1.0_f32)); // shared control

    // Input stream
    let buffer_in = Arc::clone(&buffer);
    let input_stream = input_device.build_input_stream(
        &input_config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut buf = buffer_in.lock().unwrap();
            for (i, sample) in data.iter().enumerate().take(buf.len()) {
                buf[i] = *sample;
            }
        },
        err_fn,
        None,
    )?;

    // Output stream
    let buffer_out = Arc::clone(&buffer);
    let pitch_shared = Arc::clone(&pitch_factor);
    let output_stream = output_device.build_output_stream(
        &output_config,
        move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let buf = buffer_out.lock().unwrap();
            let factor = *pitch_shared.lock().unwrap();
            let shifted = pitch_shift(&buf, factor);
            for i in 0..output.len() {
                output[i] = *shifted.get(i).unwrap_or(&0.0);
            }
        },
        err_fn,
        None,
    )?;

    input_stream.play()?;
    output_stream.play()?;

    println!("  Enter:\n  1 = Low pitch\n  2 = High pitch\n  0 = Normal pitch\n  q = Quit");

    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim() {
            "1" => {
                let mut factor = pitch_factor.lock().unwrap();
                *factor = 0.7;
                println!(" Set to LOW pitch");
            }
            "2" => {
                let mut factor = pitch_factor.lock().unwrap();
                *factor = 1.3;
                println!(" Set to HIGH pitch");
            }
            "0" => {
                let mut factor = pitch_factor.lock().unwrap();
                *factor = 1.0;
                println!(" Set to NORMAL pitch");
            }
            "q" => {
                println!(" Exiting...");
                break;
            }
            _ => {
                println!(" Unknown command. Use 1, 2, 0, or q.");
            }
        }
    }

    Ok(())
}

fn err_fn(err: cpal::StreamError) {
    eprintln!(" Stream error: {}", err);
}
