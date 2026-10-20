import raw from '../../../style/skema-style.json';

export interface StyleBox {
  width: number;
  height: number;
}

export interface SkemaStyle {
  fontFamily: string;
  labelWeight: number;
  metaWeight: number;
  cell: number;
  pad: number;
  minColumns: number;
  minRows: number;
  radius: number;
  nodeBorder: number;
  edgeWidth: string;
  dotRadius: string;
  dotOffset: number;
  labelSize: number;
  metaSize: number;
  edgeLabelSize: number;
  edgeLabelLift: number;
  textGap: number;
  textPadX: number;
  nodes: {
    small: StyleBox;
    medium: StyleBox;
    large: StyleBox;
  };
  color: {
    ink: string;
    panel: string;
    canvas: string;
    nodeFill: string;
    phosphor: string;
    phosphorDim: string;
    dot: string;
    line: string;
    frame: string;
  };
}

/** Diagram tokens shared with Rust Render (`style/skema-style.json`). */
export const skemaStyle = raw as SkemaStyle;

/** Paint the shared tokens onto an element so Editor CSS and SVG use the same values. */
export function applySkemaStyle(root: HTMLElement): void {
  const { color } = skemaStyle;
  const set = (name: string, value: string) => root.style.setProperty(name, value);
  const px = (name: string, value: number | string) => set(name, `${value}px`);

  set('--ink', color.ink);
  set('--panel', color.panel);
  set('--canvas', color.canvas);
  set('--node-fill', color.nodeFill);
  set('--phosphor', color.phosphor);
  set('--phosphor-dim', color.phosphorDim);
  set('--dot', color.dot);
  set('--line', color.line);
  set('--frame', color.frame);
  set('--font-family', skemaStyle.fontFamily);
  set('--label-weight', String(skemaStyle.labelWeight));
  set('--meta-weight', String(skemaStyle.metaWeight));
  px('--cell', skemaStyle.cell);
  px('--radius', skemaStyle.radius);
  px('--node-border', skemaStyle.nodeBorder);
  px('--edge-width', skemaStyle.edgeWidth);
  px('--dot-radius', skemaStyle.dotRadius);
  px('--dot-offset', skemaStyle.dotOffset);
  px('--label-size', skemaStyle.labelSize);
  px('--meta-size', skemaStyle.metaSize);
  px('--edge-label-size', skemaStyle.edgeLabelSize);
  px('--text-gap', skemaStyle.textGap);
  px('--text-pad-x', skemaStyle.textPadX);
}
