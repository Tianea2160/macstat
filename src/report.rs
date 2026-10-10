use std::fmt;

use crate::cpu::{CpuUsage, Usage};
use crate::memory::MemoryInfo;
use crate::style::Stylize;

pub struct MemoryReport<'a>(pub &'a MemoryInfo);

impl fmt::Display for MemoryReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let info = self.0;

        let memory = MemRow("Memory".green(), info.total);
        let pressure = &info.pressure;
        let used = MemRow("  Used", info.used);
        let used_pct = Percent(info.used as f64 / info.total as f64 * 100.0);
        let app = MemRow("    App", info.app);
        let wired = MemRow("    Wired", info.wired);
        let compressed = MemRow("    Compressed", info.compressed);
        let cached = MemRow("  Cached", info.cached);
        let free = MemRow("  Free", info.free);
        let swap = MemRow("Swap".green(), info.swap.used);
        let swap_total = Bytes(info.swap.total);
        let swap_free = Bytes(info.swap.free);

        write!(
            f,
            "\
{memory}   Pressure: {pressure}
{used}   {used_pct}
{app}
{wired}
{compressed}
{cached}
{free}
{swap}   of {swap_total} ({swap_free} free)"
        )
    }
}

pub struct CpuReport<'a> {
    pub usage: &'a CpuUsage,
    pub per_core: bool,
}

impl fmt::Display for CpuReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let title = "CPU".green();
        let cores = format!("{} cores", self.usage.cores.len());
        let header = CpuRow("", ["System", "User", "Idle"]);
        let total = usage_row("Total", &self.usage.total);

        write!(
            f,
            "\
{title:<CPU_LABEL$}{cores:>CPU_COLUMN$}
{header}
{total}"
        )?;
        if self.per_core {
            for (i, core) in self.usage.cores.iter().enumerate() {
                write!(f, "\n{}", usage_row(format!("  cpu{i}"), core))?;
            }
        }
        Ok(())
    }
}

const MEM_LABEL: usize = 16;
const MEM_VALUE: usize = 10;
const CPU_LABEL: usize = 7;
const CPU_COLUMN: usize = 9;

struct MemRow<L>(L, u64);

impl<L: fmt::Display> fmt::Display for MemRow<L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let MemRow(label, bytes) = self;
        let value = Bytes(*bytes);
        write!(f, "{label:<MEM_LABEL$}{value:>MEM_VALUE$}")
    }
}

struct CpuRow<L, V>(L, [V; 3]);

impl<L: fmt::Display, V: fmt::Display> fmt::Display for CpuRow<L, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let CpuRow(label, [system, user, idle]) = self;
        write!(
            f,
            "{label:<CPU_LABEL$}{system:>CPU_COLUMN$}{user:>CPU_COLUMN$}{idle:>CPU_COLUMN$}"
        )
    }
}

fn usage_row<L>(label: L, usage: &Usage) -> CpuRow<L, Percent> {
    CpuRow(
        label,
        [
            Percent(usage.system),
            Percent(usage.user),
            Percent(usage.idle),
        ],
    )
}

struct Bytes(u64);

impl fmt::Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

        let mut value = self.0 as f64;
        let mut unit = 0;
        while value >= 1024.0 && unit < UNITS.len() - 1 {
            value /= 1024.0;
            unit += 1;
        }

        if unit == 0 {
            f.pad(&format!("{} B", self.0))
        } else {
            f.pad(&format!("{value:.2} {}", UNITS[unit]))
        }
    }
}

struct Percent(f64);

impl fmt::Display for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&format!("{:.1}%", self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_below_one_kib_stay_in_bytes() {
        assert_eq!(Bytes(0).to_string(), "0 B");
        assert_eq!(Bytes(1023).to_string(), "1023 B");
    }

    #[test]
    fn bytes_pick_largest_fitting_unit() {
        assert_eq!(Bytes(1024).to_string(), "1.00 KiB");
        assert_eq!(Bytes(3 << 29).to_string(), "1.50 GiB");
    }

    #[test]
    fn bytes_and_percent_respect_width() {
        assert_eq!(format!("{:>10}", Bytes(1024)), "  1.00 KiB");
        assert_eq!(format!("{:>8}", Percent(3.21)), "    3.2%");
    }

    #[test]
    fn mem_row_aligns_label_and_value() {
        assert_eq!(
            MemRow("  Used", 1024).to_string(),
            "  Used            1.00 KiB"
        );
    }

    #[test]
    fn cpu_row_aligns_header_and_usage() {
        let usage = Usage {
            system: 3.21,
            user: 10.0,
            idle: 86.79,
        };
        assert_eq!(
            CpuRow("", ["System", "User", "Idle"]).to_string(),
            "          System     User     Idle"
        );
        assert_eq!(
            usage_row("Total", &usage).to_string(),
            "Total       3.2%    10.0%    86.8%"
        );
    }
}
