import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

/**
 * Svelte throws `each_key_duplicate` when two rows of an `{#each}` end up with
 * the same key, and it renders nothing. The task menu used to key every
 * separator as the literal string "sep", so the menu of every root task threw
 * and drew zero buttons — and no test caught it, because server rendering does
 * not throw on duplicate keys, and menus are only mounted after being opened,
 * which server rendering never does.
 *
 * So the check is on the source instead: a key that falls back to a fixed
 * string cannot be unique unless it also varies by position, which means the
 * block must destructure the index and use it.
 */

const read = (path: string) => readFileSync(path, 'utf8');

function svelteFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) return svelteFiles(full);
    return entry.name.endsWith('.svelte') ? [full] : [];
  });
}

interface EachBlock {
  file: string;
  key: string;
  index: string | undefined;
}

function eachBlocks(file: string): EachBlock[] {
  const source = read(file);
  const blocks: EachBlock[] = [];
  // `{#each <items> as <item>[, <index>] (<key>)}`
  const pattern = /\{#each\s+[\s\S]+?\s+as\s+([A-Za-z_$][\w$]*)\s*(?:,\s*([A-Za-z_$][\w$]*)\s*)?\(([^()]*)\)\s*\}/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(source)) !== null) {
    blocks.push({
      file: file.replace(`${process.cwd()}/`, ''),
      item: match[1],
      index: match[2],
      key: match[3].trim(),
    } as EachBlock);
  }
  return blocks;
}

const files = svelteFiles(new URL('../src', import.meta.url).pathname);
const all = files.flatMap(eachBlocks);

/** A key branch that is a fixed string, e.g. `cond ? 'sep' : item.id`. */
function hasLiteralBranch(key: string): boolean {
  return /\?[^?:]*['"][^'"]*['"][^?:]*:/.test(key);
}

describe('each block keys', () => {
  it('finds keyed blocks to check', () => {
    // If the pattern stops matching, the test below would pass vacuously.
    expect(all.length).toBeGreaterThan(5);
    expect(all.some((b) => b.index !== undefined)).toBe(true);
  });

  it('varies a literal key branch by position', () => {
    for (const block of all) {
      if (!hasLiteralBranch(block.key)) continue;
      expect(
        block.index,
        `${block.file}: the key \`${block.key}\` returns the same literal for every item of that kind, so Svelte throws each_key_duplicate. Destructure the index and include it.`,
      ).toBeDefined();
    }
  });

  it('separators in the task menu are keyed apart', () => {
    const menu = all.find((b) => b.file.endsWith('ContextMenu.svelte'));
    expect(menu, 'the task menu block was not found').toBeDefined();
    expect(menu!.index, 'the task menu key must vary by position').toBeDefined();
    expect(menu!.key).toMatch(/\$\{i\}|i\b/);
  });
});