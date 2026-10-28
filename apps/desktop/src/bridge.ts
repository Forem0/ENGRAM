import { invoke, isTauri } from '@tauri-apps/api/core';
import { assertCase } from './data';
import type { Case, Comparison, RuntimeInfo } from './model';

export const native = isTauri();

export async function runtimeInfo(): Promise<RuntimeInfo> {
  if (native) return invoke<RuntimeInfo>('get_runtime_info');
  return {
    version: '0.3.0',
    engine: 'Browser demo · Rust-generated synthetic fixture',
    schema_version: 1,
    initial_view: new URLSearchParams(location.search).get('view') ?? 'map',
    smoke_test: false,
  };
}

export async function loadDemo(): Promise<Case> {
  if (native) return assertCase(await invoke<Case>('load_demo'));
  const fixture = await import('./fixtures/demo-case.json');
  const caseData = assertCase(fixture.default);
  if (caseData.source_kind !== 'demo')
    throw new Error('Browser demonstrations accept only a synthetic case.');
  return caseData;
}

function requireNative(): void {
  if (!native)
    throw new Error(
      'Choose and inspect local sources in the ENGRAM desktop application. The browser demo uses synthetic data only.',
    );
}

export async function selectSource(): Promise<Case | null> {
  requireNative();
  const result = await invoke<Case | null>('select_source');
  return result === null ? null : assertCase(result);
}
export async function openCase(): Promise<Case | null> {
  requireNative();
  const result = await invoke<Case | null>('open_case');
  return result === null ? null : assertCase(result);
}
export async function saveCase(): Promise<string | null> {
  requireNative();
  return invoke<string | null>('save_case');
}
export async function compareCase(): Promise<Comparison | null> {
  requireNative();
  const result = await invoke<Comparison | null>('compare_case');
  if (result !== null) assertCase(result.before);
  return result;
}
export async function finishSmokeTest(): Promise<void> {
  if (native) await invoke('finish_smoke_test');
}
