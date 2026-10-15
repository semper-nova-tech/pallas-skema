import { nearestPort, snapToCell } from './canvas-points';

describe('canvas points', () => {
  it('snaps a pixel to the nearest grid cell', () => {
    expect(snapToCell(28, 28, 56)).toBe(0);
    expect(snapToCell(28 + 56 + 20, 28, 56)).toBe(1);
    expect(snapToCell(10, 28, 56)).toBe(0);
    expect(snapToCell(-40, 28, 56)).toBe(-1);
  });

  it('picks the closest port on another node', () => {
    const ports = [
      { nodeId: 'a', side: 'right' as const, x: 10, y: 10 },
      { nodeId: 'b', side: 'left' as const, x: 40, y: 12 },
      { nodeId: 'b', side: 'right' as const, x: 80, y: 12 },
    ];
    expect(nearestPort(ports, 38, 12, 'a')?.side).toBe('left');
    expect(nearestPort([{ nodeId: 'a', side: 'right', x: 10, y: 10 }], 12, 10, 'a')).toBeNull();
  });
});
