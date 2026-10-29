import { useEffect, useState } from 'react';
import type { Filters } from './data';
import type { ReviewAnnotation, ReviewStatus } from './model';
import { WorkspaceDialog } from './WorkspaceDialog';
import './review-ui.css';

export function BookmarkGlyph({ filled = false }: { filled?: boolean }) {
  return (
    <svg
      aria-hidden="true"
      viewBox="0 0 24 24"
      width="16"
      height="16"
      fill={filled ? 'currentColor' : 'none'}
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinejoin="round"
    >
      <path d="M6 3h12v18l-6-4-6 4z" />
    </svg>
  );
}

export function ReviewPanel({
  findingId,
  annotation,
  onChange,
  disabled,
  baseline,
  native,
}: {
  findingId: string;
  annotation: ReviewAnnotation;
  onChange: (annotation: ReviewAnnotation) => void;
  disabled: boolean;
  baseline: boolean;
  native: boolean;
}) {
  const [expanded, setExpanded] = useState(!!annotation.note);
  useEffect(() => setExpanded(!!annotation.note), [findingId, baseline]);
  const noteBytes = new TextEncoder().encode(annotation.note).length;
  return (
    <section className="inspector-section review-panel" aria-label="Finding review">
      <div className="review-panel-heading">
        <span className="eyebrow">{baseline ? 'BASELINE REVIEW' : 'YOUR REVIEW'}</span>
        <button
          className={`review-bookmark ${annotation.bookmarked ? 'is-bookmarked' : ''}`}
          aria-label={annotation.bookmarked ? 'Remove bookmark' : 'Bookmark finding'}
          aria-pressed={annotation.bookmarked}
          disabled={disabled || baseline}
          title={baseline ? 'Baseline annotations are read-only.' : 'Bookmark finding (B)'}
          onClick={() => onChange({ ...annotation, bookmarked: !annotation.bookmarked })}
        >
          <BookmarkGlyph filled={annotation.bookmarked} />
          {annotation.bookmarked ? 'Bookmarked' : 'Bookmark'}
        </button>
      </div>
      <label className="review-status-field">
        <span>Review status</span>
        <select
          aria-label="Finding review status"
          value={annotation.status}
          disabled={disabled || baseline}
          onChange={(event) =>
            onChange({ ...annotation, status: event.target.value as ReviewStatus })
          }
        >
          <option value="unreviewed">Unreviewed</option>
          <option value="needs_review">Needs review</option>
          <option value="reviewed">Reviewed</option>
        </select>
      </label>
      <button
        className="review-note-toggle"
        aria-expanded={expanded}
        onClick={() => setExpanded(!expanded)}
      >
        <span>{annotation.note ? 'Review note' : baseline ? 'No review note' : 'Add a note'}</span>
        <span aria-hidden="true">{expanded ? '−' : '+'}</span>
      </button>
      {expanded && (
        <div className="review-note-editor">
          <textarea
            aria-label="Review note"
            placeholder="What needs a closer look?"
            rows={4}
            maxLength={4096}
            value={annotation.note}
            readOnly={baseline}
            disabled={disabled}
            onChange={(event) => onChange({ ...annotation, note: event.target.value })}
          />
          <small>{noteBytes.toLocaleString('en')} / 4,096 bytes · plain text</small>
        </div>
      )}
      <p className="review-storage-note">
        {baseline
          ? 'Annotations from the earlier case. Source evidence stays unchanged.'
          : native
            ? 'Use Save case to keep your review in a new case file.'
            : 'Demo reviews stay in this tab. The desktop app saves them with a case.'}
      </p>
    </section>
  );
}

export function ReviewToolbar({
  filters,
  setFilters,
  summary,
  total,
  onHelp,
}: {
  filters: Filters;
  setFilters: (filters: Filters) => void;
  summary: { bookmarked: number; reviewed: number; needsReview: number; withNotes: number };
  total: number;
  onHelp: () => void;
}) {
  return (
    <div className="review-toolbar" aria-label="Review filters">
      <button
        className={`review-filter-button ${filters.bookmarkedOnly ? 'active' : ''}`}
        aria-pressed={filters.bookmarkedOnly ?? false}
        onClick={() => setFilters({ ...filters, bookmarkedOnly: !filters.bookmarkedOnly })}
      >
        <BookmarkGlyph filled={filters.bookmarkedOnly} /> Bookmarks{' '}
        <span>{summary.bookmarked}</span>
      </button>
      <select
        aria-label="Filter review status"
        value={filters.reviewStatus ?? 'all'}
        onChange={(event) =>
          setFilters({ ...filters, reviewStatus: event.target.value as Filters['reviewStatus'] })
        }
      >
        <option value="all">All review states</option>
        <option value="unreviewed">Unreviewed</option>
        <option value="needs_review">Needs review</option>
        <option value="reviewed">Reviewed</option>
      </select>
      <span className="review-progress">
        <strong>{summary.reviewed}</strong> / {total} reviewed{' '}
        <span>· {summary.withNotes} notes</span>
      </span>
      <button className="button text small search-help" onClick={onHelp}>
        Search & shortcuts <kbd>?</kbd>
      </button>
    </div>
  );
}

export function WorkspaceHelp({ onClose }: { onClose: () => void }) {
  return (
    <WorkspaceDialog title="Search & shortcuts" onClose={onClose}>
      <p>
        Combine words, quoted phrases and filters. Every term must match. Prefix a term with{' '}
        <code>-</code> to exclude it.
      </p>
      <div className="search-examples">
        <code>category:media -path:cache</code>
        <code>confidence:candidate is:bookmarked</code>
        <code>has:note "previous config"</code>
        <code>rule:secret is:needs_review</code>
      </div>
      <p className="muted">
        Fields: category, confidence, path, rule, origin, note. Review filters: is:bookmarked,
        is:reviewed, is:needs_review, is:unreviewed, has:note. Unknown operators are searched
        literally.
      </p>
      <dl className="shortcut-list">
        <dt>
          <kbd>Ctrl / ⌘ K</kbd>
        </dt>
        <dd>Focus search</dd>
        <dt>
          <kbd>Ctrl / ⌘ S</kbd>
        </dt>
        <dd>Save a new case in the desktop app</dd>
        <dt>
          <kbd>B</kbd>
        </dt>
        <dd>Bookmark the selected current finding</dd>
        <dt>
          <kbd>R</kbd>
        </dt>
        <dd>Toggle reviewed / unreviewed</dd>
        <dt>
          <kbd>1–7</kbd>
        </dt>
        <dd>Switch workspace views</dd>
        <dt>
          <kbd>?</kbd>
        </dt>
        <dd>Open this help</dd>
      </dl>
      <p className="muted">
        Letter shortcuts stay inactive while typing. Review status is your annotation; confidence
        remains the engine's evidence label.
      </p>
      <div className="dialog-actions">
        <button className="button primary" onClick={onClose}>
          Done
        </button>
      </div>
    </WorkspaceDialog>
  );
}
