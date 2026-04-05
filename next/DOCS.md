# Cocotte Language Reference

**Version 0.1.2**

Cocotte is a compiled and interpreted programming language with English-like syntax. It runs on Linux, Windows, macOS, and BSD across all common CPU architectures. Source files use the `.cot` extension. The same `cocotte` binary interprets, compiles, tests, and manages projects.

> **New here?** Jump straight to [§2 Getting Started](#2-getting-started), run the Hello World, then come back for the full reference. The whole language fits in your head in an afternoon.

---

## Table of Contents

1. [Installation](#1-installation)
2. [Getting Started](#2-getting-started)
3. [Project Structure](#3-project-structure)
4. [CLI Reference](#4-cli-reference)
5. [Syntax](#5-syntax)
6. [Built-in Functions](#6-built-in-functions)
7. [String Methods](#7-string-methods)
8. [List Methods](#8-list-methods)
9. [Map Methods](#9-map-methods)
10. [File I/O](#10-file-io)
11. [Built-in Modules](#11-built-in-modules)
12. [Writing Libraries and Modules](#12-writing-libraries-and-modules)
13. [GUI — Charlotte](#13-gui--charlotte)
14. [Charlotfile — Task Runner](#14-charlotfile--task-runner)
15. [Millet.toml — Project Config](#15-millettoml--project-config)
16. [Cross-Compilation](#16-cross-compilation)
17. [Testing](#17-testing)
18. [Complete Examples](#18-complete-examples)
19. [New Modules (0.1.2)](#19-built-in-modules--full-reference)
20. [Cross-Compilation (detailed)](#20-cross-compilation)
21. [Planned Modules](#21-planned-modules)

---

## 1. Installation

### Linux (any architecture)

```sh
curl -fsSL https://cocotte-lang.pages.dev/install.sh | sh
```

Detects your architecture (`x86_64`, `aarch64`, `armv7`, `i686`, `riscv64`) and installs to `/usr/local/bin/cocotte`.

### Manual

Download a binary from [GitHub Releases](https://github.com/TechnologiesBudgie/cocottelang/releases):

```sh
chmod +x cocotte-linux-x86_64
sudo mv cocotte-linux-x86_64 /usr/local/bin/cocotte
cocotte --version
```

### From source

Requires Rust (`cargo`):

```sh
git clone https://github.com/TechnologiesBudgie/cocottelang
cd cocottelang
cargo build --release
# binary: target/release/cocotte
```

---

## 2. Getting Started

```sh
cocotte init MyProject
cd MyProject
cocotte run
# Hello, World!
```

Run a specific file:

```sh
cocotte run src/main.cot
```

Compile to a native binary:

```sh
cocotte build --release
./dist/MyProject
```

---

## 3. Project Structure

```
MyProject/
├── Millet.toml       # project metadata and dependencies
├── Charlotfile       # task runner (like Makefile, but readable)
├── src/
│   └── main.cot      # program entry point
├── libraries/        # .cotlib files (local, single-project utilities)
├── modules/          # .cotmod files (installed distributable modules)
├── tests/            # test files (*_test.cot)
├── dist/             # compiled output (created by cocotte build)
└── .gitignore
```

**File types at a glance:**

| Extension | What it is |
|-----------|-----------|
| `.cot` | Regular source file |
| `.cotlib` | Local library — single-project utilities |
| `.cotmod` | Distributable module — shareable packages |
| `Millet.toml` | Project config (dependencies, metadata) |
| `Charlotfile` | Task runner (build steps, deploy commands) |

---

## 4. CLI Reference

### `cocotte init <name>`

Create a new project directory with all boilerplate files.

```sh
cocotte init MyApp
cd MyApp
cocotte run
```

---

### `cocotte run [file]`

Run a `.cot` file in interpreted mode. No compilation step — executes immediately.

```sh
cocotte run                        # runs src/main.cot
cocotte run src/other.cot
cocotte run --debug src/main.cot   # print interpreter state
cocotte run --bytecode             # use bytecode VM instead of tree-walk interpreter
```

---

### `cocotte build [file]`

Compile a `.cot` file to a native binary. Output goes to `dist/`.

```sh
cocotte build                                        # current OS and arch
cocotte build --release                              # optimised (LTO, strip)
cocotte build --native                               # shorthand: --release for this machine
cocotte build --os linux --arch aarch64              # cross-compile
cocotte build --os linux windows --arch x86_64 aarch64  # 4 binaries at once
```

| Flag | Description |
|------|-------------|
| `--os <OS...>` | `linux`, `windows`, `macos`, `bsd` |
| `--arch <ARCH...>` | `x86_64`, `aarch64`, `armv7`, `i686`, `riscv64` |
| `--release` | Enable optimisations |
| `--native` | Shorthand for `--release` targeting the current machine only (cannot be combined with `--os` / `--arch`) |
| `--out <dir>` | Output directory (default: `dist/`) |
| `--verbose` | Show internal build steps |

When both `--os` and `--arch` are given, all combinations are built. See [§16 Cross-Compilation](#16-cross-compilation).

> **No Rust installed?** No problem. If `cargo` is not found on your PATH, `cocotte build` will automatically download and install a minimal Rust toolchain into `~/.cocotte/toolchain/` the first time you build. Subsequent builds use the cached toolchain and are fast. You never need to install Rust manually.

---

### `cocotte new lib <name>`

Scaffold a new `.cotlib` library file.

```sh
# Inside a project — writes to libraries/<name>.cotlib, registers in Millet.toml
cocotte new lib myutils

# Outside a project — writes <name>.cotlib to current directory
cocotte new lib myutils
```

---

### `cocotte new module <name>`

Scaffold a new distributable `.cotmod` module with tests and README.

```sh
cocotte new module mymodule
# Creates: mymodule/mymodule.cotmod
#          mymodule/mymodule_test.cot
#          mymodule/README.md
```

---

### `cocotte add <file>`

Install a local library or module into the current project.

```sh
cocotte add path/to/utils.cotlib     # copies to libraries/, updates Millet.toml
cocotte add path/to/utils.cotmod     # copies to modules/, updates Millet.toml
```

Built-in modules (`json`, `math`, `os`, `http`, `sqlite`, `charlotte`) require no installation — just `module add "name"` in your code.

---

### `cocotte test [dir]`

Run all `*_test.cot` files under the given directory (default: `tests/`).

```sh
cocotte test
cocotte test tests/math_test.cot   # single file
cocotte test --verbose
```

Exits 0 if all pass, non-zero otherwise. Safe for CI.

---

### `cocotte exec <task>`

Run a task defined in the `Charlotfile`.

```sh
cocotte exec build
cocotte exec deploy
cocotte exec list       # list all available tasks
```

---

### Other commands

| Command | Description |
|---------|-------------|
| `cocotte clean` | Delete `dist/`, `build/`, cache |
| `cocotte package [--format zip\|tar]` | Archive `dist/` |
| `cocotte repl` | Interactive REPL |
| `cocotte disasm <file>` | Print bytecode disassembly |

---

## 5. Syntax

### Comments

```cocotte
# Single-line comment
var x = 10  # inline comment
```

There is no multi-line comment syntax.

---

### Variables

Declared with `var`. Dynamically typed. Reassignment uses no keyword.

```cocotte
var name   = "Alice"
var age    = 30
var active = true
var score  = nil

age = 31         # reassign — no var
```

`nil` means "no value".

---

### Types

| Type | Example |
|------|---------|
| `number` | `0`, `3.14`, `-7` |
| `string` | `"hello"`, `"line\nbreak"` |
| `bool` | `true`, `false` |
| `nil` | `nil` |
| `list` | `[1, 2, 3]`, `[]` |
| `map` | `{"key": "value"}`, `{}` |
| `func` | `func(x) return x * 2 end` |

```cocotte
print type_of(42)       # number
print type_of("hi")     # string
print type_of([1,2])    # list
print type_of(nil)      # nil
```

---

### Operators

#### Arithmetic

| Operator | Description | Example |
|----------|-------------|---------|
| `+` | Add / string concat | `5 + 3` → `8`, `"a" + "b"` → `"ab"` |
| `-` | Subtract | `5 - 3` → `2` |
| `*` | Multiply | `4 * 3` → `12` |
| `/` | Divide | `10 / 3` → `3.333...` |
| `divide A by B` | Divide (English form) | `divide 10 by 3` → `3.333...` |
| `%` | Remainder | `10 % 3` → `1` |

Both `/` and `divide A by B` are valid — use whichever reads better. For integer division use `floor(10 / 3)`.

`+` coerces numbers and bools to strings automatically: `"Score: " + 42` → `"Score: 42"`.

---

### String interpolation (f-strings)

Use `f"..."` to embed expressions directly inside a string. Wrap any expression in `{}`:

```cocotte
var name  = "Alice"
var score = 95
print f"Hello, {name}! Your score is {score}."
# Hello, Alice! Your score is 95.

var item  = "apple"
var count = 3
print f"You have {count * 2} {item}s."
# You have 6 apples.
```

- Any valid Cocotte expression works inside `{}`: variables, arithmetic, method calls, function calls.
- Use `{{` and `}}` to emit a literal brace: `f"{{not interpolated}}"` → `"{not interpolated}"`.
- Single-quoted f-strings work too: `f'Hello {name}'`.

#### Comparison

`==`  `!=`  `<`  `>`  `<=`  `>=`

#### Logical

`and`  `or`  `not`

---

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

`elif` and `else` are optional. Every `if` closes with `end`.

---

### Loops

```cocotte
# while
var i = 0
while i < 5
    print i
    i = i + 1
end

# for over a list
for item in ["apple", "banana", "cherry"]
    print item
end

# for with range (end is exclusive)
for i in range(1, 6)
    print i       # 1 2 3 4 5
end

# break and continue work in both loop types
for n in range(1, 100)
    if n > 5
        break
    end
    print n
end
```

---

### Functions

```cocotte
func add(a, b)
    return a + b
end

print add(3, 4)    # 7
```

A `return` with no value returns `nil`. A function that falls off the end also returns `nil`.

**Lambdas:**

```cocotte
var double = func(x) return x * 2 end
print double(5)    # 10
```

**Closures:**

```cocotte
func make_adder(n)
    return func(x) return x + n end
end

var add5 = make_adder(5)
print add5(10)    # 15
```

**Recursion:**

```cocotte
func fib(n)
    if n <= 1
        return n
    end
    return fib(n - 1) + fib(n - 2)
end
print fib(10)    # 55
```

---

### Classes

```cocotte
class Dog
    func init(name, breed)
        self.name  = name
        self.breed = breed
    end

    func bark()
        print self.name + ": Woof!"
    end
end

var rex = Dog("Rex", "Labrador")
rex.bark()    # Rex: Woof!
```

- `init` is the constructor, called automatically.
- `self` refers to the current instance.
- All fields and methods are public.
- No inheritance in 0.1.0 — use composition.

---

### Error handling

```cocotte
try
    var result = divide 10 by 0
catch err
    print "Error: " + err
end
```

The variable after `catch` holds the error message as a string. Use `assert` to raise errors:

```cocotte
assert(age >= 0, "age must not be negative")
assert_eq(result, 42)
```

---

## 6. Built-in Functions

Always available — no `module add` needed.

### Output / input

| Function | Description |
|----------|-------------|
| `print value` | Print to stdout with newline |
| `input("prompt")` | Read a line from stdin, return as string |

### Math

| Function | Description |
|----------|-------------|
| `abs(n)` | Absolute value |
| `sqrt(n)` | Square root |
| `pow(base, exp)` | Power |
| `floor(n)` | Round down |
| `ceil(n)` | Round up |
| `round(n)` | Round to nearest |
| `max(a, b)` | Larger of two |
| `min(a, b)` | Smaller of two |
| `sign(n)` | `-1`, `0`, or `1` |
| `clamp(v, lo, hi)` | Constrain to range |

### Conversion

| Function | Description |
|----------|-------------|
| `to_number(s)` | Parse string to number |
| `to_string(v)` | Convert anything to string |
| `to_bool(v)` | Convert to bool |
| `number_to_int(n)` | Truncate decimal |
| `format_number(n, d)` | Format with `d` decimal places |

### Type checking

| Function | Returns |
|----------|---------|
| `type_of(v)` | `"number"`, `"string"`, `"bool"`, `"nil"`, `"list"`, `"map"`, `"func"` |
| `is_number(v)` | bool |
| `is_string(v)` | bool |
| `is_list(v)` | bool |
| `is_map(v)` | bool |
| `is_bool(v)` | bool |
| `is_nil(v)` | bool |
| `is_func(v)` | bool |

### Characters

| Function | Description |
|----------|-------------|
| `char_code("A")` | Unicode code point → `65` |
| `code_char(65)` | Code point → character `"A"` |

### Collections

| Function | Description |
|----------|-------------|
| `range(start, end)` | List `[start, start+1, ..., end-1]` |
| `len(v)` | Length of string, list, or map |
| `list_of(a, b, ...)` | Create list from arguments |
| `map_of(k, v, k, v, ...)` | Create map from alternating key-value pairs |

### System

| Function | Description |
|----------|-------------|
| `exit(code)` | Terminate with exit code |
| `env_get("VAR")` | Read environment variable (nil if unset) |
| `sleep(seconds)` | Pause execution |
| `random()` | Random float in `[0.0, 1.0)` |
| `time_now()` | Unix timestamp in seconds |

### Assertion

| Function | Description |
|----------|-------------|
| `assert(cond, msg)` | Abort with `msg` if `cond` is false |
| `assert_eq(a, b)` | Abort with diff if `a != b` |

---

## 7. String Methods

Called with a dot: `"hello".upper()`

| Method | Description |
|--------|-------------|
| `.len()` | Number of characters |
| `.is_empty()` | True if zero length |
| `.upper()` | Uppercase copy |
| `.lower()` | Lowercase copy |
| `.trim()` | Remove leading/trailing whitespace |
| `.trim_left()` | Remove leading whitespace |
| `.trim_right()` | Remove trailing whitespace |
| `.get(i)` | Character at index `i` (0-based) |
| `.slice(from, to)` | Substring `[from, to)` |
| `.index_of(sub)` | First position of `sub`; `-1` if absent |
| `.contains(sub)` | True if `sub` is in the string |
| `.starts_with(prefix)` | True if string begins with `prefix` |
| `.ends_with(suffix)` | True if string ends with `suffix` |
| `.replace(from, to)` | Replace all occurrences |
| `.replace_first(from, to)` | Replace only the first occurrence |
| `.split(sep)` | Split on separator, return list |
| `.split_lines()` | Split on newlines, return list |
| `.repeat(n)` | Repeat `n` times |
| `.pad_left(n, char)` | Pad left to total length `n` |
| `.pad_right(n, char)` | Pad right to total length `n` |
| `.to_number()` | Parse as number |
| `.to_list()` | List of single-character strings |

```cocotte
print "  hello  ".trim()                    # "hello"
print "hello".slice(1, 4)                   # "ell"
print "7".pad_left(4, "0")                  # "0007"
print "a,b,c".split(",").join(" + ")        # "a + b + c"
```

---

## 8. List Methods

| Method | Description |
|--------|-------------|
| `.len()` | Number of items |
| `.is_empty()` | True if no items |
| `.get(i)` | Item at index `i` |
| `.first()` | First item |
| `.last()` | Last item |
| `.push(val)` | Append to end (in place) |
| `.pop()` | Remove and return last item (in place) |
| `.pop(i)` | Remove and return item at index `i` (in place) |
| `.insert(i, val)` | Insert `val` before index `i` (in place) |
| `.contains(val)` | True if `val` is present |
| `.index_of(val)` | Index of `val`; `-1` if absent |
| `.slice(from, to)` | Sub-list `[from, to)` |
| `.find(func)` | First item where `func` returns true; nil if none |
| `.filter(func)` | New list of items where `func` returns true |
| `.map(func)` | New list with `func` applied to every item |
| `.reduce(func, init)` | Fold all items starting from `init` |
| `.each(func)` | Call `func` on every item; returns nil |
| `.count(func)` | Number of items where `func` returns true |
| `.sort()` | Sort in place (numbers or strings) |
| `.sort_by(func)` | Sort in place using comparator `func(a, b)` → negative/zero/positive |
| `.reverse()` | Reverse in place |
| `.join(sep)` | Join all items into a string |
| `.extend(other)` | Append all items from `other` (in place) |
| `.copy()` | Shallow copy |
| `.clear()` | Remove all items (in place) |

```cocotte
var nums = [5, 3, 8, 1, 9, 2]
nums.sort()
print nums.join(", ")                                     # 1, 2, 3, 5, 8, 9
print nums.filter(func(n) return n % 2 == 0 end).join(", ")  # 2, 8
print nums.reduce(func(acc, n) return acc + n end, 0)     # 28

# Sort objects by a field
var people = [{"name": "Charlie"}, {"name": "Alice"}, {"name": "Bob"}]
people.sort_by(func(a, b)
    if a.get("name") < b.get("name") return -1 end
    if a.get("name") > b.get("name") return 1 end
    return 0
end)

# Insert and pop by index
nums.insert(0, 99)   # prepend 99
nums.pop(0)          # remove first element
```

---

## 9. Map Methods

| Method | Description |
|--------|-------------|
| `.get(key)` | Value for `key`; nil if missing |
| `.set(key, val)` | Set `key` to `val` (creates if absent) |
| `.has_key(key)` | True if `key` exists |
| `.keys()` | List of all keys |
| `.values()` | List of all values |
| `.len()` | Number of entries |
| `.remove(key)` | Remove `key` and return its value (nil if absent) |
| `.merge(other)` | Copy all entries from `other` into this map (in place) |
| `.entries()` | List of `[key, value]` pairs |

**Dot-access shorthand:** for string keys that are valid identifiers, you can read a map field using `.` instead of `.get()`:

```cocotte
var cfg = {"host": "localhost", "port": 8080}
print cfg.host    # "localhost"  — same as cfg.get("host")
print cfg.port    # 8080

cfg.set("debug", true)
for key in cfg.keys()
    print f"{key}: {cfg.get(key)}"
end

# remove and merge
var extra = {"timeout": 30}
cfg.merge(extra)
cfg.remove("debug")
print cfg.entries().len()   # 3

# Dot-access also works on JSON/SQLite results:
var user = http.get_json("https://api.example.com/me")
print f"Hello, {user.name}!"
```

---

## 10. File I/O

All file functions are built in — no module needed.

```cocotte
write_file("log.txt", "First line\n")
append_file("log.txt", "Second line\n")
var content = read_file("log.txt")
print content

for line in content.split_lines()
    if not line.trim().is_empty()
        print "> " + line
    end
end
```

### Full reference

| Function | Description |
|----------|-------------|
| `read_file(path)` | Read file, return as string |
| `write_file(path, text)` | Write text (overwrites) |
| `append_file(path, text)` | Append text |
| `delete_file(path)` | Delete file or directory |
| `file_exists(path)` | True if path exists |
| `is_file(path)` | True if regular file |
| `is_dir(path)` | True if directory |
| `file_size(path)` | Size in bytes |
| `make_dir(path)` | Create directory (and parents) |
| `list_dir(path)` | List of filenames in directory |
| `copy_file(from, to)` | Copy a file |
| `rename_file(from, to)` | Move or rename a file |

---

## 11. Built-in Modules

Load any of these with `module add "name"` — no installation required.

```cocotte
module add "http"
module add "json"
module add "sqlite"
module add "math"
module add "os"
module add "charlotte"
```

---

### `math`

```cocotte
module add "math"

print math.PI              # 3.14159265358979
print math.E               # 2.71828182845904
print math.TAU             # 6.28318530717958

print math.sin(math.PI / 2)   # 1
print math.cos(0)             # 1
print math.log(math.E)        # 1
print math.log2(8)            # 3
print math.log10(1000)        # 3
print math.sqrt(144)          # 12
print math.pow(2, 10)         # 1024
print math.floor(9.9)         # 9
print math.ceil(9.1)          # 10
print math.round(9.5)         # 10
print math.abs(-42)           # 42
print math.max(3, 7)          # 7
print math.min(3, 7)          # 3
```

| Function/Constant | Description |
|-------------------|-------------|
| `math.PI` | π |
| `math.E` | Euler's number |
| `math.TAU` | 2π |
| `math.sin(n)` | Sine |
| `math.cos(n)` | Cosine |
| `math.tan(n)` | Tangent |
| `math.asin(n)` | Arcsine |
| `math.acos(n)` | Arccosine |
| `math.atan(n)` | Arctangent |
| `math.log(n)` | Natural log |
| `math.log2(n)` | Log base 2 |
| `math.log10(n)` | Log base 10 |
| `math.exp(n)` | eⁿ |
| `math.sqrt(n)` | Square root |
| `math.pow(base, exp)` | Power |
| `math.floor(n)` | Round down |
| `math.ceil(n)` | Round up |
| `math.round(n)` | Round to nearest |
| `math.abs(n)` | Absolute value |
| `math.max(a, b)` | Larger of two |
| `math.min(a, b)` | Smaller of two |

---

### `json`

```cocotte
module add "json"

var data = {"name": "Alice", "scores": [95, 87, 92]}
var text = json.stringify(data)
print text    # {"name":"Alice","scores":[95.0,87.0,92.0]}

var parsed = json.parse(text)
print parsed.get("name")              # Alice
print parsed.get("scores").get(0)    # 95
```

| Function | Description |
|----------|-------------|
| `json.stringify(v)` | Serialize value to JSON string |
| `json.parse(s)` | Deserialize JSON string to Cocotte value |

---

### `os`

```cocotte
module add "os"

print os.platform()                    # "linux", "windows", or "macos"
print os.cwd()                         # current working directory
var out = os.exec("echo hello world")  # run shell command, return stdout
print out
```

| Function | Description |
|----------|-------------|
| `os.platform()` | `"linux"`, `"windows"`, or `"macos"` |
| `os.cwd()` | Current working directory as string |
| `os.exec(cmd)` | Run shell command, return stdout as string |

---

### `http`

HTTP **client** and **server** — backed by [ureq](https://github.com/algesten/ureq) (pure Rust, bundled TLS) for outbound requests, and a built-in synchronous TCP server for inbound requests.

#### Client functions

```cocotte
module add "http"

# GET — returns body as string
var body = http.get("https://example.com")
print body

# GET JSON — fetches and parses JSON in one call
var user = http.get_json("https://api.example.com/users/1")
print user.get("name")

# GET with custom headers
var headers = {"Authorization": "Bearer mytoken", "Accept": "application/json"}
var data = http.get_json("https://api.example.com/protected", headers)

# POST plain text
http.post("https://api.example.com/log", "something happened")

# POST JSON — serializes value automatically, sets Content-Type header
var new_user = {"name": "Alice", "email": "alice@example.com"}
var response = http.post_json("https://api.example.com/users", new_user)

# PUT
http.put("https://api.example.com/users/1", "updated body")

# DELETE
http.delete("https://api.example.com/users/1")
```

| Function | Description |
|----------|-------------|
| `http.get(url [, headers])` | GET request; returns body string |
| `http.get_json(url [, headers])` | GET request; parses JSON response into Cocotte value |
| `http.post(url, body [, headers])` | POST with plain string body |
| `http.post_json(url, value [, headers])` | POST with auto-serialized JSON body |
| `http.put(url, body [, headers])` | PUT request |
| `http.delete(url [, headers])` | DELETE request |

`headers` is an optional map of `{"Header-Name": "value"}`. All client functions return the response body as a string except `get_json`, which returns a parsed Cocotte value. Errors throw a catchable runtime error.

#### Server functions

`http.serve` and `http.serve_static` both **block forever** — call them at the end of your program.

##### `http.serve(port, handler)`

Starts an HTTP server on `port`. `handler` is a Cocotte function called for every incoming request.

**Request map** (argument received by the handler):

| Key | Type | Description |
|-----|------|-------------|
| `"method"` | string | `"GET"`, `"POST"`, `"PUT"`, `"DELETE"`, … |
| `"path"` | string | URL path, e.g. `"/api/users"` |
| `"query"` | string | Raw query string, e.g. `"q=hello&page=2"` (may be `""`) |
| `"headers"` | map | Header name (lowercase) → value |
| `"body"` | string | Raw request body (may be `""`) |

**Response map** (value returned by the handler):

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `"status"` | number | `200` | HTTP status code |
| `"body"` | string | `""` | Response body |
| `"headers"` | map | `{}` | Extra response headers |

Returning a plain string is also valid — it produces a `200` text response.

```cocotte
module add "http"
module add "json"
module add "sqlite"

var db = sqlite.open("app.db")
sqlite.exec(db, "CREATE TABLE IF NOT EXISTS items(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)")

http.serve(8080, func(req)
    var method = req.get("method")
    var path   = req.get("path")
    var json_h = {"Content-Type": "application/json; charset=utf-8"}

    if method == "GET" and path == "/items"
        var rows = sqlite.query(db, "SELECT * FROM items")
        return {"status": 200, "body": json.stringify(rows), "headers": json_h}
    end

    if method == "POST" and path == "/items"
        try
            var data = json.parse(req.get("body"))
            sqlite.exec(db, "INSERT INTO items(name) VALUES('" + data.get("name") + "')")
            return {"status": 201, "body": "{\"ok\":true}", "headers": json_h}
        catch err
            return {"status": 500, "body": "{\"error\":\"" + err + "\"}", "headers": json_h}
        end
    end

    return {"status": 404, "body": "{\"error\":\"not found\"}", "headers": json_h}
end)
```

##### `http.serve_static(port, dir)`

Serves all files in `dir` over HTTP on `port`. Content-Type is inferred from the file extension. Requests for `/` serve `dir/index.html`. Missing files return `404`. Directory traversal is blocked.

```cocotte
module add "http"

# Serve the "public" directory on port 3000
http.serve_static(3000, "public")
```

Supported Content-Types: `.html`, `.css`, `.js`, `.json`, `.png`, `.jpg`/`.jpeg`, `.svg`, `.ico`, `.woff2`, `.woff`, `.ttf`, `.webp`, `.gif`.

#### Combining client, server, and sqlite

```cocotte
module add "http"
module add "json"
module add "sqlite"

var db = sqlite.open("notes.db")
sqlite.exec(db, "CREATE TABLE IF NOT EXISTS notes(id INTEGER PRIMARY KEY AUTOINCREMENT, text TEXT)")

func handle(req)
    var j = {"Content-Type": "application/json; charset=utf-8"}

    if req.get("method") == "GET" and req.get("path") == "/"
        return {"status": 200, "body": read_file("static/index.html"),
                "headers": {"Content-Type": "text/html; charset=utf-8"}}
    end

    if req.get("method") == "GET" and req.get("path") == "/api/notes"
        return {"status": 200, "body": json.stringify(sqlite.query(db, "SELECT * FROM notes")), "headers": j}
    end

    if req.get("method") == "POST" and req.get("path") == "/api/notes"
        var d = json.parse(req.get("body"))
        sqlite.exec(db, "INSERT INTO notes(text) VALUES('" + d.get("text") + "')")
        return {"status": 201, "body": "{\"ok\":true}", "headers": j}
    end

    return {"status": 404, "body": "{\"error\":\"not found\"}", "headers": j}
end

print "Listening on port 9192"
http.serve(9192, func(req) return handle(req) end)
```

---

### `sqlite`

Embedded SQLite database backed by [rusqlite](https://github.com/rusqlite/rusqlite) — SQLite is compiled into the binary, no system package needed.

The "db handle" returned by `sqlite.open()` is just the file path string. Pass it to every function.

```cocotte
module add "sqlite"

# Open (creates the file if it doesn't exist)
var db = sqlite.open("app.db")

# Create a table
sqlite.exec(db, "CREATE TABLE IF NOT EXISTS users (
    id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    age  INTEGER
)")

# Insert rows
sqlite.exec(db, "INSERT INTO users (name, age) VALUES ('Alice', 30)")
sqlite.exec(db, "INSERT INTO users (name, age) VALUES ('Bob', 25)")

# Query all rows — returns a list of maps
var rows = sqlite.query(db, "SELECT * FROM users")
for row in rows
    print row.get("id") + ": " + row.get("name") + " (age " + row.get("age") + ")"
end

# Query one row — returns a map or nil
var user = sqlite.query_one(db, "SELECT * FROM users WHERE name = 'Alice'")
if user != nil
    print "Found: " + user.get("name")
end

# List all tables
var tables = sqlite.tables(db)
print tables.join(", ")    # users
```

| Function | Description |
|----------|-------------|
| `sqlite.open(path)` | Open or create a database file; returns db handle (string) |
| `sqlite.exec(db, sql)` | Execute SQL with no return value (CREATE, INSERT, UPDATE, DELETE) |
| `sqlite.exec_params(db, sql, params)` | **Safe** parameterised exec — pass user data as a list, never as string interpolation |
| `sqlite.query(db, sql)` | Execute SELECT; returns list of maps (one map per row) |
| `sqlite.query_params(db, sql, params)` | **Safe** parameterised SELECT |
| `sqlite.query_one(db, sql)` | Execute SELECT; returns first row as map, or nil |
| `sqlite.tables(db)` | Returns list of table name strings |

Each row map has column names as keys. Values are typed: integers and floats become `number`, text becomes `string`, NULL becomes `nil`, blobs become a hex string.

**Parameterised queries (recommended for user data):** use `?` placeholders in SQL and pass values as a list. This completely prevents SQL injection — no manual escaping needed.

```cocotte
module add "sqlite"

var db = sqlite.open("app.db")
sqlite.exec(db, "CREATE TABLE IF NOT EXISTS items(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)")

# Safe insert — user input goes in the list, never in the SQL string
var user_input = "O'Brien"
sqlite.exec_params(db, "INSERT INTO items(name) VALUES(?)", [user_input])

# Safe query
var name_filter = "Alice"
var rows = sqlite.query_params(db, "SELECT * FROM items WHERE name = ?", [name_filter])
for row in rows
    print f"{row.id}: {row.name}"
end

# Multiple params
sqlite.exec_params(db, "UPDATE items SET name=? WHERE id=?", ["Bob", 1])
```

---

### `charlotte`

See [§13 GUI — Charlotte](#13-gui--charlotte).

---

## 12. Writing Libraries and Modules

Cocotte has two kinds of reusable code files. Here is how to create each one from scratch.

| | Library (`.cotlib`) | Module (`.cotmod`) |
|--|--------------------|--------------------|
| Loaded with | `library add "libraries/mylib.cotlib"` | `module add "mymod"` |
| Addressed by | File path (relative to project root) | Name only |
| Lives in | `libraries/` | `modules/` |
| Best for | Single-project utilities | Distributable packages |

Both are plain Cocotte source files. The interpreter runs the file once and exposes every top-level definition as the namespace you call into. There is no `export` keyword — everything at the top level is automatically exported.

---

### Creating and using a library

```sh
# Inside a project — scaffolds the file and registers it in Millet.toml
cocotte new lib mymath

# Or create the file manually:
# libraries/mymath.cotlib
```

Edit `libraries/mymath.cotlib`:

```cocotte
# libraries/mymath.cotlib
# Everything defined here is visible as mymath.xxx in the caller.

func square(n)
    return n * n
end

func is_even(n)
    return n % 2 == 0
end

var PI = 3.14159265358979
```

Use it from `src/main.cot`:

```cocotte
# Path is relative to the project root, not to src/
library add "libraries/mymath.cotlib"

print mymath.square(5)    # 25
print mymath.is_even(4)   # true
print mymath.PI           # 3.14159265358979
```

The namespace name is always the filename stem (`mymath` from `mymath.cotlib`).

To install someone else's library into your project:

```sh
cocotte add path/to/their_lib.cotlib
# Copies it to libraries/ and updates Millet.toml
```

---

### Creating and distributing a module

```sh
cocotte new module mymodule
# Creates mymodule/mymodule.cotmod
#         mymodule/mymodule_test.cot
#         mymodule/README.md
```

Edit `mymodule/mymodule.cotmod`:

```cocotte
# mymodule/mymodule.cotmod
# Modules can themselves import other modules.
module add "json"

func greet(name)
    print "Hello from mymodule, " + name + "!"
end

func serialize(data)
    return json.stringify(data)
end
```

Install it into a project:

```sh
cd MyProject
cocotte add ../mymodule/mymodule.cotmod
# Copies to modules/mymodule.cotmod and updates Millet.toml
```

Use it:

```cocotte
module add "mymodule"

mymodule.greet("Alice")                          # Hello from mymodule, Alice!
print mymodule.serialize({"x": 1})               # {"x":1}
```

---

### What can go in a library or module

- Functions (`func`)
- Classes (`class`)
- Constants (`var NAME = value`)
- Other `module add` / `library add` calls (they compose)
- Anything you can write in a regular `.cot` file

---

### Registering dependencies in Millet.toml

When you use `cocotte add`, `cocotte new lib`, or `cocotte new module`, your `Millet.toml` is updated automatically. You can also edit it by hand:

```toml
[dependencies]
modules   = ["json", "http", "mymodule"]
libraries = ["libraries/mymath.cotlib", "libraries/utils.cotlib"]
```

Built-in modules (`json`, `math`, `os`, `http`, `sqlite`, `charlotte`) do not need to be listed — they are always available with `module add`.

---

## 13. GUI — Charlotte

Charlotte is Cocotte's GUI module using [egui](https://github.com/emilk/egui) + eframe. Works on Linux (Wayland + X11), Windows, and macOS.

```cocotte
module add "charlotte"

var state = {"count": 0}

charlotte.window("My App", 480, 320, func(ui)
    ui.heading("Counter")
    ui.label("Count: " + state.get("count"))
    if ui.button("Add")
        state.set("count", state.get("count") + 1)
    end
end)
```

### Persistent state

The callback runs ~60 times per second. Variables declared *inside* the callback reset every frame. To keep state across frames, store it in a `map` declared *outside* the callback — maps are reference types, so changes inside the callback stick.

```cocotte
var state = {"text": "", "items": []}

charlotte.window("App", 500, 400, func(ui)
    # state.get("text") persists between frames
    var t = ui.input("my_input", "Type here...")
    state.set("text", t)
end)
```

### Widget reference

#### Text display

```cocotte
ui.label("Normal text")
ui.heading("Large heading")
ui.monospace("fixed-width text")
ui.colored_label("red", "Red text")
ui.colored_label("#FF8800", "Orange text")
ui.separator()
ui.space()
ui.add_space(20)
```

Color names: `red`, `green`, `blue`, `yellow`, `orange`, `purple`, `cyan`, `pink`, `gray`, `white`, `black`. Or `"#RRGGBB"` hex.

#### Buttons

```cocotte
if ui.button("Click me")
    print "clicked"
end

if ui.small_button("Small")
    print "small click"
end

if ui.link("Open docs")
    os.exec("xdg-open https://example.com")
end
```

All three return `true` on the frame they are clicked.

#### Text input

```cocotte
# ui.input(key, placeholder) -> string
var name = ui.input("name_field", "Enter your name...")

# ui.multiline_input(key, placeholder) -> string
var notes = ui.multiline_input("notes_field", "Write here...")
```

`key` is a unique string per input field — it tracks state between frames. Use a different key for each field.

#### Checkbox

```cocotte
# ui.checkbox(key, label [, default]) -> bool
var enabled = ui.checkbox("feat_toggle", "Enable feature", false)
if enabled
    ui.label("Feature is ON")
end
```

#### Radio buttons

```cocotte
# ui.radio(group_key, label, value) -> bool
if ui.radio("color", "Red", "red")
    ui.colored_label("red", "Red selected")
end
if ui.radio("color", "Blue", "blue")
    ui.colored_label("blue", "Blue selected")
end
```

All radios sharing the same `group_key` form a group. Returns true when that option is selected.

#### Slider

```cocotte
# ui.slider(key, label, min, max [, default]) -> number
var volume = ui.slider("vol", "Volume", 0, 100, 50)
ui.label("Volume: " + volume)
```

#### Progress bar

```cocotte
ui.progress(0.75)    # 0.0 to 1.0
```

### Layout

```cocotte
# Row — horizontal
ui.row(func()
    ui.label("Left")
    if ui.button("Middle") end
    ui.label("Right")
end)

# Column — vertical (useful inside row)
ui.row(func()
    ui.column(func()
        ui.heading("Section A")
        ui.label("Item 1")
    end)
    ui.column(func()
        ui.heading("Section B")
        ui.label("Item 2")
    end)
end)

# Group — bordered box
ui.group(func()
    ui.label("Boxed content")
    if ui.button("Action") end
end)

# Scroll area
ui.scroll(func()
    for i in range(0, 100)
        ui.label("Row " + i)
    end
end)

# Collapsible section
ui.collapsible("Advanced Settings", func()
    var val = ui.slider("adv", "Setting", 0, 100, 50)
end)
```

### Window size

```cocotte
var w = ui.available_width()
var h = ui.available_height()
```

---

## 14. Charlotfile — Task Runner

The `Charlotfile` defines named tasks — sequences of shell commands. Replaces `Makefile`.

```toml
[project]
name   = "MyApp"
author = "You"

[variables]
OUT  = "dist"
PORT = "8080"

[tasks.run]
cocotte run

[tasks.build]
cocotte build --release

[tasks.serve]
cd ${OUT} && python3 -m http.server ${PORT}

[tasks.deploy]
cocotte build --release
scp dist/MyApp user@server:/opt/myapp/
ssh user@server "systemctl restart myapp"

[tasks.clean]
cocotte clean
rm -rf ${OUT}
```

Each `[tasks.<name>]` section contains shell commands, one per line. Commands run in order; a non-zero exit stops the task. Variable references use `${VAR}`.

```sh
cocotte exec build
cocotte exec deploy
cocotte exec list    # list all tasks
```

Multi-language projects:

```toml
[tasks.build_all]
cocotte build --release
cd backend && cargo build --release
cd frontend && npm run build
```

---

## 15. Millet.toml — Project Config

```toml
[project]
name    = "MyApp"
version = "1.0.0"
author  = "Alice"

[dependencies]
modules   = ["json", "http", "sqlite"]
libraries = ["libraries/mymath.cotlib"]
```

| Key | Description |
|-----|-------------|
| `project.name` | Project name; used as default binary name |
| `project.version` | Semantic version string |
| `project.author` | Author name |
| `dependencies.modules` | Module names (built-in or from `modules/`) |
| `dependencies.libraries` | Library paths relative to project root |

`Millet.toml` is created by `cocotte init` and updated automatically by `cocotte add`, `cocotte new lib`, and `cocotte new module`.

---

## 16. Cross-Compilation

`cocotte build` uses Rust's cross-compilation toolchain. `--os` and `--arch` accept multiple values — all combinations are built.

```sh
cocotte build --os windows --arch x86_64
cocotte build --os linux macos --arch aarch64
cocotte build --os linux windows macos --arch x86_64 aarch64   # 6 binaries
```

### Supported targets

| OS | x86_64 | aarch64 | armv7 | i686 | riscv64 |
|----|:------:|:-------:|:-----:|:----:|:-------:|
| Linux | ✓ | ✓ | ✓ | ✓ | ✓ |
| Windows | ✓ | ✓ | | ✓ | |
| macOS | ✓ | ✓ | | | |
| BSD | ✓ | ✓ | | | |

**OS aliases:** `win`, `mac`/`darwin`/`osx`, `freebsd`/`openbsd`/`netbsd`
**Arch aliases:** `amd64`/`x64`, `arm64`, `arm`, `i386`/`x86`

### Toolchain setup (Linux host examples)

```sh
# Linux → Windows x86_64
rustup target add x86_64-pc-windows-gnu
sudo apt install gcc-mingw-w64-x86-64

# Linux → Linux AArch64
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu
```

When the toolchain is absent, `cocotte build` emits a **source bundle** in `dist/<name>_<target>_src/` — a Cargo workspace the user can compile on the target machine with `cargo build --release`.

### Output naming

| Target | Filename |
|--------|----------|
| No flags (native) | `<project>` |
| `linux-x86_64` | `<project>-linux-x86_64` |
| `windows-x86_64` | `<project>-windows-x86_64.exe` |
| `macos-aarch64` | `<project>-macos-aarch64` |

---

## 17. Testing

Test files live in `tests/` and must end in `_test.cot`. They are regular `.cot` files. Any failing `assert` or `assert_eq` prints an error and exits with a non-zero code.

```cocotte
# tests/math_test.cot
library add "libraries/mymath.cotlib"

assert_eq(mymath.square(5),  25)
assert_eq(mymath.square(0),  0)
assert_eq(mymath.is_even(4), true)
assert_eq(mymath.is_even(7), false)

print "All math tests passed."
```

```sh
cocotte test               # run all *_test.cot under tests/
cocotte test --verbose     # show assertion counts
```

---

## 18. Complete Examples

### Calculator

```cocotte
func calculate(a, op, b)
    if op == "+"
        return a + b
    elif op == "-"
        return a - b
    elif op == "*"
        return a * b
    elif op == "/"
        if b == 0
            return nil
        end
        return divide a by b
    elif op == "%"
        return a % b
    end
    return nil
end

print calculate(10, "+", 5)    # 15
print calculate(10, "/", 3)    # 3.333...
```

---

### Fetch and display JSON

```cocotte
module add "http"

var users = http.get_json("https://jsonplaceholder.typicode.com/users")
for user in users
    print user.get("name") + " — " + user.get("email")
end
```

---

### POST data to an API

```cocotte
module add "http"
module add "json"

var payload = {"title": "Buy milk", "completed": false}

try
    var response = http.post_json("https://jsonplaceholder.typicode.com/todos", payload)
    print "Created! Server replied: " + response
catch err
    print "Request failed: " + err
end
```

---

### SQLite contact book

```cocotte
module add "sqlite"

var db = sqlite.open("contacts.db")

sqlite.exec(db, "CREATE TABLE IF NOT EXISTS contacts (
    id    INTEGER PRIMARY KEY AUTOINCREMENT,
    name  TEXT NOT NULL,
    phone TEXT
)")

sqlite.exec(db, "INSERT INTO contacts (name, phone) VALUES ('Alice', '555-0101')")
sqlite.exec(db, "INSERT INTO contacts (name, phone) VALUES ('Bob',   '555-0102')")

var contacts = sqlite.query(db, "SELECT * FROM contacts ORDER BY name")
for c in contacts
    print c.get("name") + ": " + c.get("phone")
end
```

---

### GUI counter

```cocotte
module add "charlotte"

var state = {"count": 0, "step": 1}

charlotte.window("Counter", 400, 280, func(ui)
    ui.heading("Counter")
    ui.separator()
    ui.label("Value: " + state.get("count"))
    ui.space()
    ui.row(func()
        if ui.button("Decrement")
            state.set("count", state.get("count") - state.get("step"))
        end
        if ui.button("Reset")
            state.set("count", 0)
        end
        if ui.button("Increment")
            state.set("count", state.get("count") + state.get("step"))
        end
    end)
    ui.separator()
    var step = ui.slider("step", "Step", 1, 10, 1)
    state.set("step", step)
end)
```

---

### Quick reference card

```
# Variable              # Math (division syntax)
var x = 42              divide x by y
var s = "hello"         floor(divide x by y)
var ok = true
var items = [1, 2, 3]   # Comparison
var data = {}           ==  !=  <  >  <=  >=

# If / elif / else       # Logical
if cond                 and  or  not
    ...
elif other
    ...
else
    ...
end

# While                  # For
while cond              for item in list
    ...                     ...
end                     end
                        for i in range(0, 10)
                            ...
                        end

# Function               # Lambda
func name(a, b)         var f = func(x) return x * 2 end
    return a + b
end

# Class                  # Error handling
class Dog               try
    func init(name)         ...
        self.name = name    catch err
    end                     print err
    func bark()         end
        print self.name
    end
end                     # Load a module     Load a library
                        module add "json"   library add "libraries/mylib.cotlib"
```

---

## 19. Built-in Modules — Full Reference

All modules load with `module add "name"` — no installation required.

### New in 0.1.2

#### `path` — Cross-platform path manipulation

```cocotte
module add "path"

print path.join("home", "user", "docs", "file.txt")  # home/user/docs/file.txt
print path.basename("/home/user/file.txt")            # file.txt
print path.dirname("/home/user/file.txt")             # /home/user
print path.ext("/home/user/file.txt")                 # .txt
print path.stem("/home/user/file.txt")                # file
print path.abs("../myfile.txt")                       # absolute path
print path.exists("/etc/hosts")                       # true
print path.is_abs("/etc/hosts")                       # true
print path.parts("/home/user")                        # ["/", "home", "user"]
print path.home()                                     # /home/yourname
```

#### `env` — Environment variables

```cocotte
module add "env"

var port = env.get("PORT")                  # nil if not set
var port = env.get_or("PORT", "8080")       # default if not set
var token = env.require("API_TOKEN")        # throws if not set

env.set("DEBUG", "true")
env.remove("TEMP_VAR")

var all_vars = env.all()                    # map of all env vars
```

#### `args` — Command-line arguments

```cocotte
module add "args"

# cocotte run app.cot --port 9000 --verbose

var all  = args.all()                       # ["--port", "9000", "--verbose"]
var n    = args.len()                       # 3
var port = args.option("--port")            # "9000"
var verbose = args.flag("--verbose")        # true
var first = args.get(0)                     # "--port"
```

#### `uuid` — UUID generation

```cocotte
module add "uuid"

var id = uuid.v4()                          # "f47ac10b-58cc-4372-a567-0e02b2c3d479"
print uuid.is_valid(id)                     # true
print uuid.is_valid("not-a-uuid")           # false
```

#### `log` — Structured logging

```cocotte
module add "log"

log.set_level("debug")    # "debug" | "info" | "warn" | "error"

log.debug("Connecting to db")
log.info("Server started on port", 8080)
log.warn("Rate limit approaching")
log.error("Connection failed:", err)
# Output: [INFO]  1712345678 Server started on port 8080
```

#### `process` — Spawn subprocesses

```cocotte
module add "process"

# Run a shell command and capture output
var result = process.run("ls -la")
print result.stdout
print result.stderr
print result.code      # exit code
print result.ok        # true if code == 0

# Run with explicit args (no shell injection risk)
var r = process.run_args("git", ["log", "--oneline", "-5"])
print r.stdout

print process.pid()   # current process ID
process.exit(0)       # exit with code
```

#### `csv` — Parse and write CSV

```cocotte
module add "csv"

var text = read_file("data.csv")

# Parse as list of lists
var rows = csv.parse(text)
for row in rows
    print row.join(" | ")
end

# Parse with headers → list of maps
var records = csv.parse_with_headers(text)
for rec in records
    print f"{rec.name}: {rec.email}"
end

# Write CSV
var data = [["name", "score"], ["Alice", "95"], ["Bob", "87"]]
write_file("out.csv", csv.stringify(data))
```

#### `crypto` — Hashing

```cocotte
module add "crypto"

print crypto.sha256("hello world")
# b94d27b9934d3e08a52e52d7da7dabfac484efe04294e576fea0

print crypto.md5("hello")
# 5d41402abc4b2a76b9719d911017c592
```

#### `base64` — Encode and decode

```cocotte
module add "base64"

var encoded = base64.encode("Hello, Cocotte!")
print encoded                              # SGVsbG8sIENvY290dGUh

var decoded = base64.decode(encoded)
print decoded                              # Hello, Cocotte!
```

---

## 20. Cross-Compilation

`cocotte build` uses Rust's cross-compilation toolchain. `--os` and `--arch` accept multiple values — all combinations are built.

```sh
cocotte build --native                               # release binary for this machine
cocotte build --os windows --arch x86_64             # Windows 64-bit
cocotte build --os linux --arch aarch64              # Linux ARM64
cocotte build --os linux windows --arch x86_64 aarch64  # 4 binaries at once
```

### What cocotte build does automatically

- Calls `rustup target add <triple>` for each target (if rustup is present)
- Writes `.cargo/config.toml` with the correct cross-linker
- Warns when a required cross-linker is missing and shows the install command
- Disables the GUI (Charlotte/egui) for cross-targets — GUI requires a native display

### Required cross-linkers (Linux host)

| Target | Install |
|--------|---------|
| Linux x86_64 | `sudo apt-get install gcc` |
| Linux aarch64 | `sudo apt-get install gcc-aarch64-linux-gnu` |
| Linux armv7 | `sudo apt-get install gcc-arm-linux-gnueabihf` |
| Linux i686 | `sudo apt-get install gcc-i686-linux-gnu` |
| Linux riscv64 | `sudo apt-get install gcc-riscv64-linux-gnu` |
| Windows x86_64 | `sudo apt-get install gcc-mingw-w64-x86-64` |
| Windows i686 | `sudo apt-get install gcc-mingw-w64-i686` |
| macOS | Requires [osxcross](https://github.com/tpoechtrager/osxcross) — not auto-configured |

### Supported targets

| OS | x86_64 | aarch64 | armv7 | i686 | riscv64 |
|----|:------:|:-------:|:-----:|:----:|:-------:|
| Linux | ✓ | ✓ | ✓ | ✓ | ✓ |
| Windows (GNU) | ✓ | ✓ | | ✓ | |
| macOS | ✓ | ✓ | | | |
| BSD | ✓ | ✓ | | | |

When a required cross-linker is not found, `cocotte build` emits a **source bundle** in `dist/<name>_<target>_src/` — a Cargo workspace the user can compile on the target machine with `cargo build --release`.

---

## 21. Planned Modules

These are not yet implemented:

| Module | Purpose |
|--------|---------|
| `regex` | Regular expressions |
| `datetime` | Date/time parsing, formatting, arithmetic |
| `template` | Simple string templating engine |
| `term` | Terminal colors, cursor movement, raw mode |
| `zip` | Compress and extract `.zip` archives |
| `toml` | Parse and write TOML files |
