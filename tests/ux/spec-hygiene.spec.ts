import { test, expect } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

function specLinesMatching(pattern: RegExp): string[] {
  const offenders: string[] = [];
  for (const file of fs.readdirSync(__dirname).filter(f => f.endsWith('.spec.ts') && f !== path.basename(__filename))) {
    const lines = fs.readFileSync(path.join(__dirname, file), 'utf8').split('\n');
    lines.forEach((line, i) => {
      if (pattern.test(line)) {
        offenders.push(`${file}:${i + 1}`);
      }
    });
  }
  return offenders;
}

test('HYGIENE-1: no spec opens a verse by clicking its row', () => {
  expect(specLinesMatching(/verse-line-[^)]*\)\s*\.click\(\s*\)/)).toEqual([]);
});

test('HYGIENE-2: no spec reads an edge entry by its wire field names; lib/edges.ts is the one reader', () => {
  expect(specLinesMatching(/\.neighbour\b|\bentries\b.*\.node\b/)).toEqual([]);
});

test('HYGIENE-3: no spec snapshots the popover chips; lib/popover.ts is the one retrying reader', () => {
  expect(specLinesMatching(/popover-head-actions \[data-testid\]/)).toEqual([]);
});
