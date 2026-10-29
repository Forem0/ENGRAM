import { describe, expect, it } from 'vitest';
import {
  mediaKind,
  mediaName,
  mediaRows,
  mediaUnmatched,
  selectedMedia,
  previewEligible,
  validateMediaPreview,
  MEDIA_PREVIEW_LIMITS,
} from './media';
import type { Finding } from './model';
import type { MediaPreview } from './media-model';
import demo from './fixtures/demo-case.json';

const png = btoa(
  String.fromCharCode(
    ...[
      137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4, 0,
      0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 248, 15, 0, 1, 5, 1, 1,
      39, 24, 227, 102, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ],
  ),
);
const finding: Finding = {
  id: 'media-observation',
  category: 'media',
  title: 'Media file: synthetic.PNG',
  path: 'media/synthetic.PNG',
  origin: 'Synthetic fixture',
  summary: 'Generated test image',
  locator: 'File',
  observed_at: null,
  bytes: 68,
  sha256: 'a'.repeat(64),
  confidence: 'observed',
  rule_id: 'file.media',
  sensitive: false,
  evidence: [],
  related_ids: [],
};
const preview: MediaPreview = {
  finding_id: finding.id,
  data_url: `data:image/png;base64,${png}`,
  width: 1,
  height: 1,
  source_width: 1,
  source_height: 1,
  source_sha256: finding.sha256!,
};

describe('media evidence grouping', () => {
  it('follows a catalogue relationship to the current case file while preserving filters and evidence', () => {
    const catalogue: Finding = {
      ...finding,
      id: 'catalogue-record',
      rule_id: 'media.photos_catalogue',
      path: 'PhotoData/Photos.sqlite',
      related_ids: [finding.id],
    };
    const document: Finding = { ...finding, id: 'document-record', category: 'document' };
    const findings = Object.freeze([finding, catalogue, document]);
    const sharedQueue = Object.freeze([catalogue]);
    const filter = 'catalogue' as const;
    const rows = mediaRows(sharedQueue, filter);
    const before = JSON.stringify({ findings, sharedQueue, rows, filter });

    expect(rows).toEqual([catalogue]);
    expect(rows).not.toContain(finding);
    const current = selectedMedia(findings, catalogue.related_ids[0]);
    expect(current).toBe(finding);
    expect(previewEligible(current!)).toBe(true);
    expect(selectedMedia(findings, 'unknown-record')).toBeNull();
    expect(selectedMedia(findings, document.id)).toBeNull();
    expect(selectedMedia(findings, null)).toBeNull();
    expect(JSON.stringify({ findings, sharedQueue, rows, filter })).toBe(before);
  });

  it('groups the actual Rust-generated fixture without manufacturing findings', () => {
    const rows = mediaRows(demo.findings as Finding[]);
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.every((row) => (demo.findings as Finding[]).includes(row))).toBe(true);
    expect(mediaRows(rows, 'thumbnails').length).toBeGreaterThan(0);
    expect(
      mediaRows(rows, 'catalogue').every((row) => row.rule_id === 'media.photos_catalogue'),
    ).toBe(true);
  });

  it('does not infer thumbnail, original or deleted status from a suggestive title or confidence', () => {
    expect(
      mediaKind({ ...finding, title: 'Deleted thumbnail original', confidence: 'candidate' }),
    ).toBe('file');
    expect(mediaUnmatched({ ...finding, confidence: 'candidate' })).toBe(false);
    expect(mediaRows([{ ...finding, category: 'document' }])).toEqual([]);
  });

  it('requires recorded thumbnail and specific association evidence for unmatched filtering', () => {
    const thumbnail = {
      ...finding,
      evidence: [
        {
          label: 'Thumbnail path',
          value: 'Path suggests a thumbnail/cache; visual identity is not verified',
        },
      ],
    };
    const unmatched = {
      ...thumbnail,
      evidence: [
        ...thumbnail.evidence,
        { label: 'Catalogue association', value: 'No match in inspected rows' },
      ],
    };
    const unavailable = {
      ...thumbnail,
      evidence: [
        ...thumbnail.evidence,
        {
          label: 'Catalogue association',
          value: 'Unavailable; no supported Photos rows were imported',
        },
      ],
    };
    expect(mediaRows([finding, thumbnail, unmatched, unavailable], 'unmatched')).toEqual([
      unmatched,
    ]);
    expect(mediaRows([finding, thumbnail], 'files')).toEqual([finding]);
    expect(mediaRows([finding, thumbnail], 'thumbnails')).toEqual([thumbnail]);
  });

  it('keeps catalogue rows distinct from readable files and uses recorded filenames', () => {
    const catalogue = {
      ...finding,
      rule_id: 'media.photos_catalogue',
      evidence: [{ label: 'Filename', value: 'catalogue-only.PNG' }],
    };
    expect(mediaName(catalogue)).toBe('catalogue-only.PNG');
    expect(previewEligible(catalogue)).toBe(false);
    expect(mediaRows([finding, catalogue], 'catalogue')).toEqual([catalogue]);
  });
});

describe('native image preview boundary', () => {
  it('accepts a bounded PNG matching its finding, dimensions and recorded content hash', () => {
    expect(validateMediaPreview(preview, finding)).toEqual(preview);
    expect(validateMediaPreview(preview, finding)).not.toBe(preview);
  });

  it('rejects stale finding identities and changed source content', () => {
    expect(() =>
      validateMediaPreview({ ...preview, finding_id: 'another-finding' }, finding),
    ).toThrow('invalid image preview');
    expect(() =>
      validateMediaPreview({ ...preview, source_sha256: 'b'.repeat(64) }, finding),
    ).toThrow('invalid image preview');
    expect(() => validateMediaPreview(preview, { ...finding, sha256: null })).toThrow(
      'invalid image preview',
    );
  });

  it('never accepts remote URLs, SVG, arbitrary text or malformed base64 as an image', () => {
    for (const data_url of [
      'https://example.invalid/image.png',
      'file:///private/image.png',
      'data:image/svg+xml;base64,PHN2Zy8+',
      'data:image/png;base64,' + 'A'.repeat(100),
      preview.data_url + '\n',
      preview.data_url + '====',
    ])
      expect(() => validateMediaPreview({ ...preview, data_url }, finding)).toThrow(
        'invalid image preview',
      );
  });

  it('cross-checks PNG header dimensions and enforces output and decode bounds', () => {
    for (const changes of [
      { width: 2 },
      { height: 0 },
      { width: 1025 },
      { source_width: Number.POSITIVE_INFINITY },
      { source_width: 100_000, source_height: 100_000 },
      { source_width: 0 },
    ])
      expect(() => validateMediaPreview({ ...preview, ...changes }, finding)).toThrow(
        'invalid image preview',
      );
    const data_url =
      'data:image/png;base64,' + 'A'.repeat(Math.ceil(MEDIA_PREVIEW_LIMITS.maxBytes / 3) * 4 + 4);
    expect(() => validateMediaPreview({ ...preview, data_url }, finding)).toThrow(
      'invalid image preview',
    );
  });

  it('rejects unexpected properties and getters before reading them', () => {
    expect(() => validateMediaPreview({ ...preview, path: '/private/source' }, finding)).toThrow(
      'invalid image preview',
    );
    const unsafe = { ...preview };
    Object.defineProperty(unsafe, 'data_url', {
      enumerable: true,
      get() {
        throw new Error('getter executed');
      },
    });
    expect(() => validateMediaPreview(unsafe, finding)).toThrow('invalid image preview');
  });
});
