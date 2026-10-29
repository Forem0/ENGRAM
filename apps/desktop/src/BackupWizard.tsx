import { useEffect, useRef, useState } from 'react';
import * as bridge from './bridge';
import type { BackupInspection } from './media-model';
import { validateBackupInspection } from './backup';
import { WorkspaceDialog } from './WorkspaceDialog';
import './backup.css';

export function BackupWizard({
  busy,
  onClose,
  onScan,
}: {
  busy: boolean;
  onClose: () => void;
  onScan: (inspection: BackupInspection) => Promise<void>;
}) {
  const [inspection, setInspection] = useState<BackupInspection | null>(null);
  const [working, setWorking] = useState<'choosing' | 'scanning' | 'closing' | null>(null);
  const [message, setMessage] = useState<{ error: boolean; text: string } | null>(null);
  const token = useRef<string | null>(null);
  const inFlight = useRef(false);
  const scanning = useRef(false);
  const mounted = useRef(true);
  const locked = busy || working !== null;

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      const pending = token.current;
      token.current = null;
      if (pending && !scanning.current) void bridge.cancelBackup(pending).catch(() => {});
    };
  }, []);

  async function choose() {
    if (!bridge.native || locked || inFlight.current) return;
    inFlight.current = true;
    setWorking('choosing');
    setMessage(null);
    const previous = token.current;
    token.current = null;
    setInspection(null);
    try {
      // A new picker replaces the previous staged source. A stale cancellation
      // must never invalidate a more recently selected source in native state.
      if (previous) await bridge.cancelBackup(previous).catch(() => {});
      const result = await bridge.selectBackup();
      if (!mounted.current) {
        if (result) void bridge.cancelBackup(result.token).catch(() => {});
        return;
      }
      if (result) {
        const next = validateBackupInspection(result);
        token.current = next.token;
        setInspection(next);
      } else {
        setMessage({
          error: false,
          text: 'Selection cancelled. Your current case and review are unchanged.',
        });
      }
    } catch (error) {
      if (mounted.current)
        setMessage({ error: true, text: error instanceof Error ? error.message : String(error) });
    } finally {
      inFlight.current = false;
      if (mounted.current) setWorking(null);
    }
  }

  async function start() {
    if (!inspection || inspection.state !== 'ready' || locked || inFlight.current) return;
    inFlight.current = true;
    scanning.current = true;
    const selected = inspection;
    // Native scan tokens are one-use, including a failed scan. Never offer a
    // retry with a consumed token or reuse a renderer-held source path.
    token.current = null;
    setWorking('scanning');
    setMessage(null);
    try {
      await onScan(selected);
    } catch (error) {
      if (mounted.current) {
        setInspection(null);
        setMessage({
          error: true,
          text: `${error instanceof Error ? error.message : String(error)} Choose the backup again to retry. Your previous case has been retained.`,
        });
      }
    } finally {
      scanning.current = false;
      inFlight.current = false;
      if (mounted.current) setWorking(null);
    }
  }

  async function close() {
    if (locked || inFlight.current) return;
    inFlight.current = true;
    setWorking('closing');
    const pending = token.current;
    token.current = null;
    if (pending) await bridge.cancelBackup(pending).catch(() => {});
    inFlight.current = false;
    if (mounted.current) onClose();
  }

  const maximumCategory = Math.max(1, ...(inspection?.categories.map((item) => item.count) ?? []));
  return (
    <WorkspaceDialog
      title="Inspect an iPhone backup"
      onClose={() => void close()}
      busy={locked}
      className="backup-wizard-dialog"
    >
      <ol className="backup-steps" aria-label="Backup inspection steps">
        <li
          className={!inspection ? 'active' : 'complete'}
          aria-current={!inspection ? 'step' : undefined}
        >
          <span>01</span>Choose a backup
        </li>
        <li
          className={inspection && working !== 'scanning' ? 'active' : ''}
          aria-current={inspection && working !== 'scanning' ? 'step' : undefined}
        >
          <span>02</span>Check compatibility
        </li>
        <li
          className={working === 'scanning' ? 'active' : ''}
          aria-current={working === 'scanning' ? 'step' : undefined}
        >
          <span>03</span>Inspect evidence
        </li>
      </ol>

      {inspection ? (
        <>
          <div className={`backup-result-heading ${inspection.state}`}>
            <span className="backup-phone-mark" aria-hidden="true">
              <svg width="28" height="36" viewBox="0 0 28 36" fill="none">
                <rect x="5" y="2" width="18" height="32" rx="4" />
                <path d="M11 6h6M12 29h4" />
              </svg>
            </span>
            <div>
              <span className="eyebrow">SELECTED LOCAL BACKUP</span>
              <h3>{inspection.name}</h3>
              <p>
                {inspection.state === 'ready'
                  ? 'Index checked. Analysis has not started.'
                  : 'This backup cannot be inspected with the current readers.'}
              </p>
            </div>
            <span className={`tag ${inspection.state === 'ready' ? 'observed' : 'candidate'}`}>
              {inspection.state === 'ready' ? 'Index supported' : 'Unavailable'}
            </span>
          </div>

          <dl className="backup-facts">
            <div>
              <dt>Device label</dt>
              <dd>{inspection.device_name ?? 'Not recorded'}</dd>
            </div>
            <div>
              <dt>iOS version</dt>
              <dd>{inspection.product_version ?? 'Not recorded'}</dd>
            </div>
            <div>
              <dt>Encryption</dt>
              <dd>
                {inspection.encrypted === null
                  ? 'Not established'
                  : inspection.encrypted
                    ? 'Encrypted'
                    : 'Unencrypted'}
              </dd>
            </div>
          </dl>

          {inspection.state === 'ready' && (
            <section className="backup-index-summary" aria-label="Backup index summary">
              <div className="backup-index-total">
                <strong>{inspection.indexed_files.toLocaleString('en')}</strong>
                <span>indexed file entries</span>
              </div>
              <div className="backup-index-categories">
                {inspection.categories.length > 0 ? (
                  inspection.categories.map((category) => (
                    <div className="backup-category" key={category.id}>
                      <span>{category.label}</span>
                      <strong>{category.count.toLocaleString('en')}</strong>
                      <span className="backup-category-track" aria-hidden="true">
                        <span style={{ width: `${(category.count / maximumCategory) * 100}%` }} />
                      </span>
                    </div>
                  ))
                ) : (
                  <p>No category totals were recorded for this index.</p>
                )}
              </div>
            </section>
          )}

          <p className="backup-coverage-note">
            Index totals describe paths listed by this backup. Analysis will determine which files
            and database records are readable within the supported formats and limits.
          </p>
          {inspection.warnings.length > 0 && (
            <details className="backup-warnings" open={inspection.state !== 'ready'}>
              <summary>
                Compatibility and coverage notes <span>{inspection.warnings.length}</span>
              </summary>
              <ul>
                {inspection.warnings.map((warning, index) => (
                  <li key={index}>{warning}</li>
                ))}
              </ul>
            </details>
          )}
          {inspection.encrypted && (
            <p className="backup-unavailable-note">
              Encrypted backup analysis is unavailable in this version. No password is requested or
              tested.
            </p>
          )}
          {inspection.state === 'ready' && (
            <p className="backup-current-case-note">
              Starting analysis opens a new observation when it succeeds. The selected backup is
              read locally; output files are saved separately.
            </p>
          )}
        </>
      ) : (
        <>
          <div className="backup-welcome">
            <span className="backup-phone-mark" aria-hidden="true">
              <svg width="44" height="52" viewBox="0 0 44 52" fill="none">
                <rect x="10" y="3" width="25" height="45" rx="5" />
                <path d="M18 8h9M20 42h5M3 23h13M8 18l-5 5 5 5" />
              </svg>
            </span>
            <span className="eyebrow">A GUIDED SOURCE CHECK</span>
            <h3>Your backup. A traceable observation.</h3>
            <p>
              Choose a local iPhone backup folder. ENGRAM checks its index and encryption state
              before you decide to begin analysis.
            </p>
          </div>
          <div className="backup-preparation">
            <div>
              <span>01</span>
              <p>
                <strong>Prepare a local backup</strong>
                <small>Use a backup you own or have permission to inspect.</small>
              </p>
            </div>
            <div>
              <span>02</span>
              <p>
                <strong>Select the backup directory</strong>
                <small>Choose the folder containing Manifest.db and Manifest.plist.</small>
              </p>
            </div>
            <div>
              <span>03</span>
              <p>
                <strong>Review its coverage</strong>
                <small>
                  Check availability before starting. Unsupported or missing data remains visible.
                </small>
              </p>
            </div>
          </div>
          {!bridge.native && (
            <p className="backup-unavailable-note">
              Select a real backup in the ENGRAM desktop application. This browser demonstration
              does not read local backups.
            </p>
          )}
        </>
      )}

      {message && (
        <p
          className={`backup-message ${message.error ? 'error' : ''}`}
          role={message.error ? 'alert' : 'status'}
        >
          {message.text}
        </p>
      )}
      {working && (
        <p className="backup-message" role="status">
          {working === 'choosing'
            ? 'Choose a directory in the native picker. Its index will be checked locally…'
            : working === 'scanning'
              ? 'Inspecting the selected backup. Your previous case is retained until analysis succeeds…'
              : 'Clearing the staged source…'}
        </p>
      )}
      <div className="dialog-actions backup-actions">
        <button className="button text" disabled={locked} onClick={() => void close()}>
          Cancel
        </button>
        {inspection ? (
          <>
            <button
              className="button secondary"
              disabled={locked || !bridge.native}
              onClick={() => void choose()}
            >
              Choose another backup
            </button>
            <button
              className="button primary"
              disabled={locked || inspection.state !== 'ready' || !bridge.native}
              onClick={() => void start()}
            >
              {working === 'scanning' ? 'Inspecting…' : 'Start analysis'}
            </button>
          </>
        ) : (
          <button
            className="button primary"
            disabled={locked || !bridge.native}
            onClick={() => void choose()}
          >
            {working === 'choosing' ? 'Checking selection…' : 'Choose backup folder'}
          </button>
        )}
      </div>
    </WorkspaceDialog>
  );
}
