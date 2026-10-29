import { useEffect, useId, useRef, type ReactNode } from 'react';

function tabStops(dialog: HTMLDialogElement, backwards = false): HTMLElement[] {
  const candidates = Array.from(
    dialog.querySelectorAll<HTMLElement>(
      'a[href], button, input, select, textarea, summary, [tabindex], [contenteditable="true"]',
    ),
  ).filter((element) => {
    if (
      element.tabIndex < 0 ||
      element.matches(':disabled') ||
      element.closest('[hidden], [inert]') ||
      element.getClientRects().length === 0
    )
      return false;
    const visibility = getComputedStyle(element).visibility;
    return visibility !== 'hidden' && visibility !== 'collapse';
  });
  // Leave movement within a radio group to the browser. Only its checked radio
  // (or its entry radio when none is checked) participates in the Tab sequence.
  return candidates
    .filter((element) => {
      if (!(element instanceof HTMLInputElement) || element.type !== 'radio' || !element.name)
        return true;
      const group = candidates.filter(
        (candidate): candidate is HTMLInputElement =>
          candidate instanceof HTMLInputElement &&
          candidate.type === 'radio' &&
          candidate.name === element.name &&
          candidate.form === element.form,
      );
      const entry = group.find((radio) => radio.checked) ?? group[backwards ? group.length - 1 : 0];
      return element === entry;
    })
    .sort((left, right) => {
      const leftOrder = left.tabIndex > 0 ? left.tabIndex : Number.MAX_SAFE_INTEGER;
      const rightOrder = right.tabIndex > 0 ? right.tabIndex : Number.MAX_SAFE_INTEGER;
      return leftOrder - rightOrder;
    });
}

export function WorkspaceDialog({
  title,
  onClose,
  children,
  busy = false,
  className = '',
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
  busy?: boolean;
  className?: string;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const label = useId();
  useEffect(() => {
    const previous = document.activeElement;
    const dialog = ref.current;
    dialog?.showModal();
    return () => {
      dialog?.close();
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, []);
  useEffect(() => {
    const dialog = ref.current;
    if (busy && dialog?.open && tabStops(dialog).length === 0) dialog.focus();
  }, [busy]);
  return (
    <dialog
      ref={ref}
      tabIndex={-1}
      className={`workspace-dialog ${className}`.trim()}
      aria-labelledby={label}
      aria-busy={busy}
      onKeyDown={(event) => {
        if (
          event.key !== 'Tab' ||
          event.defaultPrevented ||
          event.altKey ||
          event.ctrlKey ||
          event.metaKey
        )
          return;
        const dialog = event.currentTarget;
        const stops = tabStops(dialog, event.shiftKey);
        if (stops.length === 0) {
          event.preventDefault();
          dialog.focus();
          return;
        }
        const first = stops[0];
        const last = stops[stops.length - 1];
        const active = document.activeElement;
        if (event.shiftKey && (active === first || !stops.includes(active as HTMLElement))) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && (active === last || !stops.includes(active as HTMLElement))) {
          event.preventDefault();
          first.focus();
        }
      }}
      onCancel={(event) => {
        event.preventDefault();
        if (!busy) onClose();
      }}
    >
      <div className="workspace-dialog-heading">
        <h2 id={label}>{title}</h2>
        <button className="icon-button" aria-label="Close dialog" disabled={busy} onClick={onClose}>
          ×
        </button>
      </div>
      {children}
    </dialog>
  );
}
