import { describe, expect, it } from 'vitest';
import { assertCase } from './data';
import type { Case, ReviewAnnotation } from './model';
import fixture from './fixtures/demo-case.json';
import {
  DEFAULT_REVIEW,
  getReview,
  REVIEW_LIMITS,
  reviewKey,
  reviewSummary,
  updateReview,
  validateReview,
} from './review';

function source(ids: string[] = ['a', 'b', 'c']): Case {
  const template = fixture as unknown as Case;
  return {
    ...template,
    findings: ids.map((id) => ({ ...template.findings[0], id })),
  };
}

function annotation(overrides: Partial<ReviewAnnotation> = {}): ReviewAnnotation {
  return { bookmarked: false, status: 'unreviewed', note: '', ...overrides };
}

describe('review defaults and canonical validation', () => {
  it('recognizes reverted edits for distinct Unicode IDs regardless of map insertion order', () => {
    const original = {
      ...source(['é', 'e\u0301']),
      review: Object.fromEntries([
        ['é', annotation({ note: 'First observation' })],
        ['e\u0301', annotation({ note: 'Second observation' })],
      ]),
    };
    const edited = updateReview(
      original,
      'é',
      annotation({ note: 'First observation', bookmarked: true }),
    );
    expect(reviewKey(edited.review)).not.toBe(reviewKey(original.review));
    const reverted = updateReview(edited, 'é', annotation({ note: 'First observation' }));
    expect(reviewKey(reverted.review)).toBe(reviewKey(original.review));
  });

  it('returns independent defaults for cases with no review metadata', () => {
    expect(Object.isFrozen(DEFAULT_REVIEW)).toBe(true);
    const one = getReview(undefined, 'a');
    one.bookmarked = true;
    expect(getReview(undefined, 'a')).toEqual(annotation());
    expect(validateReview(undefined, source().findings)).toEqual({});
  });

  it('normalizes whitespace-only notes and drops fully default entries without mutating input', () => {
    const review = Object.freeze({
      a: Object.freeze(annotation({ note: ' \t\n ' })),
      b: Object.freeze(annotation({ bookmarked: true, note: '\t\n' })),
      c: Object.freeze(annotation({ note: ' Keep the context.\n' })),
    });
    expect(validateReview(review, source().findings)).toEqual({
      b: annotation({ bookmarked: true }),
      c: annotation({ note: ' Keep the context.\n' }),
    });
    expect(review.a.note).toBe(' \t\n ');
    expect(review.b.note).toBe('\t\n');
  });

  it('rejects a dangling finding even when its annotation would normalize to default', () => {
    expect(() => validateReview({ missing: annotation() }, source().findings)).toThrow(
      'unknown finding',
    );
  });

  it('rejects invalid known finding IDs before an edit can become an unsaveable review', () => {
    for (const id of [
      '',
      'x'.repeat(257),
      '🚀'.repeat(65),
      'id\u0000',
      'id\t',
      'id\n',
      'id\u007f',
      'id\u0085',
      'id\u009f',
      'id\ud800',
    ]) {
      const caseData = source([id]);
      expect(() =>
        validateReview(Object.fromEntries([[id, annotation()]]), caseData.findings),
      ).toThrow('finding IDs');
      expect(() => updateReview(caseData, id, annotation({ bookmarked: true }))).toThrow(
        'finding IDs',
      );
      expect(caseData.review).toBeUndefined();
    }
  });

  it('accepts IDs exactly at the UTF-8 limit and preserves prototype-like IDs', () => {
    for (const id of ['x'.repeat(256), '🚀'.repeat(64), '__proto__', 'constructor']) {
      const review = validateReview(
        Object.fromEntries([[id, annotation({ bookmarked: true })]]),
        source([id]).findings,
      );
      expect(getReview(review, id).bookmarked).toBe(true);
      expect(Object.getPrototypeOf(review)).toBe(Object.prototype);
    }
  });

  it('rejects invalid map shapes, annotation fields and review states', () => {
    for (const review of [null, [], true, 'review', new Date()]) {
      expect(() => validateReview(review, source().findings)).toThrow('map');
    }
    for (const value of [
      null,
      [],
      {},
      { bookmarked: false, status: 'pending', note: '' },
      { bookmarked: 'true', status: 'reviewed', note: '' },
      { bookmarked: true, status: 'reviewed', note: 12 },
      { ...annotation(), unsupported: true },
    ]) {
      expect(() => validateReview({ a: value }, source().findings)).toThrow('annotation');
    }
  });

  it('rejects accessor-bearing imported objects instead of executing a getter', () => {
    let called = false;
    const value = Object.defineProperty({}, 'note', {
      enumerable: true,
      get() {
        called = true;
        return '';
      },
    });
    expect(() => validateReview({ a: value }, source().findings)).toThrow('annotation');
    expect(called).toBe(false);
  });

  it('treats prototype-like finding IDs as own data properties', () => {
    const review = JSON.parse(
      '{"__proto__":{"bookmarked":true,"status":"reviewed","note":"Synthetic note"}}',
    ) as unknown;
    const normalized = validateReview(review, source(['__proto__']).findings);
    expect(Object.getPrototypeOf(normalized)).toBe(Object.prototype);
    expect(Object.prototype.hasOwnProperty.call(normalized, '__proto__')).toBe(true);
    expect(getReview(normalized, '__proto__').status).toBe('reviewed');
    expect(getReview({}, 'constructor')).toEqual(annotation());
  });
});

describe('review text and work limits', () => {
  it('permits tabs/newlines but rejects C0, DEL, C1 controls and invalid Unicode', () => {
    expect(
      validateReview({ a: annotation({ note: 'Line one\n\tLine two' }) }, source().findings).a.note,
    ).toBe('Line one\n\tLine two');
    for (const note of ['\u0000', '\r', '\u001b', '\u007f', '\u0085', '\u009f']) {
      expect(() => validateReview({ a: annotation({ note }) }, source().findings)).toThrow(
        'control',
      );
    }
    expect(() => validateReview({ a: annotation({ note: '\ud800' }) }, source().findings)).toThrow(
      'Unicode',
    );
  });

  it('counts UTF-8 bytes instead of JavaScript characters for per-note bounds', () => {
    const exact = '🚀'.repeat(REVIEW_LIMITS.maxNoteBytes / 4);
    expect(validateReview({ a: annotation({ note: exact }) }, source().findings).a.note).toBe(
      exact,
    );
    expect(() =>
      validateReview({ a: annotation({ note: `${exact}x` }) }, source().findings),
    ).toThrow('4096');
    expect(() =>
      validateReview(
        { a: annotation({ note: ' '.repeat(REVIEW_LIMITS.maxNoteBytes + 1) }) },
        source().findings,
      ),
    ).toThrow('4096');
  });

  it('enforces entry bounds before discarding default annotations', () => {
    const ids = Array.from({ length: REVIEW_LIMITS.maxEntries + 1 }, (_, index) => `id-${index}`);
    const review = Object.fromEntries(ids.map((id) => [id, annotation()]));
    expect(() => validateReview(review, source(ids).findings)).toThrow('2048');
    delete review[ids[ids.length - 1]];
    expect(validateReview(review, source(ids).findings)).toEqual({});
  });

  it('enforces the total note budget across otherwise valid entries', () => {
    const count = REVIEW_LIMITS.maxTotalNoteBytes / REVIEW_LIMITS.maxNoteBytes;
    const ids = Array.from({ length: count + 1 }, (_, index) => `id-${index}`);
    const note = 'x'.repeat(REVIEW_LIMITS.maxNoteBytes);
    const review = Object.fromEntries(ids.slice(0, count).map((id) => [id, annotation({ note })]));
    expect(Object.keys(validateReview(review, source(ids).findings))).toHaveLength(count);
    review[ids[count]] = annotation({ note: 'x' });
    expect(() => validateReview(review, source(ids).findings)).toThrow('1 MiB');
  });
});

describe('immutable case review updates', () => {
  it('updates one annotation without changing the case, caller annotation or other entries', () => {
    const before = source();
    before.review = { b: annotation({ status: 'reviewed', note: 'Existing context.' }) };
    const priorJson = JSON.stringify(before);
    const input = annotation({ bookmarked: true, note: 'New context.' });
    const after = updateReview(before, 'a', input);
    expect(after).not.toBe(before);
    expect(after.review).not.toBe(before.review);
    expect(after.review?.b).not.toBe(before.review.b);
    expect(after.review?.a).not.toBe(input);
    expect(after.findings).toBe(before.findings);
    expect(JSON.stringify(before)).toBe(priorJson);
    input.note = 'Changed form value';
    expect(after.review?.a.note).toBe('New context.');
  });

  it('removes a cleared annotation and rejects invalid edits without changing the original', () => {
    const before = source();
    before.review = { a: annotation({ bookmarked: true }) };
    expect(updateReview(before, 'a', annotation()).review).toEqual({});
    expect(before.review.a.bookmarked).toBe(true);
    expect(() => updateReview(before, 'missing', annotation())).toThrow('unknown finding');
    expect(() => updateReview(before, 'a', annotation({ note: '\u0000' }))).toThrow('control');
    expect(before.review.a.bookmarked).toBe(true);
  });

  it('allows replacing at capacity but rejects creating a new stored annotation', () => {
    const ids = Array.from({ length: REVIEW_LIMITS.maxEntries + 1 }, (_, index) => `id-${index}`);
    const before = source(ids);
    before.review = Object.fromEntries(
      ids.slice(0, -1).map((id) => [id, annotation({ bookmarked: true })]),
    );
    expect(
      updateReview(before, ids[0], annotation({ status: 'reviewed' })).review?.[ids[0]].status,
    ).toBe('reviewed');
    expect(() =>
      updateReview(before, ids[ids.length - 1], annotation({ bookmarked: true })),
    ).toThrow('2048');
    expect(
      Object.keys(updateReview(before, ids[ids.length - 1], annotation()).review ?? {}),
    ).toHaveLength(REVIEW_LIMITS.maxEntries);
  });

  it('summarizes only stored review facts with independent counts', () => {
    const caseData = source();
    caseData.review = {
      a: annotation({ bookmarked: true, status: 'reviewed', note: 'One note.' }),
      b: annotation({ status: 'needs_review', note: 'Two notes.' }),
    };
    expect(reviewSummary(caseData)).toEqual({
      bookmarked: 1,
      reviewed: 1,
      needsReview: 1,
      withNotes: 2,
    });
    expect(reviewSummary(source())).toEqual({
      bookmarked: 0,
      reviewed: 0,
      needsReview: 0,
      withNotes: 0,
    });
  });
});

describe('case import review boundary', () => {
  it('keeps old fixtures unchanged and canonicalizes review metadata in a new object', () => {
    const caseData = source();
    expect(assertCase(caseData)).toBe(caseData);
    const annotated = {
      ...caseData,
      review: { a: annotation({ bookmarked: true }), b: annotation() },
    };
    const imported = assertCase(annotated);
    expect(imported).not.toBe(annotated);
    expect(imported.review).toEqual({ a: annotation({ bookmarked: true }) });
    expect(Object.keys(annotated.review)).toEqual(['a', 'b']);
  });

  it('rejects imported dangling annotations rather than silently dropping them', () => {
    expect(() =>
      assertCase({ ...source(), review: { missing: annotation({ bookmarked: true }) } }),
    ).toThrow('unknown finding');
  });
});
