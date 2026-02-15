# AGENTS

## Scope

- **Do** keep changes small, targeted, and easy to validate.
- **Do** prefer clear, deterministic behavior over cleverness.
- **Do not** introduce new build systems or heavy dependencies without a strong
  reason.

## Repo norms

### Style and tone

Documentation in ZeroOS favors:

- Short sections with descriptive headings
- Bullet lists for requirements and tradeoffs
- Concrete commands that can be copied and run
- Minimal hype; explain what a feature does and why it exists

Code favors:

- Explicit error handling (fail-fast)
- Modularity and compile-time configuration
- Avoiding “magic” behavior and global side effects

### Commits

We follow **Conventional Commits** (see `docs/publish-workflow.md`). Use types
like `feat:`, `fix:`, `docs:`, `refactor:`, `test:`.

**Never** add `Co-Authored-By` or similar AI attribution lines to commit
messages.

## How to validate changes

`cargo matrix` and `cargo massage` are workspace-provided cargo aliases (see
`.cargo/config.toml`), not built-in Cargo subcommands.

Prefer validating in this order:

1. **Format**
   - `cargo matrix fix`
   - `cargo matrix fmt`
2. **Lint / check**
   - `cargo matrix clippy`
   - `cargo matrix check`
3. **Tests**
   - `cargo matrix test`

For a quick “do the reasonable thing” pass:

- `cargo massage`

Note: `cargo matrix` and `cargo massage` are workspace aliases defined in
`.cargo/config.toml`.

If you touch one crate only, it’s fine to run:

- `cargo test -p <crate-name>` (when that crate has host tests)
- `cargo matrix check -p <crate-name>` / `cargo matrix clippy -p <crate-name>` /
  `cargo matrix test -p <crate-name>` (when you want to match the curated matrix
  targets/features)

If validation fails, narrow the scope (single crate or target), fix, then re-run
the smallest relevant command.

## Adding or modifying crates

- Add new crates under `crates/`.
- Add them to the workspace in the root `Cargo.toml`.
- If you want the crate covered by `cargo matrix` (and CI), add it to
  `matrix.yaml` with the right target(s) and feature sets.
- Prefer `workspace = true` dependencies where possible.
- If a crate is intended to be published, ensure it’s configured in
  `release-plz.toml`.

Notes:

- You usually **do not** need to touch `matrix.yaml` for doc-only changes,
  formatting-only changes, or changes isolated to a host tool that is already in
  the matrix.
- You **should** update `matrix.yaml` when adding a new crate or when changing a
  crate’s supported targets/features in a way that should be enforced by CI.

## Editing guidelines for agents

### Rust crate structure

- Keep `main.rs` minimal: argument parsing / logging setup / calling into the
  crate.
- If you expect a binary to grow multiple subcommands, it’s fine to start with a
  `cli.rs` + `commands/*` layout early to keep `main.rs` as glue.
- Keep `lib.rs` as a small facade: `mod ...;` plus `pub use ...` for the
  intended public API.
- Put real logic in focused modules (e.g. `types.rs`, `parse.rs`, `analyze.rs`,
  `render.rs`).
- Prefer module-level feature/target gates where possible
  (`#[cfg(...)] mod foo;`) to keep boundaries clear.
- Use `cfg_if` when conditional compilation would otherwise create
  nested/duplicated `#[cfg]` attributes.
- In `no_std` crates, be strict about dependencies and allocation: keep
  guest/runtime crates `no_std` unless there is a strong reason.

### Architecture and dependencies

ZeroOS is intentionally layered and mostly `#![no_std]`. Keep dependencies
pointing “downward” and avoid cycles.

#### High-level layering

From lowest-level to highest-level:

- **Foundation**: `crates/zeroos-foundation` (core registries, shared
  coordination)
- **Arch / OS / Runtime**: `crates/zeroos-arch-*`, `crates/zeroos-os-*`,
  `crates/zeroos-runtime-*`
- **Subsystems**: allocators, VFS core, devices, scheduler, RNG
  (`crates/zeroos-allocator-*`, `crates/zeroos-vfs-core`,
  `crates/zeroos-device-*`, `crates/zeroos-scheduler-*`, `crates/zeroos-rng`)
- **Facade**: `crates/zeroos` (feature-gated wiring across the layers)
- **Platforms / Examples**: `platforms/*`, `examples/*` (integration glue and
  demos)
- **Host tools**: `xtask/`, `crates/cargo-matrix`, `crates/elf-report`,
  `platforms/spike-build` (these may use `std`)

#### Dependency rules of thumb

- `zeroos-foundation` should stay minimal and must not depend on higher layers
  (no devices, no platform code).
- `crates/zeroos-*` guest/runtime crates should remain `no_std` unless there is
  a strong reason.
- Devices should depend on VFS interfaces (`zeroos-vfs-core`) and/or foundation
  traits — not on platforms/examples.
- Platforms and examples may depend on `zeroos` (facade) and selected features;
  avoid pulling platform code into core crates.
- Host tools must not be depended on by guest crates.

When adding a new crate, be explicit about which layer it lives in and which
crates it is allowed to depend on.

### Be conservative

- Avoid reformatting unrelated code.
- Preserve public APIs unless the task explicitly requires a breaking change.
- Keep the patch focused: fewer files, smaller diffs.

### Be explicit about behavior

When changing behavior, include one of:

- a unit test
- a small smoke test command (pick the most relevant one), e.g.:
  - `./build-fibonacci.sh` (no-std guest sanity)
  - `./build-std-smoke.sh` (std/musl runtime sanity)
  - `./build-c-smoke.sh` (C toolchain sanity)
  - `./build-backtrace.sh` (backtrace capture + symbolization)
- a doc note in `docs/` if it affects developers/integrators

### Determinism and security

- Avoid introducing nondeterministic behavior (time, randomness,
  environment-dependent output) unless it is explicitly plumbed as committed
  input.
- Prefer memory-safe, bounds-checked parsing.

### Unsafe Rust policy

- Keep `unsafe` small, localized, and intentional—especially in guest/runtime
  crates.
- Prefer safe abstractions with a narrow `unsafe` core.
- Every `unsafe` block should have a brief comment describing the safety
  invariants (what must be true for it to be sound).
