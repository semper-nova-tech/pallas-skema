import { skemaStyle } from '../style/skema-style';
import { nearestPort, snapToCell } from './canvas-points';

describe('canvas points', () => {
  const pad = skemaStyle.pad;
  const cell = skemaStyle.cell;

  it('snaps a pixel to the nearest grid cell', () => {
    expect(snapToCell(pad, pad, cell)).toBe(0);
    expect(snapToCell(pad + cell + 20, pad, cell)).toBe(1);
    expect(snapToCell(10, pad, cell)).toBe(0);
    expect(snapToCell(-40, pad, cell)).toBe(-1);
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
