# Repository contribution rules

## Rust verification

For every change under `src-tauri/`, run these commands from `src-tauri/` before considering the work complete. Fix all failures and warnings; do not suppress warnings solely to pass validation.

```powershell
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --all
cargo test --workspace
```

Report the result of each command in the completion summary. If a command cannot run because of the environment, state the reason and do not claim it passed.
