# Clippy Lint Roadmap for jianpu-generator

This document is a handoff guide for AI agents working on this repository. It explains the existing lint setup, what is already enforced, and which rules to enable next.

## Project context

**jianpu-generator** is a Rust CLI (`jianpu`) that parses a custom Jianpu notation format and generates PDF, SVG, MIDI, and WAV output.

- **Edition:** 2021
- **Entry point:** `src/main.rs`
- **Error type:** `JianPuError` in `src/error.rs` — production code should return `Result<_, JianPuError>` instead of panicking
- **Goal of linting:** Make the codebase AI-agent friendly by blocking bad patterns at commit time

## Lint infrastructure (read this first)

Linting is split across three files:

| File | Purpose |
|------|---------|
| `Cargo.toml` → `[lints.rust]` / `[lints.clippy]` | Lint **levels** (`deny`, `warn`, `allow`) |
| `clippy.toml` | Clippy **thresholds** and **test allowances** |
| `scripts/git-hooks/pre-commit` | Enforces checks before every commit |

### Pre-commit hook

Install once:

```sh
./scripts/install-git-hooks.sh
```

The hook runs, in order:

1. `cargo fmt -- --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test`

### Important: `warn` is NOT enough to block commits by itself

A lint set to `warn` in `Cargo.toml` only becomes a **hard error** because the pre-commit hook passes `-D warnings` to Clippy.

That means:

- `warn` **does** block commits today (via `-D warnings`)
- But `warn` **does not** block `cargo clippy` run without `-D warnings`
- AI agents may run `cargo clippy` without that flag and think the code is fine
- **`deny` is the correct level for any rule that must never land in production code**

**Rule for this repo:** Use `deny` for all enforcement rules. Do not add new rules as `warn` unless you are explicitly doing a temporary migration with a documented cleanup task.

## Currently enforced rules

### `Cargo.toml` — hard denies

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
mem_forget = "deny"
panic = "deny"
unwrap_used = "deny"
expect_used = "deny"
len_zero = "warn"   # ← should be promoted to deny (see Phase 1)
```

### `clippy.toml` — thresholds and test allowances

Production code must use proper error handling. Tests may use shortcuts:

- `allow-unwrap-in-tests = true`
- `allow-expect-in-tests = true`
- `allow-dbg-in-tests = true`
- `allow-panic-in-tests = true`
- `allow-print-in-tests = true`
- `allow-indexing-slicing-in-tests = true`

Complexity thresholds (tighter than Clippy defaults):

- `cognitive-complexity-threshold = 25`
- `too-many-lines-threshold = 100`
- `too-many-arguments-threshold = 7`
- `type-complexity-threshold = 250`

AI hygiene:

- `disallowed-names = ["foo", "baz", "quux"]`
- `disallowed-macros` for `std::todo`, `std::unimplemented`, `std::dbg`

### Recent production-code fixes (already merged)

These patterns were removed from production code to satisfy the deny rules above:

- `unwrap()` / `expect()` → `ok_or_else`, `map_err`, `?`, or `if let`
- `panic!()` / `unreachable!()` → `return Err(JianPuError::new(...))`
- `midi::write_midi` and `wav::write_wav` now return `Result<Vec<u8>, JianPuError>`

Tests still use `unwrap`/`expect`/`panic`/`assert!` — that is intentional and allowed.

## How to verify your work

Always run the same checks as the pre-commit hook before claiming success:

```sh
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

If you add a new `deny` lint, fix **all** violations in production code (`src/**/*.rs` outside `#[cfg(test)]` modules). Do not add `#[allow(...)]` in production code to bypass lints unless there is a documented, unavoidable reason.

## Phased roadmap

Enable phases in order. Each phase should:

1. Add rules as **`deny`** in `Cargo.toml`
2. Fix all production violations
3. Pass the verification commands above
4. Commit

Estimated violation counts were measured against the codebase as of commit `112e06c`. Re-count before starting a phase.

---

### Phase 1 — High value, small diff (do next)

**Purpose:** Block more AI shortcuts with manageable fix cost.

Add to `Cargo.toml`:

```toml
[lints.clippy]
wildcard_imports = "deny"
exit = "deny"
len_zero = "deny"              # promote from warn
cognitive_complexity = "deny"  # threshold already in clippy.toml
too_many_lines = "deny"        # threshold already in clippy.toml
```

| Lint | Why | Known production violations |
|------|-----|---------------------------|
| `wildcard_imports` | AI agents love `use foo::*` | 3 files: `src/combiner.rs`, `src/grouper.rs`, `src/layout/mod.rs` |
| `exit` | Forces proper error returns | `std::process::exit(1)` in `src/main.rs` |
| `len_zero` | Prefer `is_empty()` over `len() == 0` | Already mostly fixed; promote to `deny` |
| `cognitive_complexity` | Stops sprawling generated functions | ~5 functions over threshold |
| `too_many_lines` | Same | ~9 functions over threshold |

**Fix guidance:**

- Replace `use crate::foo::*` with explicit imports
- Replace `process::exit(1)` with `return Err(...)` propagated to `main`
- Split or refactor functions that exceed complexity/line thresholds

---

### Phase 2 — Code quality (moderate effort)

Add as **`deny`** once Phase 1 is complete:

```toml
uninlined_format_args = "deny"
redundant_clone = "deny"
needless_pass_by_value = "deny"
match_wildcard_for_single_variants = "deny"
```

| Lint | Why | Estimated hits (prod + tests) |
|------|-----|-------------------------------|
| `uninlined_format_args` | Use `format!("{var}")` instead of `format!("{}", var)` | ~74 |
| `redundant_clone` | Catches unnecessary clones AI agents add | ~12 |
| `needless_pass_by_value` | Better API design | ~8 |
| `match_wildcard_for_single_variants` | Clearer control flow | ~12 |

Most `uninlined_format_args` hits are in test code (allowed to use shortcuts, but Clippy will still flag them unless fixed or the lint is scoped). Fix production code first; fix test code as needed since tests are also checked by `--all-targets`.

---

### Phase 3 — Stronger panic prevention (large refactor)

```toml
indexing_slicing = "deny"
```

| Lint | Why | Estimated hits |
|------|-----|----------------|
| `indexing_slicing` | Bans `arr[i]` and slicing that can panic; use `.get()` / `.get_mut()` | ~71 |

`allow-indexing-slicing-in-tests = true` is already set in `clippy.toml`, so test code keeps indexing freedom. Production code needs a systematic pass.

Optional, after `indexing_slicing`:

```toml
print_stdout = "deny"
print_stderr = "deny"
```

CLI output in `src/main.rs` may need targeted `#[allow(clippy::print_stdout)]` on the `main`/command handler module only — not scattered through library code.

---

### Do NOT enable as groups

Avoid blanket group enables — they create too much noise:

| Group | Why skip |
|-------|----------|
| `pedantic = "warn"` or `"deny"` | ~240 warnings; many are low-signal style nits |
| `nursery = "warn"` or `"deny"` | Unstable lints; frequent false positives |
| `cargo` lints | About `Cargo.toml` metadata, not code correctness |
| `missing_docs` | High friction unless the project commits to full API documentation |

Enable individual lints from these groups only when there is a specific, justified need.

## Patterns AI agents must follow

### Production code

```rust
// BAD — will fail clippy
let x = foo().unwrap();
let y = bar().expect("failed");
panic!("not implemented");
todo!("fix later");
dbg!(value);
use crate::ast::grouped::*;

// GOOD
let x = foo().map_err(|e| JianPuError::new(span, format!("...: {e}")))?;
let y = bar().ok_or_else(|| JianPuError::new(span, "..." ))?;
return Err(JianPuError::new(span, "..."));
use crate::ast::grouped::{Score, Measure, PartRow};
```

### Test code

Tests may use `unwrap`, `expect`, `panic!`, `assert!`, and `use super::*` thanks to `clippy.toml` allowances. Do not weaken production denies to make tests pass.

### Error handling convention

- Attach spans when available: `JianPuError::new(span, "message")`
- Propagate with `?` in functions returning `Result<_, JianPuError>`
- IO/format failures in library code → `map_err` into `JianPuError`, not `unwrap`

## Suggested target `Cargo.toml` after all phases

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
# AI-agent hard blocks
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
mem_forget = "deny"
panic = "deny"
unwrap_used = "deny"
expect_used = "deny"
wildcard_imports = "deny"
exit = "deny"
indexing_slicing = "deny"

# Quality (all deny — not warn)
len_zero = "deny"
cognitive_complexity = "deny"
too_many_lines = "deny"
uninlined_format_args = "deny"
redundant_clone = "deny"
needless_pass_by_value = "deny"
match_wildcard_for_single_variants = "deny"
```

## Commit checklist for lint work

- [ ] New rules added as `deny` in `Cargo.toml` (not `warn`)
- [ ] All production violations fixed
- [ ] `cargo fmt -- --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] No `#[allow(clippy::...)]` added to production code without justification
- [ ] Pre-commit hook still works: `./scripts/install-git-hooks.sh` if hooks were reset
