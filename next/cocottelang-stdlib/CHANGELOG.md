# Changelog

## [0.3.1] — 2026-03-31

### Fixed
- **`charlotte.rs`**: `NativeOptions` used `initial_window_size` (eframe 0.27 field) instead of `viewport: egui::ViewportBuilder` — caused 3 compile errors on the user's machine. Restored `viewport` + `with_inner_size([w, h])` and `Ok(Box::new(app))` closure wrapper.
- **`native_codegen.rs`**: Unused `catch_name` variable warning (`-D warnings` builds would fail). Renamed to `_catch_name`.

### Added
- **`Makefile`** — `make build/release/test/run/install/clean/fmt/lint` targets.
- **`justfile`** — full `just` recipe set mirroring the Makefile, with a `native` recipe for AOT demo test.
- **`Charlotfile`** — updated with all build/test/run/install tasks for `cocotte exec`.
- **`Millet.toml`** — fixed to proper `[project]` TOML format and bumped to 0.3.1.

---


All notable changes to the Cocotte programming language.

---

## [0.3.0] — 2026-03-28

### Fixed (critical — compile blockers)
- **`modules.rs`**: Removed duplicate match arms in `embed_stdlib!` macro (`ai_helpers`, `ai_utils`, `ai_lib`, `http_client` were all duplicated, causing a Rust compile error). They are now registered as proper aliases in the native `load_module` match.
- **`native_codegen.rs` — FuncDecl**: Removed an extra `}));` closing brace that generated syntactically invalid Rust in every `cocotte build --native` binary. Every program with a user-defined function was broken.
- **`codegen.rs` — `locate_runtime_src()`**: Runtime source files are now embedded at compile time via `include_str!`. Previously, installed binaries (e.g. in `/usr/local/bin`) could not find `src/` and silently fell back to a stub no-op runner. `cocotte build` now works from any install location.

### Fixed (logic bugs)
- **`native_codegen.rs` — `try-catch`**: The catch body was previously silently discarded. Now emits a proper `std::panic::catch_unwind` block with the catch variable bound to the panic message.
- **`native_codegen.rs` — `ClassDecl`**: Was emitting a comment stub. Now emits a proper factory closure that creates a `Val::Map` with per-method closures extracted from `FuncDecl` stmts, including `init` constructor dispatch.
- **`native_codegen.rs` — `VarDecl`/`Assign`**: Variables are now also registered in `_env` so closures can reference outer-scope bindings.
- **`native_codegen.rs` — `_register_builtins`**: All built-in functions (`print`, `input`, `abs`, `sqrt`, `range`, `len`, `assert`, `assert_eq`, `type_of`, `env_get`, `read_file`, `write_file`, `sleep`, `random`, `time_now`, `floor`, `ceil`, `round`, `pow`, `min`, `max`, `is_number`, `is_string`, `is_list`, `is_map`, `format_number`, `exit`, `file_exists`) are now registered in `_env` at startup so user code can call them by name in native binaries.
- **`package_manager.rs`**: Used internal `e.message` field directly; changed to `e.to_string()` which is stable and respects the `Display` impl.
- **`codegen.rs` / `emit_source_bundle`**: Now copies `stdlib/` `.cotlib` files alongside `.rs` sources so that the embedded runtime can find them via `include_str!`.

### Added
- **Charlotte — `canvas(key, w, h, fn)`**: New widget that allocates a GPU-accelerated painting canvas (backed by OpenGL or Vulkan/WGPU depending on active renderer).
- **Charlotte — `renderer_info()`**: Returns a string describing the active renderer (`"OpenGL (Glow) — forced"`, `"Vulkan/Metal/DX12 (WGPU) — forced"`, or `"Auto (WGPU with OpenGL fallback)"`).
- **`modules.rs`**: Added `"http_client"`, `"ai_helpers"`, `"ai_utils"`, `"ai_lib"` as runtime aliases to `http` and `ai` native modules respectively.
- **`Cargo.toml`**: Added explicit Wayland + X11 features for eframe so Charlotte windows work on both display protocols on Linux. Pinned versions relaxed to semver ranges for easier dependency resolution.
- **`Cargo.toml`**: Added `no-gui` feature flag for building smaller CLI-only binaries.
- **`codegen.rs`**: `codegen-units = 1` in release profile for maximum optimization.
- **Package manager stub**: `cocotte pkg` subcommands (`search`, `install`, `remove`, `update`, `list`, `info`, `publish`) are scaffolded and will connect to the live registry when it launches.

### Improved
- `cocotte new lib <n>` and `cocotte new module <n>` generate fully-commented scaffold files with usage instructions.
- `cocotte build --native` now emits clearer per-step progress output with cargo-style verbs.
- Error messages for missing modules now list all available built-in and stdlib modules.

---

## [0.2.0] — 2026-02-15

### Added
- Native AOT compilation via `cocotte build --native` (Cocotte → Rust → binary, no interpreter at runtime).
- `parallel` module (Rayon-backed `map`, `filter`, `each`, `sort`).
- `ai` module (Ollama-compatible: `generate`, `chat`, `list_models`, `embed`, `stream`).
- `http` server mode (`http.serve(port, handler)`).
- `sqlite` module with bundled SQLite (no system dependency).
- `threading` module for multi-threaded workloads.
- `cocotte pkg` package manager stub.
- `cocotte new lib <n>` / `cocotte new module <n>` scaffolding commands.
- Charlotte GUI: `set_renderer("opengl"|"wgpu"|"auto")`, automatic WGPU→OpenGL fallback.
- 68 stdlib `.cotlib` modules embedded at compile time (no install needed).
- Cross-compilation: `cocotte build --os linux windows macos bsd --arch x86_64 aarch64`.

### Fixed
- Charlotte black window bug (root cause: `Value::Map` vs `Value::Module` dispatch).
- CI security issues (no secrets in workflows).

---

## [0.1.0] — 2026-01-10

### Added
- Initial release: tree-walk interpreter, bytecode VM, REPL, `cocotte init/run/build/test/clean`.
- Core language: variables, if/elif/else, while/for, functions, closures, classes, try/catch.
- Built-in modules: `math`, `json`, `os`, `http` (client), `charlotte` (GUI stub).
- `Millet.toml` project config, `Charlotfile` task runner.
