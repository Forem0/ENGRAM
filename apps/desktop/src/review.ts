import type { Case, CaseReview, Finding, ReviewAnnotation, ReviewStatus } from './model';

export const REVIEW_LIMITS = Object.freeze({
  maxEntries: 2048,
  maxNoteBytes: 4096,
  maxTotalNoteBytes: 1024 * 1024,
});

export const DEFAULT_REVIEW: Readonly<ReviewAnnotation> = Object.freeze({
  bookmarked: false,
  status: 'unreviewed',
  note: '',
});

const statuses: readonly ReviewStatus[] = ['unreviewed', 'needs_review', 'reviewed'];
const encoder = new TextEncoder();

export function reviewKey(review: CaseReview | undefined): string {
  return JSON.stringify(
    Object.entries(review ?? {}).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)),
  );
}

function validIdentity(id: string): boolean {
  if (id.length === 0 || id.length > 256 || encoder.encode(id).length > 256) return false;
  for (const character of id) {
    const point = character.codePointAt(0)!;
    if (point < 0x20 || (point >= 0x7f && point <= 0x9f) || (point >= 0xd800 && point <= 0xdfff)) {
      return false;
    }
  }
  return true;
}

function record(value: unknown): value is Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const prototype: unknown = Object.getPrototypeOf(value);
  if (prototype !== null && prototype !== Object.prototype) return false;
  if (Object.getOwnPropertySymbols(value).length > 0) return false;
  return Object.values(Object.getOwnPropertyDescriptors(value)).every(
    (descriptor) => 'value' in descriptor && descriptor.enumerable,
  );
}

function annotation(value: unknown): { value: ReviewAnnotation; bytes: number } {
  if (
    !record(value) ||
    Object.keys(value).length !== 3 ||
    !Object.keys(value).every((key) => ['bookmarked', 'status', 'note'].includes(key)) ||
    typeof value.bookmarked !== 'boolean' ||
    typeof value.status !== 'string' ||
    !statuses.includes(value.status as ReviewStatus) ||
    typeof value.note !== 'string'
  ) {
    throw new Error('A review annotation must contain bookmarked, a valid status and a text note.');
  }
  // Every UTF-16 code unit needs at least one UTF-8 byte; reject oversized input before scanning it.
  if (value.note.length > REVIEW_LIMITS.maxNoteBytes) {
    throw new Error('A review note may contain at most 4096 UTF-8 bytes.');
  }
  for (const character of value.note) {
    const point = character.codePointAt(0)!;
    if ((point < 0x20 && point !== 0x09 && point !== 0x0a) || (point >= 0x7f && point <= 0x9f)) {
      throw new Error(
        'Review notes cannot contain control characters other than tabs and newlines.',
      );
    }
    if (point >= 0xd800 && point <= 0xdfff) {
      throw new Error('Review notes must contain valid Unicode text.');
    }
  }
  const bytes = encoder.encode(value.note).length;
  if (bytes > REVIEW_LIMITS.maxNoteBytes) {
    throw new Error('A review note may contain at most 4096 UTF-8 bytes.');
  }
  return {
    value: {
      bookmarked: value.bookmarked,
      status: value.status as ReviewStatus,
      note: value.note.trim().length === 0 ? '' : value.note,
    },
    bytes,
  };
}

/** Validate imported or edited annotations without mutating the supplied object. */
export function validateReview(review: unknown, findings: Finding[]): CaseReview {
  if (review === undefined) return {};
  if (!record(review)) throw new Error('Case review must be a map of finding IDs to annotations.');
  const entries = Object.entries(review);
  if (entries.length > REVIEW_LIMITS.maxEntries) {
    throw new Error('A case may contain at most 2048 review annotations.');
  }
  const ids = new Set(findings.map((finding) => finding.id));
  const normalized: [string, ReviewAnnotation][] = [];
  let totalBytes = 0;
  for (const [id, input] of entries) {
    if (!validIdentity(id)) {
      throw new Error(
        'Review finding IDs must be valid nonempty text of at most 256 UTF-8 bytes without control characters.',
      );
    }
    if (!ids.has(id)) throw new Error('A review annotation references an unknown finding ID.');
    const next = annotation(input);
    totalBytes += next.bytes;
    if (totalBytes > REVIEW_LIMITS.maxTotalNoteBytes) {
      throw new Error('Review notes in one case may contain at most 1 MiB of UTF-8 text.');
    }
    if (next.value.bookmarked || next.value.status !== 'unreviewed' || next.value.note !== '') {
      normalized.push([id, next.value]);
    }
  }
  // fromEntries treats IDs such as "__proto__" as data, rather than prototype setters.
  return Object.fromEntries(normalized);
}

/** Return an independent value so editing a form cannot mutate a saved case. */
export function getReview(review: CaseReview | undefined, id: string): ReviewAnnotation {
  const value = review && Object.prototype.hasOwnProperty.call(review, id) ? review[id] : undefined;
  return value ? { ...value } : { ...DEFAULT_REVIEW };
}

/** Replace one finding's annotation and return a new, fully validated case. */
export function updateReview(caseData: Case, findingId: string, input: ReviewAnnotation): Case {
  const previous = validateReview(caseData.review, caseData.findings);
  const replacement = validateReview(Object.fromEntries([[findingId, input]]), caseData.findings);
  const entries = Object.entries(previous).filter(([id]) => id !== findingId);
  if (Object.prototype.hasOwnProperty.call(replacement, findingId)) {
    entries.push([findingId, replacement[findingId]]);
  }
  const review = validateReview(Object.fromEntries(entries), caseData.findings);
  return { ...caseData, review };
}

export interface ReviewSummary {
  bookmarked: number;
  reviewed: number;
  needsReview: number;
  withNotes: number;
}

export function reviewSummary(caseData: Case): ReviewSummary {
  const review = validateReview(caseData.review, caseData.findings);
  const summary: ReviewSummary = { bookmarked: 0, reviewed: 0, needsReview: 0, withNotes: 0 };
  for (const value of Object.values(review)) {
    if (value.bookmarked) summary.bookmarked += 1;
    if (value.status === 'reviewed') summary.reviewed += 1;
    if (value.status === 'needs_review') summary.needsReview += 1;
    if (value.note !== '') summary.withNotes += 1;
  }
  return summary;
}
