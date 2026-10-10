# macstat

macstat reads macOS memory and CPU usage by calling kernel APIs (`sysctl`, Mach) directly from Rust through FFI. It comes in two forms:

- **CLI**: prints memory and CPU usage in the terminal
- **Menu bar app**: a SwiftUI `MenuBarExtra` that shows live CPU usage in the menu bar and a detail panel on click. It calls the same Rust code through a C ABI

macOS only. This is a learning project for Rust.

## Requirements

- macOS 14 or later (the menu bar app uses SwiftUI `MenuBarExtra` and `@Observable`)
- Rust. The version is pinned in `rust-toolchain.toml`, and `rustup` installs it automatically on the first `cargo` command
- Swift 6 for the menu bar app. The Command Line Tools are enough (`xcode-select --install`); Xcode is not required

Check the toolchains:

```bash
cargo --version
swift --version
```

## Project layout

```
macstat/
├─ Cargo.toml              Workspace root and the `macstat` package (`default-members` includes `ffi`)
├─ src/
│  ├─ lib.rs               Library: data collection (`cpu`, `memory`)
│  ├─ cpu.rs, memory.rs    Kernel calls and calculations
│  ├─ mach.rs              Mach port and buffer wrappers (private)
│  ├─ main.rs              CLI entry point (clap)
│  └─ report.rs, style.rs  Terminal formatting
├─ ffi/                    `macstat-ffi`: exports the library as C functions (staticlib)
│  ├─ src/lib.rs
│  └─ include/macstat.h    Hand-written C header that matches the Rust structs
└─ macos/                  SwiftPM package for the menu bar app
   ├─ Package.swift
   └─ Sources/
      ├─ CMacstat/         Module map: imports the header and links libmacstat_ffi.a
      └─ MacstatBar/       SwiftUI app
```

The call chain of the menu bar app is:

```
SwiftUI (MacstatBar) → CMacstat module map → macstat.h → libmacstat_ffi.a → macstat library → sysctl / Mach
```

## CLI

### Build

```bash
cargo build              # debug build: target/debug/macstat
cargo build --release    # optimized build: target/release/macstat
```

`cargo build` in the repository root builds every workspace package, including `macstat-ffi`, because both are listed in `default-members` in `Cargo.toml`.

### Run

Arguments after `--` go to macstat instead of cargo.

```bash
cargo run -- mem                  # memory usage
cargo run -- cpu                  # total CPU usage, sampled over 500 ms
cargo run -- cpu -p               # per-core usage as well
cargo run -- cpu -p -i 1000       # sample over 1000 ms
cargo run -- all                  # CPU and memory together
cargo run -- --help               # all commands and options
```

| Command | Options | Description |
|---|---|---|
| `mem` | | Memory usage (Used = App + Wired + Compressed, as in Activity Monitor), swap, and memory pressure |
| `cpu` | `-p, --per-core`, `-i, --interval <ms>` (default 500) | System / User / Idle share between two samples |
| `all` | same as `cpu` | `cpu` followed by `mem` |

Example:

```
$ cargo run -- mem
Memory           16.00 GiB   Pressure: Normal
  Used           11.37 GiB   71.1%
    App           5.66 GiB
    Wired         2.74 GiB
    Compressed    2.98 GiB
  Cached          3.88 GiB
  Free          184.00 MiB
Swap                   0 B   of 0 B (0 B free)
```

Section titles are green when stdout is a terminal. Set `NO_COLOR=1` to turn colors off.

### Check the numbers

Compare the output with built-in macOS tools:

```bash
vm_stat
sysctl hw.memsize vm.swapusage
top -l 1 -n 0 | grep PhysMem
```

## Menu bar app

### Build and run

The Swift package links `target/debug/libmacstat_ffi.a`, so build the Rust workspace first:

```bash
cargo build
swift run --package-path macos
```

- The menu bar shows `CPU NN%`, updated every second
- Click it to open the panel with CPU and memory details
- Quit with the **Quit macstat** button (⌘Q) in the panel, or Ctrl+C in the terminal

The app runs as an accessory app, so it has no Dock icon.

### After changing Rust code

SwiftPM does not track the Rust static library. Rebuild it before running the app again:

```bash
cargo build && swift run --package-path macos
```

### Build only

```bash
swift build --package-path macos    # binary: macos/.build/debug/MacstatBar
```

## Development checks

Run all of these before committing. CI runs the same checks.

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
swift build --package-path macos    # after `cargo build`
```

Run a subset of the tests:

```bash
cargo test -p macstat           # library and CLI tests
cargo test -p macstat-ffi       # C layout tests and FFI smoke test
cargo test cpu::                # tests whose path contains `cpu::`
```

The `macstat-ffi` layout tests check that the Rust structs match `ffi/include/macstat.h`. When you change an exported struct, update the Rust struct, the header, and these tests together.

## CI

GitHub Actions (`.github/workflows/ci.yml`) runs on a `macos-26` runner for every push to `main` and every pull request:

1. `cargo fmt --check`
2. `cargo clippy --locked -- -D warnings`
3. `cargo build --locked`
4. `cargo test --locked`
5. `cargo run --locked -- mem`
6. `swift build --package-path macos`

## Versioning

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `chore:`, ...). Bump the version only with [Commitizen](https://commitizen-tools.github.io/commitizen/):

```bash
cz bump
```

This updates `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md`, and creates a `v<version>` tag. Do not edit the version in `Cargo.toml` by hand.
