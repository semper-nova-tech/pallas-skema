/** Wire shapes for Manifest JSON (Validation is Rust-only). */

export type NodeType = 'process' | 'decision' | 'note';
export type NodeSize = 'small' | 'medium' | 'large';

export interface GridPosition {
  x: number;
  y: number;
}

export interface ManifestNode {
  id: string;
  label: string;
  type: NodeType;
  size: NodeSize;
  gridPosition: GridPosition;
}

export interface ManifestEdge {
  from: string;
  to: string;
  label?: string;
}

export interface CanvasSize {
  columns: number;
  rows: number;
}

export interface ManifestDocument {
  canvas?: CanvasSize;
  nodes: ManifestNode[];
  edges: ManifestEdge[];
}

export interface EditorJsonHandoff {
  manifestJson: string;
  manifestPath: string;
  svgPath: string;
}
