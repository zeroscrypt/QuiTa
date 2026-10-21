#!/usr/bin/env node
/**
 * Rebuild the updater manifest (latest.json) from the signatures attached to
 * a release.
 *
 * Why it exists: four platform jobs in `build.yml` run tauri-action in
 * parallel, and every one of them writes latest.json for the same release.
 * Whichever uploads last wins, and on release 1.5.0 the winner was the Intel
 * macOS job — the manifest advertised only darwin-x86_64, so updaters on
 * Apple Silicon, Windows and Linux found no row for themselves and silently
 * offered no update. That is how someone on 1.4.0 pressed "check for
 * update" and got nothing. The manifests of 1.3.2 through 1.4.1 carried all
 * eleven platform keys because the race happened not to bind that time.
 *
 * The one thing this script must not invent is a signature: it downloads the
 * .sig files attached to the release and encodes their exact bytes, so the
 * manifest cannot advertise a build other than the installers already on
 * the release. A signature missing for any updater artifact is an error,
 * not a quiet omission — a manifest with platforms missing is what broke
 * 1.5.0 for everyone not on Intel macOS.
 *
 * The manifest format the updater reads: version, pub_date, notes, and
 * platforms — a map from a platform key to a row with the download URL and
 * the signature. A row's signature is base64 of the whole .sig file; the
 * updater decodes it back into minisign's input.
 *
 * Usage:
 *   node scripts/merge-updater-json.mjs --tag v1.5.0 --repo zeroscrypt/QuiTa
 *       (dry run: prints the manifest to stdout)
 *   ... --out latest.json  (writes the file)
 *   ... --upload           (uploads it to the release and verifies the
 *                           upload reads back platform-complete)
 */

import { spawnSync } from 'node:child_process';
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

/** Platform keys a complete manifest has, in the form the releases that
 * survived the race carried them. The plain key is what the updater resolves
 * first; the suffixed keys name the bundle kind. The .dmg is absent on
 * purpose: it is the installer, while the updater on macOS downloads the
 * .app.tar.gz. */
const EXPECTED_KEYS = [
  'darwin-aarch64', 'darwin-aarch64-app',
  'darwin-x86_64', 'darwin-x86_64-app',
  'linux-x86_64', 'linux-x86_64-appimage',
  'linux-x86_64-deb', 'linux-x86_64-rpm',
  'windows-x86_64', 'windows-x86_64-msi', 'windows-x86_64-nsis',
];

/** Six artifacts carry signatures: two mac app archives, AppImage, deb,
 * rpm, msi, nsis setup. Anything else is not what the updater downloads. */
const EXPECTED_ARTIFACT_COUNT = 7;

function fail(message) {
  console.error(`::error::${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  const args = { out: '', repo: '', tag: '', upload: false };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--upload') args.upload = true;
    else if (arg === '--out') args.out = argv[(i += 1)];
    else if (arg === '--repo') args.repo = argv[(i += 1)];
    else if (arg === '--tag') args.tag = argv[(i += 1)];
    else fail(`unknown argument: ${arg}`);
  }
  if (!args.tag) fail('--tag vX.Y.Z is required');
  if (!args.repo) fail('--repo owner/name is required');
  return args;
}

function gh(args) {
  const run = spawnSync('gh', args, { encoding: 'utf8' });
  if (run.status !== 0) fail(`gh ${args.join(' ')} failed:\n${run.stderr}`);
  return run.stdout;
}

/** What a downloaded artifact is valid for. */
function platformKeysFor(artifact) {
  if (artifact.includes('aarch64.app.tar.gz')) return ['darwin-aarch64', 'darwin-aarch64-app'];
  if (artifact.includes('x64.app.tar.gz')) return ['darwin-x86_64', 'darwin-x86_64-app'];
  if (artifact.endsWith('.AppImage')) return ['linux-x86_64', 'linux-x86_64-appimage'];
  if (artifact.endsWith('.deb')) return ['linux-x86_64-deb'];
  if (artifact.endsWith('.rpm')) return ['linux-x86_64-rpm'];
  if (artifact.endsWith('.msi')) return ['windows-x86_64', 'windows-x86_64-msi'];
  if (artifact.endsWith('-setup.exe')) return ['windows-x86_64-nsis'];
  return null;
}

function downloadAssets(tag, repo, dir) {
  gh(['release', 'download', tag, '--repo', repo, '--dir', dir,
    '--pattern', '*.sig', '--pattern', 'latest.json', '--clobber']);
}

function readStoredManifest(dir) {
  try {
    const parsed = JSON.parse(readFileSync(join(dir, 'latest.json'), 'utf8'));
    if (parsed && typeof parsed === 'object' && typeof parsed.version === 'string') return parsed;
    fail('latest.json exists but is not a manifest shape this script understands');
  } catch {
    return null;
  }
  return null;
}

/** One row per updater platform, made purely from the release: the artifact
 * is whatever sits next to the .sig file, the signature is its exact bytes
 * base64-encoded. */
function collectPlatforms(dir, repo, tag) {
  mkdirSync(dir, { recursive: true });
  downloadAssets(tag, repo, dir);

  const platforms = {};
  const artifacts = [];
  for (const file of readdirSync(dir)) {
    if (!file.endsWith('.sig')) continue;
    const artifact = file.slice(0, -'.sig'.length);
    artifacts.push(artifact);
    const keys = platformKeysFor(artifact);
    if (keys === null) {
      fail(`no updater platform knows how to serve the artifact ${artifact}\n`
        + '(if the bundle list changed in tauri.conf.json, teach platformKeysFor the new name)');
    }
    for (const key of keys) {
      if (key in platforms) fail(`two artifacts claim ${key}: ${artifact}`);
      platforms[key] = {
        signature: readFileSync(join(dir, file), 'utf8'),
        url: `https://github.com/${repo}/releases/download/${tag}/${artifact}`,
      };
    }
  }
  if (artifacts.length === 0) {
    fail(`the release ${tag} carries no .sig files — a manifest is not possible,\n`
      + 'which the release notes describe as "a release without the signing key still installs, only nothing offers an update"');
  }
  if (artifacts.length !== EXPECTED_ARTIFACT_COUNT) {
    fail(`expected ${EXPECTED_ARTIFACT_COUNT} updater artifacts, found ${artifacts.length}: ${artifacts.join(', ')}`);
  }
  return platforms;
}

function signatureOf(entry) {
  return `${entry.signature}:${entry.url}`;
}

// ---- main ------------------------------------------------------------------

const args = parseArgs(process.argv.slice(2));
const work = join(tmpdir(), `quita-manifest-${args.tag.replace(/[^a-z0-9.-]/gi, '_')}`);
const platforms = collectPlatforms(work, args.repo, args.tag);

const missing = EXPECTED_KEYS.filter((key) => !(key in platforms));
if (missing.length > 0) {
  fail(`the release carries no signature for these platforms: ${missing.join(', ')}`);
}

const stored = readStoredManifest(work);
const storedPlatforms = stored?.platforms ?? {};
// Keys neither this script nor tauri-action builds are copied through, so
// the manifest evolves and loses nothing. A warning says what appeared.
for (const key of Object.keys(storedPlatforms).filter((k) => !EXPECTED_KEYS.includes(k))) {
  console.error(`::warning::copied the unknown platform ${key} from the stored manifest`);
  platforms[key] = storedPlatforms[key];
}

// Wherever the stored manifest already knows an entry, the regenerated one
// must agree byte for byte: the stored signature was made from the same
// .sig file, so a difference means this script matched the wrong file, not
// that the release changed.
let disagreements = 0;
for (const key of Object.keys(platforms).filter((k) => k in storedPlatforms)) {
  if (signatureOf(platforms[key]) !== signatureOf(storedPlatforms[key])) {
    console.error(`::error::${key}: the regenerated row disagrees with the stored one`);
    disagreements += 1;
  }
}
if (disagreements > 0) fail('aborting: the regenerated manifest contradicts the stored one');

const manifest = {
  version: args.tag.replace(/^v/, ''),
  notes: stored?.notes ?? 'See docs/DISTRIBUTION.md for install instructions.',
  pub_date: stored?.pub_date ?? new Date().toISOString(),
  platforms,
};

const identical = stored !== null && manifest.version === stored.version
  && manifest.notes === stored.notes && manifest.pub_date === stored.pub_date
  && Object.keys(storedPlatforms).length === Object.keys(platforms).length
  && EXPECTED_KEYS.every((key) => signatureOf(platforms[key]) === signatureOf(storedPlatforms[key]));

for (const key of Object.keys(platforms)) {
  const verb = key in storedPlatforms ? 'kept  ' : 'added ';
  console.log(`  ${verb} ${key} -> ${platforms[key].url.split('/').pop()}`);
}

const json = `${JSON.stringify(manifest, null, 2)}\n`;
if (identical && !args.out && !args.upload) {
  console.log('The stored manifest already matches the release; the dry run above is the proof.');
} else if (args.out) {
  writeFileSync(args.out, json);
  console.log(`written to ${args.out}`);
} else if (args.upload) {
  writeFileSync(join(work, 'latest.json'), json);
  gh(['release', 'upload', args.tag, '--repo', args.repo,
    `${join(work, 'latest.json')}#latest.json`, '--clobber']);
  console.log(`uploaded to release ${args.tag}`);

  // The upload is only worth something once it reads back from the server
  // with every platform an updater can ask for.
  const verify = join(tmpdir(), `quita-verify-${args.tag.replace(/[^a-z0-9.-]/gi, '_')}`);
  mkdirSync(verify, { recursive: true });
  downloadAssets(args.tag, args.repo, verify);
  const after = readStoredManifest(verify);
  const broken = Object.keys(platforms).filter((key) => signatureOf(after.platforms[key]) !== signatureOf(platforms[key]));
  if (broken.length > 0) fail(`after upload these keys do not read back correctly: ${broken.join(', ')}`);
  console.log(`verified on the server: ${Object.keys(platforms).length} platforms`);
} else {
  process.stdout.write(json);
}

rmSync(work, { recursive: true, force: true });
