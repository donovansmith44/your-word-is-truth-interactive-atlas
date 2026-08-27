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
  // Batch N: GET /api/narrative/event/{id} -- every narrative position the
  // given event id occupies (mirrors catechismItem's own id-keyed shape;
  // verse.narrative_positions is the verse-keyed sibling, already present
  // on the verse() response above -- no separate wrapper needed for it).
  narrativeEventPositions: (eventId: string) => getJson(`/api/narrative/event/${encodeURIComponent(eventId)}`),
  // Batch T requirement 4: GET /api/event/{id} -- an EVENT-kind PASSAGE's
  // own rich content (title/when/places/witnesses/provenance).
  event: (eventId: string) => getJson(`/api/event/${encodeURIComponent(eventId)}`),
  // Batch B2: /api/borders -> /api/polities (per-polity timerange borders).
  polities: (from: number, to: number) => getJson(`/api/polities?from=${from}&to=${to}`),
  landmarks: () => getJson('/api/landmarks'),
  // Batch R requirement 1: GET /api/land-mask -- the curated coastline
  // clip geometry.
  landMask: () => getJson('/api/land-mask'),
  // Batch S: GET /api/sources -- the Sources page's own single source of
  // truth (categories + per-source entries), generated 1:1 from
  // LICENSES.md.
  sources: () => getJson('/api/sources'),
  // Batch M-D2 (P7 closure): the two generic graph endpoints -- the SAME
  // shapes client/IExplorableClient.cs's Card()/Edges() consume, read here
  // directly (raw fetch, not through the Blazor app) for CONTRACT-lockstep
  // assertions that need to compare the generic contract's own wire shape
  // against the bespoke endpoints' own (e.g. votes-ranked order).
  node: (id: string) => getJson(`/api/node/${encodeURIComponent(id)}`),
  nodeEdges: (id: string, kind: string, opts: { cursor?: number; limit?: number } = {}) => {
    const params = new URLSearchParams({ kind, ...(opts.limit != null ? { limit: String(opts.limit) } : {}), ...(opts.cursor != null ? { cursor: String(opts.cursor) } : {}) });
    return getJson(`/api/node/${encodeURIComponent(id)}/edges?${params}`);
  },
  // Batch CORP-1: GET /api/text -- the SAME reading-window endpoint
  // client/IExplorableClient.cs's Reading() consumes, read here directly for
  // CONTRACT-lockstep assertions (kretzmann.spec.ts/concord.spec.ts compare
  // what the Concord page renders against this raw response).
  reading: (ref: string, n: number, opts: { dir?: string; corpus?: string } = {}) => {
    const params = new URLSearchParams({ ref, n: String(n), dir: opts.dir ?? 'onward', corpus: opts.corpus ?? 'bible' });
    return getJson(`/api/text?${params}`);
  },
};
