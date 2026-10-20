import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import type { SkemaOutcome, SkemaTransport } from '../editor/skema-transport';

interface ApiResult {
  ok: boolean;
  textFailure?: string;
}

@Injectable({ providedIn: 'root' })
export class SkemaApiService implements SkemaTransport {
  private readonly http = inject(HttpClient);
  private readonly base = '/api/skema';

  validateManifestJson(manifestJson: string): Promise<SkemaOutcome> {
    return this.post('validate', { manifestJson });
  }

  saveManifest(manifestPath: string, svgPath: string, manifestJson: string): Promise<SkemaOutcome> {
    return this.post('save', { manifestPath, svgPath, manifestJson });
  }

  private async post(action: string, body: Record<string, string>): Promise<SkemaOutcome> {
    const result = await firstValueFrom(this.http.post<ApiResult>(`${this.base}/${action}`, body));
    if (result.ok) {
      return { ok: true };
    }
    return { ok: false, textFailure: result.textFailure ?? 'Validation failed' };
  }
}
