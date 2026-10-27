import { describe, expect, it } from 'vitest';
import {
  assertCase,
  chronological,
  diffFindings,
  evidenceEdges,
  filterFindings,
  INITIAL_FILTERS,
  timestamp,
} from './data';
import type { Case, Finding } from './model';
import demo from './fixtures/demo-case.json';

const caseData = assertCase(demo);
function finding(id: string, overrides: Partial<Finding> = {}): Finding {
  return {
    id,
    category: 'document',
    title: 'Archive notes',
    path: 'documents/archive.txt',
    origin: 'selected source',
    summary: 'Observed text document',
    locator: 'file',
    observed_at: null,
    bytes: 40,
    sha256: null,
    confidence: 'observed',
    rule_id: 'inventory.document',
    sensitive: false,
    evidence: [],
    related_ids: [],
    ...overrides,
  };
}
function source(findings: Finding[]): Case {
  return { ...caseData, findings };
}

describe('finding filters', () => {
  it('requires every search term, across path, title and rule, without changing source order', () => {
    const first = finding('first');
    const second = finding('second', { title: 'Other item', path: 'other.png', category: 'media' });
    const rows = [first, second];
    expect(
      filterFindings(rows, { ...INITIAL_FILTERS, query: '  ARCHIVE inventory.document  ' }),
    ).toEqual([first]);
    expect(filterFindings(rows, { ...INITIAL_FILTERS, query: 'archive missing' })).toEqual([]);
    expect(rows).toEqual([first, second]);
  });
  it('combines category, confidence and recorded-link filters', () => {
    const rows = [
      finding('a'),
      finding('b', { category: 'secret', confidence: 'candidate', related_ids: ['c'] }),
      finding('c', { category: 'secret', confidence: 'observed', related_ids: ['b'] }),
    ];
    expect(
      filterFindings(rows, {
        query: '',
        category: 'secret',
        confidence: 'candidate',
        linkedOnly: true,
      }).map((row) => row.id),
    ).toEqual(['b']);
    expect(
      filterFindings(rows, { ...INITIAL_FILTERS, linkedOnly: true }).map((row) => row.id),
    ).toEqual(['b', 'c']);
    expect(filterFindings(rows, { ...INITIAL_FILTERS, query: '   ' })).toHaveLength(3);
  });
});

describe('recorded evidence edges', () => {
  it('deduplicates reciprocal links and ignores dangling and self references', () => {
    expect(
      evidenceEdges([
        finding('a', { related_ids: ['a', 'b', 'b', 'missing'] }),
        finding('b', { related_ids: ['a'] }),
        finding('c'),
      ]),
    ).toEqual([['a', 'b']]);
  });
  it('does not create links from equal paths or titles', () => {
    expect(evidenceEdges([finding('a'), finding('b')])).toEqual([]);
  });
  it('keeps unusual imported identities distinct when deduplicating', () => {
    const rows = [
      finding('a', { related_ids: ['b\u0000c'] }),
      finding('b\u0000c'),
      finding('a\u0000b', { related_ids: ['c'] }),
      finding('c'),
    ];
    expect(evidenceEdges(rows)).toHaveLength(2);
  });
});

describe('source timestamps', () => {
  it('keeps the Unix epoch and rejects absent, nonfinite and unrepresentable dates', () => {
    expect(timestamp(0)).toBe('1970-01-01T00:00:00.000Z');
    for (const value of [null, Number.NaN, Number.POSITIVE_INFINITY, 1e20])
      expect(timestamp(value)).toBeNull();
  });
  it('orders only dated findings without fabricating a date or mutating the source', () => {
    const rows = [
      finding('later', { observed_at: 100 }),
      finding('missing'),
      finding('epoch', { observed_at: 0 }),
      finding('earlier', { observed_at: 50 }),
    ];
    expect(chronological(rows).map((row) => row.id)).toEqual(['later', 'earlier', 'epoch']);
    expect(chronological(rows, false).map((row) => row.id)).toEqual(['epoch', 'earlier', 'later']);
    expect(rows[0].id).toBe('later');
  });
});

describe('comparison evidence', () => {
  it('shows removed records from the baseline and changed records from the current case', () => {
    const before = source([
      finding('removed', { title: 'Baseline title' }),
      finding('changed', { title: 'Before' }),
    ]);
    const after = source([finding('changed', { title: 'After' }), finding('added')]);
    const diff = { added: ['added'], removed: ['removed'], changed: ['changed'], unchanged: [] };
    expect(diffFindings(diff, 'removed', before, after)[0].title).toBe('Baseline title');
    expect(diffFindings(diff, 'changed', before, after)[0].title).toBe('After');
    expect(diffFindings(diff, 'added', before, after)[0].id).toBe('added');
  });
  it('does not invent records when a comparison ID is missing', () => {
    expect(
      diffFindings(
        { added: ['missing'], removed: [], changed: [], unchanged: [] },
        'added',
        source([]),
        source([]),
      ),
    ).toEqual([]);
  });
});

describe('case boundary', () => {
  it('loads the real-engine synthetic fixture with explicit source provenance', () => {
    expect(caseData.source_kind).toBe('demo');
    expect(caseData.findings.length).toBeGreaterThan(0);
    expect(caseData.modules.length).toBeGreaterThan(0);
  });
  it('fails unsupported payloads instead of substituting a demonstration', () => {
    expect(() => assertCase(null)).toThrow('unsupported case');
    expect(() => assertCase({ ...caseData, schema_version: 99 })).toThrow('unsupported case');
  });
});
