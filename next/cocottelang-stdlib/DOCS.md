# Cocotte Language — Documentation

## Table of Contents
1. [Installation](#installation)
2. [CLI Reference](#cli-reference)
3. [Language Syntax](#language-syntax)
4. [Built-in Functions](#built-in-functions)
5. [Modules](#modules)
6. [Charlotte GUI](#charlotte-gui)
7. [Libraries & Modules](#libraries--modules)
8. [Native AOT Compilation](#native-aot-compilation)
9. [Package Manager](#package-manager)
10. [Testing](#testing)
11. [Cross-Compilation](#cross-compilation)

---

## Installation

### From source
```sh
git clone https://github.com/technologiesbudgie/cocotte
cd cocotte
cargo build --release
sudo cp target/release/cocotte /usr/local/bin/
```

### Build options
```sh
cargo build --release                        # default: GUI enabled (OpenGL + Vulkan)
cargo build --release --no-default-features  # CLI only, no GUI, smaller binary
```

---


## Build Tools

Cocotte ships three task runner configs. All three produce the same results — choose whichever you prefer.

### Makefile

```sh
make build           # cargo build (debug)
make release         # cargo build --release
make build-nogui     # build without Charlotte GUI
make release-nogui   # release without GUI
make test            # run test suite
make run             # run demos/native_demo.cot
make repl            # launch REPL
make check           # cargo check
make fmt             # cargo fmt
make lint            # cargo clippy
make install         # install to /usr/local/bin
make uninstall       # remove from /usr/local/bin
make clean           # remove dist/, build/, .cocotte_cache
```

### justfile

```sh
just                 # list all recipes
just build
just release
just test
just test-verbose
just run
just native          # AOT-compile and run the demo
just repl
just install
just clean
just update          # re-pin rayon-core after cargo update
```

### Charlotfile (cocotte exec)

```sh
cocotte exec list
cocotte exec build
cocotte exec release
cocotte exec test
cocotte exec test-verbose
cocotte exec run
cocotte exec repl
cocotte exec install
cocotte exec clean
```

---

## CLI Reference

| Command | Description |
|---------|-------------|
| `cocotte init <name>` | Create a new project |
| `cocotte run [file]` | Run interpreted (tree-walk) |
| `cocotte run --bytecode [file]` | Run via bytecode VM |
| `cocotte run --debug [file]` | Run with debug output |
| `cocotte build` | Build bytecode VM binary |
| `cocotte build --native` | Build true AOT native binary |
| `cocotte build --release` | Optimised release build |
| `cocotte build --os linux windows macos bsd` | Target OS(es) |
| `cocotte build --arch x86_64 aarch64 armv7 i686 riscv64` | Target arch(es) |
| `cocotte build --out <dir>` | Output directory (default: dist/) |
| `cocotte test [dir]` | Run *_test.cot files |
| `cocotte repl` | Interactive REPL |
| `cocotte disasm <file>` | Show bytecode disassembly |
| `cocotte exec <task>` | Run a Charlotfile task |
| `cocotte exec list` | List available tasks |
| `cocotte add <file.cotlib\|.cotmod>` | Add a local library/module |
| `cocotte new lib <name>` | Scaffold a new library |
| `cocotte new module <name>` | Scaffold a new module |
| `cocotte pkg <sub>` | Package manager |
| `cocotte clean` | Remove dist/, build/, cache |
| `cocotte package [--format zip\|tar]` | Archive the built output |

---

## Language Syntax

### Variables
```cocotte
var x      = 42
var name   = "Alice"
var flag   = true
var items  = [1, 2, 3]
var config = {"host": "localhost", "port": 8080}
var empty  = nil
```

### Operators
```cocotte
x + y     x - y     x * y     divide x by y     x % y
==  !=  <  >  <=  >=
and  or  not
```

> Division uses `divide A by B` — not `/`. This is intentional (readability).

### Control flow
```cocotte
if score >= 90
    print "A"
elif score >= 80
    print "B"
else
    print "F"
end
```

### Loops
```cocotte
while i < 10
    i = i + 1
end

for item in list
    print item
end

for i in range(0, 10)
    print i
end

break
continue
```

### Functions
```cocotte
func add(a, b)
    return a + b
end

var double = func(x) return x * 2 end

func make_counter()
    var n = 0
    return func()
        n = n + 1
        return n
    end
end
```

### Classes
```cocotte
class Rectangle
    func init(w, h)
        self.w = w
        self.h = h
    end
    func area()
        return self.w * self.h
    end
end

var r = Rectangle(4, 6)
print r.area()   # 24
```

### Error handling
```cocotte
try
    var data = read_file("missing.txt")
catch err
    print "Error: " + err
end
```

---

## Built-in Functions

### Output / input
```cocotte
print value
input("prompt")       # returns string
```

### Math
```cocotte
abs(n)   sqrt(n)   pow(b, e)   floor(n)   ceil(n)   round(n)
max(a, b)   min(a, b)   sign(n)   clamp(v, lo, hi)
```

### Type / conversion
```cocotte
type_of(v)     is_number(v)    is_string(v)   is_list(v)   is_map(v)
to_number(s)   to_string(v)    format_number(n, decimals)
```

### Collections
```cocotte
range(start, end)   len(v)   list_of(...)   map_of(k, v, ...)
```

### System
```cocotte
exit(code)   env_get("VAR")   sleep(secs)   random()   time_now()
```

### File I/O
```cocotte
read_file(path)          write_file(path, text)
append_file(path, text)  delete_file(path)
file_exists(path)        is_file(path)     is_dir(path)
file_size(path)          make_dir(path)    list_dir(path)
copy_file(from, to)      rename_file(from, to)
```

### Testing
```cocotte
assert(cond, msg)   assert_eq(a, b)
```

---

## Modules

### Loading modules
```cocotte
module add "math"       # built-in
module add "strings"    # stdlib .cotlib (embedded)
module add "mymod"      # project-local modules/mymod.cotmod
```

### Built-in native modules

| Module | Key functions |
|--------|--------------|
| `math` | `math.PI`, `math.E`, `math.sin`, `math.cos`, `math.log`, `math.pow`, `math.floor`, `math.ceil`, `math.abs`, `math.sqrt` |
| `json` | `json.parse(s)`, `json.stringify(v)` |
| `os` | `os.platform()`, `os.cwd()`, `os.exec(cmd)`, `os.env_get(k)`, `os.env_set(k, v)`, `os.args()` |
| `http` | `http.get/post/put/delete(url)`, `http.get_json/post_json`, `http.serve(port, fn)` |
| `sqlite` | `sqlite.open(path)`, `sqlite.exec(db, sql)`, `sqlite.query(db, sql)`, `sqlite.query_one`, `sqlite.tables` |
| `charlotte` | GUI — see Charlotte section |
| `parallel` | `parallel.map(list, fn)`, `parallel.filter`, `parallel.each`, `parallel.sort` |
| `ai` | `ai.generate(model, prompt)`, `ai.chat(model, messages)`, `ai.embed`, `ai.list_models`, `ai.stream` |
| `threading` | `threading.spawn(fn)`, `threading.join(handle)`, `threading.channel()` |
| `network` | Low-level TCP/UDP stubs |

### Selected stdlib modules (70+ total)

```
strings  regex    dates    path     fs       hash     crypto
url      uuid     csv      json_schema  validation  logging
config   cache    env      template cli      args     base64
collections  sort   statistics  functional  pipeline  router
events   scheduler  markdown  html    ini     dotenv   git
docker   process  terminal   notify  clipboard  image  pdf
set      stack    queue    deque    heap    graph   matrices
complex  geometry  units   colors   color_utils  text  fmt
iter     random   state   rate_limit  middleware  db   ai
```

---

## Charlotte GUI

### Basic window
```cocotte
module add "charlotte"

var state = {"count": 0}

charlotte.window("My App", 800, 600, func(ui)
    ui.heading("Hello!")
    ui.label("Count: " + state.get("count"))
    if ui.button("Increment")
        state.set("count", state.get("count") + 1)
    end
    ui.separator()
    var name = ui.input("name_key", "Your name...")
    var checked = ui.checkbox("cb_key", "Enable feature", false)
    var val = ui.slider("sl_key", "Volume", 0, 100, 50)
    ui.progress(divide val by 100)
    ui.row(func()
        ui.column(func() ui.label("Left") end)
        ui.column(func() ui.label("Right") end)
    end)
end)
```

### Renderer selection (OpenGL vs Vulkan)
```cocotte
# Call BEFORE charlotte.window()
charlotte.set_renderer("opengl")   # OpenGL (Glow) — VMs, older GPUs, Raspberry Pi
charlotte.set_renderer("wgpu")     # Vulkan/Metal/DX12 (WGPU) — best performance
charlotte.set_renderer("auto")     # Auto: WGPU, fall back to OpenGL (default)

print charlotte.renderer_info()    # describes active renderer
print charlotte.has_gui()          # true if built with GUI support
print charlotte.version()          # "charlotte/egui 0.29"
```

### Canvas (raw GPU painting)
```cocotte
# Allocates a GPU-accelerated painting area inside the current window.
# Backed by OpenGL or Vulkan depending on the active renderer.
charlotte.canvas("my_canvas", 400, 300, func(painter)
    # painter API coming in v0.8
end)
```

### Widget reference

| Widget | Return | Notes |
|--------|--------|-------|
| `ui.button(label)` | `bool` | true on click frame |
| `ui.label(text)` | — | |
| `ui.heading(text)` | — | large bold text |
| `ui.separator()` | — | horizontal line |
| `ui.input(key, placeholder)` | `string` | key must be unique |
| `ui.checkbox(key, label, default)` | `bool` | |
| `ui.slider(key, label, min, max, default)` | `number` | |
| `ui.radio(key, label, value)` | `bool` | |
| `ui.progress(0.0–1.0)` | — | |
| `ui.image(path)` | — | |
| `ui.row(fn)` | — | horizontal layout |
| `ui.column(fn)` | — | vertical layout |
| `ui.scroll(fn)` | — | scrollable area |
| `ui.canvas(key, w, h, fn)` | — | GPU painting area |

> State that must persist between frames must live in a `map` declared **outside** the callback. The callback fires ~60×/sec.

---

## Libraries & Modules

### Creating a library
```sh
cocotte new lib myutils
# Creates: libraries/myutils.cotlib
```

```cocotte
# libraries/myutils.cotlib
func greet(name)
    return "Hello, " + name + "!"
end
```

```cocotte
# main.cot
library add "libraries/myutils.cotlib"
print myutils.greet("Alice")
```

### Creating a distributable module
```sh
cocotte new module mymod
# Creates: mymod/mymod.cotmod  mymod/mymod_test.cot  mymod/README.md

# Install into a project:
cocotte add mymod/mymod.cotmod

# Use it:
# module add "mymod"
```

### Installing libraries/modules
```sh
cocotte add path/to/lib.cotlib     # copy to libraries/
cocotte add path/to/mod.cotmod     # copy to modules/
```

---

## Native AOT Compilation

`cocotte build --native` transpiles Cocotte → Rust → native binary.  
The resulting binary has **no runtime dependency** — not even `cocotte` itself.

```sh
cocotte build --native              # host target, debug
cocotte build --native --release    # optimised
cocotte build --native --release --os linux --arch x86_64 aarch64
```

### What works in native mode
- All arithmetic, string operations, lists, maps
- if/elif/else, while, for, break, continue
- Functions, closures (capture by clone)
- try-catch (via `std::panic::catch_unwind`)
- Classes (factory closure + method map)
- All built-in functions (print, input, abs, sqrt, range, assert, env_get, read_file, write_file, sleep, random, time_now, floor, ceil, round, pow, min, max, len, type_of, is_number, is_string, is_list, is_map, format_number, file_exists, exit)
- VarDecl / Assign (synced to `_env` for closure access)

### Limitations in native mode (improving each release)
- `module add` / `library add` are no-ops (built-in functions are pre-registered, stdlib cotlib not evaluated at AOT time — planned for v0.7)
- Class inheritance not yet emitted
- No GUI (Charlotte) support

---

## Package Manager

```sh
cocotte pkg search <query>       # search registry
cocotte pkg install <name>       # install from registry
cocotte pkg remove  <name>       # uninstall
cocotte pkg update               # update all installed packages
cocotte pkg list                 # list installed
cocotte pkg info   <name>        # show package details
```

> The live registry launches with v0.5. Until then, use local `cocotte add` for .cotlib/.cotmod files.

---

## Testing

Test files are any `*_test.cot` file under `tests/`:

```cocotte
# tests/math_test.cot
assert_eq(2 + 2, 4)
assert_eq(to_string(42), "42")
assert(file_exists("src/main.cot"), "main.cot missing")
print "All math tests passed."
```

```sh
cocotte test             # run all tests in tests/
cocotte test --verbose   # show per-assertion output
```

---

## Cross-Compilation

```sh
cocotte build --os linux windows macos bsd \
              --arch x86_64 aarch64

# Requires:
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu
```

Supported OS values: `linux`, `windows` (alias `win`), `macos` (aliases `mac`, `darwin`, `osx`), `bsd` (aliases `freebsd`, `openbsd`, `netbsd`).

Supported arch values: `x86_64` (aliases `amd64`, `x64`), `aarch64` (alias `arm64`), `armv7` (alias `arm`), `i686` (aliases `i386`, `x86`), `riscv64`.

When no cross-toolchain is found, the compiler emits a **source bundle** — a directory the user can compile on the target machine with `cargo build --release`.
