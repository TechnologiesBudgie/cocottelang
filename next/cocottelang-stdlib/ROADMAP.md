# Cocotte Language — Roadmap

## Vision
Cocotte is a Linux-first, English-readable programming language that is:
- **Beginner-friendly** — reads like plain English, errors are clear, docs are first-class
- **Production-capable** — native binaries, memory-safe Rust internals, bundled dependencies
- **Full-stack** — GUI (Charlotte), HTTP servers, SQLite, AI, parallelism, 70+ stdlib modules
- **Open-source** — MIT/Apache-2.0 dual license

---

## ✅ v0.1 — Foundation (done)
- Tree-walk interpreter + bytecode VM
- REPL, project init, build, test, clean commands
- Core language: variables, control flow, functions, closures, classes, error handling
- `Millet.toml` + `Charlotfile` project system
- Basic modules: math, json, os, http client, charlotte (GUI)

## ✅ v0.2 — Native AOT + Ecosystem (done)
- `cocotte build --native` — true AOT via Cocotte→Rust transpiler
- 68 stdlib `.cotlib` modules embedded at compile time
- `parallel` module (Rayon), `ai` module (Ollama/OpenAI), `sqlite` module
- Charlotte GUI with OpenGL + Vulkan/WGPU + Wayland/X11
- Cross-compilation cartesian product (`--os … --arch …`)
- Package manager scaffold (`cocotte pkg`)

## ✅ v0.3 — Stability (done)
- Fixed all compile-blocking bugs (duplicate match arms, FuncDecl double-close, locate_runtime_src)
- Fixed native AOT: try-catch, ClassDecl, VarDecl/Assign env registration, all builtins
- Runtime source embedding via `include_str!` — `cocotte build` works from installed binary
- Charlotte: canvas widget (GPU-accelerated OpenGL/Vulkan), renderer_info()
- Cargo.toml: explicit Wayland+X11 features, relaxed version pins, no-gui feature flag
- **v0.3.1 patch**: fixed `NativeOptions.initial_window_size` → `viewport` (charlotte.rs eframe 0.27 compat); added Makefile, justfile, Charlotfile

---

## 🔲 v0.4 — Language Completeness
- [ ] **Class inheritance** (`class Dog extends Animal`)
- [ ] **Interfaces / traits** (`interface Printable`)
- [ ] **Pattern matching** (`match value case 1: … case _: …`)
- [ ] **Optional types** (`var x: number? = nil`)
- [ ] **String interpolation** (`"Hello {name}!"`)
- [ ] **Multi-return / destructuring** (`var a, b = swap(x, y)`)
- [ ] **`import` statements** as alias for `module add` / `library add`
- [ ] **Parameterised SQL** in sqlite module (prevent injection without manual `esc()`)

## 🔲 v0.5 — Package Registry (live)
- [ ] Launch `pkg.cocotte-lang.org` registry (JSON index + package hosting)
- [ ] `cocotte pkg install <n>` — download + checksum verify + install
- [ ] `cocotte pkg publish` — authenticated upload with signing
- [ ] `Millet.lock` auto-generation and deterministic installs
- [ ] `cocotte pkg update` with semver resolution

## 🔲 v0.6 — Developer Experience
- [ ] **LSP server** (`cocotte lsp`) — VSCode + Neovim + Zed support
- [ ] **Formatter** (`cocotte fmt`) — opinionated, zero-config
- [ ] **Linter** (`cocotte lint`) — common mistakes, unused vars, dead code
- [ ] **Debugger** (`cocotte debug`) — breakpoints, step, inspect
- [ ] **Profiler** (`cocotte profile`) — hotspot detection
- [ ] **Docs generator** (`cocotte doc`) — HTML from `# comments`

## 🔲 v0.7 — Native AOT Maturity
- [ ] Full class support in native mode (self-reference, inheritance)
- [ ] Module calls in native mode (inline Rust stubs for `math`, `json`, `os`)
- [ ] Stdlib `.cotlib` evaluation at native build time (embed output, not source)
- [ ] GCC + LLVM backend options alongside Cargo (`--backend gcc|llvm|cargo`)
- [ ] WASM target (`cocotte build --os wasm`)
- [ ] Memory usage benchmarks vs Python/Ruby

## 🔲 v0.8 — Charlotte GUI Maturity
- [ ] Full canvas API: `rect`, `circle`, `line`, `text`, `image`, `bezier`
- [ ] OpenGL raw draw calls via `charlotte.opengl_context(fn)`
- [ ] Vulkan pipeline access via `charlotte.vulkan_context(fn)`
- [ ] Multi-window support (`charlotte.new_window(...)`)
- [ ] System tray, notifications, file dialogs
- [ ] Theming API (`charlotte.set_theme(map)`)
- [ ] Web export via egui's WASM backend

## 🔲 v0.9 — Production Features
- [ ] Process supervision + auto-restart
- [ ] Hot-reload (`cocotte run --watch`)
- [ ] Signed binary distribution (rpm/deb/AppImage/Flatpak)
- [ ] Windows NT + macOS polishing (unicode paths, code signing)
- [ ] Android + iOS support (egui mobile)

## 🔲 v1.0 — Stable
- [ ] Stable language spec (no breaking changes after this)
- [ ] 100% test coverage on stdlib
- [ ] Performance within 3× of CPython on typical workloads
- [ ] Full documentation site (`docs.cocotte-lang.org`)
- [ ] Community package registry with 50+ third-party packages
