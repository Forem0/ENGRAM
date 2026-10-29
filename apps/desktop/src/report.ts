import type { Case, Finding, ReportOptions, ReportReceipt } from './model';
import { getReview } from './review';

export const REPORT_LIMITS = Object.freeze({ maxFindings: 10_000, maxBytes: 16 * 1024 * 1024 });
export type ReportScope = 'filtered' | 'all' | 'bookmarked';
const encoder = new TextEncoder();

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

function plainText(value: unknown, maxBytes: number): value is string {
  if (typeof value !== 'string' || !value || value.length > maxBytes) return false;
  for (const character of value) {
    const point = character.codePointAt(0)!;
    if (point < 0x20 || (point >= 0x7f && point <= 0x9f) || (point >= 0xd800 && point <= 0xdfff)) {
      return false;
    }
  }
  return encoder.encode(value).length <= maxBytes;
}

/** Select from the complete filtered queue, never just its visible table page. */
export function reportFindings(caseData: Case, visible: Finding[], scope: ReportScope): Finding[] {
  if (scope === 'all') return [...caseData.findings];
  if (scope === 'bookmarked') {
    return caseData.findings.filter((finding) => getReview(caseData.review, finding.id).bookmarked);
  }
  if (scope !== 'filtered') throw new Error('Choose a valid report scope.');
  const known = new Set(caseData.findings.map((finding) => finding.id));
  const selected = new Set<string>();
  for (const finding of visible) {
    if (!known.has(finding.id) || selected.has(finding.id)) {
      throw new Error('Report selection must contain unique findings from the current case.');
    }
    selected.add(finding.id);
  }
  return caseData.findings.filter((finding) => selected.has(finding.id));
}

/** Validate and copy IPC options; source evidence and paths never cross this boundary. */
export function validateReportOptions(input: unknown, findings: Finding[]): ReportOptions {
  if (
    !record(input) ||
    Object.keys(input).length !== 5 ||
    !Object.keys(input).every((key) =>
      ['format', 'finding_ids', 'include_notes', 'include_paths', 'include_details'].includes(key),
    ) ||
    !['markdown', 'csv'].includes(input.format as string) ||
    typeof input.include_notes !== 'boolean' ||
    typeof input.include_paths !== 'boolean' ||
    typeof input.include_details !== 'boolean' ||
    !Array.isArray(input.finding_ids)
  ) {
    throw new Error(
      'Report options must contain a format, finding IDs and explicit privacy choices.',
    );
  }
  const raw = input.finding_ids;
  if (raw.length === 0) throw new Error('Select at least one finding to export.');
  if (raw.length > REPORT_LIMITS.maxFindings) {
    throw new Error('A report can include at most 10,000 findings. Narrow the selection first.');
  }
  const known = new Set(findings.map((finding) => finding.id));
  const selected = new Set<string>();
  for (let index = 0; index < raw.length; index += 1) {
    const descriptor = Object.getOwnPropertyDescriptor(raw, String(index));
    const id: unknown = descriptor && 'value' in descriptor ? descriptor.value : undefined;
    if (!plainText(id, 256) || !known.has(id) || selected.has(id)) {
      throw new Error('Report selection must contain unique, valid IDs from the current case.');
    }
    selected.add(id);
  }
  return {
    format: input.format as ReportOptions['format'],
    finding_ids: findings
      .filter((finding) => selected.has(finding.id))
      .map((finding) => finding.id),
    include_notes: input.include_notes,
    include_paths: input.include_paths,
    include_details: input.include_details,
  };
}

/** A native success must describe exactly the report that was requested. */
export function validateReportReceipt(input: unknown, options: ReportOptions): ReportReceipt {
  if (
    !record(input) ||
    Object.keys(input).length !== 5 ||
    !Object.keys(input).every((key) =>
      ['path', 'format', 'finding_count', 'sha256', 'bytes'].includes(key),
    ) ||
    !plainText(input.path, 32_768) ||
    input.path.trim().length === 0 ||
    input.format !== options.format ||
    input.finding_count !== options.finding_ids.length ||
    typeof input.sha256 !== 'string' ||
    !/^[a-f0-9]{64}$/.test(input.sha256) ||
    typeof input.bytes !== 'number' ||
    !Number.isSafeInteger(input.bytes) ||
    input.bytes < 1 ||
    input.bytes > REPORT_LIMITS.maxBytes
  ) {
    throw new Error(
      'The desktop returned an invalid report receipt. Check the destination before retrying.',
    );
  }
  return {
    path: input.path,
    format: options.format,
    finding_count: options.finding_ids.length,
    sha256: input.sha256,
    bytes: input.bytes,
  };
}
