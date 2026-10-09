mod mach;
mod memory;

use std::io;

use clap::{Parser, Subcommand};

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
    Cpu {
        /// Show usage per core
        #[arg(short, long)]
        per_core: bool,
        /// Sampling interval in milliseconds
        #[arg(short, long, default_value_t = 500)]
        interval: u64,
    },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Mem => print_memory()?,
        Command::Cpu { per_core, interval } => {
            println!("cpu: per_core={per_core}, interval={interval}ms");
        }
    }
    Ok(())
}

fn print_memory() -> io::Result<()> {
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

    println!(
        "\
Memory          {total:>10}   Pressure: {pressure}
  Used          {used:>10}   {used_pct:.1}%
    App         {app:>10}
    Wired       {wired:>10}
    Compressed  {compressed:>10}
  Cached        {cached:>10}
  Free          {free:>10}
Swap            {swap_used:>10}   of {swap_total} ({swap_free} free)"
    );
    Ok(())
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
