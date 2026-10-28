import type { CaseReview, Finding, ReviewAnnotation } from './model';
import { getReview } from './review';

export const MAX_SEARCH_LENGTH = 2048;
export const MAX_SEARCH_TERMS = 128;

interface Token {
  value: string;
  negative: boolean;
  separator: number | null;
}

type Match = (finding: Finding, review: ReviewAnnotation, text: string) => boolean;

export interface CompiledSearch {
  error: string | null;
  terms: number;
  matches: (finding: Finding, review?: CaseReview) => boolean;
}

function tokenize(query: string): { tokens: Token[]; error: string | null } {
  if (query.length > MAX_SEARCH_LENGTH) {
    return { tokens: [], error: 'Search queries may contain at most 2048 characters.' };
  }
  const tokens: Token[] = [];
  let position = 0;
  while (position < query.length) {
    if (query[position].trim() === '') {
      position += 1;
      continue;
    }
    let negative = false;
    if (
      query[position] === '-' &&
      position + 1 < query.length &&
      query[position + 1].trim() !== ''
    ) {
      negative = true;
      position += 1;
    }
    let quoted = false;
    let value = '';
    let separator: number | null = null;
    while (position < query.length) {
      const character = query[position];
      if (character === '"') {
        quoted = !quoted;
        position += 1;
      } else if (
        quoted &&
        character === '\\' &&
        (query[position + 1] === '"' || query[position + 1] === '\\')
      ) {
        value += query[position + 1];
        position += 2;
      } else if (!quoted && character.trim() === '') {
        break;
      } else {
        if (!quoted && character === ':' && separator === null) separator = value.length;
        value += character;
        position += 1;
      }
    }
    if (quoted) return { tokens, error: 'Close the quoted phrase before searching.' };
    if (value.length === 0) return { tokens, error: 'A quoted search phrase cannot be empty.' };
    tokens.push({ value: value.toLocaleLowerCase('en'), negative, separator });
    if (tokens.length > MAX_SEARCH_TERMS) {
      return { tokens, error: 'Use at most 128 search terms in one query.' };
    }
  }
  return { tokens, error: null };
}

function matcher(token: Token): Match {
  const field = token.separator === null ? '' : token.value.slice(0, token.separator);
  const value = token.separator === null ? '' : token.value.slice(token.separator + 1);
  if (
    field === 'category' &&
    ['media', 'secret', 'document', 'browser', 'message', 'location', 'database'].includes(value)
  ) {
    return (finding) => finding.category === value;
  }
  if (field === 'confidence' && ['observed', 'linked', 'candidate'].includes(value)) {
    return (finding) => finding.confidence === value;
  }
  if (value !== '') {
    if (field === 'path') return (finding) => finding.path.toLocaleLowerCase('en').includes(value);
    if (field === 'rule')
      return (finding) => finding.rule_id.toLocaleLowerCase('en').includes(value);
    if (field === 'origin')
      return (finding) => finding.origin.toLocaleLowerCase('en').includes(value);
    if (field === 'note') return (_, review) => review.note.toLocaleLowerCase('en').includes(value);
    if (field === 'is') {
      if (value === 'bookmarked') return (_, review) => review.bookmarked;
      if (['reviewed', 'needs_review', 'unreviewed'].includes(value)) {
        return (_, review) => review.status === value;
      }
    }
    if (field === 'has' && value === 'note') return (_, review) => review.note.trim() !== '';
  }
  // Unknown operators and unsupported values remain literal text, including their colon.
  return (_, __, text) => text.includes(token.value);
}

/** Compile once per query. Matching uses literal substrings, never query-built regular expressions. */
export function compileSearch(query: string): CompiledSearch {
  const parsed = tokenize(query);
  const predicates = parsed.tokens.map((token) => ({
    test: matcher(token),
    negative: token.negative,
  }));
  return {
    error: parsed.error,
    terms: parsed.tokens.length,
    matches(finding, review) {
      if (parsed.error !== null) return false;
      if (predicates.length === 0) return true;
      const annotation = getReview(review, finding.id);
      const text = [
        finding.title,
        finding.path,
        finding.origin,
        finding.rule_id,
        finding.locator,
        finding.summary,
        finding.category,
        annotation.note,
      ]
        .join(' ')
        .toLocaleLowerCase('en');
      return predicates.every(({ test, negative }) => test(finding, annotation, text) !== negative);
    },
  };
}

export function describeSearch(query: string): { error: string | null; terms: number } {
  const { error, terms } = compileSearch(query);
  return { error, terms };
}
