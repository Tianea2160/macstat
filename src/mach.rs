use std::ffi::CStr;
use std::io;
use std::mem::{self, MaybeUninit};

use libc::{KERN_SUCCESS, integer_t, kern_return_t, mach_port_t, vm_statistics64};
use mach2::mach_init::mach_host_self;
use mach2::mach_port::mach_port_deallocate;
use mach2::traps::mach_task_self;
use mach2::vm_page_size::vm_kernel_page_size;

pub struct HostPort(mach_port_t);

impl HostPort {
    pub fn new() -> io::Result<Self> {
        // SAFETY: Takes no arguments and returns the name of this process's send right to the host port.
        let port = unsafe { mach_host_self() };
        if port == 0 {
            return Err(io::Error::other("mach_host_self returned MACH_PORT_NULL"));
        }
        Ok(HostPort(port))
    }

    pub fn vm_statistics(&self) -> io::Result<vm_statistics64> {
        const REQUIRED_BYTES: usize =
            mem::offset_of!(vm_statistics64, internal_page_count) + size_of::<u32>();

        let mut stats = MaybeUninit::<vm_statistics64>::zeroed();
        let mut count = libc::HOST_VM_INFO64_COUNT;

        // SAFETY: stats is large enough for count integer_t values, and count is initialized to that capacity.
        let ret = unsafe {
            libc::host_statistics64(
                self.0,
                libc::HOST_VM_INFO64,
                stats.as_mut_ptr().cast(),
                &mut count,
            )
        };
        if ret != KERN_SUCCESS {
            return Err(kern_error("host_statistics64", ret));
        }

        let filled = count as usize * size_of::<integer_t>();
        if filled < REQUIRED_BYTES {
            return Err(io::Error::other(format!(
                "host_statistics64: kernel filled {filled} bytes, need {REQUIRED_BYTES}"
            )));
        }

        // SAFETY: zeroed() initialized every byte and the kernel overwrote the leading part. All fields are integers, so any bit pattern is valid.
        Ok(unsafe { stats.assume_init() })
    }
}

impl Drop for HostPort {
    fn drop(&mut self) {
        // SAFETY: self.0 is the send right obtained in new(), and HostPort is never duplicated, so it is released exactly once.
        unsafe {
            mach_port_deallocate(mach_task_self(), self.0);
        }
    }
}

pub fn page_size() -> u64 {
    // SAFETY: Only reads a global that is set at process startup and never changes afterward.
    unsafe { vm_kernel_page_size as u64 }
}

fn kern_error(call: &str, code: kern_return_t) -> io::Error {
    // SAFETY: mach_error_string always returns a pointer to a static NUL-terminated string.
    let message = unsafe { CStr::from_ptr(libc::mach_error_string(code)) };
    io::Error::other(format!("{call}: {} ({code})", message.to_string_lossy()))
}
