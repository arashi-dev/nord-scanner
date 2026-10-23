#!/usr/bin/env python3
"""
Semantic Versioning Manager & Bump Utility for Nord Scanner.

Inspects git history and changes since the previous version/tag,
analyzes conventional commits and diffs, determines the appropriate
Semantic Version (MAJOR, MINOR, or PATCH), and optionally updates
all project version configuration files.

Rules enforced:
- Default: Stable version in `X.Y.Z` format.
- Prerelease/Beta: ONLY when explicitly requested via `--beta`.
- Synchronized files: package.json, src-tauri/Cargo.toml, src-tauri/tauri.conf.json, Cargo.lock.
"""

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path


def run_git(args: list[str], check: bool = True) -> str:
    """Run a git command and return stripped stdout."""
    res = subprocess.run(
        ["git"] + args,
        capture_output=True,
        text=True,
        check=False,
    )
    if check and res.returncode != 0:
        raise RuntimeError(f"Git command failed: git {' '.join(args)}\nError: {res.stderr.strip()}")
    return res.stdout.strip()


def get_latest_tag() -> str | None:
    """Find the most recent semantic version git tag."""
    tags = run_git(["tag", "-l", "v*", "--sort=-v:refname"], check=False).splitlines()
    for tag in tags:
        tag = tag.strip()
        if re.match(r"^v?\d+\.\d+\.\d+(-beta\.\d+)?$", tag):
            return tag
    return None


def get_initial_commit() -> str:
    """Find the first commit in the repository."""
    commits = run_git(["rev-list", "--max-parents=0", "HEAD"]).splitlines()
    return commits[0] if commits else "HEAD"


def get_current_file_versions(repo_root: Path) -> dict[str, str]:
    """Read version strings from package.json, Cargo.toml, and tauri.conf.json."""
    versions = {}

    # 1. package.json
    pkg_json_path = repo_root / "package.json"
    if pkg_json_path.exists():
        with open(pkg_json_path, "r", encoding="utf-8") as f:
            data = json.load(f)
            versions["package.json"] = data.get("version", "unknown")

    # 2. src-tauri/Cargo.toml
    cargo_path = repo_root / "src-tauri" / "Cargo.toml"
    if cargo_path.exists():
        content = cargo_path.read_text(encoding="utf-8")
        match = re.search(r'(?ms)^\[package\].*?^version\s*=\s*"([^"]+)"', content)
        if match:
            versions["src-tauri/Cargo.toml"] = match.group(1)

    # 3. src-tauri/tauri.conf.json
    tauri_conf_path = repo_root / "src-tauri" / "tauri.conf.json"
    if tauri_conf_path.exists():
        with open(tauri_conf_path, "r", encoding="utf-8") as f:
            data = json.load(f)
            versions["src-tauri/tauri.conf.json"] = data.get("version", "unknown")

    return versions


def parse_semver(version_str: str) -> tuple[int, int, int, int | None]:
    """Parse 'X.Y.Z' or 'X.Y.Z-beta.N' into (major, minor, patch, beta_num)."""
    cleaned = version_str.lstrip("v")
    m = re.match(r"^(\d+)\.(\d+)\.(\d+)(?:-beta\.(\d+))?$", cleaned)
    if not m:
        raise ValueError(f"Invalid semver string: {version_str}")
    major, minor, patch = int(m.group(1)), int(m.group(2)), int(m.group(3))
    beta = int(m.group(4)) if m.group(4) is not None else None
    return major, minor, patch, beta


def format_semver(major: int, minor: int, patch: int, beta: int | None = None) -> str:
    """Format components into semver string."""
    base = f"{major}.{minor}.{patch}"
    if beta is not None:
        return f"{base}-beta.{beta}"
    return base


def analyze_changes(repo_root: Path, base_ref: str | None) -> dict:
    """
    Analyze commits and diffs since base_ref.
    Categorizes changes into breaking, feat, fix, chore/other.
    """
    if base_ref:
        rev_range = f"{base_ref}..HEAD"
        commits_output = run_git(["log", rev_range, "--format=%H%x09%s%x09%b%x1e"], check=False)
    else:
        commits_output = run_git(["log", "--format=%H%x09%s%x09%b%x1e"], check=False)

    commit_entries = commits_output.split("\x1e")
    breaking_commits = []
    feature_commits = []
    fix_commits = []
    other_commits = []

    for entry in commit_entries:
        entry = entry.strip()
        if not entry:
            continue
        parts = entry.split("\x09")
        commit_hash = parts[0]
        subject = parts[1] if len(parts) > 1 else ""
        body = parts[2] if len(parts) > 2 else ""

        full_msg = f"{subject}\n{body}".strip()
        is_breaking = (
            "BREAKING CHANGE:" in full_msg
            or "BREAKING-CHANGE:" in full_msg
            or re.match(r"^[a-zA-Z]+(?:\([^\)]+\))?!:", subject)
        )

        if is_breaking:
            breaking_commits.append((commit_hash[:7], subject))
        elif re.match(r"^feat(?:\([^\)]+\))?:", subject, re.IGNORECASE):
            feature_commits.append((commit_hash[:7], subject))
        elif re.match(r"^fix(?:\([^\)]+\))?:", subject, re.IGNORECASE):
            fix_commits.append((commit_hash[:7], subject))
        else:
            other_commits.append((commit_hash[:7], subject))

    # Diff stat
    if base_ref:
        diff_stat = run_git(["diff", "--stat", f"{base_ref}..HEAD"], check=False)
    else:
        diff_stat = run_git(["diff", "--stat", "HEAD~1..HEAD"], check=False)

    # Determine recommended bump type
    if breaking_commits:
        recommended = "major"
        reason = f"Detected {len(breaking_commits)} breaking change commit(s)"
    elif feature_commits:
        recommended = "minor"
        reason = f"Detected {len(feature_commits)} feature commit(s)"
    elif fix_commits:
        recommended = "patch"
        reason = f"Detected {len(fix_commits)} fix commit(s)"
    elif other_commits:
        recommended = "patch"
        reason = f"Detected maintenance/refactor/ci commit(s) ({len(other_commits)})"
    else:
        recommended = "none"
        reason = "No changes detected since base reference"

    return {
        "base_ref": base_ref,
        "breaking": breaking_commits,
        "features": feature_commits,
        "fixes": fix_commits,
        "other": other_commits,
        "diff_stat": diff_stat,
        "recommended_bump": recommended,
        "reason": reason,
    }


def compute_next_version(
    current_ver: str,
    bump_type: str,
    beta: bool = False,
) -> str:
    """
    Computes the next semver string.
    
    If beta is True:
      - Appends or increments `-beta.N`.
    If beta is False:
      - Always returns stable `X.Y.Z`.
    """
    cur_major, cur_minor, cur_patch, cur_beta = parse_semver(current_ver)

    # If current is beta and user wants stable release without bumping major/minor/patch:
    if cur_beta is not None and not beta and bump_type in ("none", "auto", "graduate"):
        return format_semver(cur_major, cur_minor, cur_patch)

    if bump_type == "major":
        next_major = cur_major + 1
        next_minor = 0
        next_patch = 0
    elif bump_type == "minor":
        next_major = cur_major
        next_minor = cur_minor + 1
        next_patch = 0
    elif bump_type == "patch":
        next_major = cur_major
        next_minor = cur_minor
        next_patch = cur_patch + 1
    elif bump_type == "none":
        next_major = cur_major
        next_minor = cur_minor
        next_patch = cur_patch
    else:
        raise ValueError(f"Unknown bump type: {bump_type}")

    if beta:
        # Check if we are bumping the current beta or starting a new beta cycle
        if (
            cur_beta is not None
            and next_major == cur_major
            and next_minor == cur_minor
            and next_patch == cur_patch
        ):
            next_beta = cur_beta + 1
        else:
            next_beta = 1
        return format_semver(next_major, next_minor, next_patch, next_beta)
    else:
        return format_semver(next_major, next_minor, next_patch, None)


def update_version_files(repo_root: Path, new_version: str) -> None:
    """Update version in package.json, src-tauri/Cargo.toml, and src-tauri/tauri.conf.json."""
    # 1. package.json
    pkg_path = repo_root / "package.json"
    if pkg_path.exists():
        with open(pkg_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        data["version"] = new_version
        with open(pkg_path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2)
            f.write("\n")
        print(f"[UPDATED] {pkg_path.relative_to(repo_root)} -> {new_version}")

    # 2. src-tauri/Cargo.toml
    cargo_path = repo_root / "src-tauri" / "Cargo.toml"
    if cargo_path.exists():
        content = cargo_path.read_text(encoding="utf-8")
        # Replace only version under [package]
        pattern = r'(?ms)(^\[package\].*?^version\s*=\s*)"[^"]+"'
        new_content, count = re.subn(pattern, rf'\g<1>"{new_version}"', content, count=1)
        if count > 0:
            cargo_path.write_text(new_content, encoding="utf-8")
            print(f"[UPDATED] {cargo_path.relative_to(repo_root)} -> {new_version}")
        else:
            print(f"[WARNING] Could not find [package] version in {cargo_path}")

    # 3. src-tauri/tauri.conf.json
    tauri_conf_path = repo_root / "src-tauri" / "tauri.conf.json"
    if tauri_conf_path.exists():
        with open(tauri_conf_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        data["version"] = new_version
        with open(tauri_conf_path, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2)
            f.write("\n")
        print(f"[UPDATED] {tauri_conf_path.relative_to(repo_root)} -> {new_version}")

    # 4. Sync Cargo.lock
    print("[INFO] Running 'cargo check' to update Cargo.lock...")
    res = subprocess.run(["cargo", "check", "--package", "nord-scanner"], cwd=repo_root, capture_output=True, text=True)
    if res.returncode == 0:
        print("[UPDATED] Cargo.lock synchronized successfully")
    else:
        print("[NOTICE] 'cargo check' finished with exit code:", res.returncode)


def main():
    parser = argparse.ArgumentParser(
        description="Nord Scanner Semantic Versioning & Release Tool"
    )
    parser.add_argument(
        "--check",
        "--analyze",
        action="store_true",
        help="Analyze git changes since the last version and recommend bump",
    )
    parser.add_argument(
        "--bump",
        choices=["auto", "major", "minor", "patch", "none"],
        default=None,
        help="Perform the bump on project files (auto uses change analysis)",
    )
    parser.add_argument(
        "--set-version",
        type=str,
        default=None,
        help="Explicitly set version string directly (e.g. 1.0.0 or 1.0.0-beta.1)",
    )
    parser.add_argument(
        "--beta",
        action="store_true",
        help="Explicitly request a beta prerelease version (e.g. X.Y.Z-beta.1)",
    )
    parser.add_argument(
        "--commit",
        action="store_true",
        help="Git commit the version bump files",
    )
    parser.add_argument(
        "--tag",
        action="store_true",
        help="Create annotated git tag v<version>",
    )
    parser.add_argument(
        "--push",
        action="store_true",
        help="Push git commit and tag to remote repository",
    )

    args = parser.parse_args()
    git_root = run_git(["rev-parse", "--show-toplevel"], check=False)
    if git_root and os.path.isdir(git_root):
        repo_root = Path(git_root).resolve()
    else:
        repo_root = Path(__file__).resolve().parents[4]

    print("==================================================")
    print("      Nord Scanner Semantic Version Manager       ")
    print("==================================================")

    # 1. Current state
    file_versions = get_current_file_versions(repo_root)
    latest_tag = get_latest_tag()

    print(f"Repository Root: {repo_root}")
    print(f"Latest Git Tag : {latest_tag or 'None (Initial release)'}")
    print("Project File Versions:")
    for fpath, ver in file_versions.items():
        print(f"  - {fpath}: {ver}")

    current_ver = (
        file_versions.get("package.json")
        or file_versions.get("src-tauri/Cargo.toml")
        or (latest_tag.lstrip("v") if latest_tag else "0.1.0")
    )
    print(f"Active Base Version: {current_ver}")
    print("--------------------------------------------------")

    # 2. Analyze changes
    base_ref = latest_tag
    analysis = analyze_changes(repo_root, base_ref)

    print("Git Change Analysis:")
    print(f"  - Comparing from: {base_ref or 'repo start'} to HEAD")
    print(f"  - Breaking Changes: {len(analysis['breaking'])}")
    for h, s in analysis["breaking"]:
        print(f"      * [{h}] {s}")
    print(f"  - Features: {len(analysis['features'])}")
    for h, s in analysis["features"]:
        print(f"      * [{h}] {s}")
    print(f"  - Fixes: {len(analysis['fixes'])}")
    for h, s in analysis["fixes"]:
        print(f"      * [{h}] {s}")
    print(f"  - Other commits: {len(analysis['other'])}")
    for h, s in analysis["other"][:5]:
        print(f"      * [{h}] {s}")
    if len(analysis["other"]) > 5:
        print(f"      * ... and {len(analysis['other']) - 5} more")

    print(f"  - Recommended Bump: {analysis['recommended_bump'].upper()}")
    print(f"  - Reason: {analysis['reason']}")
    print("--------------------------------------------------")

    # 3. Target version calculation
    if args.set_version:
        next_ver = args.set_version
        print(f"Target Version (explicit override): {next_ver}")
    elif args.bump:
        bump_choice = analysis["recommended_bump"] if args.bump == "auto" else args.bump
        if bump_choice == "none":
            bump_choice = "patch"  # fallback if forced bump with no commits
        next_ver = compute_next_version(current_ver, bump_choice, beta=args.beta)
        print(f"Target Version ({bump_choice.upper()} bump, beta={args.beta}): {next_ver}")
    else:
        # Default analysis projection
        bump_choice = analysis["recommended_bump"] if analysis["recommended_bump"] != "none" else "patch"
        projected_stable = compute_next_version(current_ver, bump_choice, beta=False)
        projected_beta = compute_next_version(current_ver, bump_choice, beta=True)
        print("Projected Next Versions:")
        print(f"  - Stable (Default): {projected_stable}")
        print(f"  - Beta (Explicit only): {projected_beta}")
        next_ver = projected_stable

    # If only checking / analyzing, finish here
    if not args.bump and not args.set_version:
        print("--------------------------------------------------")
        print("To apply this bump, run:")
        print(f"  python .agents/skills/semantic-versioning/scripts/bump_version.py --bump auto")
        if args.beta:
            print("  (add --beta for prerelease, --commit --tag to tag the release)")
        return

    # 4. Apply version updates
    print("--------------------------------------------------")
    print(f"Applying Version Bump to: {next_ver}")
    update_version_files(repo_root, new_version=next_ver)

    # 5. Git Commit & Tag
    if args.commit or args.tag or args.push:
        tag_name = f"v{next_ver}"
        commit_msg = f"chore(release): bump version to {tag_name}"

        # Stage files
        files_to_stage = [
            "package.json",
            "src-tauri/Cargo.toml",
            "src-tauri/tauri.conf.json",
            "Cargo.lock",
        ]
        run_git(["add"] + files_to_stage)

        if args.commit or args.tag or args.push:
            print(f"[GIT] Committing version bump: '{commit_msg}'")
            run_git(["commit", "-m", commit_msg], check=False)

        if args.tag or args.push:
            print(f"[GIT] Creating annotated tag: {tag_name}")
            run_git(["tag", "-a", tag_name, "-m", f"Release {tag_name}"])

        if args.push:
            current_branch = run_git(["rev-parse", "--abbrev-ref", "HEAD"])
            print(f"[GIT] Pushing branch '{current_branch}' and tag '{tag_name}'...")
            run_git(["push", "origin", current_branch])
            run_git(["push", "origin", tag_name])

    print("==================================================")
    print(f"SUCCESS: Version bump to {next_ver} completed!")
    print("==================================================")


if __name__ == "__main__":
    main()
