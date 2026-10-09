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
        // SAFETY: 인자 없이 현재 프로세스가 가진 host 포트의 send right 이름을 돌려준다.
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

        // SAFETY: stats는 count개의 integer_t를 담을 수 있는 크기이고, count는 그 크기로 초기화되어 있다.
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

        // SAFETY: zeroed()로 전체가 0으로 초기화됐고 커널이 앞부분을 덮어썼다. 모든 필드가 정수라 어떤 비트 패턴도 유효하다.
        Ok(unsafe { stats.assume_init() })
    }
}

impl Drop for HostPort {
    fn drop(&mut self) {
        // SAFETY: self.0은 new()에서 받은 send right이고, HostPort는 복제되지 않으므로 정확히 한 번만 반납된다.
        unsafe {
            mach_port_deallocate(mach_task_self(), self.0);
        }
    }
}

pub fn page_size() -> u64 {
    // SAFETY: 프로세스 시작 시 설정된 뒤 바뀌지 않는 전역 변수를 읽기만 한다.
    unsafe { vm_kernel_page_size as u64 }
}

fn kern_error(call: &str, code: kern_return_t) -> io::Error {
    // SAFETY: mach_error_string은 항상 NUL로 끝나는 정적 문자열을 가리키는 포인터를 반환한다.
    let message = unsafe { CStr::from_ptr(libc::mach_error_string(code)) };
    io::Error::other(format!("{call}: {} ({code})", message.to_string_lossy()))
}
