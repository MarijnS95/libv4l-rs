use std::{ffi::CStr, io};

use v4l::{capability::Flags, prelude::*};

fn main() -> io::Result<()> {
    let path = "/dev/video0";
    println!("Using device: {}\n", path);

    let dev = Device::with_path(path)?;

    let caps = dev.query_caps()?;
    println!("Device capabilities:\n{}", caps);

    let controls = dev.query_controls()?;
    println!("Device controls:");
    let mut max_name_len = 0;
    for ctrl in &controls {
        if ctrl.name.len() > max_name_len {
            max_name_len = ctrl.name.len();
        }
    }
    for ctrl in controls {
        println!(
            "{:indent$} : [{}, {}]",
            ctrl.name,
            ctrl.minimum,
            ctrl.maximum,
            indent = max_name_len
        );
    }

    if caps.capabilities.contains(Flags::VIDEO_CAPTURE) {
        let inputs = dev.enum_inputs()?;
        println!("{} possible input(s)", inputs.len());
        if let Ok(input) = dbg!(dev.input()) {
            let input = inputs[input as usize];
            println!(
                "Current input name: {}",
                CStr::from_bytes_until_nul(&input.name)
                    .unwrap()
                    .to_string_lossy()
            );
            println!("Current input type: {}", input.type_);
            println!("Current input std: {}", input.std);
        }
    }

    if caps.capabilities.contains(Flags::VIDEO_OUTPUT) {
        let outputs = dev.enum_outputs()?;
        println!("{} possible output(s)", outputs.len());
        if let Ok(output) = dbg!(dev.output()) {
            let output = outputs[output as usize];
            println!(
                "Current output name: {}",
                CStr::from_bytes_until_nul(&output.name)
                    .unwrap()
                    .to_string_lossy()
            );
            println!("Current output type: {}", output.type_);
            println!("Current output std: {}", output.std);
        }
    }

    Ok(())
}
