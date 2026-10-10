use std::io;
use std::ops::Add;
use std::thread;
use std::time::Duration;

use libc::{
    CPU_STATE_IDLE, CPU_STATE_NICE, CPU_STATE_SYSTEM, CPU_STATE_USER, processor_cpu_load_info,
};

use crate::mach::HostPort;

pub struct CpuUsage {
    pub total: Usage,
    pub cores: Vec<Usage>,
}

pub struct Usage {
    pub system: f64,
    pub user: f64,
    pub idle: f64,
}

#[derive(Clone, Copy)]
struct Ticks {
    user: u32,
    system: u32,
    idle: u32,
    nice: u32,
}

impl From<&processor_cpu_load_info> for Ticks {
    fn from(info: &processor_cpu_load_info) -> Self {
        let ticks = info.cpu_ticks;
        Ticks {
            user: ticks[CPU_STATE_USER as usize],
            system: ticks[CPU_STATE_SYSTEM as usize],
            idle: ticks[CPU_STATE_IDLE as usize],
            nice: ticks[CPU_STATE_NICE as usize],
        }
    }
}

impl Ticks {
    fn since(self, earlier: Ticks) -> Delta {
        let diff = |now: u32, before: u32| u64::from(now.wrapping_sub(before));
        Delta {
            user: diff(self.user, earlier.user) + diff(self.nice, earlier.nice),
            system: diff(self.system, earlier.system),
            idle: diff(self.idle, earlier.idle),
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Delta {
    user: u64,
    system: u64,
    idle: u64,
}

impl Add for Delta {
    type Output = Delta;

    fn add(self, other: Delta) -> Delta {
        Delta {
            user: self.user + other.user,
            system: self.system + other.system,
            idle: self.idle + other.idle,
        }
    }
}

impl Delta {
    fn total(self) -> u64 {
        self.user + self.system + self.idle
    }

    fn usage(self) -> Usage {
        let total = self.total();
        let percent = |part: u64| {
            if total == 0 {
                0.0
            } else {
                part as f64 * 100.0 / total as f64
            }
        };
        Usage {
            system: percent(self.system),
            user: percent(self.user),
            idle: percent(self.idle),
        }
    }
}

fn sample(host: &HostPort) -> io::Result<Vec<Ticks>> {
    let load = host.processor_load()?;
    Ok(load.cpus().iter().map(Ticks::from).collect())
}

pub fn measure(interval: Duration) -> io::Result<CpuUsage> {
    let host = HostPort::new()?;
    let before = sample(&host)?;
    thread::sleep(interval);
    let after = sample(&host)?;

    if before.len() != after.len() {
        return Err(io::Error::other(format!(
            "CPU count changed from {} to {} while sampling",
            before.len(),
            after.len()
        )));
    }

    let deltas: Vec<Delta> = after
        .iter()
        .zip(&before)
        .map(|(now, earlier)| now.since(*earlier))
        .collect();
    let total = deltas.iter().fold(Delta::default(), |sum, d| sum + *d);
    if total.total() == 0 {
        return Err(io::Error::other(format!(
            "no CPU ticks elapsed in {}ms; use a longer interval",
            interval.as_millis()
        )));
    }

    Ok(CpuUsage {
        total: total.usage(),
        cores: deltas.into_iter().map(Delta::usage).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticks(user: u32, system: u32, idle: u32, nice: u32) -> Ticks {
        Ticks {
            user,
            system,
            idle,
            nice,
        }
    }

    #[test]
    fn since_handles_wraparound() {
        let earlier = ticks(u32::MAX - 2, 0, 0, 0);
        let now = ticks(5, 0, 0, 0);
        assert_eq!(now.since(earlier).user, 8);
    }

    #[test]
    fn since_counts_nice_as_user() {
        let earlier = ticks(10, 0, 0, 3);
        let now = ticks(20, 0, 0, 7);
        assert_eq!(now.since(earlier).user, 14);
    }

    #[test]
    fn usage_splits_by_share_of_total() {
        let delta = Delta {
            user: 30,
            system: 10,
            idle: 60,
        };
        let usage = delta.usage();
        assert_eq!(usage.user, 30.0);
        assert_eq!(usage.system, 10.0);
        assert_eq!(usage.idle, 60.0);
    }

    #[test]
    fn usage_of_empty_delta_is_zero() {
        let usage = Delta::default().usage();
        assert_eq!(usage.user + usage.system + usage.idle, 0.0);
    }

    #[test]
    fn total_sums_ticks_before_dividing() {
        let busy = Delta {
            user: 90,
            system: 0,
            idle: 10,
        };
        let quiet = Delta {
            user: 0,
            system: 0,
            idle: 300,
        };
        let usage = (busy + quiet).usage();
        assert_eq!(usage.user, 22.5);
        assert_eq!(usage.idle, 77.5);
    }
}
