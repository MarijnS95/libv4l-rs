use crate::{device::Handle, v4l2};
use std::time::Duration;
use std::{io, sync::Arc};

/// Holds a request queue. The caller can enqueue commands on this queue
/// by placing its fd in the `request_fd` field of certain V4L2 ioctls,
/// queue it by calling [`Self::queue()`] and blocking for completion
/// by polling on the fd or dequeueing capture buffers directly.
///
/// <https://www.kernel.org/doc/html/latest/userspace-api/media/mediactl/request-api.html>
pub struct Request {
    handle: Arc<Handle>,
}

impl Request {
    pub(crate) fn new(handle: Handle) -> Self {
        Self {
            handle: Arc::new(handle),
        }
    }

    /// Queue this request
    #[doc(alias = "MEDIA_REQUEST_IOC_QUEUE")]
    pub fn queue(&self) -> io::Result<()> {
        unsafe {
            v4l2::ioctl(
                self.handle.fd(),
                v4l2::vidioc::MEDIA_REQUEST_IOC_QUEUE,
                // TODO: This variadic ioctl shouldn't receive an argument at all!
                std::ptr::null_mut(),
            )
        }
    }

    /// Reinitializes the request, clearing any existing data
    ///
    /// This is a convenience to not have to `Drop` and reallocate a new request.
    #[doc(alias = "MEDIA_REQUEST_IOC_REINIT")]
    pub fn reinit(&mut self) -> io::Result<()> {
        unsafe {
            v4l2::ioctl(
                self.handle.fd(),
                v4l2::vidioc::MEDIA_REQUEST_IOC_REINIT,
                // TODO: This variadic ioctl shouldn't receive an argument at all!
                std::ptr::null_mut(),
            )
        }
    }

    // TODO: Provide simple poll function
    pub fn poll(&self, _timeout: Duration) -> io::Result</* poll successful */ ()> {
        todo!()
    }
}
