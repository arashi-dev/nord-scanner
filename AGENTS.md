# Workspace Rules & Agent Instructions

This document provides persistent instructions and operational rules for AI agents operating in the `nifty-darwin` (Nord Scanner) repository.

---

## 📌 Project Overview
- **Project**: Nord Scanner
- **Stack**: Tauri v2, React 19, Vite, Tailwind CSS, TypeScript, Rust (Tokio, reqwest, rusqlite, rustls, ratatui)
- **Platforms**: Windows, macOS, Linux, Android
- **CI/CD**: GitHub Actions release workflow triggered on version tags (`v*`) in [`.github/workflows/release.yml`](.github/workflows/release.yml)

---

## 🏷️ Semantic Versioning Instructions (MANDATORY)

Whenever the user instructs to **"push new version"**, **"bump version"**, **"release"**, **"create tag"**, or any variation thereof, follow these instructions strictly:

### 1. Audit Previous Version to Current State
- **Never guess or blindly bump patch.**
- Locate the latest release tag (`git tag -l "v*" --sort=-v:refname`) or current version in [`package.json`](./package.json).
- Inspect all commits and diffs since the last version until `HEAD`:
  ```powershell
  # Check commits since last tag:
  git log <last-tag>..HEAD --oneline

  # Check diff stats:
  git diff <last-tag>..HEAD --stat
  ```
- Alternatively, run the TypeScript audit script:
  ```powershell
  npx tsx .agents/skills/semantic-versioning/scripts/bump-version.ts --check
  ```

### 2. Determine the Correct Version Bump
Categorize all changes introduced since the previous version:
- **MAJOR (`X+1.0.0`)**: Incompatible API changes, breaking changes to CLI/backend/protocols, removal of existing features or options.
- **MINOR (`X.Y+1.0`)**: Backwards-compatible new features, new UI screens, new filter options, new CLI flags or commands.
- **PATCH (`X.Y.Z+1`)**: Backwards-compatible bug fixes, optimizations, UI tweaks, dependencies, CI/CD, or documentation.

### 3. Beta / Prerelease Rule
> [!IMPORTANT]
> - **Stable by default**: Every new version MUST use the stable `X.Y.Z` format.
> - **Beta condition**: Beta versions (e.g. `X.Y.Z-beta.1`) MUST ONLY be used if the user **explicitly** requests a beta release (e.g. "bump beta", "release beta version"). Never create a beta version unless explicitly told to do so.
> - When graduating from beta to stable (without explicit beta request), the `-beta.N` suffix is dropped and the stable version `X.Y.Z` is released.

### 4. Synchronize All Version Files
When bumping the version, you MUST update all of the following files simultaneously:
1. [`package.json`](./package.json) &rarr; `"version": "X.Y.Z"`
2. [`src-tauri/Cargo.toml`](./src-tauri/Cargo.toml) &rarr; `version = "X.Y.Z"` under `[package]`
3. [`src-tauri/tauri.conf.json`](./src-tauri/tauri.conf.json) &rarr; `"version": "X.Y.Z"`
4. [`Cargo.lock`](./Cargo.lock) &rarr; Updated automatically via `cargo check --package nord-scanner`

### 5. Git Commit, Tagging & Push
- **Commit**: Commit the version file changes:
  ```powershell
  git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json Cargo.lock
  git commit -m "chore(release): bump version to vX.Y.Z"
  ```
- **Tag**: Create an annotated git tag matching GitHub Actions release trigger format `v*`:
  ```powershell
  git tag -a vX.Y.Z -m "Release vX.Y.Z"
  ```
- **Push**: When the user requests to push or deploy the release:
  ```powershell
  git push origin <branch> && git push origin vX.Y.Z
  ```

---

## 🛠️ Workspace Skill Reference

This project has a dedicated skill configured in [`.agents/skills/semantic-versioning/SKILL.md`](.agents/skills/semantic-versioning/SKILL.md).

You can run the bundled TypeScript helper script to inspect or execute version bumps automatically:
```powershell
# 1. Analyze changes & show recommendation (dry-run)
npx tsx .agents/skills/semantic-versioning/scripts/bump-version.ts --check

# 2. Automatically apply recommended bump (stable X.Y.Z)
npx tsx .agents/skills/semantic-versioning/scripts/bump-version.ts --bump auto

# 3. Apply recommended bump as beta (ONLY if explicitly requested by user)
npx tsx .agents/skills/semantic-versioning/scripts/bump-version.ts --bump auto --beta

# 4. Bump, commit, and tag
npx tsx .agents/skills/semantic-versioning/scripts/bump-version.ts --bump auto --commit --tag

# 5. Bump, commit, tag, and push
npx tsx .agents/skills/semantic-versioning/scripts/bump-version.ts --bump auto --commit --tag --push
```
