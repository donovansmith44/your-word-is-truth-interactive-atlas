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
  place: (id: string) => getJson(`/api/place/${id}`),
  // Batch E: `/api/place/{id}?from=&to=` -- the window-scoped `history` payload.
  placeHistory: (id: string, from: number, to: number) => getJson(`/api/place/${id}?from=${from}&to=${to}`),
  narratives: () => getJson('/api/narratives'),
  borders: (from: number, to: number) => getJson(`/api/borders?from=${from}&to=${to}`),
  landmarks: () => getJson('/api/landmarks'),
};
