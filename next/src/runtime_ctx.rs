// runtime_ctx.rs — Shared interpreter pointer for native modules
//
// Both the charlotte GUI module and the http server module need to call back
// into the Cocotte interpreter from native code.  The interpreter registers
// itself here at the start of every `run()` call, and native modules read the
// pointer to invoke Cocotte callbacks.
//
// Safety contract:
//   - The pointer is only valid while `Interpreter::run()` is on the call stack.
//   - Cocotte is single-threaded; the thread-local ensures no data races.

use std::cell::RefCell;

thread_local! {
    static ACTIVE_INTERP: RefCell<usize> = RefCell::new(0);
}

/// Called by the interpreter at the start of every `run()` invocation.
pub fn set_active_interpreter(ptr: usize) {
    ACTIVE_INTERP.with(|p| *p.borrow_mut() = ptr);
}

/// Returns the raw pointer to the active interpreter, or 0 if none.
pub fn get_active_interpreter() -> usize {
    ACTIVE_INTERP.with(|p| *p.borrow())
}
