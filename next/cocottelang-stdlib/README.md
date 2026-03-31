<div align="center">

# 🍳 Cocotte

**An English-like, Linux-first programming language**  
Interpreted · Native AOT · GUI · HTTP · SQLite · AI · 70+ stdlib modules

[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)
[![Version](https://img.shields.io/badge/version-0.3.1-green)](#)

</div>

---

## Quick start

```sh
# Build from source
cargo build --release
./target/release/cocotte --version

# Create a project
cocotte init MyApp
cd MyApp
cocotte run          # interpreted (instant)
cocotte build        # bytecode VM binary
cocotte build --native  # true AOT — no interpreter at runtime
```


## Build tools

Cocotte ships a **Makefile**, **justfile** and **Charlotfile** so you can use whichever task runner you prefer.

```sh
# Make
make build          # debug binary
make release        # optimised binary
make test           # run test suite
make install        # install to /usr/local/bin
make clean

# just (cargo install just)
just build
just release
just test
just native         # compile and run demo via --native AOT

# Cocotte task runner (once installed)
cocotte exec list
cocotte exec build
cocotte exec test
cocotte exec install
```


## Hello World

```cocotte
var name = "World"
print "Hello, " + name + "!"
```

## Language tour

```cocotte
# Functions
func add(a, b)
    return a + b
end
print add(2, 3)    # 5

# Classes
class Point
    func init(x, y)
        self.x = x
        self.y = y
    end
    func distance()
        return sqrt(self.x * self.x + self.y * self.y)
    end
end
var p = Point(3, 4)
print p.distance()   # 5

# List pipeline
var result = range(1, 11)
    .filter(func(n) return n % 2 == 0 end)
    .map(func(n) return n * n end)
    .reduce(func(acc, n) return acc + n end, 0)
print result   # 220

# Error handling
try
    var data = read_file("config.json")
catch err
    print "Could not read config: " + err
end
```

## GUI with Charlotte

```cocotte
module add "charlotte"

var count = 0

charlotte.window("Counter", 400, 300, func(ui)
    ui.heading("My App")
    ui.label("Count: " + count)
    if ui.button("Increment")
        count = count + 1
    end
end)
```

Charlotte supports **OpenGL** and **Vulkan/WGPU**, Wayland and X11 on Linux.  
Select the renderer:

```cocotte
charlotte.set_renderer("opengl")   # force OpenGL — works on VMs, older GPUs, Pi
charlotte.set_renderer("wgpu")     # force Vulkan/Metal/DX12 — best performance
charlotte.set_renderer("auto")     # auto: WGPU with OpenGL fallback (default)
print charlotte.renderer_info()
```

## Modules

```cocotte
# Built-in native modules (no install needed)
module add "math"       # math.PI, math.sin, math.cos, math.log, math.floor …
module add "json"       # json.parse(s), json.stringify(v)
module add "os"         # os.platform(), os.cwd(), os.exec(cmd)
module add "http"       # http.get/post/put/delete, http.serve(port, handler)
module add "sqlite"     # sqlite.open/exec/query/tables
module add "charlotte"  # GUI: window, button, label, input, slider, canvas …
module add "parallel"   # parallel.map/filter/each/sort (Rayon-backed)
module add "ai"         # ai.generate/chat/embed/stream (Ollama/OpenAI)
module add "threading"  # threading.spawn/join/channel

# 70+ stdlib .cotlib modules (embedded, zero install)
module add "strings"    module add "regex"    module add "dates"
module add "path"       module add "csv"      module add "hash"
module add "crypto"     module add "url"      module add "uuid"
module add "validation" module add "logging"  module add "config"
# … and 60+ more — see docs/modules.md
```

## Build native binaries

```sh
# Native binary (AOT — no interpreter, no runtime dependency)
cocotte build --native --release

# Cross-compile
cocotte build --os linux windows --arch x86_64 aarch64

# All targets
cocotte build --native --release --os linux windows macos bsd \
              --arch x86_64 aarch64
```

Native binaries are built via Cargo (Rust). No external toolchain needed for the host target.  
Cross-compilation requires the matching `rustup target` + cross-linker.

## Project structure

```
MyApp/
├── Millet.toml        # project config + dependencies
├── Charlotfile        # task runner (like Makefile)
├── src/
│   └── main.cot       # entry point
├── libraries/
│   └── utils.cotlib   # local libraries
├── modules/           # installed third-party modules
├── tests/
│   └── math_test.cot  # *_test.cot files, run with cocotte test
└── dist/              # compiled output
```

## Creating libraries and modules

```sh
# Create a new library (within a project)
cocotte new lib myutils
# → libraries/myutils.cotlib

# Create a standalone distributable module
cocotte new module mymod
# → mymod/mymod.cotmod  + mymod/mymod_test.cot  + mymod/README.md

# Install a library into the current project
cocotte add path/to/mylib.cotlib

# Install a module
cocotte add path/to/mymod.cotmod
```

## Package manager

```sh
cocotte pkg search <query>       # search registry
cocotte pkg install <name>       # install from registry
cocotte pkg remove  <name>       # uninstall
cocotte pkg update               # update all
cocotte pkg list                 # list installed
cocotte pkg info   <name>        # show details
```

> The live registry at `pkg.cocotte-lang.org` launches with v0.5.

## CLI reference

```sh
cocotte init <name>              # create project
cocotte run [file] [--debug] [--bytecode]
cocotte build [--release] [--native] [--os …] [--arch …] [--out dir]
cocotte test [dir] [--verbose]
cocotte repl
cocotte disasm <file>
cocotte exec <task>              # run Charlotfile task
cocotte exec list
cocotte add <file.cotlib|.cotmod>
cocotte new lib <name>
cocotte new module <name>
cocotte pkg <subcommand>
cocotte clean
cocotte package [--format zip|tar]
```

## Build features

```sh
cargo build --release                        # default: GUI enabled
cargo build --release --no-default-features  # CLI only, smaller binary
cargo build --release --features gui         # explicit GUI
```

## License

Dual-licensed under **MIT** and **Apache 2.0** — your choice.  
© Technologies Budgie. All rights reserved.
