#!/usr/bin/env node
/**
 * Semantic Versioning Manager & Bump Utility for Nord Scanner.
 * Written in TypeScript for Node.js / tsx.
 *
 * Rules enforced:
 * - Default: Stable version in `X.Y.Z` format.
 * - Prerelease/Beta: ONLY when explicitly requested via `--beta`.
 * - Synchronized files: package.json, src-tauri/Cargo.toml, src-tauri/tauri.conf.json, Cargo.lock.
 */

import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { parseArgs } from "node:util";

interface FileVersions {
  [filePath: string]: string;
}

interface CommitAnalysis {
  baseRef: string | null;
  breaking: Array<{ hash: string; subject: string }>;
  features: Array<{ hash: string; subject: string }>;
  fixes: Array<{ hash: string; subject: string }>;
  other: Array<{ hash: string; subject: string }>;
  recommendedBump: "major" | "minor" | "patch" | "none";
  reason: string;
}

interface ParsedSemVer {
  major: number;
  minor: number;
  patch: number;
  beta: number | null;
}

function runGit(args: string[]): string {
  try {
    return execSync(`git ${args.join(" ")}`, {
      encoding: "utf-8",
      stdio: ["pipe", "pipe", "pipe"],
    }).trim();
  } catch (error: any) {
    return "";
  }
}

function getRepoRoot(): string {
  const root = runGit(["rev-parse", "--show-toplevel"]);
  if (root && fs.existsSync(root)) {
    return path.resolve(root);
  }
  // Fallback: 4 levels up from this script
  return path.resolve(__dirname, "../../../..");
}

function getLatestTag(): string | null {
  const tagsOutput = runGit(["tag", "-l", '"v*"', "--sort=-v:refname"]);
  if (!tagsOutput) return null;
  const tags = tagsOutput.split(/\r?\n/).map((t) => t.trim());
  for (const tag of tags) {
    if (/^v?\d+\.\d+\.\d+(-beta\.\d+)?$/.test(tag)) {
      return tag;
    }
  }
  return null;
}

function getFileVersions(repoRoot: string): FileVersions {
  const versions: FileVersions = {};

  // 1. package.json
  const pkgPath = path.join(repoRoot, "package.json");
  if (fs.existsSync(pkgPath)) {
    const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf-8"));
    versions["package.json"] = pkg.version ?? "unknown";
  }

  // 2. src-tauri/Cargo.toml
  const cargoPath = path.join(repoRoot, "src-tauri", "Cargo.toml");
  if (fs.existsSync(cargoPath)) {
    const cargo = fs.readFileSync(cargoPath, "utf-8");
    const match = cargo.match(/(?:^\[package\][\s\S]*?^version\s*=\s*)"([^"]+)"/m);
    if (match) {
      versions["src-tauri/Cargo.toml"] = match[1];
    }
  }

  // 3. src-tauri/tauri.conf.json
  const tauriPath = path.join(repoRoot, "src-tauri", "tauri.conf.json");
  if (fs.existsSync(tauriPath)) {
    const tauri = JSON.parse(fs.readFileSync(tauriPath, "utf-8"));
    versions["src-tauri/tauri.conf.json"] = tauri.version ?? "unknown";
  }

  return versions;
}

function parseSemVer(versionStr: string): ParsedSemVer {
  const cleaned = versionStr.replace(/^v/, "");
  const match = cleaned.match(/^(\d+)\.(\d+)\.(\d+)(?:-beta\.(\d+))?$/);
  if (!match) {
    throw new Error(`Invalid semver format: ${versionStr}`);
  }
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10),
    beta: match[4] !== undefined ? parseInt(match[4], 10) : null,
  };
}

function formatSemVer(major: number, minor: number, patch: number, beta: number | null = null): string {
  const base = `${major}.${minor}.${patch}`;
  if (beta !== null && beta !== undefined) {
    return `${base}-beta.${beta}`;
  }
  return base;
}

function analyzeGitChanges(repoRoot: string, baseRef: string | null): CommitAnalysis {
  const revRange = baseRef ? `${baseRef}..HEAD` : "HEAD";
  const logOutput = baseRef
    ? runGit(["log", revRange, "--format=%H%x09%s%x09%b%x1e"])
    : runGit(["log", "--format=%H%x09%s%x09%b%x1e"]);

  const breaking: Array<{ hash: string; subject: string }> = [];
  const features: Array<{ hash: string; subject: string }> = [];
  const fixes: Array<{ hash: string; subject: string }> = [];
  const other: Array<{ hash: string; subject: string }> = [];

  const rawEntries = logOutput.split("\x1e");
  for (const raw of rawEntries) {
    const entry = raw.trim();
    if (!entry) continue;

    const [hash, subject = "", body = ""] = entry.split("\x09");
    const fullMessage = `${subject}\n${body}`.trim();

    const isBreaking =
      fullMessage.includes("BREAKING CHANGE:") ||
      fullMessage.includes("BREAKING-CHANGE:") ||
      /^[a-zA-Z]+(?:\([^)]+\))?!:/.test(subject);

    const shortHash = hash.slice(0, 7);

    if (isBreaking) {
      breaking.push({ hash: shortHash, subject });
    } else if (/^feat(?:\([^)]+\))?:/i.test(subject)) {
      features.push({ hash: shortHash, subject });
    } else if (/^fix(?:\([^)]+\))?:/i.test(subject)) {
      fixes.push({ hash: shortHash, subject });
    } else {
      other.push({ hash: shortHash, subject });
    }
  }

  let recommendedBump: "major" | "minor" | "patch" | "none" = "patch";
  let reason = "Default maintenance bump";

  if (breaking.length > 0) {
    recommendedBump = "major";
    reason = `Detected ${breaking.length} breaking change commit(s)`;
  } else if (features.length > 0) {
    recommendedBump = "minor";
    reason = `Detected ${features.length} feature commit(s)`;
  } else if (fixes.length > 0) {
    recommendedBump = "patch";
    reason = `Detected ${fixes.length} fix commit(s)`;
  } else if (other.length > 0) {
    recommendedBump = "patch";
    reason = `Detected ${other.length} chore/ci/refactor commit(s)`;
  } else {
    recommendedBump = "none";
    reason = "No changes detected since base reference";
  }

  return {
    baseRef,
    breaking,
    features,
    fixes,
    other,
    recommendedBump,
    reason,
  };
}

function computeNextVersion(
  currentVer: string,
  bumpType: "major" | "minor" | "patch" | "none",
  isBetaRequested: boolean
): string {
  const cur = parseSemVer(currentVer);

  // If current is beta and user wants stable without explicit bump
  if (cur.beta !== null && !isBetaRequested && bumpType === "none") {
    return formatSemVer(cur.major, cur.minor, cur.patch);
  }

  let nextMajor = cur.major;
  let nextMinor = cur.minor;
  let nextPatch = cur.patch;

  if (bumpType === "major") {
    nextMajor += 1;
    nextMinor = 0;
    nextPatch = 0;
  } else if (bumpType === "minor") {
    nextMinor += 1;
    nextPatch = 0;
  } else if (bumpType === "patch") {
    nextPatch += 1;
  }

  if (isBetaRequested) {
    let nextBeta = 1;
    if (
      cur.beta !== null &&
      nextMajor === cur.major &&
      nextMinor === cur.minor &&
      nextPatch === cur.patch
    ) {
      nextBeta = cur.beta + 1;
    }
    return formatSemVer(nextMajor, nextMinor, nextPatch, nextBeta);
  }

  return formatSemVer(nextMajor, nextMinor, nextPatch, null);
}

function updateVersionFiles(repoRoot: string, newVersion: string) {
  // 1. package.json
  const pkgPath = path.join(repoRoot, "package.json");
  if (fs.existsSync(pkgPath)) {
    const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf-8"));
    pkg.version = newVersion;
    fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n", "utf-8");
    console.log(`[UPDATED] package.json -> ${newVersion}`);
  }

  // 2. src-tauri/Cargo.toml
  const cargoPath = path.join(repoRoot, "src-tauri", "Cargo.toml");
  if (fs.existsSync(cargoPath)) {
    const cargo = fs.readFileSync(cargoPath, "utf-8");
    const updated = cargo.replace(
      /(^\[package\][\s\S]*?^version\s*=\s*)"[^"]+"/m,
      `$1"${newVersion}"`
    );
    fs.writeFileSync(cargoPath, updated, "utf-8");
    console.log(`[UPDATED] src-tauri/Cargo.toml -> ${newVersion}`);
  }

  // 3. src-tauri/tauri.conf.json
  const tauriPath = path.join(repoRoot, "src-tauri", "tauri.conf.json");
  if (fs.existsSync(tauriPath)) {
    const tauri = JSON.parse(fs.readFileSync(tauriPath, "utf-8"));
    tauri.version = newVersion;
    fs.writeFileSync(tauriPath, JSON.stringify(tauri, null, 2) + "\n", "utf-8");
    console.log(`[UPDATED] src-tauri/tauri.conf.json -> ${newVersion}`);
  }

  // 4. Update Cargo.lock
  console.log("[INFO] Running 'cargo check --package nord-scanner' to sync Cargo.lock...");
  try {
    execSync("cargo check --package nord-scanner", {
      cwd: repoRoot,
      stdio: "inherit",
    });
    console.log("[UPDATED] Cargo.lock synchronized successfully");
  } catch (err: any) {
    console.warn("[WARNING] 'cargo check' exited with error:", err.message);
  }
}

function main() {
  const { values: options } = parseArgs({
    options: {
      check: { type: "boolean", short: "c" },
      analyze: { type: "boolean", short: "a" },
      bump: { type: "string", short: "b" },
      beta: { type: "boolean" },
      "set-version": { type: "string" },
      commit: { type: "boolean" },
      tag: { type: "boolean", short: "t" },
      push: { type: "boolean", short: "p" },
      help: { type: "boolean", short: "h" },
    },
    allowPositionals: false,
  });

  if (options.help) {
    console.log(`
Nord Scanner Semantic Versioning Utility (TypeScript)

Usage:
  npx tsx bump-version.ts [options]

Options:
  -c, --check, -a, --analyze  Analyze git history and recommend bump (dry run)
  -b, --bump <type>           Apply bump: auto, major, minor, patch
  --beta                      Request a beta prerelease version (e.g. X.Y.Z-beta.1)
  --set-version <version>     Directly set an explicit version string
  --commit                    Git commit the version bump files
  -t, --tag                   Create annotated git tag v<version>
  -p, --push                  Push commit and tag to remote
  -h, --help                  Show this help message
`);
    return;
  }

  console.log("==================================================");
  console.log("      Nord Scanner Semantic Version Manager       ");
  console.log("==================================================");

  const repoRoot = getRepoRoot();
  const fileVersions = getFileVersions(repoRoot);
  const latestTag = getLatestTag();

  console.log(`Repository Root: ${repoRoot}`);
  console.log(`Latest Git Tag : ${latestTag ?? "None (Initial release)"}`);
  console.log("Project File Versions:");
  for (const [filePath, ver] of Object.entries(fileVersions)) {
    console.log(`  - ${filePath}: ${ver}`);
  }

  const currentVer =
    fileVersions["package.json"] ||
    fileVersions["src-tauri/Cargo.toml"] ||
    (latestTag ? latestTag.replace(/^v/, "") : "0.1.0");

  console.log(`Active Base Version: ${currentVer}`);
  console.log("--------------------------------------------------");

  const analysis = analyzeGitChanges(repoRoot, latestTag);
  console.log("Git Change Analysis:");
  console.log(`  - Comparing from: ${latestTag ?? "repo start"} to HEAD`);
  console.log(`  - Breaking Changes: ${analysis.breaking.length}`);
  for (const b of analysis.breaking) {
    console.log(`      * [${b.hash}] ${b.subject}`);
  }
  console.log(`  - Features: ${analysis.features.length}`);
  for (const f of analysis.features) {
    console.log(`      * [${f.hash}] ${f.subject}`);
  }
  console.log(`  - Fixes: ${analysis.fixes.length}`);
  for (const fx of analysis.fixes) {
    console.log(`      * [${fx.hash}] ${fx.subject}`);
  }
  console.log(`  - Other commits: ${analysis.other.length}`);
  for (const o of analysis.other.slice(0, 5)) {
    console.log(`      * [${o.hash}] ${o.subject}`);
  }
  if (analysis.other.length > 5) {
    console.log(`      * ... and ${analysis.other.length - 5} more`);
  }

  console.log(`  - Recommended Bump: ${analysis.recommendedBump.toUpperCase()}`);
  console.log(`  - Reason: ${analysis.reason}`);
  console.log("--------------------------------------------------");

  let nextVer: string;

  if (options["set-version"]) {
    nextVer = options["set-version"];
    console.log(`Target Version (explicit override): ${nextVer}`);
  } else if (options.bump) {
    let bumpType: "major" | "minor" | "patch" | "none" =
      options.bump === "auto" ? analysis.recommendedBump : (options.bump as any);

    if (bumpType === "none") {
      bumpType = "patch";
    }

    nextVer = computeNextVersion(currentVer, bumpType, Boolean(options.beta));
    console.log(
      `Target Version (${bumpType.toUpperCase()} bump, beta=${Boolean(options.beta)}): ${nextVer}`
    );
  } else {
    // Dry-run projection
    const bumpType = analysis.recommendedBump === "none" ? "patch" : analysis.recommendedBump;
    const projectedStable = computeNextVersion(currentVer, bumpType, false);
    const projectedBeta = computeNextVersion(currentVer, bumpType, true);

    console.log("Projected Next Versions:");
    console.log(`  - Stable (Default): ${projectedStable}`);
    console.log(`  - Beta (Explicit only): ${projectedBeta}`);
    console.log("--------------------------------------------------");
    console.log("To apply this bump, run:");
    console.log(
      "  npx tsx .agents/skills/semantic-versioning/scripts/bump-version.ts --bump auto"
    );
    if (options.beta) {
      console.log("  (add --beta for prerelease, --commit --tag to tag the release)");
    }
    return;
  }

  // Apply updates
  console.log("--------------------------------------------------");
  console.log(`Applying Version Bump to: ${nextVer}`);
  updateVersionFiles(repoRoot, nextVer);

  // Git operations
  if (options.commit || options.tag || options.push) {
    const tagName = `v${nextVer}`;
    const commitMsg = `chore(release): bump version to ${tagName}`;

    const filesToStage = [
      "package.json",
      "src-tauri/Cargo.toml",
      "src-tauri/tauri.conf.json",
      "Cargo.lock",
    ];
    execSync(`git add ${filesToStage.join(" ")}`, { cwd: repoRoot });

    if (options.commit || options.tag || options.push) {
      console.log(`[GIT] Committing version bump: '${commitMsg}'`);
      try {
        execSync(`git commit -m "${commitMsg}"`, { cwd: repoRoot, stdio: "inherit" });
      } catch {
        console.log("[GIT] No changes to commit or commit failed.");
      }
    }

    if (options.tag || options.push) {
      console.log(`[GIT] Creating annotated tag: ${tagName}`);
      try {
        execSync(`git tag -a ${tagName} -m "Release ${tagName}"`, {
          cwd: repoRoot,
          stdio: "inherit",
        });
      } catch (err: any) {
        console.warn(`[GIT] Tag ${tagName} creation notice:`, err.message);
      }
    }

    if (options.push) {
      const branch = runGit(["rev-parse", "--abbrev-ref", "HEAD"]) || "main";
      console.log(`[GIT] Pushing branch '${branch}' and tag '${tagName}'...`);
      execSync(`git push origin ${branch}`, { cwd: repoRoot, stdio: "inherit" });
      execSync(`git push origin ${tagName}`, { cwd: repoRoot, stdio: "inherit" });
    }
  }

  console.log("==================================================");
  console.log(`SUCCESS: Version bump to ${nextVer} completed!`);
  console.log("==================================================");
}

main();
