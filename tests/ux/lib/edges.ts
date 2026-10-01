export type NodeRef = { id: string; kind: string; label: string };
export type EdgeRef = { id: string; kind: string; label: string };
export type Position = { position: 'node'; node: NodeRef } | { position: 'edge'; edge: EdgeRef };
export type Neighbour = Position;
export type WordSpan = { from: { unit: { corpus: string; book: string; chapter: number; verse: number }; word: number }; to: { unit: { corpus: string; book: string; chapter: number; verse: number }; word: number } };
export type EdgeEntry = { edge: EdgeRef; neighbour: Neighbour; loci?: WordSpan[] };
export type EdgePage = { kind: string; entries: EdgeEntry[]; next: number | null; version: string };
export type EdgeRecord = EdgeRef & { subject: Position; object: Position; provenance?: string; edge_summary: { kind: string; count: number }[] };
export type Element = { element: 'node'; node: NodeRef } | { element: 'edge'; edge: EdgeRecord } | { element: 'missing'; id: string };
export type ElementPage = { elements: Element[]; version: string };

export function neighbourNode(entry: EdgeEntry): NodeRef {
  return positionNode(entry.neighbour, `edge ${entry.edge.id}`);
}

export function positionNode(position: Position, what: string): NodeRef {
  if (position.position !== 'node') {
    throw new Error(`${what} leads to an edge, not a node`);
  }
  return position.node;
}

export function elementEdge(element: Element, what: string): EdgeRecord {
  if (element.element !== 'edge') {
    throw new Error(`${what} is not an edge`);
  }
  return element.edge;
}

export function elementNode(element: Element, what: string): NodeRef {
  if (element.element !== 'node') {
    throw new Error(`${what} is not a node`);
  }
  return element.node;
}
