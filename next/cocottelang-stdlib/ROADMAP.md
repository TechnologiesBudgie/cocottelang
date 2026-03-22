
# ROADMAP

This document outlines the current state and future plans for the Cocotte programming language and its standard library.

## Current State

### Working Features

*   **Interpreter:** The tree-walking interpreter (`cocotte run`) is functional.
*   **Bytecode VM:** A bytecode VM (`cocotte run --bytecode`) is available.
*   **Build System:** The build system (`cocotte build`) can package Cocotte scripts into standalone executables by embedding the interpreter.
*   **Core Syntax:** Most of the core language syntax (variables, functions, classes, control flow) is implemented.
*   **Built-in Functions:** A good set of built-in functions for I/O, string manipulation, and type conversions are available globally.
*   **REPL:** An interactive REPL (`cocotte repl`) is available for experimentation.

### Partially Implemented / Incomplete

*   **Standard Library:** Many modules exist but are not correctly loaded by the `module add` statement. Some modules contain bugs.
*   **Native Compilation:** The `--native` flag is a stub and does not yet produce true native binaries.
*   **Package Manager:** A dummy `pkg` command exists but is not yet implemented.
*   **Cross-Platform Support:** Dummy files for different OSes have been added, but full compatibility is not yet implemented.

## Future Plans

### Short Term

*   **Fix Standard Library:** Ensure all standard library modules can be loaded correctly with `module add` and fix all known bugs.
*   **Threading/Concurrency:** Add a core module for threading and concurrency primitives.
*   **Complete Package Manager:** Implement the package manager for installing, updating, and removing Cocotte packages.

### Medium Term

*   **Native Compilation (AOT):** Implement a true Ahead-of-Time (AOT) compiler that generates native machine code (e.g., via LLVM or Cranelift) for the `--native` flag. This will provide significant performance improvements.
*   **Full Cross-Platform Compatibility:** Implement full support for macOS, Windows, BSD, and ChromeOS.
*   **GUI Library (Charlotte):** Continue to expand and improve the `charlotte` GUI library.

### Long Term

*   **Language Features:** Explore and add new language features, such as improved error handling, decorators, and more advanced type system features.
*   **Performance Optimization:** Profile and optimize the interpreter, VM, and compiled binaries.
*   **Community and Ecosystem:** Foster a community around Cocotte and build a rich ecosystem of libraries and tools.
