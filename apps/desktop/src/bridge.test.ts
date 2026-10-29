import { beforeEach, describe, expect, it, vi } from 'vitest';
import demo from './fixtures/demo-case.json';
import type { Case, ReportOptions } from './model';
import { reviewKey } from './review';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke, isTauri: () => true }));
import {
  cancelBackup,
  compareCase,
  exportReport,
  loadDemo,
  openCase,
  previewMedia,
  saveCase,
  scanBackup,
  selectBackup,
  selectSource,
} from './bridge';

beforeEach(() => mocks.invoke.mockReset());

describe('media and backup native boundaries', () => {
  const inspection = {
    token: 'backup-0000000000000001',
    name: 'Synthetic backup',
    state: 'ready',
    encrypted: false,
    device_name: 'Synthetic fixture',
    product_version: null,
    indexed_files: 7,
    categories: [{ id: 'media', label: 'Media paths', count: 3 }],
    warnings: ['Index counts describe paths, not readable findings.'],
  };
  it('stages a picker selection without scanning and sends only its opaque token later', async () => {
    mocks.invoke.mockResolvedValueOnce(inspection).mockResolvedValueOnce(demo);
    expect(await selectBackup()).toEqual(inspection);
    expect(mocks.invoke.mock.calls).toEqual([['select_backup']]);
    expect(await scanBackup(inspection.token)).toEqual(demo);
    expect(mocks.invoke.mock.calls[1]).toEqual(['scan_backup', { token: inspection.token }]);
  });
  it('keeps picker cancellation and staged selection cancellation explicit', async () => {
    mocks.invoke.mockResolvedValueOnce(null).mockResolvedValueOnce(undefined);
    expect(await selectBackup()).toBeNull();
    await cancelBackup(inspection.token);
    expect(mocks.invoke.mock.calls).toEqual([
      ['select_backup'],
      ['cancel_backup', { token: inspection.token }],
    ]);
  });
  it('rejects malformed backup metadata and does not convert it into a ready source', async () => {
    mocks.invoke.mockResolvedValueOnce({ ...inspection, encrypted: true });
    await expect(selectBackup()).rejects.toThrow('invalid backup inspection');
    await expect(scanBackup('')).rejects.toThrow('Select a backup again');
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
  });
  it('passes only known media and case identities, never paths or renderer-owned bytes', async () => {
    const finding = (demo as Case).findings.find((item) => item.rule_id === 'file.media')!;
    mocks.invoke.mockRejectedValueOnce('Image bytes are not attached to this case.');
    await expect(previewMedia(demo as Case, finding.id)).rejects.toBe(
      'Image bytes are not attached to this case.',
    );
    expect(mocks.invoke.mock.calls).toEqual([
      ['preview_media', { caseId: demo.id, findingId: finding.id }],
    ]);
  });
  it('does not invoke image reading for unknown or non-media findings', async () => {
    await expect(previewMedia(demo as Case, 'unknown')).rejects.toThrow('recorded media file');
    const document = (demo as Case).findings.find((item) => item.category !== 'media')!;
    await expect(previewMedia(demo as Case, document.id)).rejects.toThrow('recorded media file');
    expect(mocks.invoke).not.toHaveBeenCalled();
  });
});

describe('native command boundary', () => {
  it('uses Rust-owned pickers without arbitrary frontend source or destination arguments', async () => {
    mocks.invoke.mockResolvedValue(null);
    expect(await selectSource()).toBeNull();
    expect(await openCase()).toBeNull();
    expect(await saveCase(demo as Case)).toBeNull();
    expect(mocks.invoke.mock.calls).toEqual([
      ['select_source'],
      ['open_case'],
      ['save_case', { caseId: demo.id, review: {} }],
    ]);
  });
  it('keeps cancellation distinct from an empty success', async () => {
    mocks.invoke.mockResolvedValueOnce(null).mockResolvedValueOnce(demo);
    expect(await openCase()).toBeNull();
    expect(await loadDemo()).toEqual(demo);
  });
  it('propagates engine failures and never silently falls back to synthetic data', async () => {
    mocks.invoke.mockRejectedValueOnce('Selected backup is encrypted');
    await expect(selectSource()).rejects.toBe('Selected backup is encrypted');
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(mocks.invoke).toHaveBeenCalledWith('select_source');
  });
  it('sends only the case identity and review annotations to persistence', async () => {
    mocks.invoke.mockResolvedValueOnce('cases/new.engram.json').mockResolvedValueOnce({
      before: demo,
      diff: { added: [], removed: [], changed: [], unchanged: [] },
    });
    expect(await saveCase(demo as Case)).toBe('cases/new.engram.json');
    expect((await compareCase())?.before.source_kind).toBe('demo');
    expect(mocks.invoke.mock.calls).toEqual([
      ['save_case', { caseId: demo.id, review: {} }],
      ['compare_case'],
    ]);
  });
  it('rejects malformed annotations before invoking a native picker', async () => {
    await expect(
      saveCase({
        ...demo,
        review: { unknown: { bookmarked: true, status: 'reviewed', note: 'Check duplicate' } },
      } as Case),
    ).rejects.toThrow();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });
});

describe('native report export boundary', () => {
  const options: ReportOptions = {
    format: 'markdown',
    finding_ids: [demo.findings[0].id],
    include_notes: false,
    include_paths: false,
    include_details: false,
  };
  const report = {
    path: 'exports/report.md',
    format: 'markdown',
    finding_count: 1,
    sha256: 'b'.repeat(64),
    bytes: 900,
  };
  it('sends only the current identity, validated review and selected IDs/options', async () => {
    mocks.invoke.mockResolvedValueOnce(report);
    expect(await exportReport(demo as Case, options)).toEqual(report);
    expect(mocks.invoke).toHaveBeenCalledWith('export_report', {
      caseId: demo.id,
      review: {},
      options,
    });
    const payload = mocks.invoke.mock.calls[0][1];
    expect(payload).not.toHaveProperty('findings');
    expect(payload).not.toHaveProperty('source_root');
    expect(payload).not.toHaveProperty('path');
    expect(payload).not.toHaveProperty('content');
  });
  it('retains current unsaved review for successful, cancelled and failed exports', async () => {
    const source = {
      ...demo,
      review: {
        [demo.findings[0].id]: {
          bookmarked: true,
          status: 'needs_review' as const,
          note: 'My unsaved note',
        },
      },
    } as Case;
    const before = reviewKey(source.review);
    mocks.invoke
      .mockResolvedValueOnce(report)
      .mockResolvedValueOnce(null)
      .mockRejectedValueOnce('Destination already exists');
    expect(await exportReport(source, options)).toEqual(report);
    expect(await exportReport(source, options)).toBeNull();
    await expect(exportReport(source, options)).rejects.toBe('Destination already exists');
    expect(reviewKey(source.review)).toBe(before);
    expect(source.review?.[demo.findings[0].id].note).toBe('My unsaved note');
  });
  it('rejects invalid selections/review before invoking the native command', async () => {
    await expect(exportReport(demo as Case, { ...options, finding_ids: [] })).rejects.toThrow(
      'at least one',
    );
    await expect(
      exportReport(demo as Case, { ...options, finding_ids: ['not-in-the-case'] }),
    ).rejects.toThrow();
    await expect(
      exportReport(
        {
          ...demo,
          review: { unknown: { bookmarked: true, status: 'reviewed', note: '' } },
        } as Case,
        options,
      ),
    ).rejects.toThrow();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });
  it('does not mistake malformed native success for a completed export', async () => {
    mocks.invoke
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({ ...report, finding_count: 12 });
    await expect(exportReport(demo as Case, options)).rejects.toThrow('invalid report receipt');
    await expect(exportReport(demo as Case, options)).rejects.toThrow('invalid report receipt');
  });
});
