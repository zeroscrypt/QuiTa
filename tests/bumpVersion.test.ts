import { describe, expect, it } from 'vitest';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

/**
 * The auto-release workflow bumps the version in three files and tags the
 * commit. If those files ever disagree, the updater compares the version in
 * the binary with the one in the release manifest, finds nothing newer, and
 * the green banner simply stops appearing — no error, just silence. That is
 * expensive to debug later, so it is checked here instead.
 */

const SCRIPT = join(process.cwd(), 'scripts', 'bump-version.mjs');

function makeProject(version: string): string {
  const dir = mkdtempSync(join(tmpdir(), 'quita-version-'));
  writeFileSync(join(dir, 'package.json'), JSON.stringify({ name: 'x', version }, null, 2) + '\n');
  mkdirSync(join(dir, 'src-tauri'));
  writeFileSync(
    join(dir, 'src-tauri', 'tauri.conf.json'),
    JSON.stringify({ version, productName: 'QuiTa' }, null, 2) + '\n',
  );
  writeFileSync(
    join(dir, 'src-tauri', 'Cargo.toml'),
    `[package]\nname = "quita"\nversion = "${version}"\nedition = "2021"\n`,
  );
  return dir;
}

function readVersion(dir: string): Record<string, string> {
  const pkg = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'));
  const conf = JSON.parse(readFileSync(join(dir, 'src-tauri', 'tauri.conf.json'), 'utf8'));
  const cargo = readFileSync(join(dir, 'src-tauri', 'Cargo.toml'), 'utf8');
  return {
    package: pkg.version,
    tauri: conf.version,
    cargo: /version = "([^"]+)"/.exec(cargo)?.[1] ?? '',
  };
}

function run(dir: string, ...args: string[]): string {
  // Without the override the script would edit the real repository, which is
  // exactly the accident this arrangement prevents.
  return execFileSync('node', [SCRIPT, ...args], {
    cwd: dir,
    encoding: 'utf8',
    env: { ...process.env, QUITA_PROJECT_ROOT: dir },
  });
}

describe('bump-version', () => {
  it('raises the version in all three files at once', () => {
    const dir = makeProject('1.0.0');
    try {
      run(dir, '1.0.1');
      expect(readVersion(dir)).toEqual({ package: '1.0.1', tauri: '1.0.1', cargo: '1.0.1' });
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('accepts a tag with a leading v', () => {
    // The workflow passes the tag straight through; stripping `v` in shell
    // quoting is exactly the kind of thing that breaks on a different runner.
    const dir = makeProject('1.0.0');
    try {
      run(dir, 'v2.0.0');
      expect(readVersion(dir)).toEqual({ package: '2.0.0', tauri: '2.0.0', cargo: '2.0.0' });
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('leaves everything alone when the version already matches', () => {
    const dir = makeProject('1.0.1');
    try {
      const before = readFileSync(join(dir, 'package.json'), 'utf8');
      const output = run(dir, '1.0.1');
      expect(output).toContain('already set');
      expect(readFileSync(join(dir, 'package.json'), 'utf8')).toBe(before);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('never changes files in --check mode', () => {
    const dir = makeProject('1.0.0');
    try {
      const before = readFileSync(join(dir, 'package.json'), 'utf8');
      // The run fails here, and that is the point: the build workflow reads
      // this exit code to refuse a tag that disagrees with the sources. What
      // must not happen is the file being rewritten on the way to failing.
      expect(() => run(dir, '--check', '1.0.1')).toThrow();
      expect(readFileSync(join(dir, 'package.json'), 'utf8')).toBe(before);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('succeeds in --check mode when the versions already agree', () => {
    const dir = makeProject('1.0.0');
    try {
      expect(run(dir, '--check', '1.0.0')).toContain('already set everywhere');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('names the files that disagree', () => {
    // "3 of 3 files" is not actionable on a release day; the path is.
    const dir = makeProject('1.0.0');
    try {
      let stderr = '';
      try {
        execFileSync('node', [SCRIPT, '--check', '2.0.0'], {
          cwd: dir,
          encoding: 'utf8',
          env: { ...process.env, QUITA_PROJECT_ROOT: dir },
          stdio: 'pipe',
        });
      } catch (error) {
        stderr = String((error as { stderr?: string }).stderr ?? '');
      }
      expect(stderr).toContain('src-tauri/Cargo.toml');
      expect(stderr).toContain('src-tauri/tauri.conf.json');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('keeps the rest of each file intact', () => {
    // A rewrite that reformats Cargo.toml would produce a noisy diff and could
    // break the package metadata cargo reads.
    const dir = makeProject('1.0.0');
    try {
      run(dir, '1.0.1');
      const cargo = readFileSync(join(dir, 'src-tauri', 'Cargo.toml'), 'utf8');
      expect(cargo).toContain('name = "quita"');
      expect(cargo).toContain('edition = "2021"');
      const conf = JSON.parse(readFileSync(join(dir, 'src-tauri', 'tauri.conf.json'), 'utf8'));
      expect(conf.productName).toBe('QuiTa');
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('finds the number behind any tag prefix', () => {
    // Tags created before include-component-in-tag was turned off look like
    // `QuiTa-v1.0.1`; a script that cannot read those cannot verify them.
    const dir = makeProject('1.0.0');
    try {
      run(dir, 'QuiTa-v1.0.1');
      expect(readVersion(dir)).toEqual({ package: '1.0.1', tauri: '1.0.1', cargo: '1.0.1' });
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it('refuses anything without a version in it', () => {
    const dir = makeProject('1.0.0');
    try {
      expect(() => run(dir, 'latest')).toThrow();
      expect(() => run(dir, '1.0')).toThrow();
      expect(() => run(dir, '')).toThrow();
      expect(readVersion(dir)).toEqual({ package: '1.0.0', tauri: '1.0.0', cargo: '1.0.0' });
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});