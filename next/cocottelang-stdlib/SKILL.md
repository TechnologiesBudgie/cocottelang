---
name: cocotte-lang
description: >
  Expert-level Cocotte programming language assistant. Use this skill
  whenever the user asks to write, generate, fix, explain, or review any
  Cocotte code — including scripts, full projects, libraries (.cotlib),
  modules (.cotmod), GUI apps (Charlotte/egui), HTTP servers, AI apps,
  threaded programs, cross-platform builds, Charlotfile tasks,
  Millet.toml configuration, and multi-language hybrid projects.
  Also trigger for questions about Cocotte syntax, CLI usage, project
  structure, the Cocotte ecosystem, native AOT compilation, or the
  package manager. When the user says "write me a Cocotte …",
  "how do I do X in Cocotte", "fix this .cot file", or mentions any
  Cocotte file extension (.cot, .cotlib, .cotmod, Charlotfile,
  Millet.toml), always consult this skill first.
---

# Cocotte Language — Expert Reference (v0.2.0)

## File extensions

| Extension     | Purpose                              |
|---------------|--------------------------------------|
| `.cot`        | Source file                          |
| `.cotlib`     | Local library (shared functions)     |
| `.cotmod`     | Installed module                     |
| `Millet.toml` | Project config (deps, metadata)      |
| `Millet.lock` | Pinned package versions (auto)       |
| `Charlotfile` | Task runner (like Makefile)          |

---

## Project layout

```
MyApp/
├── Millet.toml
├── Millet.lock
├── Charlotfile
├── src/
│   └── main.cot          # entry point
├── libraries/
│   └── utils.cotlib      # local libraries
├── modules/              # installed .cotmod packages
├── tests/
│   └── math_test.cot     # *_test.cot files
└── dist/                 # compiled output
```

---

## CLI quick reference

```sh
cocotte init MyApp
cocotte run [file]                          # tree-walk interpreter
cocotte run --bytecode [file]               # bytecode VM
cocotte build [--release] [--out dir]       # embed interpreter in binary
cocotte build --native [--release]          # true AOT: Cocotte→Rust→binary
cocotte build --os linux windows macos bsd
cocotte build --arch x86_64 aarch64 armv7 i686 riscv64
cocotte build --os linux windows --arch x86_64 aarch64   # cartesian product
cocotte add <module>                        # install module
cocotte add <file.cotlib>                   # add local library
cocotte test [--verbose]
cocotte exec <task>                         # run Charlotfile task
cocotte exec list
cocotte clean
cocotte package [--format zip|tar]
cocotte repl
cocotte disasm <file>
cocotte pkg search <query>                  # search registry
cocotte pkg install <n> [n2 ...]         # install package(s)
cocotte pkg remove <n>                   # remove package
cocotte pkg update [name]                   # update all or one
cocotte pkg list                            # show installed
cocotte pkg info <n>                     # package details
```

---

## Core syntax

### Variables and types

```cocotte
var x      = 42
var name   = "Alice"
var flag   = true
var items  = [1, 2, 3]
var config = {"host": "localhost", "port": 8080}
var empty  = nil
```

Types: `number`, `string`, `bool`, `nil`, `list`, `map`, `func`.  
Variables are dynamically typed; reassignment uses no keyword.

### Operators

```cocotte
# Arithmetic
x + y    x - y    x * y    divide x by y    x % y
floor(divide x by y)   # integer division

# Comparison
==  !=  <  >  <=  >=

# Logical
and   or   not
```

Division uses `divide A by B` (keyword form, not `/`). This is intentional.

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

for i in range(0, 10)   # [0, 1, ..., 9]
    print i
end

break     # exit loop
continue  # next iteration
```

### Functions

```cocotte
func add(a, b)
    return a + b
end

# Lambda / anonymous
var double = func(x) return x * 2 end

# Closure
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
print r.area()    # 24
```

- `init` is the constructor; called as `ClassName(args...)`.
- `self` refers to the current instance.
- No inheritance in v0.2 — use composition.

### Error handling

```cocotte
try
    var data = read_file("missing.txt")
catch err
    print "Error: " + err
end
```

### List index assignment

```cocotte
var items = [10, 20, 30]
items[1] = 99
print items    # [10, 99, 30]
```

---

## Built-in functions (selection)

```cocotte
# Output / input
print value
input("prompt")       # returns string

# Math
abs(n)   sqrt(n)   pow(b, e)   floor(n)   ceil(n)   round(n)
max(a, b)   min(a, b)   sign(n)   clamp(v, lo, hi)

# Conversion
to_number(s)   to_string(v)   format_number(n, decimals)

# Type inspection
type_of(v)   is_number(v)   is_string(v)   is_list(v)   is_map(v)

# Collections
range(start, end)   len(v)   list_of(...)   map_of(k,v, ...)

# System
exit(code)   env_get("VAR")   sleep(secs)   random()   time_now()

# Testing
assert(cond, msg)   assert_eq(a, b)
```

---

## String methods

```cocotte
s.len()            s.is_empty()        s.upper()          s.lower()
s.trim()           s.trim_left()       s.trim_right()
s.get(i)           s.slice(from, to)   s.index_of(sub)
s.contains(sub)    s.starts_with(pre)  s.ends_with(suf)
s.replace(a, b)    s.replace_first(a, b)
s.split(sep)       s.split_lines()
s.repeat(n)        s.pad_left(n, ch)   s.pad_right(n, ch)
s.to_number()      s.to_list()         s.reverse()
```

---

## List methods

```cocotte
lst.len()           lst.is_empty()
lst.get(i)          lst.first()         lst.last()
lst.push(val)       lst.pop()
lst.contains(val)   lst.index_of(val)   lst.slice(from, to)
lst.sort()          lst.reverse()       lst.copy()   lst.clear()
lst.extend(other)   lst.join(sep)

# Functional
lst.map(func(x) ... end)
lst.filter(func(x) ... end)
lst.reduce(func(acc, x) ... end, init)
lst.each(func(x) ... end)
lst.find(func(x) ... end)
lst.count(func(x) ... end)
```

---

## Map methods

```cocotte
m.get(key)         m.set(key, val)
m.has_key(key)     m.keys()           m.values()    m.len()
```

---

## File I/O (built-in, no module needed)

```cocotte
read_file(path)             write_file(path, text)
append_file(path, text)     delete_file(path)
file_exists(path)           is_file(path)     is_dir(path)
file_size(path)             make_dir(path)    list_dir(path)
copy_file(from, to)         rename_file(from, to)
```

---

## Modules

```cocotte
module add "charlotte"   # GUI
module add "math"        # math.PI, math.sin, math.cos, math.log, …
module add "json"        # json.stringify(v), json.parse(s), json.stringify_pretty(v)
module add "os"          # os.platform(), os.arch(), os.cwd(), os.exec(cmd)
module add "http"        # HTTP client + server
module add "sqlite"      # SQLite database
module add "threading"   # real threads, join, channels, mutexes
module add "parallel"    # data-parallel map/filter/each (Rayon)
module add "ai"          # Ollama + OpenAI: generate, chat, embed, cosine_similarity
```

All 69 stdlib modules are also available:

```cocotte
module add "strings"      # title_case, edit_distance, slugify, word_wrap …
module add "time"         # Timer class, measure()
module add "regex"        # matches, find_all, replace_all, is_email …
module add "sort"         # sort.by(list, key_fn), merge_sort, binary_search
module add "statistics"   # mean, median, std_dev, percentile …
module add "validation"   # is_email, is_url, validate_map …
module add "ai_utils"     # ChatSession, classify, summarise, text_similarity …
# … and 60+ more (path, fs, dates, hash, crypto, csv, url, uuid, cli, …)
```

---

## threading module

```cocotte
module add "threading"

var tid = threading.spawn(func() return 42 end)
print threading.join(tid)   # 42 — blocks until thread exits

var tid2 = threading.spawn(func(x) return x * 2 end, 21)
print threading.join(tid2)  # 42

threading.sleep(0.5)        # sleep seconds (float)
print threading.num_cpus()  # logical CPU count

# Channels
var ch = threading.channel()
threading.spawn(func() threading.send(ch, "hi") end)
print threading.recv(ch)                     # blocks
var msg = threading.recv_timeout(ch, 1.0)    # nil on timeout

# Mutex
var mu = threading.mutex()
threading.with_lock(mu, func()
    # critical section
end)
```

---

## parallel module

```cocotte
module add "parallel"

var sq = parallel.map([1,2,3,4], func(x) return x * x end)  # [1,4,9,16]
var ev = parallel.filter(range(1,11), func(n) return n % 2 == 0 end)
parallel.each(items, func(item) process(item) end)

print parallel.num_threads()   # rayon pool size
parallel.set_threads(8)
```

---

## ai module

```cocotte
module add "ai"

# Local Ollama
var r = ai.generate("llama3", "What is Rust?")
var r2 = ai.generate("llama3", "Explain closures.", {"temperature": 0.5})

# Chat (multi-turn)
var msgs = [{"role": "user", "content": "Hello!"}]
var reply = ai.chat("llama3", msgs)

# Embeddings
var vec = ai.embed("nomic-embed-text", "hello")
var sim = ai.cosine_similarity(va, vb)  # 0..1

# OpenAI
var key = env_get("OPENAI_API_KEY")
var r3  = ai.openai_chat(key, "gpt-4o", [{"role": "user", "content": "Hi"}])

var models = ai.models()  # list local models
```

---

## ai_utils stdlib module

```cocotte
module add "ai_utils"

var s = ai_utils.session("llama3")
s.system("You are a pirate.")
print s.chat("What is the weather?")
print s.turns()   # 1

# Shortcuts
ai_utils.generate("llama3", "prompt")
ai_utils.classify("llama3", text, ["positive", "negative", "neutral"])
ai_utils.summarise("llama3", long_text, 3)  # 3 sentences
ai_utils.text_similarity("nomic-embed-text", "cat", "kitten")  # ~0.92
ai_utils.find_similar("nomic-embed-text", "query", candidates_list)
ai_utils.list_models()
```

---

## http module

### Client

```cocotte
module add "http"

var body = http.get("https://example.com")
var user = http.get_json("https://api.example.com/users/1")
var hdrs = {"Authorization": "Bearer token"}
var data = http.get_json("https://api.example.com/me", hdrs)

http.post("https://api.example.com/log", "plain body")
http.post_json("https://api.example.com/users", {"name": "Alice"})
http.put("https://api.example.com/users/1", "body")
http.patch("https://api.example.com/users/1", "body")
http.delete("https://api.example.com/users/1")
```

| Function | Returns |
|----------|---------|
| `http.get(url [, headers])` | string |
| `http.get_json(url [, headers])` | parsed Cocotte value |
| `http.post(url, body [, headers])` | string |
| `http.post_json(url, value [, headers])` | string |
| `http.put(url, body [, headers])` | string |
| `http.patch(url, body [, headers])` | string |
| `http.delete(url [, headers])` | string |

### Server

```cocotte
http.serve(port, func(req)
    # req keys: method, path, query, headers (map), body
    return {"status": 200, "body": "ok", "headers": {}}
end)
```

---

## sqlite module

```cocotte
module add "sqlite"

var db = sqlite.open("/data/app.db")
sqlite.exec(db, "CREATE TABLE IF NOT EXISTS items(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)")

func esc(s)
    if s == nil return "" end
    return to_string(s).replace("'", "''")
end

sqlite.exec(db, "INSERT INTO items(name) VALUES('" + esc(user_input) + "')")
var rows = sqlite.query(db, "SELECT * FROM items")
var one  = sqlite.query_one(db, "SELECT * FROM items WHERE id=1")
var tbls = sqlite.tables(db)
sqlite.close(db)
```

| Function | Description |
|----------|-------------|
| `sqlite.open(path)` | Open/create DB; returns db handle |
| `sqlite.exec(db, sql)` | Run CREATE/INSERT/UPDATE/DELETE |
| `sqlite.query(db, sql)` | SELECT → list of maps |
| `sqlite.query_one(db, sql)` | SELECT → map or nil |
| `sqlite.tables(db)` | List of table name strings |
| `sqlite.close(db)` | Release connection |

---

## GUI — Charlotte

```cocotte
module add "charlotte"

# Renderer selection (call before window)
charlotte.set_renderer("opengl")  # force OpenGL (old devices, VMs, RPi)
charlotte.set_renderer("wgpu")    # force WGPU
charlotte.set_renderer("auto")    # default: try WGPU, fall back to OpenGL

var state = {"count": 0}

charlotte.window("App", 400, 300, func(ui)
    ui.heading("Counter")
    ui.label("Count: " + state.get("count"))

    if ui.button("Add")
        state.set("count", state.get("count") + 1)
    end

    ui.separator()
    var val     = ui.slider("s", "Step", 1, 10, 1)
    var name    = ui.input("n", "Your name...")
    var checked = ui.checkbox("c", "Enable", false)
    ui.progress(0.6)

    ui.row(func()
        ui.column(func() ui.label("Col A") end)
        ui.column(func() ui.label("Col B") end)
    end)

    ui.scroll(func()
        for i in range(0, 50)
            ui.label("Item " + i)
        end
    end)
end)
```

State that must survive between frames must live in a `map` outside the callback — the callback fires ~60×/sec.

---

## Libraries (.cotlib)

```cocotte
# libraries/utils.cotlib
func clamp(v, lo, hi)
    if v < lo return lo end
    if v > hi return hi end
    return v
end
```

```cocotte
# main.cot
library add "libraries/utils.cotlib"
print utils.clamp(150, 0, 100)   # 100
```

---

## Charlotfile

```toml
[project]
name = "MyApp"

[variables]
OUT = "dist"

[tasks.build]
cocotte build --release

[tasks.native]
cocotte build --native --release

[tasks.deploy]
cocotte build --native --release
scp ${OUT}/MyApp user@host:/opt/app/

[tasks.clean]
cocotte clean
rm -rf ${OUT}
```

---

## Millet.toml

```toml
[project]
name    = "MyApp"
version = "1.0.0"
author  = "You"

[dependencies]
modules   = ["json", "http", "sqlite"]
libraries = ["libraries/utils.cotlib"]
```

---

## Building binaries

```sh
# Standard (interpreter embedded — self-contained, runs anywhere)
cocotte build --release

# Native AOT (Cocotte→Rust→binary — true CPU code, no interpreter overhead)
cocotte build --native --release

# Cross-compile
cocotte build --os linux windows macos bsd --arch x86_64 aarch64
cocotte build --native --os linux windows --arch x86_64 aarch64
```

**`cocotte build` internals:**  
`codegen.rs` contains `generate_lib_rs()` with a hardcoded list of modules.
Whenever a new `.rs` file is added to `src/`, it **must** be added there, or
`cocotte build` produces a broken stub. The current list includes:
`ast`, `lexer`, `parser`, `error`, `value`, `environment`, `interpreter`,
`builtins`, `modules`, `compiler`, `bytecode`, `vm`, `charlotfile`,
`codegen`, `native_codegen`, `runtime_ctx`, `http_server`, `package_manager`,
and `charlotte` (gui-only).

---

## Common patterns

### Read JSON config, fall back to defaults

```cocotte
module add "json"

func load_config(path, defaults)
    if not file_exists(path)
        return defaults
    end
    try
        return json.parse(read_file(path))
    catch err
        print "Config parse error: " + err
        return defaults
    end
end
```

### Functional pipeline

```cocotte
var result = range(1, 101)
    .filter(func(n) return n % 2 == 0 end)
    .map(func(n) return n * n end)
    .reduce(func(acc, n) return acc + n end, 0)
print result   # sum of squares of even numbers 1-100
```

### Parallel data processing

```cocotte
module add "parallel"

var results = parallel.map(large_list, func(item)
    # runs on all CPU cores simultaneously
    return heavy_computation(item)
end)
```

### AI chat app

```cocotte
module add "ai_utils"

var s = ai_utils.session("llama3")
s.system("You are a helpful assistant.")

while true
    var msg = input("You: ")
    if msg == "quit" exit(0) end
    print "AI: " + s.chat(msg)
end
```

---

## Code generation rules

1. Use `divide A by B` for division — never `/`.
2. Close every `if`, `while`, `for`, `func`, `class` block with `end`.
3. Use `module add` / `library add` at the top of the file.
4. State in Charlotte GUI callbacks must live in an outer `map`.
5. Widget `key` strings must be unique across the entire window.
6. Prefer `for item in list` over manual index loops.
7. Prefer `.map` / `.filter` / `.reduce` over explicit loops for data transformations.
8. Use `try / catch err` around all file, network, and database operations.
9. Test files go in `tests/`, named `*_test.cot`, using `assert_eq`.
10. When generating a full project, always include `Millet.toml`, `Charlotfile`, and a working `src/main.cot`.
11. Use `http.serve` for REST APIs; always call it at the **end** of the program — it blocks forever.
12. Use `esc()` (replace `'` with `''`) when interpolating user data into SQL strings.
13. When adding a new `.rs` file to the Cocotte runtime, add it to `generate_lib_rs()` in `codegen.rs` or `cocotte build` will produce a broken stub binary.
14. Use `parallel.map` for CPU-bound batch work; use `threading.spawn` + `threading.join` for I/O-bound concurrency or when you need the return value.
15. Use `charlotte.set_renderer("opengl")` before `charlotte.window(...)` when targeting older hardware, VMs, or Raspberry Pi.
