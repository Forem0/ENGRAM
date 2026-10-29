import { beforeEach, describe, expect, it, vi } from 'vitest';
import demo from './fixtures/demo-case.json';
import type { Case } from './model';

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke, isTauri: () => false }));
import { exportReport } from './bridge';
beforeEach(() => mocks.invoke.mockReset());
describe('browser report boundary', () => {
  it('does not open a picker, save or invoke native commands from the browser demonstration', async () => {
    await expect(
      exportReport(demo as Case, {
        format: 'csv',
        finding_ids: [demo.findings[0].id],
        include_notes: false,
        include_paths: false,
        include_details: false,
      }),
    ).rejects.toThrow('desktop application');
    expect(mocks.invoke).not.toHaveBeenCalled();
  });
});
