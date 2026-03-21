# CocotteDemo — Full Feature Demo Project

This project demonstrates everything Cocotte can do.
Drop the `cocotte` binary in your PATH, then follow the steps below.

---

## Quick Start

```
# Add cocotte to PATH first:
sudo cp cocotte_linux_x86_64 /usr/local/bin/cocotte
sudo chmod +x /usr/local/bin/cocotte

# Run the main demo (variables, strings, lists, maps, functions, classes, modules):
cocotte run src/main.cot
```

---

## All Demos

### Via task runner (recommended)

```
cocotte exec run             # main demo — all core features
cocotte exec run-classes     # OOP: classes, objects, state machines
cocotte exec run-functional  # functional: HOF, closures, map/filter/reduce
cocotte exec run-files       # file I/O, JSON, CSV, config files
cocotte exec run-gui         # GUI window (requires gui feature — see below)
cocotte exec run-all         # run everything except GUI in sequence
```

### Direct

```
cocotte run src/main.cot
cocotte run src/classes_demo.cot
cocotte run src/functional_demo.cot
cocotte run src/files_demo.cot
cocotte run src/gui_demo.cot
```

---

## Tests

```
cocotte test                 # runs all *_test.cot files in tests/
```

Tests cover: arithmetic, strings, lists, maps, control flow, functions,
closures, error handling, and the utils library.

---

## Build

```
cocotte build src/main.cot --release
./dist/CocotteDemo
```

This compiles main.cot into a standalone native binary (requires `cargo` in PATH).

---

## GUI (Charlotte)

The GUI demo uses egui, which needs to be compiled in.

**Step 1** — In `Cargo.toml` of the cocotte source, uncomment:
```toml
eframe = "0.29"
egui   = "0.29"
```
and the `[features]` section.

**Step 2** — Rebuild cocotte:
```
cargo build --release --features gui
```

**Step 3** — Run the GUI demo:
```
cocotte run src/gui_demo.cot
```

The GUI app includes:
- Counter with slider and progress bar
- Calculator with math functions
- Notes app with done/delete
- Widget showcase (checkboxes, radio, sliders, colors, text inputs)
- Activity log

---

## Project Structure

```
CocotteDemo/
  Millet.toml              project config
  Charlotfile               task runner
  README.md                 this file
  src/
    main.cot               core features demo
    classes_demo.cot       OOP demo
    functional_demo.cot    functional programming demo
    files_demo.cot         file I/O demo
    gui_demo.cot           Charlotte GUI demo
  libraries/
    utils.cotlib           utility library (math, string, list helpers)
  tests/
    core_test.cot          100+ tests for core language
    utils_test.cot         tests for utils library
  dist/                    compiled binaries (created by cocotte build)
```

---

## What This Tests

| Feature | Where |
|---|---|
| Variables, types, nil | main.cot |
| Arithmetic, math module | main.cot |
| All string methods | main.cot |
| All list methods (map/filter/reduce/etc.) | main.cot, functional_demo.cot |
| Map methods | main.cot, classes_demo.cot |
| if/elif/else, while, for, break, continue | main.cot |
| Functions, recursion, HOF, closures | main.cot, functional_demo.cot |
| Classes, OOP, self | classes_demo.cot |
| Error handling (try/catch) | main.cot, files_demo.cot |
| File read/write/append/delete/copy/rename | files_demo.cot |
| Directory create/list/delete | files_demo.cot |
| JSON module | main.cot, files_demo.cot |
| Math module | main.cot |
| OS module | files_demo.cot |
| Local library (.cotlib) | main.cot, tests/ |
| Charlotfile task runner | all via `cocotte exec` |
| GUI with Charlotte | gui_demo.cot |
| Build to native binary | `cocotte exec build` |
| Testing with assert_eq | tests/ |
