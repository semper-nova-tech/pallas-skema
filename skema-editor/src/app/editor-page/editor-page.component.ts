import { Component, OnInit, inject, signal } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import { MatButtonModule } from '@angular/material/button';
import { MatToolbarModule } from '@angular/material/toolbar';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import {
  addNodeAt,
  applyNodeGridMove,
  applyNodeLabelEdit,
  loadManifest,
  saveManifest,
  type EditorJsonHandoff,
  type EditorSession,
} from '../../editor';
import { SkemaApiService } from '../skema-api.service';

const CELL_PX = 48;

@Component({
  selector: 'app-editor-page',
  standalone: true,
  imports: [MatToolbarModule, MatButtonModule, MatFormFieldModule, MatInputModule],
  templateUrl: './editor-page.component.html',
  styleUrl: './editor-page.component.scss',
})
export class EditorPageComponent implements OnInit {
  private readonly http = inject(HttpClient);
  private readonly transport = inject(SkemaApiService);

  readonly session = signal<EditorSession | null>(null);
  readonly selectedNodeId = signal<string | null>(null);
  readonly statusMessage = signal<string>('Loading handoff…');

  ngOnInit(): void {
    void this.bootstrap();
  }

  async bootstrap(): Promise<void> {
    try {
      const handoff = await firstValueFrom(this.http.get<EditorJsonHandoff>('/handoff.json'));
      if (!handoff) {
        this.statusMessage.set('No handoff.json; run skema open-editor and copy handoff to public/handoff.json');
        return;
      }
      const loaded = await loadManifest(handoff, this.transport);
      this.session.set(loaded);
      this.statusMessage.set(`State: ${loaded.lifecycleState}`);
    } catch {
      this.statusMessage.set('Failed to load handoff.json');
    }
  }

  nodeStyle(node: { gridPosition: { x: number; y: number } }): Record<string, string> {
    return {
      left: `${node.gridPosition.x * CELL_PX}px`,
      top: `${node.gridPosition.y * CELL_PX}px`,
    };
  }

  selectNode(id: string): void {
    this.selectedNodeId.set(id);
  }

  async moveSelected(dx: number, dy: number): Promise<void> {
    const current = this.session();
    const nodeId = this.selectedNodeId();
    if (!current?.manifest || !nodeId) {
      return;
    }
    const node = current.manifest.nodes.find((n) => n.id === nodeId);
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
    this.session.set(next);
    this.statusMessage.set(next.textFailure ?? `State: ${next.lifecycleState}`);
  }

  onLabelInput(event: Event): void {
    const current = this.session();
    const nodeId = this.selectedNodeId();
    if (!current || !nodeId) {
      return;
    }
    const value = (event.target as HTMLInputElement).value;
    const next = applyNodeLabelEdit(current, nodeId, value);
    this.session.set(next);
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
    return current.manifest.nodes.find((n) => n.id === nodeId)?.label ?? '';
  }

  async onSave(): Promise<void> {
    const current = this.session();
    if (!current) {
      return;
    }
    const next = await saveManifest(current, this.transport);
    this.session.set(next);
    this.statusMessage.set(next.textFailure ?? 'Saved and rendered SVG');
  }
}
