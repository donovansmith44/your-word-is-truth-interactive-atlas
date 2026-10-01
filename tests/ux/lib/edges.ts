export type NodeRef = { id: string; kind: string; label: string };
export type EdgeRef = { id: string; kind: string; label: string };
export type Position = { position: 'node'; node: NodeRef } | { position: 'edge'; edge: EdgeRef };
export type Neighbour = Position;
export type WordSpan = { from: { unit: { corpus: string; book: string; chapter: number; verse: number }; word: number }; to: { unit: { corpus: string; book: string; chapter: number; verse: number }; word: number } };
export type EdgeEntry = { edge: string; end: 'from' | 'to'; neighbour: Neighbour; loci?: WordSpan[] };
export type EdgeCard = EdgeRef & { from: Position; to: Position; provenance: string; edge_summary: { kind: string; count: number }[]; version: string };
export type EdgePage = { kind: string; entries: EdgeEntry[]; next: number | null; version: string };

export function neighbourNode(entry: EdgeEntry): NodeRef {
  return positionNode(entry.neighbour, `edge ${entry.edge}`);
}

export function positionNode(position: Position, what: string): NodeRef {
  if (position.position !== 'node') {
    throw new Error(`${what} leads to an edge, not a node`);
  }
  return position.node;
}

export function edgePosition(card: EdgeCard): Position {
  return { position: 'edge', edge: { id: card.id, kind: card.kind, label: card.label } };
}
