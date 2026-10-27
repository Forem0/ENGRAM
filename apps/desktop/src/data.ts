import type { Case, CaseDiff, Category, Confidence, Finding } from './model';

export interface Filters {
  query: string;
  category: Category | 'all';
  confidence: Confidence | 'all';
  linkedOnly: boolean;
}
export const INITIAL_FILTERS: Filters = {
  query: '',
  category: 'all',
  confidence: 'all',
  linkedOnly: false,
};

export function filterFindings(findings: Finding[], filters: Filters): Finding[] {
  const terms = filters.query.trim().toLocaleLowerCase('en').split(/\s+/).filter(Boolean);
  return findings.filter((finding) => {
    if (filters.category !== 'all' && finding.category !== filters.category) return false;
    if (filters.confidence !== 'all' && finding.confidence !== filters.confidence) return false;
    if (filters.linkedOnly && finding.related_ids.length === 0) return false;
    if (terms.length === 0) return true;
    const searchable = [
      finding.title,
      finding.path,
      finding.origin,
      finding.rule_id,
      finding.locator,
      finding.summary,
      finding.category,
    ]
      .join(' ')
      .toLocaleLowerCase('en');
    return terms.every((term) => searchable.includes(term));
  });
}

export function evidenceEdges(findings: Finding[]): [string, string][] {
  const ids = new Set(findings.map((finding) => finding.id));
  const unique = new Map<string, [string, string]>();
  for (const finding of findings)
    for (const id of finding.related_ids) {
      if (id === finding.id || !ids.has(id)) continue;
      const pair: [string, string] = finding.id < id ? [finding.id, id] : [id, finding.id];
      unique.set(JSON.stringify(pair), pair);
    }
  return [...unique.values()];
}

export function timestamp(value: number | null): string | null {
  if (value === null || !Number.isFinite(value)) return null;
  const date = new Date(value * 1000);
  if (Number.isNaN(date.getTime())) return null;
  return date.toISOString();
}

export function dateLabel(value: number | null, withTime = true): string {
  const iso = timestamp(value);
  if (!iso) return 'Timestamp unavailable';
  return (
    new Intl.DateTimeFormat('en-GB', {
      dateStyle: 'medium',
      ...(withTime ? { timeStyle: 'short' as const } : {}),
      timeZone: 'UTC',
    }).format(new Date(iso)) + (withTime ? ' UTC' : '')
  );
}

export function chronological(findings: Finding[], newest = true): Finding[] {
  return findings
    .filter((finding) => timestamp(finding.observed_at) !== null)
    .sort((a, b) => {
      const difference = (a.observed_at ?? 0) - (b.observed_at ?? 0);
      return (newest ? -difference : difference) || a.id.localeCompare(b.id);
    });
}

export function bytesLabel(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return 'Unavailable';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KiB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MiB`;
  return `${(bytes / 1024 ** 3).toFixed(1)} GiB`;
}

export function sourceLabel(source: Case['source_kind']): string {
  return {
    demo: 'Synthetic demonstration',
    folder: 'Selected folder',
    ios_backup: 'iPhone backup',
  }[source];
}

export function caseIndex(caseData: Case): Map<string, Finding> {
  return new Map(caseData.findings.map((finding) => [finding.id, finding]));
}
export type Change = keyof CaseDiff;
export function diffFindings(diff: CaseDiff, change: Change, before: Case, after: Case): Finding[] {
  const index = caseIndex(change === 'removed' ? before : after);
  return diff[change].flatMap((id) => {
    const finding = index.get(id);
    return finding ? [finding] : [];
  });
}

export function assertCase(value: unknown): Case {
  if (
    !value ||
    typeof value !== 'object' ||
    !('schema_version' in value) ||
    value.schema_version !== 1 ||
    !('findings' in value) ||
    !Array.isArray(value.findings) ||
    !('stats' in value) ||
    !('modules' in value) ||
    !('warnings' in value)
  ) {
    throw new Error('The engine returned an unsupported case. Your previous case remains open.');
  }
  return value as Case;
}
