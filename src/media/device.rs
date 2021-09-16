use super::*;
use crate::v4l_sys::*;
use crate::wrap_c_str_slice_until_nul;
use crate::{device::Handle, v4l2};
use std::{io, mem, path::Path, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Device {
    /// Raw handle
    handle: Arc<Handle>,
}

impl Device {
    pub fn new(index: usize) -> io::Result<Self> {
        Self::with_path(format!("/dev/media{index}"))
    }

    pub fn with_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let fd = v4l2::open(path, libc::O_RDWR)?;

        Ok(Device {
            handle: Arc::new(Handle { fd }),
        })
    }

    #[doc(alias = "MEDIA_IOC_DEVICE_INFO")]
    pub fn device_info(&self) -> io::Result<DeviceInfo> {
        let mut info = mem::MaybeUninit::<media_device_info>::uninit();
        unsafe {
            v4l2::ioctl(
                self.handle.fd(),
                v4l2::vidioc::MEDIA_IOC_DEVICE_INFO,
                info.as_mut_ptr().cast(),
            )
        }?;
        Ok(unsafe { info.assume_init() }.into())
    }

    #[doc(alias = "MEDIA_IOC_ENUM_ENTITIES")]
    pub fn enum_entities(&self) -> io::Result<Vec<EntityDesc>> {
        // Hold this struct as iterator, the ioctl overwrites its fields
        let desc: media_entity_desc = unsafe { mem::zeroed() };
        (0..)
            .scan(desc, |desc, _| {
                // In order to get the next item, reuse the id from the
                // previous iteration and OR this bit into it:
                desc.id |= MEDIA_ENT_ID_FLAG_NEXT;
                match unsafe {
                    v4l2::ioctl(
                        self.handle.fd(),
                        v4l2::vidioc::MEDIA_IOC_ENUM_ENTITIES,
                        <*mut _>::cast(desc),
                    )
                } {
                    Ok(()) => Some(Ok((*desc).into())),
                    // Iteration ends when "id" is considered an invalid argument
                    Err(e) if e.kind() == io::ErrorKind::InvalidInput => None,
                    // TODO: This keeps the iteration going, potentially endlessly if a non-InvalidInput error keeps being returned?
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<io::Result<Vec<_>>>()
    }
}

/// Device info
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[doc(alias = "media_device_info")]
pub struct DeviceInfo {
    /// Driver name, e.g. uvc for usb video class devices
    pub driver: String,
    /// Card name
    pub model: String,
    /// Serial number
    pub serial: String,
    /// Bus name, e.g. USB or PCI
    pub bus: String,
    pub media_version: Version,
    pub hw_revision: u32,
    pub driver_version: Version,
}

impl From<media_device_info> for DeviceInfo {
    fn from(info: media_device_info) -> Self {
        Self {
            driver: wrap_c_str_slice_until_nul(&info.driver)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            model: wrap_c_str_slice_until_nul(&info.model)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            serial: wrap_c_str_slice_until_nul(&info.serial)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            bus: wrap_c_str_slice_until_nul(&info.bus_info)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            media_version: info.media_version.into(),
            hw_revision: info.hw_revision,
            driver_version: info.driver_version.into(),
        }
    }
}
