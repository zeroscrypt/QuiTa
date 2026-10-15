#!/usr/bin/env node
/**
 * Raises the version in every place the project keeps it.
 *
 * The three files are the ones that must never disagree:
 *   package.json          — what pnpm reports
 *   src-tauri/Cargo.toml  — what cargo reports
 *   src-tauri/tauri.conf.json — what the binary reports
 *
 * The updater compares the last two, so a mismatch does not fail loudly: the
 * green banner simply never appears. That is why this is one script and not a
 * `sed` line in a workflow.
 *
 * Usage: node scripts/bump-version.mjs 1.0.1
 *        node scripts/bump-version.mjs --check 1.0.1   (verify, change nothing)
 */

import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve as resolvePath } from 'node:path';

// Anchored to the project root, not to wherever the script was invoked from:
// CI runs it from the repository root, a developer may run it from anywhere.
// The override exists so the tests can point it at a scratch directory — a
// script that can only edit the real project cannot be tested without editing
// the real project.
const ROOT = process.env.QUITA_PROJECT_ROOT
  ? resolvePath(process.env.QUITA_PROJECT_ROOT)
  : join(dirname(fileURLToPath(import.meta.url)), '..');
const resolve = (relative) => join(ROOT, relative);

const SEMVER = /^\d+\.\d+\.\d+$/;

function fail(message) {
  console.error(`bump-version: ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const check = argv.includes('--check');
  const raw = argv.find((arg) => !arg.startsWith('--'));
  if (!raw) fail('a version is required, e.g. 1.0.1');
  // A tag may arrive as `v1.0.1`, as a bare `1.0.1`, or — from tags created
  // before include-component-in-tag was turned off — as `QuiTa-v1.0.1`.
  // Taking the number out with a pattern keeps callers from having to strip
  // anything, and shell quoting is where that kind of thing usually breaks.
  const version = /\d+\.\d+\.\d+/.exec(raw)?.[0] ?? raw;
  if (!SEMVER.test(version)) fail(`"${raw}" contains no X.Y.Z version`);
  return { version, check };
}

/** Rewrites only the `version` field, leaving formatting alone. */
function setJsonField(path, field, value, check) {
  const full = resolve(path);
  const raw = readFileSync(full, 'utf8');
  const data = JSON.parse(raw);
  if (data[field] === value) return false;
  if (!check) {
    data[field] = value;
    const indent = /^\{\n(\s+)/.exec(raw)?.[1].length ?? 2;
    writeFileSync(full, `${JSON.stringify(data, null, indent)}\n`);
  }
  return true;
}

function setCargoVersion(path, value, check) {
  const full = resolve(path);
  const raw = readFileSync(full, 'utf8');
  const updated = raw.replace(/^version = ".*"$/m, `version = "${value}"`);
  if (updated === raw) return false;
  if (!check) writeFileSync(full, updated);
  return true;
}

const { version, check } = parseArgs(process.argv.slice(2));

const FILES = [
  ['package.json', 'version'],
  ['src-tauri/tauri.conf.json', 'version'],
  ['src-tauri/Cargo.toml', 'version'],
];

const touched = FILES.filter(([path, field]) =>
  path.endsWith('.toml')
    ? setCargoVersion(path, version, check)
    : setJsonField(path, field, version, check),
);

if (touched.length === 0) {
  console.log(`version ${version} is already set everywhere`);
} else {
  const names = FILES.filter(([path, field]) =>
    path.endsWith('.toml')
      ? /^\d+\.\d+\.\d+$/.test(version) && !readFileSync(resolve(path), 'utf8').includes(`version = "${version}"`)
      : JSON.parse(readFileSync(resolve(path), 'utf8'))[field] !== version,
  ).map(([path]) => path);
  console.log(
    `${check ? 'would set' : 'set'} version ${version} in ${touched.length} of ${FILES.length} file(s): ${names.join(', ')}`,
  );

  // Checking used to only print what it would change and exit 0. The build
  // workflow called it and read the exit code, so a tag that disagreed with
  // Cargo.toml or tauri.conf.json built anyway: the updater compares the
  // version in the binary against the release manifest, and a mismatch does
  // not fail — the update banner simply never appears.
  if (check) {
    console.error(
      `bump-version: ${names.join(', ')} disagree with the requested version ${version}`,
    );
    process.exit(1);
  }
}

// The lockfile records the package version too; a stale entry makes
// `pnpm install --frozen-lockfile` fail in CI with a confusing message.
try {
  const lock = readFileSync(resolve('pnpm-lock.yaml'), 'utf8');
  if (lock.includes(`'${version}'`) || !/version:\s*\d+\.\d+\.\d+/.test(lock)) {
    console.log('pnpm-lock.yaml carries no root version to update');
  }
} catch {
  // No lockfile in this checkout; nothing to sync.
}