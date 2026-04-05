# Changelog

## 0.1.3
**Cross-compilation fixes:**
- Automatically writes `.cargo/config.toml` with the correct cross-linker for each target — no more silent linker failures
- Calls `rustup target add <triple>` automatically before each cross build
- Disables `egui`/`eframe` (Charlotte GUI) for cross-compilation targets — it can't cross-compile and was breaking all cross builds
- macOS cross-compilation now gives a clear error message pointing to osxcross instead of silently emitting a broken source bundle
- MSVC Windows targets now redirect to the GNU ABI (mingw) with a clear message
- Per-linker install hints: shows `sudo apt-get install gcc-aarch64-linux-gnu` etc. when the cross-linker is missing

**New modules (all zero new Cargo dependencies — pure Rust):**
- `path` — `path.join`, `path.basename`, `path.dirname`, `path.ext`, `path.stem`, `path.abs`, `path.exists`, `path.is_abs`, `path.parts`, `path.home`
- `env` — `env.get`, `env.get_or`, `env.set`, `env.remove`, `env.all`, `env.require`
- `args` — `args.all`, `args.get`, `args.len`, `args.flag`, `args.option`
- `uuid` — `uuid.v4`, `uuid.is_valid` (uses `/dev/urandom`, no crate needed)
- `log` — `log.debug/info/warn/error`, `log.set_level` with timestamps and ANSI colours
- `process` — `process.run`, `process.run_args`, `process.exit`, `process.pid`
- `csv` — `csv.parse`, `csv.parse_with_headers`, `csv.stringify`
- `crypto` — `crypto.sha256`, `crypto.md5` (pure-Rust implementations)
- `base64` — `base64.encode`, `base64.decode` (pure-Rust)

**Docs:**
- Full reference sections for all new modules with examples
- Cross-compilation section rewritten with linker install table
- Table of contents updated

## 0.1.2
- **f-strings:** `f"Hello {name}! Score: {score * 2}"`
- **`/` division operator:** `10 / 3` works alongside `divide 10 by 3`
- **Parameterised SQL:** `sqlite.exec_params` and `sqlite.query_params`
- **Map new methods:** `.remove`, `.merge`, `.entries`
- **Map dot-access:** `row.name` as shorthand for `row.get("name")`
- **List new methods:** `.pop(i)`, `.insert(i, val)`, `.sort_by(func)`
- **`--native` build flag**
- **Bundled Rust toolchain:** auto-downloads into `~/.cocotte/toolchain/` if cargo not found or rustup has no default toolchain
- **JSON integer fix:** `json.stringify` emits `95` not `95.0`

## 0.1.1
- Enhanced Charlotte
- Added HTTP and SQL modules

## 0.1.0
- Initial release (compiler + CLI basic)
- Minimal Charlotte implemented
