use std::convert::TryFrom;
use std::mem::MaybeUninit;
use std::path::Path;
use std::sync::Arc;
use std::{io, mem};

use libc;

use crate::capability::Capabilities;
use crate::control::{self, Control, Description};
use crate::v4l2;
use crate::v4l2::videodev::v4l2_ext_controls;
use crate::v4l_sys::*;

/// Linux capture device abstraction
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Device {
    /// Raw handle
    handle: Arc<Handle>,
}

impl Device {
    /// Returns a capture device by index
    ///
    /// Devices are usually enumerated by the system.
    /// An index of zero thus represents the first device the system got to know about.
    ///
    /// # Arguments
    ///
    /// * `index` - Index (0: first, 1: second, ..)
    ///
    /// # Example
    ///
    /// ```
    /// use v4l::device::Device;
    /// let dev = Device::new(0);
    /// ```
    pub fn new(index: usize) -> io::Result<Self> {
        Self::with_path(format!("/dev/video{index}"))
    }

    /// Returns a capture device by path
    ///
    /// Linux device nodes are usually found in /dev/videoX or /sys/class/video4linux/videoX.
    ///
    /// # Arguments
    ///
    /// * `path` - Path (e.g. "/dev/video0")
    ///
    /// # Example
    ///
    /// ```
    /// use v4l::device::Device;
    /// let dev = Device::with_path("/dev/video0");
    /// ```
    pub fn with_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let fd = v4l2::open(path, libc::O_RDWR | libc::O_NONBLOCK)?;

        Ok(Device {
            handle: Arc::new(Handle::new(fd)),
        })
    }

    /// Returns the raw device handle
    pub fn handle(&self) -> Arc<Handle> {
        self.handle.clone()
    }

    /// Returns video4linux framework defined information such as card, driver, etc.
    pub fn query_caps(&self) -> io::Result<Capabilities> {
        let mut v4l2_caps = MaybeUninit::<v4l2_capability>::uninit();
        unsafe {
            v4l2::ioctl(
                self.handle().fd(),
                v4l2::vidioc::VIDIOC_QUERYCAP,
                v4l2_caps.as_mut_ptr().cast(),
            )?;

            Ok(Capabilities::from(v4l2_caps.assume_init()))
        }
    }

    /// Returns the supported controls for a device such as gain, focus, white balance, etc.
    pub fn query_controls(&self) -> io::Result<Vec<Description>> {
        let mut controls = Vec::new();
        unsafe {
            let mut v4l2_ctrl: v4l2_query_ext_ctrl = mem::zeroed();

            loop {
                v4l2_ctrl.id |= V4L2_CTRL_FLAG_NEXT_CTRL;
                v4l2_ctrl.id |= V4L2_CTRL_FLAG_NEXT_COMPOUND;
                match v4l2::ioctl(
                    self.handle().fd(),
                    v4l2::vidioc::VIDIOC_QUERY_EXT_CTRL,
                    &mut v4l2_ctrl as *mut _ as *mut std::os::raw::c_void,
                ) {
                    Ok(_) => {
                        // get the basic control information
                        let mut control = Description::from(v4l2_ctrl);

                        // if this is a menu control, enumerate its items
                        if control.typ == control::Type::Menu
                            || control.typ == control::Type::IntegerMenu
                        {
                            let mut items = Vec::new();

                            for i in (v4l2_ctrl.minimum..=v4l2_ctrl.maximum)
                                .step_by(v4l2_ctrl.step as usize)
                            {
                                let mut v4l2_menu = v4l2_querymenu {
                                    id: v4l2_ctrl.id,
                                    index: i as u32,
                                    ..mem::zeroed()
                                };
                                let res = v4l2::ioctl(
                                    self.handle().fd(),
                                    v4l2::vidioc::VIDIOC_QUERYMENU,
                                    &mut v4l2_menu as *mut _ as *mut std::os::raw::c_void,
                                );

                                // BEWARE OF DRAGONS!
                                // The API docs [1] state VIDIOC_QUERYMENU should may return EINVAL
                                // for some indices between minimum and maximum when an item is not
                                // supported by a driver.
                                //
                                // I have no idea why it is advertised in the first place then, but
                                // have seen this happen with a Logitech C920 HD Pro webcam.
                                // In case of errors, let's just skip the offending index.
                                //
                                // [1] https://github.com/torvalds/linux/blob/master/Documentation/userspace-api/media/v4l/vidioc-queryctrl.rst#description
                                if res.is_err() {
                                    continue;
                                }

                                let item =
                                    control::MenuItem::try_from((control.typ, v4l2_menu)).unwrap();
                                items.push((v4l2_menu.index, item));
                            }

                            control.items = Some(items);
                        }

                        controls.push(control);
                    }
                    Err(e) => {
                        if controls.is_empty() || e.kind() != io::ErrorKind::InvalidInput {
                            return Err(e);
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        Ok(controls)
    }

    /// Returns the current control value from its [`Description`]
    ///
    /// # Arguments
    ///
    /// * `desc` - Control description
    pub fn control(&self, desc: &Description) -> io::Result<Control> {
        unsafe {
            // query the actual control value
            let mut v4l2_ctrl = v4l2_ext_control {
                id: desc.id,
                ..mem::zeroed()
            };
            let mut v4l2_ctrls = v4l2_ext_controls {
                count: 1,
                controls: &mut v4l2_ctrl,
                ..mem::zeroed()
            };
            v4l2::ioctl(
                self.handle().fd(),
                v4l2::vidioc::VIDIOC_G_EXT_CTRLS,
                &mut v4l2_ctrls as *mut _ as *mut std::os::raw::c_void,
            )?;

            let value = match desc.typ {
                control::Type::Integer64 => {
                    control::Value::Integer(v4l2_ctrl.__bindgen_anon_1.value64)
                }
                control::Type::Integer | control::Type::Menu => {
                    control::Value::Integer(v4l2_ctrl.__bindgen_anon_1.value as i64)
                }
                control::Type::Boolean => {
                    control::Value::Boolean(v4l2_ctrl.__bindgen_anon_1.value == 1)
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "cannot handle control type",
                    ))
                }
            };

            Ok(Control { id: desc.id, value })
        }
    }

    /// Modifies the control value
    ///
    /// # Arguments
    ///
    /// * `ctrl` - Control to be set
    pub fn set_control(&self, ctrl: Control) -> io::Result<()> {
        self.set_controls(vec![ctrl])
    }

    /// Modifies the control values atomically
    ///
    /// # Arguments
    ///
    /// * `ctrls` - Vec of the controls to be set
    pub fn set_controls(&self, ctrls: Vec<Control>) -> io::Result<()> {
        unsafe {
            let mut control_list: Vec<v4l2_ext_control> = vec![];
            let mut class: Option<u32> = None;

            if ctrls.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "ctrls cannot be empty",
                ));
            }

            for ref ctrl in ctrls {
                let mut control = v4l2_ext_control {
                    id: ctrl.id,
                    ..mem::zeroed()
                };
                class = match class {
                    Some(c) => {
                        if c != (control.id & 0xFFFF0000) {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "All controls must be in the same class",
                            ));
                        } else {
                            Some(c)
                        }
                    }
                    None => Some(control.id & 0xFFFF0000),
                };

                match ctrl.value {
                    control::Value::None => {}
                    control::Value::Integer(val) => {
                        control.__bindgen_anon_1.value64 = val;
                        control.size = 0;
                    }
                    control::Value::Boolean(val) => {
                        control.__bindgen_anon_1.value64 = val as i64;
                        control.size = 0;
                    }
                    control::Value::String(ref val) => {
                        control.__bindgen_anon_1.string = val.as_ptr() as *mut std::os::raw::c_char;
                        control.size = val.len() as u32;
                    }
                    control::Value::CompoundU8(ref val) => {
                        control.__bindgen_anon_1.p_u8 = val.as_ptr() as *mut u8;
                        control.size = (val.len() * std::mem::size_of::<u8>()) as u32;
                    }
                    control::Value::CompoundU16(ref val) => {
                        control.__bindgen_anon_1.p_u16 = val.as_ptr() as *mut u16;
                        control.size = (val.len() * std::mem::size_of::<u16>()) as u32;
                    }
                    control::Value::CompoundU32(ref val) => {
                        control.__bindgen_anon_1.p_u32 = val.as_ptr() as *mut u32;
                        control.size = (val.len() * std::mem::size_of::<u32>()) as u32;
                    }
                    control::Value::CompoundPtr(ref val) => {
                        control.__bindgen_anon_1.ptr = val.as_ptr() as *mut std::os::raw::c_void;
                        control.size = (val.len() * std::mem::size_of::<u8>()) as u32;
                    }
                };

                control_list.push(control);
            }

            let class = class.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "failed to determine control class",
                )
            })?;

            let mut controls = v4l2_ext_controls {
                count: control_list.len() as u32,
                controls: control_list.as_mut_ptr(),

                which: class,
                ..mem::zeroed()
            };

            v4l2::ioctl(
                self.handle().fd(),
                v4l2::vidioc::VIDIOC_S_EXT_CTRLS,
                &mut controls as *mut _ as *mut std::os::raw::c_void,
            )
        }
    }

    /// Enumerate video inputs
    ///
    /// <https://www.kernel.org/doc/html/latest/userspace-api/media/v4l/vidioc-enuminput.html>

    #[doc(alias = "VIDIOC_ENUMINPUT")]
    pub fn enum_inputs(&self) -> io::Result<Vec<v4l2_input>> {
        (0..)
            .scan((), |(), index| {
                let mut input = v4l2_input {
                    index,
                    ..unsafe { mem::zeroed() }
                };

                match unsafe {
                    v4l2::ioctl(
                        self.handle().fd(),
                        v4l2::vidioc::VIDIOC_ENUMINPUT,
                        &mut input as *mut _ as *mut _,
                    )
                } {
                    Ok(()) => Some(Ok(input)),
                    Err(e) if e.kind() == io::ErrorKind::InvalidInput => None,
                    // TODO: this would keep collecting errors until we finally bail with InvalidInput...
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<io::Result<Vec<v4l2_input>>>()
    }

    /// Query the current video input
    ///
    /// Information about this video input is available via [`Self::enum_inputs()`].
    ///
    /// <https://www.kernel.org/doc/html/latest/userspace-api/media/v4l/vidioc-g-input.html>
    #[doc(alias = "VIDIOC_G_INPUT")]
    pub fn input(&self) -> io::Result<u32> {
        let mut index = MaybeUninit::<u32>::uninit();
        unsafe {
            v4l2::ioctl(
                self.handle().fd(),
                v4l2::vidioc::VIDIOC_G_INPUT,
                index.as_mut_ptr().cast(),
            )
        }?;

        Ok(unsafe { index.assume_init() })
    }

    /// Select the current video input
    ///
    /// Information about available video inputs is available via [`Self::enum_inputs()`].
    ///
    /// <https://www.kernel.org/doc/html/latest/userspace-api/media/v4l/vidioc-g-input.html>
    #[doc(alias = "VIDIOC_S_INPUT")]
    pub fn set_input(&self, mut index: u32) -> io::Result<()> {
        unsafe {
            v4l2::ioctl(
                self.handle().fd(),
                v4l2::vidioc::VIDIOC_S_INPUT,
                <*mut _>::cast(&mut index),
            )
        }
    }

    #[doc(alias = "VIDIOC_ENUMOUTPUT")]
    pub fn enum_outputs(&self) -> io::Result<Vec<v4l2_output>> {
        (0..)
            .scan((), |(), index| {
                let mut output = v4l2_output {
                    index,
                    ..unsafe { mem::zeroed() }
                };

                match unsafe {
                    v4l2::ioctl(
                        self.handle().fd(),
                        v4l2::vidioc::VIDIOC_ENUMOUTPUT,
                        &mut output as *mut _ as *mut _,
                    )
                } {
                    Ok(()) => Some(Ok(output)),
                    Err(e) if e.kind() == io::ErrorKind::InvalidInput => None,
                    // TODO: this would keep collecting errors until we finally bail with Invalidoutput...
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<io::Result<Vec<v4l2_output>>>()
    }

    /// Query the current video output
    ///
    /// Information about this video output is available via [`Self::enum_outputs()`].
    ///
    /// <https://www.kernel.org/doc/html/latest/userspace-api/media/v4l/vidioc-g-output.html>
    #[doc(alias = "VIDIOC_G_OUTPUT")]
    pub fn output(&self) -> io::Result<u32> {
        let mut index = MaybeUninit::<u32>::uninit();
        unsafe {
            v4l2::ioctl(
                self.handle().fd(),
                v4l2::vidioc::VIDIOC_G_OUTPUT,
                index.as_mut_ptr().cast(),
            )
        }?;

        Ok(unsafe { index.assume_init() })
    }

    /// Select the current video output
    ///
    /// Information about available video outputs is available via [`Self::enum_outputs()`].
    ///
    /// <https://www.kernel.org/doc/html/latest/userspace-api/media/v4l/vidioc-g-output.html>
    #[doc(alias = "VIDIOC_S_OUTPUT")]
    pub fn set_output(&self, mut index: u32) -> io::Result<()> {
        unsafe {
            v4l2::ioctl(
                self.handle().fd(),
                v4l2::vidioc::VIDIOC_S_OUTPUT,
                <*mut _>::cast(&mut index),
            )
        }
    }
}

impl io::Read for Device {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let ret = libc::read(
                self.handle().fd(),
                buf.as_mut_ptr() as *mut std::os::raw::c_void,
                buf.len(),
            );
            match ret {
                -1 => Err(io::Error::last_os_error()),
                ret => Ok(ret as usize),
            }
        }
    }
}

impl io::Write for Device {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            let ret = libc::write(
                self.handle().fd(),
                buf.as_ptr() as *const std::os::raw::c_void,
                buf.len(),
            );

            match ret {
                -1 => Err(io::Error::last_os_error()),
                ret => Ok(ret as usize),
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        // write doesn't use a buffer, so it effectively flushes with each call
        // therefore, we don't have anything to flush later
        Ok(())
    }
}

/// Device handle for low-level access.
///
/// Acquiring a handle facilitates (possibly mutating) interactions with the device.
// TODO: Replace with OwnedFd.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Handle {
    pub(crate) fd: std::os::raw::c_int,
}

impl Handle {
    fn new(fd: std::os::raw::c_int) -> Self {
        Self { fd }
    }

    /// Returns the raw file descriptor
    pub fn fd(&self) -> std::os::raw::c_int {
        self.fd
    }

    /// Polls the file descriptor for I/O events
    ///
    /// # Arguments
    ///
    /// * `events`  - The events you are interested in (e.g. POLLIN)
    ///
    /// * `timeout` - Timeout in milliseconds
    ///               A value of zero returns immedately, even if the fd is not ready.
    ///               A negative value means infinite timeout (blocking).
    pub fn poll(&self, events: i16, timeout: i32) -> io::Result<i32> {
        match unsafe {
            libc::poll(
                [libc::pollfd {
                    fd: self.fd,
                    events,
                    revents: 0,
                }]
                .as_mut_ptr(),
                1,
                timeout,
            )
        } {
            -1 => Err(io::Error::last_os_error()),
            ret => {
                // A return value of zero means that we timed out. A positive value signifies the
                // number of fds with non-zero revents fields (aka I/O activity).
                assert!(ret == 0 || ret == 1);
                Ok(ret)
            }
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        v4l2::close(self.fd).unwrap();
    }
}
