use std::time::Duration;

use macstat::cpu;
use macstat::memory::{self, Pressure};

#[repr(C)]
#[derive(Default)]
pub struct MacstatMemory {
    pub ok: bool,
    pub total: u64,
    pub used: u64,
    pub app: u64,
    pub wired: u64,
    pub compressed: u64,
    pub cached: u64,
    pub free: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub swap_free: u64,
    pub pressure: i32,
}

#[repr(C)]
#[derive(Default)]
pub struct MacstatCpu {
    pub ok: bool,
    pub cores: u32,
    pub system: f64,
    pub user: f64,
    pub idle: f64,
}

// SAFETY: The symbol name is prefixed with `macstat_` and is defined only here,
// so it cannot clash with another exported symbol.
#[unsafe(no_mangle)]
pub extern "C" fn macstat_memory_read() -> MacstatMemory {
    let Ok(info) = memory::read() else {
        return MacstatMemory::default();
    };
    MacstatMemory {
        ok: true,
        total: info.total,
        used: info.used,
        app: info.app,
        wired: info.wired,
        compressed: info.compressed,
        cached: info.cached,
        free: info.free,
        swap_total: info.swap.total,
        swap_used: info.swap.used,
        swap_free: info.swap.free,
        pressure: pressure_level(&info.pressure),
    }
}

// SAFETY: The symbol name is prefixed with `macstat_` and is defined only here,
// so it cannot clash with another exported symbol.
#[unsafe(no_mangle)]
pub extern "C" fn macstat_cpu_measure(interval_ms: u64) -> MacstatCpu {
    let Ok(usage) = cpu::measure(Duration::from_millis(interval_ms)) else {
        return MacstatCpu::default();
    };
    MacstatCpu {
        ok: true,
        cores: u32::try_from(usage.cores.len()).unwrap_or(u32::MAX),
        system: usage.total.system,
        user: usage.total.user,
        idle: usage.total.idle,
    }
}

fn pressure_level(pressure: &Pressure) -> i32 {
    match pressure {
        Pressure::Normal => 1,
        Pressure::Warning => 2,
        Pressure::Critical => 4,
        Pressure::Unknown(level) => *level,
    }
}

#[cfg(test)]
mod tests {
    use std::mem::{offset_of, size_of};

    use super::*;

    #[test]
    fn memory_layout_matches_header() {
        assert_eq!(offset_of!(MacstatMemory, total), 8);
        assert_eq!(offset_of!(MacstatMemory, pressure), 88);
        assert_eq!(size_of::<MacstatMemory>(), 96);
    }

    #[test]
    fn cpu_layout_matches_header() {
        assert_eq!(offset_of!(MacstatCpu, cores), 4);
        assert_eq!(offset_of!(MacstatCpu, system), 8);
        assert_eq!(size_of::<MacstatCpu>(), 32);
    }

    #[test]
    fn reads_memory_and_cpu() {
        let memory = macstat_memory_read();
        assert!(memory.ok);
        assert!(memory.used <= memory.total);

        let cpu = macstat_cpu_measure(100);
        assert!(cpu.ok);
        assert!(cpu.cores > 0);
    }
}
