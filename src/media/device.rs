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

    #[doc(alias = "MEDIA_IOC_ENUM_LINKS")]
    pub fn enum_links(&self, entity: &EntityDesc) -> io::Result<Vec<Link>> {
        // todo maybeunit?
        let mut pads = vec![unsafe { mem::zeroed() }; entity.pads as usize];
        let mut links = vec![unsafe { mem::zeroed() }; entity.links as usize];
        let mut links_enum = media_links_enum {
            entity: entity.id,
            pads: pads.as_mut_ptr(),
            links: links.as_mut_ptr(),
            reserved: [0; 4],
        };

        unsafe {
            v4l2::ioctl(
                self.handle.fd(),
                v4l2::vidioc::MEDIA_IOC_ENUM_LINKS,
                <*mut _>::cast(&mut links_enum),
            )
        }?;

        // TODO: Check if there are any pads not involved in a link?
        // (ie. cannot possibly be connected to anything?)
        // dbg!(pads.into_iter().map(|l| l.into()).collect::<Vec<Pad>>());

        Ok(links.into_iter().map(|l| l.into()).collect())
    }

    /// Enables or disables a link as per [`LinkFlags::ENABLED`]. Links marked with [`LinkFlags::IMMUTABLE`] can not be enabled or disabled.
    ///
    /// Link configuration has no side effect on other links. If an enabled link at the sink pad prevents the link from being enabled, the driver returns with an EBUSY error code.
    ///
    /// Only links marked with the [`LinkFlags::DYNAMIC`] link flag can be enabled/disabled while streaming media data. Attempting to enable or disable a streaming non-dynamic link will return an EBUSY error code.
    ///
    /// See also <https://www.kernel.org/doc/html/latest/userspace-api/media/mediactl/media-ioc-setup-link.html>
    // TODO: Make a better API for this? Bit weird that the caller can also
    // play with the link flags on their own.
    #[doc(alias = "MEDIA_IOC_SETUP_LINK")]
    pub fn setup_link(&self, mut link: Link, enabled: bool) -> io::Result<()> {
        assert!(
            !link.flags.contains(LinkFlags::IMMUTABLE),
            "Only mutable links can be modified"
        );
        // TODO: Check this assert only if the link is currently actively streaming:
        // assert!(
        //     link.flags.contains(LinkFlags::DYNAMIC),
        //     "Only dynamic links can be enabled/disabled"
        // );

        link.flags.set(LinkFlags::ENABLED, enabled);

        let mut link: media_link_desc = link.into();

        unsafe {
            v4l2::ioctl(
                self.handle.fd(),
                v4l2::vidioc::MEDIA_IOC_SETUP_LINK,
                <*mut _>::cast(&mut link),
            )
        }?;

        Ok(())
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
