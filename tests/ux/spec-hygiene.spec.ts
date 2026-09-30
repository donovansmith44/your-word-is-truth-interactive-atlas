import { test, expect } from '@playwright/test';
import * as fs from 'fs';
import * as path from 'path';

test('HYGIENE-1: no spec opens a verse by clicking its row', () => {
  const dir = __dirname;
  const offenders: string[] = [];
  for (const file of fs.readdirSync(dir).filter(f => f.endsWith('.spec.ts'))) {
    const lines = fs.readFileSync(path.join(dir, file), 'utf8').split('\n');
    lines.forEach((line, i) => {
      if (/verse-line-[^)]*\)\s*\.click\(\s*\)/.test(line)) {
        offenders.push(`${file}:${i + 1}`);
      }
    });
  }
  expect(offenders).toEqual([]);
});
