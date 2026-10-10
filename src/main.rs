mod cpu;
mod mach;
mod memory;
mod style;

use std::io;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};

use crate::style::Stylize;

#[derive(Parser)]
#[command(version, about = "macOS 메모리·CPU 사용량 조회")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show memory usage
    Mem,
    /// Show CPU usage
    Cpu(CpuArgs),
    /// Show CPU and memory usage together
    All(CpuArgs),
}

#[derive(Args)]
struct CpuArgs {
    /// Show usage per core
    #[arg(short, long)]
    per_core: bool,
    /// Sampling interval in milliseconds
    #[arg(short, long, default_value_t = 500, value_parser = clap::value_parser!(u64).range(1..))]
    interval: u64,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Mem => println!("{}", format_memory()?),
        Command::Cpu(args) => println!("{}", format_cpu(&args)?),
        Command::All(args) => {
            let cpu = format_cpu(&args)?;
            let memory = format_memory()?;
            println!("{cpu}\n\n{memory}");
        }
    }
    Ok(())
}

fn format_memory() -> io::Result<String> {
    let info = memory::read()?;

    let total = format_bytes(info.total);
    let used = format_bytes(info.used);
    let used_pct = info.used as f64 / info.total as f64 * 100.0;
    let app = format_bytes(info.app);
    let wired = format_bytes(info.wired);
    let compressed = format_bytes(info.compressed);
    let cached = format_bytes(info.cached);
    let free = format_bytes(info.free);
    let swap_used = format_bytes(info.swap.used);
    let swap_total = format_bytes(info.swap.total);
    let swap_free = format_bytes(info.swap.free);
    let pressure = info.pressure;
    let title = "Memory".green();
    let swap = "Swap".green();

    Ok(format!(
        "\
{title}          {total:>10}   Pressure: {pressure}
  Used          {used:>10}   {used_pct:.1}%
    App         {app:>10}
    Wired       {wired:>10}
    Compressed  {compressed:>10}
  Cached        {cached:>10}
  Free          {free:>10}
{swap}            {swap_used:>10}   of {swap_total} ({swap_free} free)"
    ))
}

fn format_cpu(args: &CpuArgs) -> io::Result<String> {
    let usage = cpu::measure(Duration::from_millis(args.interval))?;
    let cpus = usage.cores.len();

    let mut rows = vec![format_usage("Total", &usage.total)];
    if args.per_core {
        rows.extend(
            usage
                .cores
                .iter()
                .enumerate()
                .map(|(i, core)| format_usage(&format!("  cpu{i}"), core)),
        );
    }
    let rows = rows.join("\n");
    let title = "CPU".green();

    Ok(format!(
        "\
{title}       {cpus} cores
          System     User     Idle
{rows}"
    ))
}

fn format_usage(label: &str, usage: &cpu::Usage) -> String {
    format!(
        "{label:<8} {:>6.1}%  {:>6.1}%  {:>6.1}%",
        usage.system, usage.user, usage.idle
    )
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}
