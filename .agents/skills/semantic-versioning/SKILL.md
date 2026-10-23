---
name: semantic-versioning
description: >-
  Automates and guides semantic versioning (SemVer 2.0.0) for this project.
  Use this skill whenever the user asks to bump the version, release a new version,
  push a new version, create a tag/release, or determine the next semantic version
  (major, minor, patch, or beta) based on changes since the last release.
---

# Semantic Versioning & Release Workflow

This skill governs versioning for this project following the [Semantic Versioning 2.0.0](https://semver.org/) specification.

## Core Principles

1. **Change-Driven Determination**:
   Never guess or blindly bump patch. Always analyze git commit history and diffs between the previous version/tag and the current state to identify exact changes.
2. **Stable by Default**:
   Every release version MUST be in stable `X.Y.Z` format unless the user explicitly requests a beta/prerelease version.
3. **Beta Only on Explicit Request**:
   Beta versions (`X.Y.Z-beta.N`) are ONLY created when the user explicitly instructs (e.g., "bump beta", "release beta version").
4. **Synchronized Manifests**:
   All project version files must remain strictly synchronized:
   - [`package.json`](../../package.json) (`"version": "X.Y.Z"`)
   - [`src-tauri/Cargo.toml`](../../src-tauri/Cargo.toml) (`version = "X.Y.Z"` under `[package]`)
   - [`src-tauri/tauri.conf.json`](../../src-tauri/tauri.conf.json) (`"version": "X.Y.Z"`)
   - [`Cargo.lock`](../../Cargo.lock) (synchronized via `cargo check --package nord-scanner`)

---

## SemVer Decision Matrix

| Bump Type | Condition / Triggers | Example Transition |
| :--- | :--- | :--- |
| **MAJOR** (`X+1.0.0`) | Breaking changes, incompatible API modifications, removed commands/options, architecture overhaul breaking backwards compatibility. | `0.1.0` &rarr; `1.0.0` or `1.2.3` &rarr; `2.0.0` |
| **MINOR** (`X.Y+1.0`) | New backwards-compatible features, new CLI flags or commands, new UI screens or settings, new protocol support. | `0.1.0` &rarr; `0.2.0` or `1.2.3` &rarr; `1.3.0` |
| **PATCH** (`X.Y.Z+1`) | Backwards-compatible bug fixes, performance improvements, internal refactors, dependency updates, documentation, CI workflows. | `0.1.0` &rarr; `0.1.1` or `1.2.3` &rarr; `1.2.4` |
| **BETA** (`X.Y.Z-beta.N`) | **ONLY when explicitly requested by the user**. Starts at `-beta.1` or increments `-beta.(N+1)`. | `0.1.0` &rarr; `0.2.0-beta.1` |

---

## Standard Workflow

### Step 1: Discover Previous Version and Inspect Changes

Check the latest git tag and project version files:
```powershell
# Check latest release tag
git tag -l "v*" --sort=-v:refname

# Check commit history since last tag (or repo start if no tag exists)
git log <last-tag>..HEAD --oneline

# Check diff stats
git diff <last-tag>..HEAD --stat
```

Alternatively, run the automated analysis script:
```powershell
python .agents/skills/semantic-versioning/scripts/bump_version.py --check
```

### Step 2: Categorize Changes

Review the commits between previous version and `HEAD`:
- **Breaking Changes** (`feat!:`, `fix!:`, `BREAKING CHANGE:`):
  &rarr; **MAJOR** bump.
- **New Features** (`feat:`, `feat(...)`):
  &rarr; **MINOR** bump.
- **Fixes & Chores** (`fix:`, `perf:`, `refactor:`, `docs:`, `ci:`, `chore:`):
  &rarr; **PATCH** bump.

### Step 3: Check for Explicit Beta Request

- If user explicitly requested beta:
  - Append `-beta.1` to the calculated next version (e.g., `0.2.0-beta.1`).
  - If current version was already a beta (e.g., `0.2.0-beta.1`), increment beta number (`0.2.0-beta.2`).
- If user did NOT explicitly request beta:
  - Must be stable `X.Y.Z` format.
  - If current version was a beta, graduating to stable drops the `-beta.N` suffix (e.g., `0.2.0-beta.2` &rarr; `0.2.0`).

### Step 4: Apply Bump to Files

Run the automated bump script or update files directly:

```powershell
# For automatic detection and update (stable by default):
python .agents/skills/semantic-versioning/scripts/bump_version.py --bump auto

# When user explicitly requested beta:
python .agents/skills/semantic-versioning/scripts/bump_version.py --bump auto --beta

# When committing and tagging:
python .agents/skills/semantic-versioning/scripts/bump_version.py --bump auto --commit --tag

# When user asked to push:
python .agents/skills/semantic-versioning/scripts/bump_version.py --bump auto --commit --tag --push
```

### Step 5: Verify Files and Lockfile

Ensure all files are consistent:
1. `package.json` has the new version.
2. `src-tauri/Cargo.toml` has the new version under `[package]`.
3. `src-tauri/tauri.conf.json` has the new version.
4. Run `cargo check --package nord-scanner` so `Cargo.lock` is updated.
5. Review with `git diff`.

---

## References & Utilities

- Helper script: [bump_version.py](./scripts/bump_version.py)
- Reference guide: [semver-guide.md](./references/semver-guide.md)
