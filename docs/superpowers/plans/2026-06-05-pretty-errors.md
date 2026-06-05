# Pretty Error Reporting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace terse byte-offset error output with rustc-style diagnostics using the `ariadne` crate, showing the source line and an underline pointing to the offending token.

**Architecture:** `JianPuError` gains `path: Option<PathBuf>` and a `.with_path()` builder; deep parser internals are unchanged. A new `src/error_reporter.rs` owns all ariadne rendering logic. `main.rs` attaches the path via `map_err` at the top-level boundary and calls `error_reporter::render` instead of `eprintln!`.

**Tech Stack:** Rust, `ariadne 0.6` (diagnostic renderer)

---

### Task 1: Add ariadne dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add ariadne to Cargo.toml**

Edit the `[dependencies]` section of `Cargo.toml` to add:

```toml
ariadne = "0.6"
```

- [ ] **Step 2: Verify it resolves and compiles**

```bash
cargo build
```

Expected: compiles successfully (no errors, possibly new download output).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add ariadne 0.6 for pretty diagnostics"
```

---

### Task 2: Update `src/error.rs`

Remove the unused `Location::Bar` variant, add `path: Option<PathBuf>` and a `.with_path()` builder to `JianPuError`, and simplify `Display` to show only the message.

**Files:**
- Modify: `src/error.rs`

- [ ] **Step 1: Write the failing tests first**

Replace the entire `#[cfg(test)]` block at the bottom of `src/error.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_shows_message() {
        let e = JianPuError::new(Span::new(10, 20), "bad token");
        assert_eq!(format!("{}", e), "error: bad token");
    }

    #[test]
    fn with_path_attaches_path() {
        let e = JianPuError::new(Span::new(0, 1), "oops")
            .with_path("/tmp/test.jianpu");
        assert_eq!(e.path.unwrap().to_str().unwrap(), "/tmp/test.jianpu");
    }

    #[test]
    fn without_path_is_none() {
        let e = JianPuError::new(Span::new(0, 1), "oops");
        assert!(e.path.is_none());
    }
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test error::tests
```

Expected: compile errors — `with_path` doesn't exist yet, `Display` format doesn't match.

- [ ] **Step 3: Rewrite `src/error.rs`**

Replace the entire file contents with:

```rust
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }
}

#[derive(Debug)]
pub struct JianPuError {
    pub span: Span,
    pub message: String,
    pub path: Option<PathBuf>,
}

impl JianPuError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            path: None,
        }
    }

    pub fn with_path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = Some(path.as_ref().to_path_buf());
        self
    }
}

impl std::fmt::Display for JianPuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error: {}", self.message)
    }
}

impl std::error::Error for JianPuError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_shows_message() {
        let e = JianPuError::new(Span::new(10, 20), "bad token");
        assert_eq!(format!("{}", e), "error: bad token");
    }

    #[test]
    fn with_path_attaches_path() {
        let e = JianPuError::new(Span::new(0, 1), "oops")
            .with_path("/tmp/test.jianpu");
        assert_eq!(e.path.unwrap().to_str().unwrap(), "/tmp/test.jianpu");
    }

    #[test]
    fn without_path_is_none() {
        let e = JianPuError::new(Span::new(0, 1), "oops");
        assert!(e.path.is_none());
    }
}
```

Note: `Location` and `Location::Bar` are removed entirely. `JianPuError` now stores `span: Span` directly.

- [ ] **Step 4: Fix compile errors caused by removed `Location`**

The only remaining references to `Location` and `.at_bar()` are in `src/error.rs` itself (now removed) and the test-only `parse_and_group_err` helper in `src/grouper.rs` which is already flagged as unused. Verify with:

```bash
cargo build 2>&1 | grep "error\["
```

If any errors reference `Location::Span`, `Location::Bar`, or `.at_bar()`, fix them by replacing `location: Location::Span(span)` → direct `span` field usage (the new struct has `span: Span` directly).

- [ ] **Step 5: Run all tests**

```bash
cargo test
```

Expected: all tests pass. The old `bar_error_display_*` and `span_error_display` tests are gone; the three new ones pass.

- [ ] **Step 6: Commit**

```bash
git add src/error.rs
git commit -m "refactor: simplify JianPuError — add path field, drop Location::Bar"
```

---

### Task 3: Create `src/error_reporter.rs`

**Files:**
- Create: `src/error_reporter.rs`
- Modify: `src/main.rs` (add `mod error_reporter;`)

- [ ] **Step 1: Write the failing test (in the new file)**

Create `src/error_reporter.rs` with just the test:

```rust
use crate::error::{JianPuError, Span};

pub fn render(e: &JianPuError) {
    render_to_writer(e, std::io::stderr());
}

fn render_to_writer(e: &JianPuError, writer: impl std::io::Write) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn write_temp_file(name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn render_output_contains_message() {
        let path = write_temp_file("test_render.jianpu", "1 2 x 4\n");
        let e = JianPuError::new(Span::new(4, 5), "expected pitch digit 0-7")
            .with_path(&path);

        let mut buf = Vec::new();
        render_to_writer(&e, &mut buf);
        let output = String::from_utf8_lossy(&buf);
        assert!(
            output.contains("expected pitch digit 0-7"),
            "output was: {output}"
        );
    }

    #[test]
    fn render_falls_back_when_path_is_none() {
        let e = JianPuError::new(Span::new(0, 1), "some error");
        let mut buf = Vec::new();
        render_to_writer(&e, &mut buf);
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("some error"), "output was: {output}");
    }

    #[test]
    fn render_falls_back_when_file_unreadable() {
        let e = JianPuError::new(Span::new(0, 1), "some error")
            .with_path("/nonexistent/path.jianpu");
        let mut buf = Vec::new();
        render_to_writer(&e, &mut buf);
        let output = String::from_utf8_lossy(&buf);
        assert!(output.contains("some error"), "output was: {output}");
    }
}
```

Also add `mod error_reporter;` to `src/main.rs` (anywhere near the other `mod` declarations at the top).

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test error_reporter
```

Expected: fails with `todo!()` panic.

- [ ] **Step 3: Implement `render_to_writer`**

Replace the `todo!()` body with the full implementation:

```rust
fn render_to_writer(e: &JianPuError, mut writer: impl std::io::Write) {
    use ariadne::{Label, Report, ReportKind, Source};

    let Some(path) = &e.path else {
        writeln!(writer, "error: {}", e.message).ok();
        return;
    };

    let Ok(source) = std::fs::read_to_string(path) else {
        writeln!(writer, "error: {}", e.message).ok();
        return;
    };

    let filename = path.to_string_lossy().into_owned();
    let span = e.span.start..e.span.end;

    Report::build(ReportKind::Error, filename.as_str(), e.span.start)
        .with_message(&e.message)
        .with_label(
            Label::new((filename.as_str(), span))
                .with_message(&e.message),
        )
        .finish()
        .write(
            (filename.as_str(), Source::from(source.as_str())),
            writer,
        )
        .unwrap();
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test error_reporter
```

Expected: all 3 tests pass.

- [ ] **Step 5: Run full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/error_reporter.rs src/main.rs
git commit -m "feat: add error_reporter with ariadne pretty diagnostics"
```

---

### Task 4: Wire `path` into errors in `src/main.rs`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Update `parse_and_group` to attach the path**

Current `parse_and_group`:

```rust
fn parse_and_group(input: &Path) -> Result<ast::grouped::Score, error::JianPuError> {
    let content = std::fs::read_to_string(input).map_err(|e| {
        error::JianPuError::new(
            error::Span::new(0, 0),
            format!("could not read {:?}: {}", input, e),
        )
    })?;
    let filename = input.to_string_lossy().to_string();
    let doc = parser::parse(&content, &filename)?;
    grouper::group(doc)
}
```

Replace with:

```rust
fn parse_and_group(input: &Path) -> Result<ast::grouped::Score, error::JianPuError> {
    let content = std::fs::read_to_string(input).map_err(|e| {
        error::JianPuError::new(
            error::Span::new(0, 0),
            format!("could not read {:?}: {}", input, e),
        )
    })?;
    let filename = input.to_string_lossy().to_string();
    let doc = parser::parse(&content, &filename)
        .map_err(|e| e.with_path(input))?;
    grouper::group(doc)
        .map_err(|e| e.with_path(input))
}
```

- [ ] **Step 2: Replace `eprintln!` with `error_reporter::render` in `main`**

Current error handling in `main`:

```rust
if let Err(e) = result {
    eprintln!("error: {}", e);
    std::process::exit(1);
}
```

Replace with:

```rust
if let Err(e) = result {
    error_reporter::render(&e);
    std::process::exit(1);
}
```

- [ ] **Step 3: Run full test suite**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Step 4: Smoke test with the real demo file**

```bash
cargo run -- generate pdf demo.jianpu /tmp/demo_out.pdf
```

Expected: succeeds with `written to "/tmp/demo_out.pdf"`.

Now introduce a deliberate error to verify pretty output. Edit `demo.jianpu` temporarily, inserting `x` as a note (e.g. change `1 - 2 0` to `1 - x 0`), then run:

```bash
cargo run -- generate pdf demo.jianpu /tmp/demo_out.pdf
```

Expected output to stderr resembles:
```
error: expected pitch digit 0-7, got: x
  --> demo.jianpu:9:7
   |
 9 | 1 - x 0
   |     ^ expected pitch digit 0-7
```

Revert the demo file after verifying.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: attach file path to errors and render with ariadne"
```
