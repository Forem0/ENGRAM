import { beforeEach, describe, expect, it, vi } from 'vitest';
import demo from './fixtures/demo-case.json';
import type { Case } from './model';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke, isTauri: () => true }));
import { compareCase, loadDemo, openCase, saveCase, selectSource } from './bridge';

beforeEach(() => mocks.invoke.mockReset());

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
