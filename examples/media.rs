use std::io;

use v4l::media::Device;

fn main() -> io::Result<()> {
    let dev = Device::new(0)?;
    dbg!(dev.device_info()?);
    for e in dev.enum_entities()? {
        dbg!(&e.name, e.device_path(), dev.enum_links(&e)?);
        if let Some(d) = e.device()? {
            dbg!(d.query_caps()?);
        }
    }
    Ok(())
}
