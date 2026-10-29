import { useMemo, useRef, useState } from 'react';
import { bytesLabel } from './data';
import type { Case, Finding, ReportFormat, ReportOptions, ReportReceipt } from './model';
import { reportFindings, validateReportOptions, type ReportScope } from './report';
import { getReview } from './review';
import { WorkspaceDialog } from './WorkspaceDialog';
import './report-export.css';

const scopes: { id: ReportScope; label: string; detail: string }[] = [
  {
    id: 'filtered',
    label: 'Current results',
    detail: 'The complete filtered queue, across every page.',
  },
  {
    id: 'bookmarked',
    label: 'Bookmarked',
    detail: 'Your bookmarked findings across the open case.',
  },
  { id: 'all', label: 'Entire case', detail: 'All findings recorded in this observation.' },
];

export function ReportExport({
  caseData,
  visible,
  native,
  busy,
  unsavedReview,
  onExport,
  onClose,
}: {
  caseData: Case;
  visible: Finding[];
  native: boolean;
  busy: boolean;
  unsavedReview: boolean;
  onExport: (options: ReportOptions) => Promise<ReportReceipt | null>;
  onClose: () => void;
}) {
  const [scope, setScope] = useState<ReportScope>('filtered');
  const [format, setFormat] = useState<ReportFormat>('markdown');
  const [includePaths, setIncludePaths] = useState(false);
  const [includeDetails, setIncludeDetails] = useState(false);
  const [includeNotes, setIncludeNotes] = useState(false);
  const [message, setMessage] = useState<{ error: boolean; text: string } | null>(null);
  const [receipt, setReceipt] = useState<ReportReceipt | null>(null);
  const [copied, setCopied] = useState(false);
  const inFlight = useRef(false);
  const selection = useMemo(
    () => reportFindings(caseData, visible, scope),
    [caseData, visible, scope],
  );
  const counts = useMemo(
    () =>
      Object.fromEntries(
        scopes.map((item) => [item.id, reportFindings(caseData, visible, item.id).length]),
      ),
    [caseData, visible],
  );
  const validation = useMemo(() => {
    try {
      return {
        options: validateReportOptions(
          {
            format,
            finding_ids: selection.map((finding) => finding.id),
            include_notes: includeNotes,
            include_paths: includePaths,
            include_details: includeDetails,
          },
          caseData.findings,
        ),
        error: null,
      };
    } catch (error) {
      return { options: null, error: error instanceof Error ? error.message : String(error) };
    }
  }, [caseData.findings, format, selection, includePaths, includeDetails, includeNotes]);
  const notes = selection.filter((finding) =>
    getReview(caseData.review, finding.id).note.trim(),
  ).length;

  async function submit() {
    if (!native || busy || inFlight.current || !validation.options) return;
    inFlight.current = true;
    setMessage(null);
    try {
      const result = await onExport(validation.options);
      if (result) setReceipt(result);
      else
        setMessage({ error: false, text: 'Export cancelled. Your case and review are unchanged.' });
    } catch (error) {
      setMessage({ error: true, text: error instanceof Error ? error.message : String(error) });
    } finally {
      inFlight.current = false;
    }
  }

  async function copyChecksum() {
    if (!receipt) return;
    try {
      await navigator.clipboard.writeText(receipt.sha256);
      setCopied(true);
    } catch {
      setMessage({
        error: true,
        text: 'Clipboard unavailable. Select the checksum below and copy it manually.',
      });
    }
  }

  return (
    <WorkspaceDialog
      title={receipt ? 'Report exported' : 'Export investigation'}
      onClose={onClose}
      busy={busy}
      className="report-export-dialog"
    >
      {receipt ? (
        <>
          <div className="report-complete-mark" aria-hidden="true">
            ✓
          </div>
          <p className="report-intro">
            A separate report was written locally. Your source evidence is unchanged.
          </p>
          <dl className="report-receipt">
            <div>
              <dt>Report file</dt>
              <dd>{receipt.path}</dd>
            </div>
            <div>
              <dt>Format</dt>
              <dd>{receipt.format === 'csv' ? 'CSV table' : 'Markdown report'}</dd>
            </div>
            <div>
              <dt>Contents</dt>
              <dd>
                {receipt.finding_count.toLocaleString('en')}{' '}
                {receipt.finding_count === 1 ? 'finding' : 'findings'} · {bytesLabel(receipt.bytes)}
              </dd>
            </div>
            <div className="report-checksum">
              <dt>SHA-256</dt>
              <dd>
                <code>{receipt.sha256}</code>
                <button
                  type="button"
                  className="button text small"
                  onClick={() => void copyChecksum()}
                >
                  {copied ? 'Copied' : 'Copy checksum'}
                </button>
              </dd>
            </div>
          </dl>
          <p className="report-persistence-note">
            This report does not replace a case file.{' '}
            {unsavedReview
              ? 'Your review still has unsaved changes; use Save case to retain them.'
              : 'Use Save case to keep a file that can reopen this investigation.'}
          </p>
          {message && (
            <p className="report-message" role={message.error ? 'alert' : 'status'}>
              {message.text}
            </p>
          )}
          <div className="dialog-actions">
            <button
              className="button secondary"
              onClick={() => {
                setReceipt(null);
                setCopied(false);
                setMessage(null);
              }}
            >
              Export another report
            </button>
            <button className="button primary" onClick={onClose}>
              Done
            </button>
          </div>
        </>
      ) : (
        <>
          <p className="report-intro">
            Export findings from the open case as a readable report or a table. Nothing is uploaded.
          </p>
          <fieldset className="report-scope" disabled={busy}>
            <legend>
              01 <span>Choose the findings</span>
            </legend>
            <div className="report-scope-grid">
              {scopes.map((item) => (
                <label
                  key={item.id}
                  className={`report-scope-card ${scope === item.id ? 'active' : ''}`}
                >
                  <input
                    type="radio"
                    name="report-scope"
                    value={item.id}
                    checked={scope === item.id}
                    onChange={() => {
                      setScope(item.id);
                      setMessage(null);
                    }}
                  />
                  <span className="report-scope-title">
                    {item.label}
                    <strong>{counts[item.id]?.toLocaleString('en')}</strong>
                  </span>
                  <small>{item.detail}</small>
                </label>
              ))}
            </div>
          </fieldset>
          <div className="report-options-grid">
            <fieldset className="report-format" disabled={busy}>
              <legend>
                02 <span>Choose a format</span>
              </legend>
              <label className={format === 'markdown' ? 'active' : ''}>
                <input
                  type="radio"
                  name="report-format"
                  checked={format === 'markdown'}
                  onChange={() => setFormat('markdown')}
                />
                <span>
                  <strong>Markdown report</strong>
                  <small>Readable findings, review states and coverage.</small>
                </span>
                <b>.md</b>
              </label>
              <label className={format === 'csv' ? 'active' : ''}>
                <input
                  type="radio"
                  name="report-format"
                  checked={format === 'csv'}
                  onChange={() => setFormat('csv')}
                />
                <span>
                  <strong>CSV table</strong>
                  <small>One finding per row, ready for a spreadsheet.</small>
                </span>
                <b>.csv</b>
              </label>
            </fieldset>
            <fieldset className="report-privacy" disabled={busy}>
              <legend>
                03 <span>Control the details</span>
              </legend>
              <label>
                <input
                  type="checkbox"
                  checked={includePaths}
                  onChange={(event) => setIncludePaths(event.target.checked)}
                />
                <span>
                  <strong>Include source paths</strong>
                  <small>Otherwise paths use filenames and the source root is omitted.</small>
                </span>
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={includeDetails}
                  onChange={(event) => setIncludeDetails(event.target.checked)}
                />
                <span>
                  <strong>Include evidence details</strong>
                  <small>
                    Source text can contain personal data and paths, even when source paths are off.
                  </small>
                </span>
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={includeNotes}
                  onChange={(event) => setIncludeNotes(event.target.checked)}
                />
                <span>
                  <strong>
                    Include analyst notes <em>{notes}</em>
                  </strong>
                  <small>Notes are plaintext and are not automatically redacted.</small>
                </span>
              </label>
            </fieldset>
          </div>
          <section className="report-selection-summary" aria-label="Report selection summary">
            <div>
              <span className="report-file-mark" aria-hidden="true">
                {format === 'csv' ? 'CSV' : 'MD'}
              </span>
              <div>
                <strong>
                  {selection.length.toLocaleString('en')}{' '}
                  {selection.length === 1 ? 'finding' : 'findings'} selected
                </strong>
                <small>
                  {caseData.title} ·{' '}
                  {includeDetails ? 'Evidence details included' : 'Compact finding summary'}
                </small>
              </div>
            </div>
            {selection.length > 0 && (
              <ul aria-label="Selected finding sample">
                {selection.slice(0, 3).map((finding) => (
                  <li key={finding.id}>{finding.title}</li>
                ))}
                {selection.length > 3 && (
                  <li className="muted">
                    + {(selection.length - 3).toLocaleString('en')} more{' '}
                    {selection.length - 3 === 1 ? 'finding' : 'findings'}
                  </li>
                )}
              </ul>
            )}
          </section>
          <p className="report-persistence-note">
            {unsavedReview
              ? 'Current bookmarks and review states are included, even if unsaved. '
              : 'Review states and bookmarks are included. '}
            Notes are included only when enabled. Exporting does not save or replace the case file.
          </p>
          {!native && (
            <p className="report-native-note">
              Configure the report here. Writing a file requires the ENGRAM desktop application.
            </p>
          )}
          {validation.error && (
            <p className="report-message error" role="alert">
              {validation.error}
            </p>
          )}
          {message && (
            <p
              className={`report-message ${message.error ? 'error' : ''}`}
              role={message.error ? 'alert' : 'status'}
            >
              {message.text}
            </p>
          )}
          {busy && (
            <p className="report-message" role="status">
              Preparing the report and choosing its destination…
            </p>
          )}
          <div className="dialog-actions">
            <button className="button secondary" disabled={busy} onClick={onClose}>
              Cancel
            </button>
            <button
              className="button primary"
              disabled={!native || busy || !validation.options}
              onClick={() => void submit()}
            >
              {busy ? 'Exporting…' : 'Choose destination & export'}
            </button>
          </div>
        </>
      )}
    </WorkspaceDialog>
  );
}
