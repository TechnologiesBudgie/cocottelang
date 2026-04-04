# Changelog

## 0.1.2
- **f-strings:** `f"Hello {name}! Score: {score * 2}"` — embed any expression inside `{}`
- **`/` division operator:** `10 / 3` now works alongside `divide 10 by 3`
- **Parameterised SQL:** `sqlite.exec_params(db, sql, [val, ...])` and `sqlite.query_params` — safe user input, no SQL injection possible
- **Map new methods:** `.remove(key)`, `.merge(other)`, `.entries()` → list of `[key, val]` pairs
- **Map dot-access:** `row.name` works as shorthand for `row.get("name")` on any map
- **List new methods:** `.pop(i)` remove by index, `.insert(i, val)`, `.sort_by(func)` custom comparator
- **`--native` build flag:** `cocotte build --native` compiles a release binary for the current machine without needing `--os`/`--arch`
- **Bundled Rust toolchain:** `cocotte build` now auto-downloads a minimal Rust toolchain into `~/.cocotte/toolchain/` if `cargo` is not on PATH — no manual Rust install required
- **JSON integer fix:** `json.stringify` now emits `95` instead of `95.0` for whole numbers

## 0.1.1
- Enhanced Charlotte
- Added HTTP and SQL modules

## 0.1.0
- Initial release (compiler + CLI basic)
- Minimal Charlotte implemented
