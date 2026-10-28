import type {
  Case,
  CaseDiff,
  CaseReview,
  Category,
  Confidence,
  Finding,
  ReviewStatus,
} from './model';
import { getReview, validateReview } from './review';
import { compileSearch } from './search';

export interface Filters {
  query: string;
  category: Category | 'all';
  confidence: Confidence | 'all';
  linkedOnly: boolean;
  bookmarkedOnly?: boolean;
  reviewStatus?: ReviewStatus | 'all';
}
export const INITIAL_FILTERS: Filters = {
  query: '',
  category: 'all',
  confidence: 'all',
  linkedOnly: false,
  bookmarkedOnly: false,
  reviewStatus: 'all',
};

export function filterFindings(
  findings: Finding[],
  filters: Filters,
  review?: CaseReview,
): Finding[] {
  const search = compileSearch(filters.query);
  if (search.error !== null) return [];
  return findings.filter((finding) => {
    if (filters.category !== 'all' && finding.category !== filters.category) return false;
    if (filters.confidence !== 'all' && finding.confidence !== filters.confidence) return false;
    if (filters.linkedOnly && finding.related_ids.length === 0) return false;
    const annotation = getReview(review, finding.id);
    if (filters.bookmarkedOnly && !annotation.bookmarked) return false;
    if (
      filters.reviewStatus &&
      filters.reviewStatus !== 'all' &&
      annotation.status !== filters.reviewStatus
    )
      return false;
    return search.matches(finding, review);
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
  const caseData = value as Case;
  if ('review' in caseData) {
    return { ...caseData, review: validateReview(caseData.review, caseData.findings) };
  }
  return caseData;
}
