# Studio-4T — Claude Guidelines

## Rust

- Never use shorthands. Always write out field names explicitly, even when the variable name matches the field name.

  ```rust
  // BAD
  Foo { x, y }

  // GOOD
  Foo { x: x, y: y }
  ```

- Never use the `?` operator. Always expand it to an explicit `match` block.

  ```rust
  // BAD
  let val = some_result?;

  // GOOD
  let val = match some_result {
      Ok(val) => val,
      Err(e) => return Err(e.into()),
  };
  ```

  For `Option`-returning functions:
  ```rust
  // BAD
  let val = some_option?;

  // GOOD
  let val = match some_option {
      Some(val) => val,
      None => return None,
  };
  ```

## Workflow

This project is human-delivered, AI-developed. The human must stay in full control of what ships.

- **One logical change per session.** Never bundle unrelated changes into a single response. If a task touches more than ~3 files, split it into steps and confirm with the user between each step.
- **Explain before committing.** Always describe what changed and why in plain language before reporting the work as done. No code jargon — write as if explaining to someone who will review the diff.
- **Never mix refactoring with bug fixes.** Each commit must have a single concern. If a bug fix requires a refactor, do them in separate steps.
- **Always verify the build compiles** after any Rust change before reporting done. Run `cargo build` inside `src-tauri/` and confirm it succeeds.
- **Let the user commit.** Do not create git commits unless explicitly asked. Explain the change, then wait.
