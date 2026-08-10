# moirspy-copt

Native COPT backend for `moirspy`.

This crate is currently a project scaffold. Generate the COPT 8.0.6 C API
bindings into `crates/moi-solver-copt/src/bindings/gen80.rs` before implementing
the native environment and model wrappers. The final extension will discover
the native library through `COPT_HOME` or an explicit `dll_path`.
