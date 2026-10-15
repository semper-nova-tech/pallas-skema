export type PortSide = 'left' | 'right' | 'top' | 'bottom';

export const PORT_SIDES: readonly PortSide[] = ['left', 'right', 'top', 'bottom'];

/** Nearest grid cell for a canvas pixel. Negative cells are left for Rust Validation. */
export function snapToCell(pixel: number, pad: number, cell: number): number {
  return Math.round((pixel - pad) / cell);
}

export function portOffset(side: PortSide, width: number, height: number): { x: number; y: number } {
  if (side === 'left') {
    return { x: 0, y: height / 2 };
  }
  if (side === 'right') {
    return { x: width, y: height / 2 };
  }
  if (side === 'top') {
    return { x: width / 2, y: 0 };
  }
  return { x: width / 2, y: height };
}

export function nearestPort<T extends { nodeId: string; x: number; y: number }>(
  ports: readonly T[],
  x: number,
  y: number,
  exceptNodeId: string,
): T | null {
  let best: T | null = null;
  let bestDistance = Number.POSITIVE_INFINITY;
  for (const port of ports) {
    if (port.nodeId === exceptNodeId) {
      continue;
    }
    const distance = (port.x - x) ** 2 + (port.y - y) ** 2;
    if (distance < bestDistance) {
      best = port;
      bestDistance = distance;
    }
  }
  return best;
}
