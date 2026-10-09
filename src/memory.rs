use std::ffi::CStr;
use std::fmt;
use std::io;
use std::mem::{self, MaybeUninit};
use std::ptr;

use crate::mach::{self, HostPort};

pub struct MemoryInfo {
    pub total: u64,
    pub used: u64,
    pub app: u64,
    pub wired: u64,
    pub compressed: u64,
    pub cached: u64,
    pub free: u64,
    pub swap: Swap,
    pub pressure: Pressure,
}

pub struct Swap {
    pub total: u64,
    pub used: u64,
    pub free: u64,
}

pub enum Pressure {
    Normal,
    Warning,
    Critical,
    Unknown(i32),
}

impl Pressure {
    fn from_level(level: i32) -> Self {
        match level {
            1 => Pressure::Normal,
            2 => Pressure::Warning,
            4 => Pressure::Critical,
            other => Pressure::Unknown(other),
        }
    }
}

impl fmt::Display for Pressure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Pressure::Normal => write!(f, "Normal"),
            Pressure::Warning => write!(f, "Warning"),
            Pressure::Critical => write!(f, "Critical"),
            Pressure::Unknown(level) => write!(f, "Unknown ({level})"),
        }
    }
}

pub fn read() -> io::Result<MemoryInfo> {
    let total = sysctl::<u64>(c"hw.memsize")?;
    let swap = sysctl::<libc::xsw_usage>(c"vm.swapusage")?;
    let level = sysctl::<i32>(c"kern.memorystatus_vm_pressure_level")?;

    let host = HostPort::new()?;
    let vm = host.vm_statistics()?;
    let page_size = mach::page_size();
    let pages = |count: u32| u64::from(count) * page_size;

    let app = pages(vm.internal_page_count.saturating_sub(vm.purgeable_count));
    let wired = pages(vm.wire_count);
    let compressed = pages(vm.compressor_page_count);

    Ok(MemoryInfo {
        total,
        used: app + wired + compressed,
        app,
        wired,
        compressed,
        cached: pages(vm.external_page_count) + pages(vm.purgeable_count),
        free: pages(vm.free_count),
        swap: Swap {
            total: swap.xsu_total,
            used: swap.xsu_used,
            free: swap.xsu_avail,
        },
        pressure: Pressure::from_level(level),
    })
}

fn sysctl<T: Copy>(name: &CStr) -> io::Result<T> {
    let mut value = MaybeUninit::<T>::uninit();
    let mut size = size_of::<T>();

    // SAFETY: name은 NUL로 끝나는 C 문자열이고, value는 size 바이트만큼 쓸 수 있는 공간이다.
    let ret = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            value.as_mut_ptr().cast(),
            &mut size,
            ptr::null_mut(),
            0,
        )
    };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    if size != size_of::<T>() {
        return Err(io::Error::other(format!(
            "{name:?}: expected {} bytes, got {size}",
            mem::size_of::<T>()
        )));
    }

    // SAFETY: 커널이 T 크기만큼 정확히 채웠고, 이 모듈에서 쓰는 T는 모든 비트 패턴이 유효한 정수 또는 정수로만 된 구조체다.
    Ok(unsafe { value.assume_init() })
}
