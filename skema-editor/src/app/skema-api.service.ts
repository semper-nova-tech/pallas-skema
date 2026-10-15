import { Injectable, inject } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';
import type { SkemaTransport } from '../editor/skema-transport';

interface ApiResult {
  ok: boolean;
  textFailure?: string;
}

@Injectable({ providedIn: 'root' })
export class SkemaApiService implements SkemaTransport {
  private readonly http = inject(HttpClient);
  private readonly base = '/api/skema';

  validateManifestFile(manifestPath: string): Promise<{ ok: true } | { ok: false; textFailure: string }> {
    return this.post('validate', { manifestPath });
  }

  renderManifest(manifestPath: string, svgPath: string): Promise<{ ok: true } | { ok: false; textFailure: string }> {
    return this.post('render', { manifestPath, svgPath });
  }

  writeManifestFile(manifestPath: string, manifestJson: string): Promise<void> {
    return firstValueFrom(
      this.http.post<void>(`${this.base}/write-manifest`, { manifestPath, manifestJson }),
    );
  }

  writeTempManifest(manifestJson: string): Promise<string> {
    return firstValueFrom(
      this.http.post<{ path: string }>(`${this.base}/temp-manifest`, { manifestJson }),
    ).then((res) => res.path);
  }

  removeTempFile(path: string): Promise<void> {
    return firstValueFrom(this.http.post<void>(`${this.base}/remove-temp`, { path }));
  }

  private async post(
    action: string,
    body: Record<string, string>,
  ): Promise<{ ok: true } | { ok: false; textFailure: string }> {
    const result = await firstValueFrom(this.http.post<ApiResult>(`${this.base}/${action}`, body));
    if (result.ok) {
      return { ok: true };
    }
    return { ok: false, textFailure: result.textFailure ?? 'Validation failed' };
  }
}
