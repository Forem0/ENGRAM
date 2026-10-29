import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import demo from './fixtures/demo-case.json';
import type { Case, Finding, ReportOptions, ReportReceipt } from './model';
import { ReportExport } from './ReportExport';
import {
  REPORT_LIMITS,
  reportFindings,
  validateReportOptions,
  validateReportReceipt,
} from './report';

const caseData = demo as Case;
const first = caseData.findings[0];
const second = caseData.findings[1];
function options(overrides: Partial<ReportOptions> = {}): ReportOptions {
  return {
    format: 'markdown',
    finding_ids: [first.id],
    include_notes: false,
    include_paths: false,
    include_details: false,
    ...overrides,
  };
}
function receipt(overrides: Partial<ReportReceipt> = {}): ReportReceipt {
  return {
    path: 'exports/investigation.report.md',
    format: 'markdown',
    finding_count: 1,
    sha256: 'a'.repeat(64),
    bytes: 1234,
    ...overrides,
  };
}

describe('report selection and native boundary', () => {
  it('exports the entire filtered queue in case order, rather than its visible page', () => {
    const findings = Array.from({ length: 135 }, (_, index) => ({
      ...first,
      id: `trace-${index}`,
    }));
    const source = { ...caseData, findings };
    const result = reportFindings(source, [...findings].reverse(), 'filtered');
    expect(result).toEqual(findings);
    expect(
      validateReportOptions(options({ finding_ids: result.map((finding) => finding.id) }), findings)
        .finding_ids,
    ).toHaveLength(135);
    expect(reportFindings(source, [], 'filtered')).toEqual([]);
    expect(reportFindings(source, [], 'all')).toEqual(findings);
    expect(reportFindings(source, [], 'all')).not.toBe(findings);
  });
  it('selects current bookmarks across the case and does not fall back to all findings', () => {
    const reviewed = {
      ...caseData,
      review: {
        [second.id]: {
          bookmarked: true,
          status: 'needs_review' as const,
          note: 'Check this source',
        },
      },
    };
    expect(reportFindings(reviewed, [first], 'bookmarked')).toEqual([second]);
    expect(reportFindings(caseData, caseData.findings, 'bookmarked')).toEqual([]);
    expect(reportFindings(reviewed, [], 'filtered')).toEqual([]);
    expect(reviewed.review[second.id].note).toBe('Check this source');
  });
  it('rejects unknown or duplicate queue records instead of producing a misleading report scope', () => {
    expect(() => reportFindings(caseData, [{ ...first, id: 'unknown' }], 'filtered')).toThrow();
    expect(() => reportFindings(caseData, [first, first], 'filtered')).toThrow();
    expect(() => reportFindings(caseData, [first], 'other' as 'all')).toThrow();
  });
  it('preserves privacy flags and returns independent canonical options', () => {
    const input = options({ finding_ids: [second.id, first.id] });
    const result = validateReportOptions(input, caseData.findings);
    expect(result).toEqual(options({ finding_ids: [first.id, second.id] }));
    expect(result).not.toBe(input);
    expect(result.finding_ids).not.toBe(input.finding_ids);
    result.finding_ids.pop();
    expect(input.finding_ids).toHaveLength(2);
  });
  it('rejects empty/oversized selections without truncation', () => {
    expect(() => validateReportOptions(options({ finding_ids: [] }), caseData.findings)).toThrow(
      'at least one',
    );
    const findings = Array.from({ length: REPORT_LIMITS.maxFindings + 1 }, (_, index) => ({
      ...first,
      id: String(index),
    }));
    expect(() =>
      validateReportOptions(
        options({ finding_ids: findings.map((finding) => finding.id) }),
        findings,
      ),
    ).toThrow('10,000');
    expect(
      validateReportOptions(
        options({
          finding_ids: findings.slice(0, REPORT_LIMITS.maxFindings).map((finding) => finding.id),
        }),
        findings,
      ).finding_ids,
    ).toHaveLength(REPORT_LIMITS.maxFindings);
  });
  it('requires explicit booleans, an exact option shape and known unique bounded Unicode IDs', () => {
    for (const input of [
      { ...options(), format: 'html' },
      { ...options(), include_paths: 'false' },
      { ...options(), include_notes: undefined },
      { ...options(), source_root: '/private/source' },
      { ...options(), finding_ids: [first.id, first.id] },
      { ...options(), finding_ids: ['unknown'] },
      { ...options(), finding_ids: new Array(1) },
      Object.create(options()),
    ])
      expect(() => validateReportOptions(input, caseData.findings)).toThrow();
    for (const id of ['', 'x\n', 'x\u0085', '\ud800', 'x'.repeat(257), 'é'.repeat(129)]) {
      expect(() =>
        validateReportOptions(options({ finding_ids: [id] }), [{ ...first, id }]),
      ).toThrow();
    }
    expect(
      validateReportOptions(options({ finding_ids: ['é'.repeat(128)] }), [
        { ...first, id: 'é'.repeat(128) },
      ]).finding_ids,
    ).toHaveLength(1);
  });
  it('does not invoke option accessors while validating IPC input', () => {
    const input = options();
    Object.defineProperty(input, 'include_paths', {
      get: () => {
        throw new Error('accessor called');
      },
      enumerable: true,
    });
    expect(() => validateReportOptions(input, caseData.findings)).toThrow('Report options');
    const ids: string[] = [];
    Object.defineProperty(ids, '0', {
      get: () => {
        throw new Error('accessor called');
      },
      enumerable: true,
    });
    expect(() => validateReportOptions(options({ finding_ids: ids }), caseData.findings)).toThrow(
      'Report selection',
    );
  });
  it('accepts only a complete receipt matching requested format/count and bounded bytes', () => {
    expect(validateReportReceipt(receipt(), options())).toEqual(receipt());
    for (const input of [
      null,
      '',
      {},
      { ...receipt(), extra: true },
      receipt({ path: '\nunsafe' }),
      receipt({ path: ' ' }),
      receipt({ path: 'x'.repeat(32769) }),
      receipt({ format: 'csv' }),
      receipt({ finding_count: 0 }),
      receipt({ sha256: 'A'.repeat(64) }),
      receipt({ sha256: 'abc' }),
      receipt({ bytes: 0 }),
      receipt({ bytes: 1.2 }),
      receipt({ bytes: Number.NaN }),
      receipt({ bytes: REPORT_LIMITS.maxBytes + 1 }),
    ])
      expect(() => validateReportReceipt(input, options())).toThrow('invalid report receipt');
  });
});

describe('report configuration display', () => {
  function render(source: Case, visible: Finding[] = source.findings) {
    return renderToStaticMarkup(
      createElement(ReportExport, {
        caseData: source,
        visible,
        native: false,
        busy: false,
        unsavedReview: true,
        onExport: async () => null,
        onClose: () => undefined,
      }),
    );
  }
  it('shows independent disclosure choices off by default and a native-only file save', () => {
    const html = render(caseData);
    expect(html.match(/type="checkbox"/g) ?? []).toHaveLength(3);
    expect(html).not.toMatch(/type="checkbox"[^>]*checked/);
    expect(html).toContain('Writing a file requires the ENGRAM desktop application');
    expect(html).toContain('Notes are plaintext and are not automatically redacted');
    expect(html).toContain('Source text can contain personal data and paths');
    expect(html).toContain('Exporting does not save or replace the case file');
    expect(html).toMatch(/disabled=""[^>]*>Choose destination &amp; export/);
  });
  it('keeps source-provided titles as text and does not expose source root in the selection preview', () => {
    const unsafe = '<script>alert(1)</script>';
    const html = render({
      ...caseData,
      title: unsafe,
      source_root: '/private/my-backup',
      findings: [{ ...first, title: unsafe, path: '/private/my-backup/secrets.txt' }],
    });
    expect(html).not.toContain('<script>');
    expect(html).toContain('&lt;script&gt;alert(1)&lt;/script&gt;');
    expect(html).toContain('1 finding selected');
    expect(html).not.toContain('/private/my-backup');
  });
  it('shows an empty filtered selection instead of silently exporting the full case', () => {
    const html = render(caseData, []);
    expect(html).toContain('0 findings selected');
    expect(html).toContain('Select at least one finding to export');
  });
});
