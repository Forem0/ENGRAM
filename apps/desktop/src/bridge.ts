import { invoke, isTauri } from '@tauri-apps/api/core';
import { assertCase } from './data';
import { validateReview } from './review';
import { validateReportOptions, validateReportReceipt } from './report';
import type { Case, Comparison, ReportOptions, ReportReceipt, RuntimeInfo } from './model';
import type { BackupInspection, MediaPreview } from './media-model';
import { validateBackupInspection } from './backup';
import { validateMediaPreview } from './media';

export const native = isTauri();

export async function runtimeInfo(): Promise<RuntimeInfo> {
  if (native) return invoke<RuntimeInfo>('get_runtime_info');
  return {
    version: '0.7.0',
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
export async function selectBackup(): Promise<BackupInspection | null> {
  requireNative();
  const result = await invoke<unknown>('select_backup');
  return result === null ? null : validateBackupInspection(result);
}
export async function scanBackup(token: string): Promise<Case> {
  requireNative();
  if (typeof token !== 'string' || !token || token.length > 256)
    throw new Error('Select a backup again before scanning.');
  return assertCase(await invoke<unknown>('scan_backup', { token }));
}
export async function cancelBackup(token: string): Promise<void> {
  requireNative();
  if (typeof token !== 'string' || !token || token.length > 256)
    throw new Error('Invalid backup selection.');
  await invoke('cancel_backup', { token });
}
export async function previewMedia(caseData: Case, findingId: string): Promise<MediaPreview> {
  requireNative();
  const finding = caseData.findings.find((item) => item.id === findingId);
  if (!finding || finding.category !== 'media' || finding.rule_id !== 'file.media')
    throw new Error('Choose a recorded media file to preview.');
  const result = await invoke<unknown>('preview_media', { caseId: caseData.id, findingId });
  return validateMediaPreview(result, finding);
}
export async function openCase(): Promise<Case | null> {
  requireNative();
  const result = await invoke<Case | null>('open_case');
  return result === null ? null : assertCase(result);
}
export async function saveCase(caseData: Case): Promise<string | null> {
  requireNative();
  const review = validateReview(caseData.review ?? {}, caseData.findings);
  return invoke<string | null>('save_case', { caseId: caseData.id, review });
}
export async function compareCase(): Promise<Comparison | null> {
  requireNative();
  const result = await invoke<Comparison | null>('compare_case');
  if (result !== null) assertCase(result.before);
  return result;
}
export async function exportReport(
  caseData: Case,
  input: ReportOptions,
): Promise<ReportReceipt | null> {
  requireNative();
  const options = validateReportOptions(input, caseData.findings);
  const review = validateReview(caseData.review ?? {}, caseData.findings);
  const result = await invoke<unknown>('export_report', { caseId: caseData.id, review, options });
  return result === null ? null : validateReportReceipt(result, options);
}
export async function finishSmokeTest(): Promise<void> {
  if (native) await invoke('finish_smoke_test');
}
