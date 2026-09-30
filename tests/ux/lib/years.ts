function formatYear(y: number): string { return y < 0 ? `${-y} BC` : `AD ${y}`; }
function formatRange(from: number, to: number): string {
  return from === to ? formatYear(from) : `${formatYear(from)} – ${formatYear(to)}`;
}

// Batch E: mirrors client/YearText.cs's FormatClaim -- a leading "c."
// appears exactly when `note` is curated (e.g. "traditional"), matching the
// brief's own worked example ("Established c. 1003 BC · Destroyed 586 BC").
export function formatClaim(from: number, to: number, note: string | null): string {
  const range = formatRange(from, to);
  return note === null ? range : `c. ${range}`;
}
