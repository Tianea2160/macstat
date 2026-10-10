use std::ffi::CStr;
use std::io;
use std::mem::{self, MaybeUninit};
use std::{ptr, slice};

use libc::{
    CPU_STATE_MAX, KERN_SUCCESS, PROCESSOR_CPU_LOAD_INFO, integer_t, kern_return_t,
    mach_msg_type_number_t, mach_port_t, natural_t, processor_cpu_load_info,
    processor_info_array_t, vm_address_t, vm_statistics64,
};
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

    pub fn processor_load(&self) -> io::Result<ProcessorLoad> {
        let mut cpu_count: natural_t = 0;
        let mut info: processor_info_array_t = ptr::null_mut();
        let mut info_count: mach_msg_type_number_t = 0;

        // SAFETY: All three out-pointers point to live locals of exactly the types the kernel writes.
        let ret = unsafe {
            libc::host_processor_info(
                self.0,
                PROCESSOR_CPU_LOAD_INFO,
                &mut cpu_count,
                &mut info,
                &mut info_count,
            )
        };
        if ret != KERN_SUCCESS {
            return Err(kern_error("host_processor_info", ret));
        }
        if info.is_null() {
            return Err(io::Error::other(
                "host_processor_info returned a null array",
            ));
        }

        let load = ProcessorLoad {
            info,
            info_count,
            cpu_count: cpu_count as usize,
        };
        let expected = load.cpu_count * CPU_STATE_MAX as usize;
        if load.info_count as usize != expected {
            return Err(io::Error::other(format!(
                "host_processor_info: got {} values for {} CPUs, expected {expected}",
                load.info_count, load.cpu_count
            )));
        }
        Ok(load)
    }
}

pub struct ProcessorLoad {
    info: processor_info_array_t,
    info_count: mach_msg_type_number_t,
    cpu_count: usize,
}

impl ProcessorLoad {
    pub fn cpus(&self) -> &[processor_cpu_load_info] {
        // SAFETY: info is non-null and holds cpu_count * CPU_STATE_MAX integer_t values (checked in processor_load). processor_cpu_load_info is a repr(C) array of CPU_STATE_MAX c_uint with the same size and alignment, and any bit pattern is valid.
        unsafe { slice::from_raw_parts(self.info.cast(), self.cpu_count) }
    }
}

impl Drop for ProcessorLoad {
    fn drop(&mut self) {
        let size = self.info_count as usize * size_of::<integer_t>();
        // SAFETY: info was allocated in this task by host_processor_info with info_count integer_t values, and ProcessorLoad is never duplicated, so it is freed exactly once.
        unsafe {
            libc::vm_deallocate(mach_task_self(), self.info as vm_address_t, size);
        }
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
