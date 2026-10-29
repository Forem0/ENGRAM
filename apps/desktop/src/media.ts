import type { Finding } from './model';
import type { MediaPreview } from './media-model';

export type MediaFilter = 'all' | 'files' | 'thumbnails' | 'catalogue' | 'unmatched';
export type MediaKind = 'file' | 'thumbnail' | 'catalogue' | 'record';
export const MEDIA_PAGE_SIZE = 6;
export const MEDIA_PREVIEW_LIMITS = Object.freeze({
  maxBytes: 4 * 1024 * 1024,
  maxEdge: 1024,
  maxSourcePixels: 16_000_000,
});

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

/** Groups describe recorded rules and evidence, not an inferred deletion history. */
export function mediaKind(finding: Finding): MediaKind {
  if (finding.rule_id === 'media.photos_catalogue') return 'catalogue';
  if (finding.rule_id !== 'file.media') return 'record';
  return finding.evidence.some((field) => field.label === 'Thumbnail path') ? 'thumbnail' : 'file';
}

export function mediaUnmatched(finding: Finding): boolean {
  return (
    mediaKind(finding) === 'thumbnail' &&
    finding.evidence.some(
      (field) =>
        field.label === 'Catalogue association' &&
        ['No match in inspected rows', 'Ambiguous filename matches'].includes(field.value),
    )
  );
}

export function mediaRows(findings: Finding[], filter: MediaFilter = 'all'): Finding[] {
  return findings.filter((finding) => {
    if (finding.category !== 'media') return false;
    const kind = mediaKind(finding);
    if (filter === 'all') return true;
    if (filter === 'files') return kind === 'file';
    if (filter === 'thumbnails') return kind === 'thumbnail';
    if (filter === 'catalogue') return kind === 'catalogue';
    if (filter === 'unmatched') return mediaUnmatched(finding);
    return false;
  });
}

export function mediaName(finding: Finding): string {
  if (mediaKind(finding) === 'catalogue') {
    const filename = finding.evidence.find((field) => field.label === 'Filename')?.value;
    if (filename) return filename;
  }
  return finding.title.replace(/^Media file:\s*/, '');
}

export function previewEligible(finding: Finding): boolean {
  return (
    finding.category === 'media' &&
    finding.rule_id === 'file.media' &&
    typeof finding.sha256 === 'string' &&
    /^[a-f0-9]{64}$/.test(finding.sha256) &&
    /\.(png|jpe?g)$/i.test(finding.path)
  );
}

function positiveInteger(value: unknown, maximum: number): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value > 0 && value <= maximum;
}

/** Only bounded native-generated PNGs matching the requested observation can be rendered. */
export function validateMediaPreview(input: unknown, finding: Finding): MediaPreview {
  const failure =
    'The desktop returned an invalid image preview. Select the source again to retry.';
  if (
    !record(input) ||
    Object.keys(input).length !== 7 ||
    !Object.keys(input).every((key) =>
      [
        'finding_id',
        'data_url',
        'width',
        'height',
        'source_width',
        'source_height',
        'source_sha256',
      ].includes(key),
    ) ||
    !previewEligible(finding) ||
    input.finding_id !== finding.id ||
    input.source_sha256 !== finding.sha256 ||
    !positiveInteger(input.width, MEDIA_PREVIEW_LIMITS.maxEdge) ||
    !positiveInteger(input.height, MEDIA_PREVIEW_LIMITS.maxEdge) ||
    !positiveInteger(input.source_width, MEDIA_PREVIEW_LIMITS.maxSourcePixels) ||
    !positiveInteger(input.source_height, MEDIA_PREVIEW_LIMITS.maxSourcePixels) ||
    input.source_width * input.source_height > MEDIA_PREVIEW_LIMITS.maxSourcePixels ||
    input.width > input.source_width ||
    input.height > input.source_height ||
    typeof input.data_url !== 'string'
  )
    throw new Error(failure);

  const prefix = 'data:image/png;base64,';
  if (!input.data_url.startsWith(prefix)) throw new Error(failure);
  const body = input.data_url.slice(prefix.length);
  const maximumEncoded = Math.ceil(MEDIA_PREVIEW_LIMITS.maxBytes / 3) * 4;
  if (
    body.length < 44 ||
    body.length > maximumEncoded ||
    body.length % 4 !== 0 ||
    !/^[A-Za-z0-9+/]+={0,2}$/.test(body)
  )
    throw new Error(failure);
  const padding = body.endsWith('==') ? 2 : body.endsWith('=') ? 1 : 0;
  if ((body.length / 4) * 3 - padding > MEDIA_PREVIEW_LIMITS.maxBytes) throw new Error(failure);

  let header: string;
  try {
    header = atob(body.slice(0, 44));
  } catch {
    throw new Error(failure);
  }
  const signature = [137, 80, 78, 71, 13, 10, 26, 10];
  const uint = (offset: number) =>
    header.charCodeAt(offset) * 2 ** 24 +
    header.charCodeAt(offset + 1) * 2 ** 16 +
    header.charCodeAt(offset + 2) * 2 ** 8 +
    header.charCodeAt(offset + 3);
  if (
    !signature.every((value, index) => header.charCodeAt(index) === value) ||
    uint(8) !== 13 ||
    header.slice(12, 16) !== 'IHDR' ||
    uint(16) !== input.width ||
    uint(20) !== input.height
  )
    throw new Error(failure);

  return {
    finding_id: finding.id,
    data_url: input.data_url,
    width: input.width,
    height: input.height,
    source_width: input.source_width,
    source_height: input.source_height,
    source_sha256: finding.sha256!,
  };
}
