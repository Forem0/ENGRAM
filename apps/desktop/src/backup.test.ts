import { describe, expect, it } from 'vitest';
import { validateBackupInspection } from './backup';
import type { BackupInspection } from './media-model';

const inspection: BackupInspection = {
  token: 'backup-0123456789abcdef',
  name: 'Synthetic backup',
  state: 'ready',
  encrypted: false,
  device_name: 'Synthetic iPhone',
  product_version: 'test-fixture',
  indexed_files: 3,
  categories: [
    { id: 'media', label: 'Media paths', count: 2 },
    { id: 'database', label: 'Database paths', count: 1 },
  ],
  warnings: ['Index totals do not guarantee file readability.'],
};

describe('backup inspection boundary', () => {
  it('copies valid inspection metadata without exposing a source path', () => {
    const result = validateBackupInspection(inspection);
    expect(result).toEqual(inspection);
    expect(result).not.toBe(inspection);
    expect(result.categories).not.toBe(inspection.categories);
    expect(result.categories[0]).not.toBe(inspection.categories[0]);
    expect(result.warnings).not.toBe(inspection.warnings);
    expect(Object.keys(result)).not.toContain('path');
  });

  it('keeps encrypted, unknown and empty observations distinct', () => {
    expect(
      validateBackupInspection({ ...inspection, encrypted: true, state: 'unavailable' }).state,
    ).toBe('unavailable');
    expect(
      validateBackupInspection({
        ...inspection,
        encrypted: null,
        state: 'unavailable',
        indexed_files: 0,
        categories: [],
      }).encrypted,
    ).toBeNull();
    expect(
      validateBackupInspection({ ...inspection, indexed_files: 0, categories: [] }).indexed_files,
    ).toBe(0);
    expect(() => validateBackupInspection({ ...inspection, encrypted: true })).toThrow(
      'invalid backup inspection',
    );
    expect(() => validateBackupInspection({ ...inspection, encrypted: null })).toThrow(
      'invalid backup inspection',
    );
  });

  it('rejects malformed token authority, unknown states and unsupported payload shape', () => {
    for (const changes of [
      { token: '' },
      { token: '/private/source' },
      { token: 'token with spaces1234' },
      { state: 'scanned' },
      { source_root: '/private/backup' },
      { device_name: '' },
      { name: 'bad\u0000name' },
    ])
      expect(() => validateBackupInspection({ ...inspection, ...changes })).toThrow(
        'invalid backup inspection',
      );
  });

  it('rejects invented or unbounded counts and duplicate category identities', () => {
    for (const indexed_files of [-1, 1.5, Number.NaN, Number.POSITIVE_INFINITY, 1_000_001])
      expect(() => validateBackupInspection({ ...inspection, indexed_files })).toThrow(
        'invalid backup inspection',
      );
    expect(() =>
      validateBackupInspection({
        ...inspection,
        categories: [...inspection.categories, inspection.categories[0]],
      }),
    ).toThrow('invalid backup inspection');
    expect(() =>
      validateBackupInspection({
        ...inspection,
        categories: [{ id: 'media', label: 'Media paths', count: -1 }],
      }),
    ).toThrow('invalid backup inspection');
  });

  it('rejects sparse arrays and accessors instead of skipping or invoking them', () => {
    expect(() => validateBackupInspection({ ...inspection, categories: new Array(2) })).toThrow(
      'invalid backup inspection',
    );
    expect(() => validateBackupInspection({ ...inspection, warnings: new Array(2) })).toThrow(
      'invalid backup inspection',
    );
    const unsafe = { ...inspection };
    Object.defineProperty(unsafe, 'token', {
      enumerable: true,
      get() {
        throw new Error('getter executed');
      },
    });
    expect(() => validateBackupInspection(unsafe)).toThrow('invalid backup inspection');
  });
});
