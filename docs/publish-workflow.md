# Publish Workflow

This repository uses [`release-plz`](https://release-plz.ieni.dev/) to automate the release process of crates.

## Workflow

1.  **Development**: Developers work on features and fix bugs. Conventional Commits should be used to allow `release-plz` to generate meaningful changelogs.

    ### Commit Rules & Versioning

    We follow [Conventional Commits](https://www.conventionalcommits.org/). The commit message structure determines the version bump:

    | Commit Type | SemVer Bump | Description | Example |
    | :--- | :--- | :--- | :--- |
    | `fix:` | **Patch** (0.0.x) | A bug fix | `fix: correct memory leak in allocator` |
    | `feat:` | **Minor** (0.x.0) | A new feature | `feat: add new scheduler algorithm` |
    | `BREAKING CHANGE:` or `!` | **Major** (x.0.0) | A breaking API change | `feat!: remove deprecated syscalls` |
    | `docs:`, `chore:`, `style:`, `refactor:`, `perf:`, `test:` | **None** | Changes that don't affect the published binary/library code (usually) | `docs: update readme` |

    > **Note:** Since `zeroos` crates are currently in `0.x.x` (pre-1.0), breaking changes may often only trigger a **Minor** bump depending on specific configuration, but `release-plz` generally treats breaking changes as breaking. However, for 0.x.x, Cargo considers `0.1` to `0.2` a "breaking" change (equivalent to Major).

2.  **Pull Request**: When code is merged into the `main` branch, `release-plz` (running in CI) checks for changes.
3.  **Release PR**: If changes are detected, `release-plz` creates a "Release PR". This PR contains:
    *   Version bumps in `Cargo.toml`.
    *   Updates to `CHANGELOG.md` files.
4.  **Review**: Maintainers review the Release PR.
5.  **Merge & Publish**: When the Release PR is merged, `release-plz` (running in CI) will:
    *   Create a git tag for the new version.
    *   Publish the crates to crates.io.

## Versioning Strategy

*   **ZeroOS Group**: The `zeroos` crate and all `zeroos-*` crates share a single version number. When one changes, they are all released with the same version to ensure compatibility.
*   **Independent Crates**: `cargo-matrix` and `htif` are versioned independently.
*   **Internal Crates**: Examples (`examples/`), platforms (`platforms/`), and tasks (`xtask`) are marked as `publish = false` and are not published to crates.io.

## Configuration

The configuration is located in `release-plz.toml`.

### Adding a new crate

Because `release-plz` is configured with `release = false` by default (whitelist mode), new crates are **ignored** automatically. To publish a new crate, you must explicitly enable it:

1.  **Add to Workspace**: Add the crate to `[workspace.members]` in `Cargo.toml`.
2.  **Enable Release**: Add an entry in `release-plz.toml`:
    ```toml
    [[package]]
    name = "my-new-crate"
    release = true
    # version_group = "zeroos"  <-- Uncomment if part of the ZeroOS group
    ```
3.  **Check Dependencies**: Ensure internal dependencies use `workspace = true` where applicable.

