export const API = 'http://localhost:8000';
async function getJson(path: string): Promise<any> {
  const r = await fetch(`${API}${path}`);
  if (!r.ok) { const body = await r.json().catch(() => ({})); return { __status: r.status, ...body }; }
  return r.json();
}
export const api = {
  raw: getJson,
  sceneTime: (from: number, to: number) => getJson(`/api/scene?from=${from}&to=${to}`),
  sceneScripture: (sref: string) => getJson(`/api/scene/scripture?ref=${encodeURIComponent(sref)}`),
  books: () => getJson('/api/books'),
  eras: () => getJson('/api/eras'),
  chapter: (cref: string) => getJson(`/api/chapter/${cref}`),
  verse: (vref: string) => getJson(`/api/verse/${vref}`),
  // Batch G1: GET /api/xrefs/{sref} -- span-aggregated cross-references.
  xrefs: (sref: string) => getJson(`/api/xrefs/${sref}`),
  // Batch F: GET /api/catechism/{sref} -- span-aggregated catechism
  // citations (mirrors xrefs above); GET /api/catechism/item/{id} -- one
  // catechism item's own full content.
  catechism: (sref: string) => getJson(`/api/catechism/${sref}`),
  catechismItem: (id: string) => getJson(`/api/catechism/item/${id}`),
  place: (id: string) => getJson(`/api/place/${id}`),
  // Batch E: `/api/place/{id}?from=&to=` -- the window-scoped `history` payload.
  placeHistory: (id: string, from: number, to: number) => getJson(`/api/place/${id}?from=${from}&to=${to}`),
  narratives: () => getJson('/api/narratives'),
  // Batch B2: /api/borders -> /api/polities (per-polity timerange borders).
  polities: (from: number, to: number) => getJson(`/api/polities?from=${from}&to=${to}`),
  landmarks: () => getJson('/api/landmarks'),
  // Batch R requirement 1: GET /api/land-mask -- the curated coastline
  // clip geometry.
  landMask: () => getJson('/api/land-mask'),
};
