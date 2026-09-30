export type NodeRef = { id: string; kind: string; label: string };
export type Neighbour = { position: 'node'; node: NodeRef } | { position: 'edge'; edge: { id: string } };
export type WordSpan = { from: { unit: { corpus: string; book: string; chapter: number; verse: number }; word: number }; to: { unit: { corpus: string; book: string; chapter: number; verse: number }; word: number } };
export type EdgeEntry = { edge: string; neighbour: Neighbour; loci?: WordSpan[] };
export type EdgePage = { kind: string; entries: EdgeEntry[]; next: number | null; version: string };

export function neighbourNode(entry: EdgeEntry): NodeRef {
  if (entry.neighbour.position !== 'node') {
    throw new Error(`edge ${entry.edge} leads to an edge, not a node`);
  }
  return entry.neighbour.node;
}
