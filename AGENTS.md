# macstat

A Rust CLI that reads macOS memory and CPU usage by calling kernel APIs (sysctl, Mach) directly through FFI, plus a SwiftUI menu bar app that shows the same data through a C ABI exported from Rust. This is a learning project: the user is learning Rust by building it.

## Layout

- `src/lib.rs`: the `macstat` library (`cpu`, `memory`, private `mach`). It only collects data
- `src/main.rs`, `src/report.rs`, `src/style.rs`: the CLI binary. It formats the library data for the terminal
- `ffi/`: the `macstat-ffi` crate (`staticlib`). It exports the library as C functions; `ffi/include/macstat.h` is the matching C header
- `macos/`: a SwiftPM package for the menu bar app (`MenuBarExtra`). `CMacstat` wraps the header with a module map and links `libmacstat_ffi.a` from `target/debug`

## Working style

- IMPORTANT: The user's understanding matters more than the output. Before changing code, explain what you will do and why, then proceed in small steps of about one function each
- Before adding a dependency (crate), explain what it does and why it is needed, and get agreement. Current dependencies are only `clap`, `libc`, and `mach2`. The Swift app uses only system frameworks; ask before adding a Swift package too
- Do not use crates that hide OS calls, such as `sysinfo`. Working with FFI directly is the learning goal
- When a Rust concept (ownership, traits, `unsafe`, `Drop`, etc.) appears for the first time, explain it briefly. Comparisons with Java help
- Explain to the user in Korean, keeping code identifiers and technical terms in their original form
- Write all documentation files (`AGENTS.md`, `README.md`, `CHANGELOG.md`, etc.) and code comments in English

## Commands

- Run: `cargo run -- mem`, `cargo run -- cpu -p -i 1000` (arguments after `--` go to the program)
- Run the menu bar app: `cargo build && swift run --package-path macos` (stop it with its Quit button or Ctrl+C). Run `cargo build` first whenever Rust code changes, because SwiftPM does not track the static library
- `Cargo.toml` sets `default-members = [".", "ffi"]`, so plain `cargo build`/`test`/`clippy` in the root cover both packages. Keep new workspace members in `default-members` too
- All three must pass before finishing a task: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`. If `macos/` or the header changed, `swift build --package-path macos` must pass too
- CI (`.github/workflows/ci.yml`, `macos-26` runner) runs the same checks with `--locked`, plus `cargo run -- mem` and `swift build --package-path macos`. If you change the check commands, update CI too. CI must stay on a macOS runner because the crate links and runs only on macOS
- The Rust version is pinned in `rust-toolchain.toml`, which both local builds and CI follow. To upgrade Rust, change only this file
- Verify output against built-in macOS tools: `vm_stat`, `sysctl hw.memsize vm.swapusage`, `top -l 1 -n 0 | grep PhysMem`

## FFI and unsafe rules

- Confine `unsafe` to small functions or types and expose only safe APIs. Keep `main.rs` free of `unsafe`
- Put a `// SAFETY:` comment in English above every `unsafe` block explaining why it is safe
- Do not guess C function signatures, structs, or constants. Check both of these before writing them
  - Crate sources: `~/.cargo/registry/src/*/libc-*/src/unix/bsd/apple/mod.rs`, `~/.cargo/registry/src/*/mach2-*/src/`
  - SDK headers: `$(xcrun --show-sdk-path)/usr/include/mach/`, `.../sys/sysctl.h`
- Import from `mach2`: `mach_host_self`, `mach_task_self` (deprecated in `libc`), `mach_port_deallocate`, `vm_kernel_page_size` (absent from `libc`)
- Import from `libc`: `host_statistics64`, `host_processor_info`, `vm_deallocate`, `sysctlbyname` (absent from `mach2`)
- Wrap resources received from the kernel (Mach ports, arrays allocated by `host_processor_info`) in types that implement `Drop` so they are released automatically. Do not derive `Clone`/`Copy` on these types

## macOS kernel gotchas

- `libc::vm_statistics64` has more fields than the SDK, so the kernel fills only the leading part (248 of 416 bytes on this Mac). Start from `MaybeUninit::zeroed()` and use the returned `count` to check that the needed fields were filled
- For `sysctl`, check that the returned size equals `size_of::<T>()` exactly before calling `assume_init()`
- Convert pages to bytes with `vm_kernel_page_size`, not `vm_page_size`. Under Rosetta (x86_64), `vm_page_size` is 4096 while kernel statistics use 16384-byte pages
- Widen page counts (`u32`) with `u64::from` before multiplying. Use `saturating_sub` for subtraction
- CPU ticks (`cpu_ticks`) are cumulative `u32` values that wrap around to 0. Compute the difference between two samples with `wrapping_sub`
- `kern.memorystatus_vm_pressure_level` values are 1=Normal, 2=Warning, 4=Critical (`DISPATCH_MEMORYPRESSURE_*` in `dispatch/source.h`)
- Compute memory usage the same way as Activity Monitor: Used = App (internal − purgeable) + Wired + Compressed, Cached = external + purgeable

## Exporting to Swift (`ffi` crate)

- Export functions with `#[unsafe(no_mangle)] pub extern "C" fn macstat_*`, with a `// SAFETY:` comment above the attribute
- Exchange `#[repr(C)]` structs by value and report failure with an `ok: bool` field. Avoid raw pointers and heap memory across the boundary unless a matching `macstat_*_free` function is added
- `ffi/include/macstat.h` is written by hand and must match the Rust structs exactly. When a struct changes, update the header and the `offset_of!`/`size_of` layout tests in `ffi/src/lib.rs` together
- The Swift app must call the blocking `macstat_cpu_measure` off the main actor (for example with `Task.detached`)

## Code style

- The only comments are `// SAFETY:`. Keep the `///` doc comments on clap fields because they become the `--help` text
- Propagate errors with `io::Result` and `?`. Do not use `unwrap()` outside tests
- Print each command's output with a single `println!`. When indentation is needed, use a string literal with real line breaks instead of a trailing `\` (which strips leading whitespace on the next line)

## Environment

- macOS only. The development machine is an Apple M4 (arm64) with 16 KiB pages
- Swift 6 from the Command Line Tools is enough for `swift build`/`swift run`; Xcode is not required. The app targets macOS 14+ and builds in the Swift 6 language mode (strict concurrency)
- If RustRover shows errors such as `Unresolved import` while `cargo check` passes, the IDE index is stale. Fix it with Reload in the Cargo tool window or File → Reload All from Disk

## Git

- This is a public repository: https://github.com/Tianea2160/macstat (default branch `main`)
- Write commit messages in English using Conventional Commits (`feat:`, `fix:`, `chore:`, etc.). `cz bump` uses these prefixes to compute the next version
- Bump versions only with `cz bump` (`.cz.toml`). Do not edit `version` in `Cargo.toml` by hand. `cz bump` updates `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md`, and creates a `v$version` tag
- `.claude/settings.local.json` is force-tracked with `git add -f`, so it is public. Before committing, make sure it contains no absolute paths or secrets
