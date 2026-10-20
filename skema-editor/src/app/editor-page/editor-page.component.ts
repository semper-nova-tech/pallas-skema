import { ChangeDetectionStrategy, Component, inject, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { MatButtonModule } from '@angular/material/button';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';
import { loadManifest, saveManifest, type EditorSession } from '../../editor';
import {
  applyConnectEdge,
  applyNodeGridMove,
  applyEdgeLabelEdit,
  applyNodeLabelEdit,
  applyNodeWithEdge,
  addNodeAt,
  removeEdge,
} from '../../editor/apply-edit';
import {
  PORT_SIDES,
  nearestPort,
  portOffset,
  snapToCell,
  type PortSide,
} from '../../editor/canvas-points';
import type { EditorJsonHandoff, ManifestNode } from '../../editor/manifest.types';
import { skemaStyle } from '../../style/skema-style';
import { SkemaApiService } from '../skema-api.service';

const CELL_PX = skemaStyle.cell;
const CANVAS_PAD = skemaStyle.pad;

interface PortHit {
  nodeId: string;
  side: PortSide;
  x: number;
  y: number;
}

interface LinkDrag {
  fromNodeId: string;
  fromSide: PortSide;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

@Component({
  selector: 'app-editor-page',
  imports: [MatButtonModule, MatFormFieldModule, MatInputModule, MatSelectModule],
  templateUrl: './editor-page.component.html',
  styleUrl: './editor-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '(document:pointermove)': 'onPointerMove($event)',
    '(document:pointerup)': 'onPointerUp($event)',
    '(document:pointerdown)': 'onPointerDown($event)',
    '(document:auxclick)': 'onAuxClick($event)',
    '(document:contextmenu)': 'onContextMenu($event)',
    '(document:mouseout)': 'onMouseOut($event)',
    '(document:pointercancel)': 'onPointerCancel()',
  },
})
export class EditorPageComponent {
  private readonly http = inject(HttpClient);
  private readonly transport = inject(SkemaApiService);

  readonly portSides = PORT_SIDES;
  readonly canvasColumns = skemaStyle.minColumns;
  readonly canvasRows = skemaStyle.minRows;
  readonly session = signal<EditorSession | null>(null);
  readonly selectedNodeId = signal<string | null>(null);
  readonly edgeTargetId = signal<string | null>(null);
  readonly statusMessage = signal<string>('Loading handoff…');
  readonly justSaved = signal(false);
  readonly dragPreview = signal<{ nodeId: string; x: number; y: number } | null>(null);
  readonly linkDrag = signal<LinkDrag | null>(null);
  readonly armedPort = signal<{ nodeId: string; side: PortSide } | null>(null);
  readonly renamingNodeId = signal<string | null>(null);
  readonly hoveredEdge = signal<number | null>(null);
  readonly editingEdge = signal<number | null>(null);
  readonly edgeDraft = signal('');

  private savedTimer = 0;
  private edgeLeaveTimer = 0;
  private canvas: HTMLElement | null = null;
  private nodeDrag: { nodeId: string; originX: number; originY: number; moved: boolean } | null = null;
  private linkLive = false;

  constructor() {
    void this.bootstrap();
  }

  async bootstrap(): Promise<void> {
    try {
      const handoff = await firstValueFrom(this.http.get<EditorJsonHandoff>('/handoff.json'));
      if (!handoff) {
        this.statusMessage.set('No handoff; start this page from skema open-editor');
        return;
      }
      const loaded = await loadManifest(handoff, this.transport);
      this.session.set(loaded);
      this.statusMessage.set(`State: ${loaded.lifecycleState}`);
    } catch {
      this.statusMessage.set('Failed to load handoff.json');
    }
  }

  nodeBox(node: ManifestNode): { width: number; height: number } {
    return skemaStyle.nodes[node.size];
  }

  nodeTransform(node: ManifestNode): string {
    const origin = this.nodeOrigin(node);
    return `translate(${origin.x}px, ${origin.y}px)`;
  }

  canvasWidth(): number {
    const nodes = this.session()?.manifest?.nodes ?? [];
    const content = nodes.reduce((max, node) => {
      const box = this.nodeBox(node);
      return Math.max(max, CANVAS_PAD + node.gridPosition.x * CELL_PX + box.width + CANVAS_PAD);
    }, 0);
    return Math.max(this.canvasColumns * CELL_PX, content);
  }

  canvasHeight(): number {
    const nodes = this.session()?.manifest?.nodes ?? [];
    const content = nodes.reduce((max, node) => {
      const box = this.nodeBox(node);
      return Math.max(max, CANVAS_PAD + node.gridPosition.y * CELL_PX + box.height + CANVAS_PAD);
    }, 0);
    return Math.max(this.canvasRows * CELL_PX, content);
  }

  edgeDrawings(): Array<{
    index: number;
    from: string;
    to: string;
    label?: string;
    x1: number;
    y1: number;
    x2: number;
    y2: number;
    labelX: number;
    labelY: number;
  }> {
    const manifest = this.session()?.manifest;
    if (!manifest) {
      return [];
    }
    return manifest.edges.flatMap((edge, index) => {
      const from = manifest.nodes.find((node) => node.id === edge.from);
      const to = manifest.nodes.find((node) => node.id === edge.to);
      if (!from || !to) {
        return [];
      }
      const start = this.facingPort(from, this.nodeCenter(to));
      const end = this.facingPort(to, this.nodeCenter(from));
      return [
        {
          index,
          from: edge.from,
          to: edge.to,
          label: edge.label,
          x1: start.x,
          y1: start.y,
          x2: end.x,
          y2: end.y,
          labelX: (start.x + end.x) / 2,
          labelY: (start.y + end.y) / 2 - skemaStyle.edgeLabelLift,
        },
      ];
    });
  }

  isArmed(nodeId: string, side: PortSide): boolean {
    const armed = this.armedPort();
    return armed?.nodeId === nodeId && armed.side === side;
  }

  beginNodeDrag(event: PointerEvent, node: ManifestNode): void {
    if (event.button !== 0 || this.linkDrag()) {
      return;
    }
    const canvas = (event.currentTarget as HTMLElement).closest('.canvas') as HTMLElement | null;
    const point = this.pointOn(canvas, event);
    if (!canvas || !point) {
      return;
    }
    event.preventDefault();
    this.canvas = canvas;
    this.selectNode(node.id);
    const origin = this.nodeOrigin(node);
    this.nodeDrag = {
      nodeId: node.id,
      originX: point.x - origin.x,
      originY: point.y - origin.y,
      moved: false,
    };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  beginLink(event: PointerEvent, node: ManifestNode, side: PortSide): void {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const canvas = (event.currentTarget as HTMLElement).closest('.canvas') as HTMLElement | null;
    if (!canvas) {
      return;
    }
    const port = this.portHits(node).find((hit) => hit.side === side);
    if (!port) {
      return;
    }
    this.canvas = canvas;
    this.linkLive = true;
    this.nodeDrag = null;
    this.dragPreview.set(null);
    this.selectNode(node.id);
    this.linkDrag.set({
      fromNodeId: node.id,
      fromSide: side,
      x1: port.x,
      y1: port.y,
      x2: port.x,
      y2: port.y,
    });
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  onPointerMove(event: PointerEvent): void {
    const point = this.pointOn(this.canvas, event);
    if (!point) {
      return;
    }
    const drag = this.nodeDrag;
    if (drag) {
      const x = point.x - drag.originX;
      const y = point.y - drag.originY;
      if (Math.abs(x - this.gridOrigin(drag.nodeId).x) > 3 || Math.abs(y - this.gridOrigin(drag.nodeId).y) > 3) {
        drag.moved = true;
      }
      this.dragPreview.set({ nodeId: drag.nodeId, x, y });
      return;
    }
    const link = this.linkDrag();
    if (!link || !this.linkLive) {
      return;
    }
    const armed = nearestPort(this.allPorts(), point.x, point.y, link.fromNodeId);
    this.armedPort.set(armed ? { nodeId: armed.nodeId, side: armed.side } : null);
    this.linkDrag.set({
      ...link,
      x2: armed?.x ?? point.x,
      y2: armed?.y ?? point.y,
    });
  }

  onPointerUp(event: PointerEvent): void {
    if (event.button !== 0) {
      return;
    }
    if (this.nodeDrag) {
      void this.finishNodeDrag();
      return;
    }
    if (this.linkLive) {
      void this.finishLink();
    }
  }

  onPointerDown(event: PointerEvent): void {
    if (event.button === 1 && this.linkLive) {
      event.preventDefault();
      this.cancelLink();
    }
  }

  onAuxClick(event: MouseEvent): void {
    if (event.button === 1) {
      event.preventDefault();
    }
  }

  onContextMenu(event: MouseEvent): void {
    const link = this.linkDrag();
    if (!link || !this.linkLive) {
      return;
    }
    event.preventDefault();
    const point = this.pointOn(this.canvas, event);
    this.cancelLink();
    if (point) {
      void this.spawnNodeAt(link.fromNodeId, point);
    }
  }

  onMouseOut(event: MouseEvent): void {
    if (this.linkLive && event.relatedTarget === null) {
      this.cancelLink();
    }
  }

  onPointerCancel(): void {
    this.nodeDrag = null;
    this.dragPreview.set(null);
    this.cancelLink();
  }

  otherNodeIds(): string[] {
    const current = this.session();
    const selected = this.selectedNodeId();
    if (!current?.manifest || !selected) {
      return [];
    }
    return current.manifest.nodes.filter((node) => node.id !== selected).map((node) => node.id);
  }

  selectNode(id: string): void {
    this.selectedNodeId.set(id);
    this.edgeTargetId.set(null);
  }

  async moveSelected(dx: number, dy: number): Promise<void> {
    const current = this.session();
    const nodeId = this.selectedNodeId();
    if (!current?.manifest || !nodeId) {
      return;
    }
    const node = current.manifest.nodes.find((item) => item.id === nodeId);
    if (!node) {
      return;
    }
    const next = await applyNodeGridMove(
      current,
      this.transport,
      nodeId,
      node.gridPosition.x + dx,
      node.gridPosition.y + dy,
    );
    this.applySession(next, next.textFailure ?? `State: ${next.lifecycleState}`);
  }

  async connectEdge(): Promise<void> {
    const current = this.session();
    const fromId = this.selectedNodeId();
    const toId = this.edgeTargetId();
    if (!current || !fromId || !toId) {
      return;
    }
    const next = await applyConnectEdge(current, this.transport, fromId, toId);
    this.applySession(next, next.textFailure ?? 'Edge connected');
  }

  onLabelInput(event: Event): void {
    const current = this.session();
    const nodeId = this.renamingNodeId() ?? this.selectedNodeId();
    if (!current || !nodeId) {
      return;
    }
    const value = (event.target as HTMLInputElement).value;
    this.session.set(applyNodeLabelEdit(current, nodeId, value));
  }

  startRename(nodeId: string, event: Event): void {
    event.preventDefault();
    event.stopPropagation();
    this.selectNode(nodeId);
    this.renamingNodeId.set(nodeId);
  }

  stopRename(): void {
    this.renamingNodeId.set(null);
  }

  edgeTipOpen(index: number): boolean {
    return this.hoveredEdge() === index || this.editingEdge() === index;
  }

  showEdgeTip(index: number): void {
    window.clearTimeout(this.edgeLeaveTimer);
    this.hoveredEdge.set(index);
  }

  hideEdgeTip(index: number): void {
    window.clearTimeout(this.edgeLeaveTimer);
    this.edgeLeaveTimer = window.setTimeout(() => {
      if (this.editingEdge() === index) {
        return;
      }
      if (this.hoveredEdge() === index) {
        this.hoveredEdge.set(null);
      }
    }, 220);
  }

  startEdgeEdit(index: number, label: string | undefined, event: Event): void {
    event.preventDefault();
    event.stopPropagation();
    this.editingEdge.set(index);
    this.edgeDraft.set(label ?? '');
    this.hoveredEdge.set(index);
  }

  onEdgeDraft(event: Event): void {
    this.edgeDraft.set((event.target as HTMLInputElement).value);
  }

  commitEdgeEdit(index: number): void {
    const current = this.session();
    if (!current || this.editingEdge() !== index) {
      return;
    }
    this.session.set(applyEdgeLabelEdit(current, index, this.edgeDraft()));
    this.editingEdge.set(null);
    this.statusMessage.set(this.edgeDraft().trim() ? 'Edge name updated' : 'Edge name cleared');
  }

  deleteEdge(index: number, event: Event): void {
    event.preventDefault();
    event.stopPropagation();
    const current = this.session();
    if (!current) {
      return;
    }
    this.session.set(removeEdge(current, index));
    this.hoveredEdge.set(null);
    this.editingEdge.set(null);
    this.statusMessage.set('Edge removed');
  }

  addSampleNode(): void {
    const current = this.session();
    if (!current?.manifest) {
      return;
    }
    const id = `node-${current.manifest.nodes.length + 1}`;
    const next = addNodeAt(current, {
      id,
      label: id,
      type: 'process',
      size: 'medium',
      gridPosition: { x: 0, y: 0 },
    });
    this.session.set(next);
    this.selectedNodeId.set(id);
  }

  selectedNodeLabel(): string {
    const current = this.session();
    const nodeId = this.selectedNodeId();
    if (!current?.manifest || !nodeId) {
      return '';
    }
    return current.manifest.nodes.find((node) => node.id === nodeId)?.label ?? '';
  }

  async onSave(): Promise<void> {
    const current = this.session();
    if (!current) {
      return;
    }
    const next = await saveManifest(current, this.transport);
    this.session.set(next);
    if (next.textFailure) {
      this.justSaved.set(false);
      this.statusMessage.set(next.textFailure);
      return;
    }
    this.statusMessage.set('Saved. SVG written from this canvas.');
    this.justSaved.set(true);
    window.clearTimeout(this.savedTimer);
    this.savedTimer = window.setTimeout(() => this.justSaved.set(false), 700);
  }

  private async finishNodeDrag(): Promise<void> {
    const drag = this.nodeDrag;
    const preview = this.dragPreview();
    this.nodeDrag = null;
    if (!drag || !preview || !drag.moved) {
      this.dragPreview.set(null);
      return;
    }
    const cellX = snapToCell(preview.x, CANVAS_PAD, CELL_PX);
    const cellY = snapToCell(preview.y, CANVAS_PAD, CELL_PX);
    this.dragPreview.set({
      nodeId: drag.nodeId,
      x: CANVAS_PAD + cellX * CELL_PX,
      y: CANVAS_PAD + cellY * CELL_PX,
    });
    const current = this.session();
    if (!current) {
      this.dragPreview.set(null);
      return;
    }
    const next = await applyNodeGridMove(current, this.transport, drag.nodeId, cellX, cellY);
    this.dragPreview.set(null);
    this.applySession(next, next.textFailure ?? 'Snapped to the grid');
  }

  private async finishLink(): Promise<void> {
    const link = this.linkDrag();
    const armed = this.armedPort();
    this.cancelLink();
    const current = this.session();
    if (!link || !armed || !current) {
      return;
    }
    const next = await applyConnectEdge(current, this.transport, link.fromNodeId, armed.nodeId);
    this.applySession(next, next.textFailure ?? 'Edge connected');
  }

  private async spawnNodeAt(fromId: string, point: { x: number; y: number }): Promise<void> {
    const current = this.session();
    if (!current?.manifest) {
      return;
    }
    const id = `node-${current.manifest.nodes.length + 1}`;
    const next = await applyNodeWithEdge(
      current,
      this.transport,
      {
        id,
        label: id,
        type: 'process',
        size: 'medium',
        gridPosition: {
          x: snapToCell(point.x, CANVAS_PAD, CELL_PX),
          y: snapToCell(point.y, CANVAS_PAD, CELL_PX),
        },
      },
      fromId,
    );
    if (!next.textFailure) {
      this.selectedNodeId.set(id);
    }
    this.applySession(next, next.textFailure ?? 'Node added on the grid and connected');
  }

  private cancelLink(): void {
    this.linkLive = false;
    this.linkDrag.set(null);
    this.armedPort.set(null);
  }

  private applySession(next: EditorSession, message: string): void {
    this.session.set(next);
    this.statusMessage.set(message);
  }

  private nodeOrigin(node: ManifestNode): { x: number; y: number } {
    const preview = this.dragPreview();
    if (preview?.nodeId === node.id) {
      return { x: preview.x, y: preview.y };
    }
    return this.gridOrigin(node.id, node);
  }

  private gridOrigin(nodeId: string, node?: ManifestNode): { x: number; y: number } {
    const resolved = node ?? this.session()?.manifest?.nodes.find((item) => item.id === nodeId);
    const x = resolved?.gridPosition.x ?? 0;
    const y = resolved?.gridPosition.y ?? 0;
    return { x: CANVAS_PAD + x * CELL_PX, y: CANVAS_PAD + y * CELL_PX };
  }

  private nodeCenter(node: ManifestNode): { x: number; y: number } {
    const origin = this.nodeOrigin(node);
    const box = this.nodeBox(node);
    return { x: origin.x + box.width / 2, y: origin.y + box.height / 2 };
  }

  private facingPort(node: ManifestNode, toward: { x: number; y: number }): { x: number; y: number } {
    const ports = this.portHits(node);
    return nearestPort(ports, toward.x, toward.y, '') ?? this.nodeCenter(node);
  }

  private portHits(node: ManifestNode): PortHit[] {
    const origin = this.nodeOrigin(node);
    const box = this.nodeBox(node);
    return PORT_SIDES.map((side) => {
      const offset = portOffset(side, box.width, box.height);
      return { nodeId: node.id, side, x: origin.x + offset.x, y: origin.y + offset.y };
    });
  }

  private allPorts(): PortHit[] {
    const nodes = this.session()?.manifest?.nodes ?? [];
    return nodes.flatMap((node) => this.portHits(node));
  }

  private pointOn(canvas: HTMLElement | null, event: { clientX: number; clientY: number }): { x: number; y: number } | null {
    if (!canvas) {
      return null;
    }
    const rect = canvas.getBoundingClientRect();
    return { x: event.clientX - rect.left, y: event.clientY - rect.top };
  }
}
