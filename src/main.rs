mod cpu;
mod mach;
mod memory;
mod report;
mod style;

use std::io;
use std::time::Duration;

use clap::{Args, Parser, Subcommand};

use crate::report::{CpuReport, MemoryReport};

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
        Command::Mem => println!("{}", MemoryReport(&memory::read()?)),
        Command::Cpu(args) => {
            let usage = measure_cpu(&args)?;
            println!("{}", cpu_report(&usage, &args));
        }
        Command::All(args) => {
            let usage = measure_cpu(&args)?;
            let memory = memory::read()?;
            println!("{}\n\n{}", cpu_report(&usage, &args), MemoryReport(&memory));
        }
    }
    Ok(())
}

fn measure_cpu(args: &CpuArgs) -> io::Result<cpu::CpuUsage> {
    cpu::measure(Duration::from_millis(args.interval))
}

fn cpu_report<'a>(usage: &'a cpu::CpuUsage, args: &CpuArgs) -> CpuReport<'a> {
    CpuReport {
        usage,
        per_core: args.per_core,
    }
}
