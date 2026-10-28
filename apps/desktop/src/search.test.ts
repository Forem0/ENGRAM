import { describe, expect, it } from 'vitest';
import { filterFindings, INITIAL_FILTERS } from './data';
import type { CaseReview, Finding } from './model';
import { compileSearch, describeSearch, MAX_SEARCH_LENGTH, MAX_SEARCH_TERMS } from './search';

function finding(id = 'a', overrides: Partial<Finding> = {}): Finding {
  return {
    id,
    category: 'document',
    title: 'Old archive notes',
    path: 'exports/old photos/notes.txt',
    origin: 'Synthetic folder',
    summary: 'Observed text document',
    locator: 'File',
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

const review: CaseReview = {
  a: { bookmarked: true, status: 'needs_review', note: 'Follow up with the owner.' },
  b: { bookmarked: false, status: 'reviewed', note: '' },
};

function matches(query: string, row = finding(), annotations?: CaseReview): boolean {
  return compileSearch(query).matches(row, annotations);
}

describe('advanced finding search', () => {
  it('preserves case-insensitive plain AND search across existing finding fields', () => {
    expect(matches('  ARCHIVE inventory.document  ')).toBe(true);
    expect(matches('archive missing')).toBe(false);
    expect(matches('')).toBe(true);
    expect(matches(' \t\n')).toBe(true);
  });

  it('requires quoted words to be contiguous and combines them with other terms', () => {
    expect(matches('"old photos" notes')).toBe(true);
    expect(matches('"photos old"')).toBe(false);
    expect(matches('"old archive" "text document"')).toBe(true);
    expect(matches('"archive text"')).toBe(false);
  });

  it('excludes literal terms and phrases without changing the other AND terms', () => {
    expect(matches('archive -missing')).toBe(true);
    expect(matches('archive -notes')).toBe(false);
    expect(matches('archive -"old photos"')).toBe(false);
    expect(matches('-"photos old"')).toBe(true);
  });

  it('matches enum fields exactly and path/rule/origin fields as literal substrings', () => {
    expect(
      matches(
        'category:document confidence:observed path:"old photos" rule:inventory origin:synthetic',
      ),
    ).toBe(true);
    expect(matches('category:media')).toBe(false);
    expect(matches('confidence:candidate')).toBe(false);
    expect(matches('path:archive')).toBe(false);
    expect(matches('rule:document -origin:real')).toBe(true);
    expect(matches('CATEGORY:DOCUMENT CONFIDENCE:OBSERVED')).toBe(true);
  });

  it('can exclude recognized fields', () => {
    expect(matches('-category:secret -confidence:candidate')).toBe(true);
    expect(matches('-category:document')).toBe(false);
    expect(matches('-path:"old photos"')).toBe(false);
  });

  it('matches annotation fields and uses unreviewed defaults for absent entries', () => {
    expect(matches('is:bookmarked is:needs_review has:note', finding('a'), review)).toBe(true);
    expect(matches('is:reviewed -is:bookmarked -has:note', finding('b'), review)).toBe(true);
    expect(matches('is:unreviewed -has:note', finding('missing'), review)).toBe(true);
    expect(matches('is:bookmarked', finding('missing'), review)).toBe(false);
    expect(matches('is:reviewed', finding('a'), review)).toBe(false);
    expect(matches('is:unreviewed')).toBe(true);
  });

  it('does not add unrelated evidence values to broad search', () => {
    expect(
      matches(
        'not-in-the-search-index',
        finding('a', {
          evidence: [{ label: 'Synthetic detail', value: 'not-in-the-search-index' }],
        }),
      ),
    ).toBe(false);
  });

  it('searches reviewer notes through a case-insensitive scoped field and quoted phrase', () => {
    expect(matches('note:"FOLLOW UP" category:document', finding('a'), review)).toBe(true);
    expect(matches('note:archive', finding('a'), review)).toBe(false);
    expect(matches('note:owner', finding('b'), review)).toBe(false);
    expect(matches('note:owner', finding('missing'), review)).toBe(false);
  });

  it('includes notes in general AND search and supports phrase or scoped-note exclusions', () => {
    expect(matches('archive "follow up"', finding('a'), review)).toBe(true);
    expect(matches('archive -"follow up"', finding('a'), review)).toBe(false);
    expect(matches('archive -note:"the owner"', finding('a'), review)).toBe(false);
    expect(matches('archive -note:missing', finding('a'), review)).toBe(true);
    expect(matches('archive -note:owner', finding('b'), review)).toBe(true);
  });

  it('retains unknown operators and unsupported operator values as literal text', () => {
    const row = finding('a', {
      summary: 'owner:alice category:unknown has:attachment is:pending path:',
    });
    expect(matches('owner:alice', row)).toBe(true);
    expect(matches('owner:bob', row)).toBe(false);
    expect(matches('category:unknown has:attachment is:pending path:', row)).toBe(true);
    expect(matches('category:unknown', finding())).toBe(false);
    expect(matches('-owner:alice', row)).toBe(false);
    expect(describeSearch('owner:alice category:unknown').error).toBeNull();
  });

  it('treats an explicitly quoted operator as literal text', () => {
    expect(matches('"category:document"')).toBe(false);
    expect(matches('"category:document"', finding('a', { summary: 'category:document' }))).toBe(
      true,
    );
  });

  it('supports escaped quotes and literal Windows path separators inside phrases', () => {
    const row = finding('a', { title: 'A "quoted" file', path: 'C:\\exports\\old photos\\a.txt' });
    expect(matches('"A \\"quoted\\" file"', row)).toBe(true);
    expect(matches('path:"C:\\exports\\old photos"', row)).toBe(true);
  });

  it('matches regex-looking text literally without executing it', () => {
    const row = finding('a', { title: '(a+)+$ [x] .*' });
    expect(matches('(a+)+$ [x] .*', row)).toBe(true);
    expect(matches('(a+)+$', finding('a', { title: 'aaaaaaaaaa' }))).toBe(false);
  });

  it('treats a standalone hyphen as a literal term', () => {
    expect(matches('-')).toBe(false);
    expect(matches('-', finding('a', { title: 'old-file' }))).toBe(true);
  });
});

describe('search diagnostics and bounds', () => {
  it('reports unmatched and empty quotes without broadening a malformed query', () => {
    for (const query of ['"old photos', 'path:"old photos', '""', '-""']) {
      expect(describeSearch(query).error).not.toBeNull();
      expect(matches(query)).toBe(false);
      expect(filterFindings([finding()], { ...INITIAL_FILTERS, query })).toEqual([]);
    }
    expect(describeSearch('"old photos" -category:secret')).toEqual({ error: null, terms: 2 });
  });

  it('bounds the query before tokenization and separately limits term count', () => {
    expect(describeSearch('a'.repeat(MAX_SEARCH_LENGTH)).error).toBeNull();
    expect(describeSearch('a'.repeat(MAX_SEARCH_LENGTH + 1)).error).toContain('2048');
    expect(describeSearch(Array(MAX_SEARCH_TERMS).fill('a').join(' ')).error).toBeNull();
    expect(
      describeSearch(
        Array(MAX_SEARCH_TERMS + 1)
          .fill('a')
          .join(' '),
      ).error,
    ).toContain('128');
  });
});

describe('review-aware finding filters', () => {
  it('combines existing filters, review controls and the compiled query without reordering input', () => {
    const rows = [finding('b'), finding('a', { related_ids: ['b'] }), finding('c')];
    const result = filterFindings(
      rows,
      {
        ...INITIAL_FILTERS,
        query: 'path:exports -confidence:candidate has:note',
        category: 'document',
        confidence: 'observed',
        linkedOnly: true,
        bookmarkedOnly: true,
        reviewStatus: 'needs_review',
      },
      review,
    );
    expect(result.map((row) => row.id)).toEqual(['a']);
    expect(rows.map((row) => row.id)).toEqual(['b', 'a', 'c']);
  });

  it('keeps callers without review options or a review map backwards compatible', () => {
    const rows = [finding('a'), finding('b')];
    expect(
      filterFindings(rows, {
        query: 'archive',
        category: 'all',
        confidence: 'all',
        linkedOnly: false,
      }),
    ).toEqual(rows);
    expect(filterFindings(rows, { ...INITIAL_FILTERS, reviewStatus: 'unreviewed' })).toEqual(rows);
    expect(filterFindings(rows, { ...INITIAL_FILTERS, bookmarkedOnly: true })).toEqual([]);
  });
});
