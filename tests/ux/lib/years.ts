export const SPAN = { from: -4004, to: 100 };
export function formatYear(y: number): string { return y < 0 ? `${-y} BC` : `AD ${y}`; }
export function parseYearText(s: string): number {
  const bc = s.match(/^(\d+) BC$/); if (bc) return -Number(bc[1]);
  const ad = s.match(/^AD (\d+)$/); if (ad) return Number(ad[1]);
  throw new Error(`unparseable year text: ${s}`);
}
export function formatRange(from: number, to: number): string {
  return from === to ? formatYear(from) : `${formatYear(from)} – ${formatYear(to)}`;
}
