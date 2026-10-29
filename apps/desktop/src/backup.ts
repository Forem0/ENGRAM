import type { BackupInspection } from './media-model';

function record(value: unknown): value is Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const prototype: unknown = Object.getPrototypeOf(value);
  return (
    (prototype === null || prototype === Object.prototype) &&
    Object.getOwnPropertySymbols(value).length === 0 &&
    Object.values(Object.getOwnPropertyDescriptors(value)).every(
      (descriptor) => 'value' in descriptor && descriptor.enumerable,
    )
  );
}

function label(value: unknown, maximum = 1024): value is string {
  return (
    typeof value === 'string' &&
    value.trim().length > 0 &&
    value.length <= maximum &&
    !/[\u0000-\u001f\u007f-\u009f]/.test(value)
  );
}

function count(value: unknown): value is number {
  return (
    typeof value === 'number' && Number.isSafeInteger(value) && value >= 0 && value <= 1_000_000
  );
}

function boundedArray(value: unknown, maximum: number): value is unknown[] {
  if (!Array.isArray(value) || value.length > maximum) return false;
  for (let index = 0; index < value.length; index += 1) {
    const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
    if (!descriptor || !('value' in descriptor) || !descriptor.enumerable) return false;
  }
  return true;
}

/** Metadata describes the selected index; it is not a successful analysis result. */
export function validateBackupInspection(input: unknown): BackupInspection {
  const failure = 'The desktop returned an invalid backup inspection. Choose the backup again.';
  if (
    !record(input) ||
    Object.keys(input).length !== 9 ||
    !Object.keys(input).every((key) =>
      [
        'token',
        'name',
        'state',
        'encrypted',
        'device_name',
        'product_version',
        'indexed_files',
        'categories',
        'warnings',
      ].includes(key),
    ) ||
    typeof input.token !== 'string' ||
    !/^[A-Za-z0-9_-]{16,256}$/.test(input.token) ||
    !label(input.name) ||
    !['ready', 'unavailable'].includes(input.state as string) ||
    (input.encrypted !== null && typeof input.encrypted !== 'boolean') ||
    (input.device_name !== null && !label(input.device_name)) ||
    (input.product_version !== null && !label(input.product_version, 256)) ||
    !count(input.indexed_files) ||
    !boundedArray(input.categories, 32) ||
    !boundedArray(input.warnings, 256) ||
    !input.warnings.every((warning) => label(warning, 16_384)) ||
    (input.state === 'ready' && input.encrypted !== false)
  )
    throw new Error(failure);

  const seen = new Set<string>();
  const categories = input.categories.map((category: unknown) => {
    if (
      !record(category) ||
      Object.keys(category).length !== 3 ||
      !Object.keys(category).every((key) => ['id', 'label', 'count'].includes(key)) ||
      !label(category.id, 64) ||
      !label(category.label, 256) ||
      !count(category.count) ||
      seen.has(category.id)
    )
      throw new Error(failure);
    seen.add(category.id);
    return { id: category.id, label: category.label, count: category.count };
  });
  return {
    token: input.token,
    name: input.name,
    state: input.state as BackupInspection['state'],
    encrypted: input.encrypted as BackupInspection['encrypted'],
    device_name: input.device_name as string | null,
    product_version: input.product_version as string | null,
    indexed_files: input.indexed_files,
    categories,
    warnings: [...input.warnings] as string[],
  };
}
