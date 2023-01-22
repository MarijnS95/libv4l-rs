use std::io;

use v4l::{control::Value, prelude::*};

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
        if ctrl.typ == v4l::control::Type::CtrlClass {
            println!();
            println!("{}", ctrl.name);
            println!();
        } else if let Some(items) = &ctrl.items {
            assert_eq!(ctrl.typ, v4l::control::Type::Menu);
            let value = dev.control(&ctrl)?;
            let Value::Integer(index) = value.value else {
                panic!("Menu value must be integer index")
            };
            let item = items.iter().find(|(i, _)| *i == index as u32).expect("");
            // dbg!(&value, items);
            println!(
                "\t{:indent$} : {} = {}",
                ctrl.name,
                index,
                item.1,
                indent = max_name_len
            );
        } else {
            let value = dev.control(&ctrl)?;
            println!(
                "\t{:indent$} : [{}, {}] = {:?}",
                ctrl.name,
                ctrl.minimum,
                ctrl.maximum,
                value.value,
                indent = max_name_len
            );
        }
    }

    Ok(())
}
