# Semantic Versioning (SemVer 2.0.0) Specification & Rules

## Format Specification

```text
MAJOR.MINOR.PATCH[-PRERELEASE]
```

- **MAJOR**: Incompatible API changes, breaking architectural shifts, or removed functionality.
- **MINOR**: New features added in a backward-compatible manner.
- **PATCH**: Backward-compatible bug fixes, optimizations, maintenance, internal refactors.
- **PRERELEASE** (e.g., `-beta.N`): Pre-release candidate for testing. **ONLY used when user explicitly requests beta.**

## Conventional Commits Mapping

Commit Message Pattern | Category | SemVer Impact
:--- | :--- | :---
`feat!:`, `fix!:`, `refactor!:`, `BREAKING CHANGE:` | Breaking Change | **MAJOR**
`feat:`, `feat(scope):` | New Feature | **MINOR**
`fix:`, `fix(scope):` | Bug Fix | **PATCH**
`perf:`, `refactor:` | Performance / Refactoring | **PATCH**
`docs:`, `style:`, `test:` | Documentation & Code style | **PATCH**
`ci:`, `chore:`, `build:` | CI/CD, Dependencies, Build tooling | **PATCH**

## Pre-1.0.0 Software (`0.x.y`)

In early development (`0.x.y`):
- `0.y.z` where `y > 0`:
  - `0.(y+1).0` indicates new features or significant enhancements.
  - `0.y.(z+1)` indicates bug fixes and maintenance.
- Moving to `1.0.0` signifies the first stable, production-ready public release.
- When substantial breaking changes or first complete feature set is reached, a major bump to `1.0.0` is appropriate.

## Beta Prereleases

- When the user explicitly requests a beta:
  - If current is `0.1.0` and next planned is `0.2.0`, the first beta is `0.2.0-beta.1`.
  - Next beta for the same cycle increments the number: `0.2.0-beta.2`.
- When releasing a stable version from a beta without explicit beta request:
  - The `-beta.N` suffix is dropped: `0.2.0-beta.2` &rarr; `0.2.0`.
