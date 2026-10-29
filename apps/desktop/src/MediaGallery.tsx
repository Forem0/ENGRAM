import { useEffect, useMemo, useRef, useState } from 'react';
import * as bridge from './bridge';
import { bytesLabel, dateLabel } from './data';
import type { Case, Finding } from './model';
import type { MediaPreview } from './media-model';
import {
  MEDIA_PAGE_SIZE,
  mediaKind,
  mediaName,
  mediaRows,
  mediaUnmatched,
  selectedMedia,
  previewEligible,
  validateMediaPreview,
  type MediaFilter,
  type MediaKind,
} from './media';
import './media.css';

const filters: { id: MediaFilter; label: string }[] = [
  { id: 'all', label: 'All media' },
  { id: 'files', label: 'Files' },
  { id: 'thumbnails', label: 'Thumbnail paths' },
  { id: 'catalogue', label: 'Catalogue rows' },
  { id: 'unmatched', label: 'Unmatched' },
];
const kindLabels: Record<MediaKind, string> = {
  file: 'Image / media file',
  thumbnail: 'Thumbnail-path file',
  catalogue: 'Photos catalogue row',
  record: 'Media record',
};

function MediaGlyph({ kind = 'file' }: { kind?: MediaKind }) {
  return (
    <svg viewBox="0 0 36 36" fill="none" aria-hidden="true">
      {kind === 'catalogue' ? (
        <>
          <ellipse cx="18" cy="9" rx="11" ry="4" />
          <path d="M7 9v17c0 5 22 5 22 0V9M7 17c0 5 22 5 22 0" />
        </>
      ) : (
        <>
          <rect x="5" y="6" width="26" height="24" rx="4" />
          <circle cx="13" cy="14" r="2.5" />
          <path d="m6 26 8-8 5 5 5-6 7 9" />
          {kind === 'thumbnail' && <path d="M12 34h19a4 4 0 0 0 4-4V14" />}
        </>
      )}
    </svg>
  );
}

export function MediaGallery({
  caseData,
  visible,
  selected,
  onSelect,
  busy,
}: {
  caseData: Case;
  visible: Finding[];
  selected: Finding | null;
  onSelect: (finding: Finding) => void;
  busy: boolean;
}) {
  const [filter, setFilter] = useState<MediaFilter>('all');
  const [page, setPage] = useState(0);
  const [result, setResult] = useState<{ key: string; preview: MediaPreview } | null>(null);
  const [loading, setLoading] = useState<string | null>(null);
  const [error, setError] = useState<{ key: string; text: string } | null>(null);
  const request = useRef(0);
  const inFlight = useRef(false);
  const rows = useMemo(() => mediaRows(visible, filter), [visible, filter]);
  const allMedia = useMemo(() => mediaRows(visible), [visible]);
  const counts = useMemo(
    () => Object.fromEntries(filters.map((item) => [item.id, mediaRows(visible, item.id).length])),
    [visible],
  );
  const current = selectedMedia(caseData.findings, selected?.id);
  const outsideFilters = current !== null && !rows.some((finding) => finding.id === current.id);
  const key = JSON.stringify([caseData.id, current?.id, current?.sha256]);
  const latestKey = useRef(key);
  latestKey.current = key;
  const preview = result?.key === key ? result.preview : null;
  const message = error?.key === key ? error.text : null;
  const pageCount = Math.max(1, Math.ceil(rows.length / MEDIA_PAGE_SIZE));
  const currentPage = Math.min(page, pageCount - 1);
  const related = current
    ? caseData.findings.filter((finding) => current.related_ids.includes(finding.id)).slice(0, 4)
    : [];

  useEffect(() => setPage(0), [visible, filter, caseData.id]);
  useEffect(() => {
    request.current += 1;
    inFlight.current = false;
    setResult(null);
    setError(null);
    setLoading(null);
    return () => {
      request.current += 1;
    };
  }, [key]);

  async function openPreview() {
    if (!bridge.native || !current || !previewEligible(current) || busy || inFlight.current) return;
    inFlight.current = true;
    const sequence = ++request.current;
    const expectedKey = key;
    setLoading(expectedKey);
    setError(null);
    try {
      const image = validateMediaPreview(await bridge.previewMedia(caseData, current.id), current);
      if (request.current === sequence && latestKey.current === expectedKey)
        setResult({ key: expectedKey, preview: image });
    } catch (failure) {
      if (request.current === sequence && latestKey.current === expectedKey)
        setError({
          key: expectedKey,
          text: failure instanceof Error ? failure.message : String(failure),
        });
    } finally {
      if (request.current === sequence) {
        inFlight.current = false;
        setLoading(null);
      }
    }
  }

  return (
    <section className="panel media-gallery" aria-label="Media gallery">
      <div className="media-gallery-heading">
        <div>
          <span className="eyebrow">MEDIA EXPLORER</span>
          <h2>See the trace. Keep the context.</h2>
          <p>Files, thumbnail paths and catalogue records from the current observation.</p>
        </div>
        <div className="media-total">
          <strong>{allMedia.length.toLocaleString('en')}</strong>
          <span>matching media</span>
        </div>
      </div>

      <div className="media-filter-tabs" aria-label="Media kind filters">
        {filters.map((item) => (
          <button
            key={item.id}
            className={filter === item.id ? 'active' : ''}
            aria-pressed={filter === item.id}
            onClick={() => setFilter(item.id)}
          >
            {item.label} <span>{counts[item.id]}</span>
          </button>
        ))}
      </div>

      <div className="media-selection" aria-label="Selected media">
        <div className={`media-viewer ${preview ? 'has-image' : ''}`}>
          {preview && current ? (
            <>
              <img
                src={preview.data_url}
                alt={`Preview of ${mediaName(current)} from the selected source`}
                width={preview.width}
                height={preview.height}
                onError={() => {
                  setResult(null);
                  setError({
                    key,
                    text: 'The generated image could not be displayed. No source file was changed.',
                  });
                }}
              />
              <span className="media-viewer-caption">
                Native-generated preview · metadata stripped
              </span>
            </>
          ) : (
            <div className="media-viewer-placeholder">
              <span className="media-preview-glyph">
                <MediaGlyph kind={current ? mediaKind(current) : 'file'} />
              </span>
              <strong>{current ? mediaName(current) : 'Select a media finding'}</strong>
              <p>
                {!current
                  ? 'Choose a card below to inspect its source and available image.'
                  : mediaKind(current) === 'catalogue'
                    ? 'This is a database record. Follow a recorded relationship to inspect a matching file.'
                    : 'Image contents open only when you request a preview.'}
              </p>
              {current && (
                <button
                  className="button primary"
                  disabled={!bridge.native || busy || loading === key || !previewEligible(current)}
                  onClick={() => void openPreview()}
                >
                  {loading === key ? 'Preparing image…' : 'Preview image'}
                </button>
              )}
              {current && !bridge.native && (
                <small>Viewing source images requires the desktop application.</small>
              )}
              {current &&
                bridge.native &&
                !previewEligible(current) &&
                mediaKind(current) !== 'catalogue' && (
                  <small>
                    Preview requires a hashed PNG or JPEG file from a source selected in this
                    session.
                  </small>
                )}
            </div>
          )}
          {message && (
            <p className="media-preview-error" role="alert">
              {message}
            </p>
          )}
          {loading === key && (
            <span className="sr-only" role="status">
              Preparing a bounded local image preview.
            </span>
          )}
        </div>

        <div className="media-selection-context">
          <span className="eyebrow">SOURCE CONTEXT</span>
          <h3>{current ? mediaName(current) : 'The evidence stays beside the image.'}</h3>
          {current ? (
            <>
              <span className={`media-kind-label ${mediaKind(current)}`}>
                {kindLabels[mediaKind(current)]}
              </span>
              {outsideFilters && (
                <span className="media-outside-filter">Outside current filters</span>
              )}
              <dl>
                <div>
                  <dt>Recorded source</dt>
                  <dd>{current.path}</dd>
                </div>
                <div>
                  <dt>Timestamp</dt>
                  <dd>{dateLabel(current.observed_at)}</dd>
                </div>
                <div>
                  <dt>Rule</dt>
                  <dd className="mono">{current.rule_id}</dd>
                </div>
                <div>
                  <dt>{preview ? 'Source dimensions' : 'Recorded size'}</dt>
                  <dd>
                    {preview
                      ? `${preview.source_width.toLocaleString('en')} × ${preview.source_height.toLocaleString('en')} px`
                      : bytesLabel(current.bytes)}
                  </dd>
                </div>
              </dl>
              {preview && (
                <div className="media-image-receipt">
                  <span>Source SHA-256 verified</span>
                  <code>{preview.source_sha256}</code>
                  <button className="button text small" onClick={() => setResult(null)}>
                    Hide image
                  </button>
                </div>
              )}
              {related.length > 0 && (
                <div className="media-related">
                  <span className="eyebrow">RECORDED RELATIONSHIPS</span>
                  {related.map((finding) => (
                    <button key={finding.id} onClick={() => onSelect(finding)}>
                      <span className="media-related-copy">
                        <strong>
                          {finding.category === 'media' ? mediaName(finding) : finding.title}
                        </strong>
                        <small>
                          {finding.category === 'media'
                            ? kindLabels[mediaKind(finding)]
                            : finding.category}
                          {mediaKind(finding) === 'catalogue' && ` · ${finding.locator}`}
                        </small>
                        <span className="media-related-path" title={finding.path}>
                          {finding.path}
                        </span>
                      </span>
                      <span aria-hidden="true">↗</span>
                    </button>
                  ))}
                </div>
              )}
            </>
          ) : (
            <p>
              Inspect paths, catalogue associations and recorded relationships without assuming a
              deletion date.
            </p>
          )}
        </div>
      </div>

      {rows.length === 0 ? (
        <div className="media-empty">
          <strong>No media matches this view.</strong>
          <p>Change the media filter or the shared search and review filters.</p>
        </div>
      ) : (
        <>
          <div className="media-card-grid" aria-label="Media findings">
            {rows
              .slice(currentPage * MEDIA_PAGE_SIZE, (currentPage + 1) * MEDIA_PAGE_SIZE)
              .map((finding) => {
                const kind = mediaKind(finding);
                const active = finding.id === current?.id;
                return (
                  <button
                    key={finding.id}
                    className={`media-card ${kind} ${active ? 'selected' : ''}`}
                    aria-pressed={active}
                    aria-label={`Inspect media ${mediaName(finding)}`}
                    onClick={() => onSelect(finding)}
                  >
                    <span className="media-card-visual">
                      {active && preview ? (
                        <img
                          src={preview.data_url}
                          alt=""
                          width={preview.width}
                          height={preview.height}
                        />
                      ) : (
                        <MediaGlyph kind={kind} />
                      )}
                      <span className={`media-card-confidence ${finding.confidence}`}>
                        {finding.confidence}
                      </span>
                    </span>
                    <span className="media-card-content">
                      <strong title={mediaName(finding)}>{mediaName(finding)}</strong>
                      <small>
                        {mediaUnmatched(finding)
                          ? 'No unique catalogue association'
                          : kindLabels[kind]}
                      </small>
                      <span title={finding.path}>{finding.path}</span>
                    </span>
                  </button>
                );
              })}
          </div>
          <div className="media-pagination">
            <span>
              {currentPage * MEDIA_PAGE_SIZE + 1}–
              {Math.min((currentPage + 1) * MEDIA_PAGE_SIZE, rows.length)} of{' '}
              {rows.length.toLocaleString('en')} media findings
            </span>
            <div>
              <button
                className="button text small"
                disabled={currentPage === 0}
                onClick={() => setPage(currentPage - 1)}
              >
                Previous
              </button>
              <span>
                {currentPage + 1} / {pageCount}
              </span>
              <button
                className="button text small"
                disabled={currentPage + 1 >= pageCount}
                onClick={() => setPage(currentPage + 1)}
              >
                Next
              </button>
            </div>
          </div>
        </>
      )}
      <p className="media-footnote">
        Thumbnail groups reflect recorded path evidence. Catalogue associations do not establish
        visual identity or permanent deletion.
      </p>
    </section>
  );
}
