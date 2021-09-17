// API Plans:
// Move top-level Device into video::Device?
// Map devices to their "owning" media device, ie:
//     impl Device fn media_device() -> Option<media::Device> {..} }

mod device;
mod entity;
mod link;
mod request;

pub use device::{Device, DeviceInfo};
pub use entity::{EntityDesc, EntityType};
pub use link::{Link, LinkFlags, Pad, PadFlags};
pub use request::Request;

// TODO: Move version helper elsewhere, reuse for v4l2_capability

/// Version number MAJOR.MINOR.PATCH
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl std::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl From<u32> for Version {
    fn from(v: u32) -> Self {
        Self {
            major: ((v >> 16) & 0xff) as u8,
            minor: ((v >> 8) & 0xff) as u8,
            patch: (v & 0xff) as u8,
        }
    }
}
