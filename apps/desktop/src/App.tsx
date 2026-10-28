import { useEffect, useMemo, useState, type ReactNode } from 'react';
import * as bridge from './bridge';
import { AtlasTraceMap } from './AtlasTraceMap';
import {
  bytesLabel,
  chronological,
  dateLabel,
  diffFindings,
  evidenceEdges,
  filterFindings,
  INITIAL_FILTERS,
  sourceLabel,
  timestamp,
  type Change,
  type Filters,
} from './data';
import {
  CATEGORIES,
  VIEWS,
  asView,
  categoryInfo,
  type Case,
  type Category,
  type Comparison,
  type Finding,
  type RuntimeInfo,
  type View,
} from './model';

type IconName =
  | 'map'
  | 'findings'
  | 'timeline'
  | 'connections'
  | 'compare'
  | 'modules'
  | 'folder'
  | 'open'
  | 'save'
  | 'search'
  | 'arrow'
  | 'shield'
  | 'close'
  | 'copy'
  | 'check';
const paths: Record<IconName, string[]> = {
  map: [
    'M6 5a2 2 0 1 1-4 0 2 2 0 0 1 4 0ZM14 12a2 2 0 1 1-4 0 2 2 0 0 1 4 0ZM22 5a2 2 0 1 1-4 0 2 2 0 0 1 4 0ZM6 21a2 2 0 1 1-4 0 2 2 0 0 1 4 0ZM22 21a2 2 0 1 1-4 0 2 2 0 0 1 4 0ZM6 6l4 4M18 6l-4 4M6 19l4-5M18 19l-4-5',
  ],
  findings: ['M5 4h14v16H5zM8 8h8M8 12h8M8 16h5'],
  timeline: ['M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM12 6v6l4 2'],
  connections: [
    'M5 6h.01M19 6h.01M12 18h.01M6 7l5 9M18 7l-5 9M7 6h10',
    'M7 6a2 2 0 1 0-4 0 2 2 0 0 0 4 0ZM21 6a2 2 0 1 0-4 0 2 2 0 0 0 4 0ZM14 18a2 2 0 1 0-4 0 2 2 0 0 0 4 0Z',
  ],
  compare: ['M4 8h15M15 4l4 4-4 4M20 16H5M9 12l-4 4 4 4'],
  modules: ['M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z'],
  folder: ['M3 6h7l2 2h9v12H3zM3 6V4h7l2 2'],
  open: ['M3 20V6h7l2 2h9v3M3 20l4-9h15l-4 9H3Z'],
  save: ['M4 3h13l3 3v15H4zM8 3v6h8V3M8 21v-8h8v8'],
  search: ['M10.5 17a6.5 6.5 0 1 0 0-13 6.5 6.5 0 0 0 0 13ZM16 16l5 5'],
  arrow: ['M4 12h15M14 7l5 5-5 5'],
  shield: ['M12 3l8 3v6c0 4-4 7-8 9-4-2-8-5-8-9V6l8-3Z', 'M8 12l3 3 5-6'],
  close: ['M6 6l12 12M18 6 6 18'],
  copy: ['M8 8h12v13H8zM16 8V3H3v13h5'],
  check: ['M5 12l4 4 10-10'],
};

function Icon({ name, size = 18 }: { name: IconName; size?: number }) {
  return (
    <svg
      aria-hidden="true"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {paths[name].map((path, index) => (
        <path key={index} d={path} />
      ))}
    </svg>
  );
}

function Tag({ children, state = '' }: { children: ReactNode; state?: string }) {
  return <span className={`tag ${state}`}>{children}</span>;
}
function Empty({
  title,
  children,
  action,
}: {
  title: string;
  children: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="empty-state">
      <div className="empty-mark">
        <Icon name="map" size={30} />
      </div>
      <h3>{title}</h3>
      <p>{children}</p>
      {action}
    </div>
  );
}

function Pager({
  count,
  page,
  setPage,
  size = 50,
}: {
  count: number;
  page: number;
  setPage: (page: number) => void;
  size?: number;
}) {
  const pages = Math.max(1, Math.ceil(count / size));
  return (
    <div className="pagination">
      <span>
        {count === 0
          ? '0 records'
          : `${page * size + 1}–${Math.min((page + 1) * size, count)} of ${count.toLocaleString('en')} records`}
      </span>
      <div>
        <button
          className="button text small"
          disabled={page === 0}
          onClick={() => setPage(page - 1)}
        >
          Previous
        </button>
        <span className="page-number">
          {page + 1} / {pages}
        </span>
        <button
          className="button text small"
          disabled={page + 1 >= pages}
          onClick={() => setPage(page + 1)}
        >
          Next
        </button>
      </div>
    </div>
  );
}

function FilterBar({
  filters,
  setFilters,
  showLinked = true,
}: {
  filters: Filters;
  setFilters: (filters: Filters) => void;
  showLinked?: boolean;
}) {
  return (
    <div className="filters">
      <div className="filter-top">
        <label className="search-field">
          <Icon name="search" />
          <input
            aria-label="Search findings"
            placeholder="Search findings"
            value={filters.query}
            onChange={(event) => setFilters({ ...filters, query: event.target.value })}
          />
          {filters.query && (
            <button
              className="icon-button"
              title="Clear search"
              aria-label="Clear search"
              onClick={() => setFilters({ ...filters, query: '' })}
            >
              <Icon name="close" size={14} />
            </button>
          )}
        </label>
        <select
          aria-label="Filter confidence"
          value={filters.confidence}
          onChange={(event) =>
            setFilters({ ...filters, confidence: event.target.value as Filters['confidence'] })
          }
        >
          <option value="all">All confidence</option>
          <option value="observed">Observed</option>
          <option value="linked">Linked</option>
          <option value="candidate">Candidate</option>
        </select>
        {showLinked && (
          <label className="checkbox">
            <input
              type="checkbox"
              checked={filters.linkedOnly}
              onChange={(event) => setFilters({ ...filters, linkedOnly: event.target.checked })}
            />
            Linked only
          </label>
        )}
      </div>
      <div className="category-tabs" aria-label="Filter category">
        <button
          className={filters.category === 'all' ? 'active' : ''}
          aria-pressed={filters.category === 'all'}
          onClick={() => setFilters({ ...filters, category: 'all' })}
        >
          All traces
        </button>
        {CATEGORIES.map((category) => (
          <button
            key={category.id}
            className={filters.category === category.id ? 'active' : ''}
            aria-pressed={filters.category === category.id}
            onClick={() => setFilters({ ...filters, category: category.id })}
          >
            <span className="dot" style={{ background: category.color }} />
            {category.label}
          </button>
        ))}
      </div>
    </div>
  );
}

function CategoryGlyph({ category, size = 20 }: { category: Category; size?: number }) {
  const info = categoryInfo(category);
  const symbols: Record<Category, string[]> = {
    media: ['M4 4h16v16H4zM4 16l5-5 4 4 3-3 4 4', 'M9 8h.01'],
    secret: ['M8 11V8a4 4 0 0 1 8 0v3M6 11h12v10H6zM12 15v2'],
    document: ['M6 3h8l4 4v14H6zM14 3v5h4M9 12h6M9 16h6'],
    browser: ['M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM3 12h18M12 3c5 5 5 13 0 18-5-5-5-13 0-18Z'],
    message: ['M4 4h16v13H9l-5 4V4ZM8 8h8M8 12h5'],
    location: ['M19 10c0 5-7 11-7 11S5 15 5 10a7 7 0 0 1 14 0ZM15 10a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z'],
    database: [
      'M20 6c0 2-4 3-8 3S4 8 4 6s4-3 8-3 8 1 8 3ZM4 6v12c0 2 4 3 8 3s8-1 8-3V6M4 12c0 2 4 3 8 3s8-1 8-3',
    ],
  };
  return (
    <span className={`category-mark category-${category}`} style={{ color: info.color }}>
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        {symbols[category].map((path, index) => (
          <path d={path} key={index} />
        ))}
      </svg>
    </span>
  );
}

function findingTitle(finding: Finding) {
  return finding.title.replace(/^(SQLite database|Media file|Document or archive):\s*/, '');
}

function FindingsTable({
  findings,
  selected,
  onSelect,
  label = 'Findings',
  compact = false,
  onViewAll,
}: {
  findings: Finding[];
  selected: Finding | null;
  onSelect: (finding: Finding) => void;
  label?: string;
  compact?: boolean;
  onViewAll?: () => void;
}) {
  const [page, setPage] = useState(0);
  const size = compact ? 3 : 10;
  useEffect(() => setPage(0), [findings]);
  const currentPage = Math.min(page, Math.max(0, Math.ceil(findings.length / size) - 1));
  if (!findings.length)
    return (
      <Empty title="No findings match">
        Try another category, confidence label or search term. Results describe the supplied source.
      </Empty>
    );
  return (
    <section
      className={`panel findings-panel ${compact ? 'compact-findings' : ''}`}
      aria-label={label}
    >
      <div className="table-heading">
        <h2>
          {label} <span>({findings.length.toLocaleString('en')})</span>
        </h2>
        {compact ? (
          <span>Artifacts matching your filters.</span>
        ) : (
          <span>Every row retains its source evidence.</span>
        )}
        {compact && onViewAll && (
          <button className="button text small" onClick={onViewAll}>
            View all <Icon name="arrow" size={14} />
          </button>
        )}
      </div>
      <div className="finding-table" role="table" aria-label={label}>
        <div className="finding-table-head" role="row">
          <span role="columnheader">Name</span>
          <span role="columnheader">Category</span>
          <span role="columnheader" className="path-cell">
            Source
          </span>
          <span role="columnheader">Confidence</span>
          <span role="columnheader" className="links-cell">
            Links
          </span>
        </div>
        {findings
          .slice(compact ? 0 : currentPage * size, compact ? size : currentPage * size + size)
          .map((finding) => (
            <button
              role="row"
              key={finding.id}
              className={`finding-row ${selected?.id === finding.id ? 'selected' : ''}`}
              onClick={() => onSelect(finding)}
              aria-label={`Inspect ${finding.title}`}
            >
              <span role="cell" className="artifact-cell">
                <CategoryGlyph category={finding.category} />
                <span>
                  <strong title={finding.title}>{findingTitle(finding)}</strong>
                  <small title={finding.path}>{finding.path}</small>
                </span>
              </span>
              <span role="cell" className="category-cell">
                <span
                  className="dot"
                  style={{ background: categoryInfo(finding.category).color }}
                />
                {categoryInfo(finding.category).label}
              </span>
              <span role="cell" className="path-cell mono" title={finding.path}>
                {finding.path}
              </span>
              <span role="cell">
                <Tag state={finding.confidence}>{finding.confidence}</Tag>
              </span>
              <span role="cell" className="links-cell">
                {finding.related_ids.length}
              </span>
            </button>
          ))}
      </div>
      {!compact && (
        <Pager count={findings.length} page={currentPage} setPage={setPage} size={size} />
      )}
    </section>
  );
}

function Timeline({
  findings,
  onSelect,
  selected,
}: {
  findings: Finding[];
  onSelect: (finding: Finding) => void;
  selected: Finding | null;
}) {
  const [newest, setNewest] = useState(true);
  const [page, setPage] = useState(0);
  const ordered = useMemo(() => chronological(findings, newest), [findings, newest]);
  useEffect(() => setPage(0), [ordered]);
  const currentPage = Math.min(page, Math.max(0, Math.ceil(ordered.length / 50) - 1));
  return (
    <section className="panel timeline-panel">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">SOURCE CHRONOLOGY</span>
          <p>
            {ordered.length} dated findings · {findings.length - ordered.length} without a usable
            timestamp
          </p>
        </div>
        <button className="button text small" onClick={() => setNewest(!newest)}>
          {newest ? 'Newest first' : 'Oldest first'} <span aria-hidden="true">↕</span>
        </button>
      </div>
      {ordered.length === 0 ? (
        <Empty title="No dates to place">
          The current findings do not contain a supported timestamp. ENGRAM does not invent dates to
          fill the timeline.
        </Empty>
      ) : (
        <>
          <div className="timeline-list">
            {ordered
              .slice(currentPage * 50, currentPage * 50 + 50)
              .map((finding, index, pageFindings) => {
                const day = timestamp(finding.observed_at)?.slice(0, 10);
                const previous =
                  index > 0 ? timestamp(pageFindings[index - 1].observed_at)?.slice(0, 10) : '';
                return (
                  <div className="timeline-entry" key={finding.id}>
                    {day !== previous && (
                      <h3 className="timeline-date">{dateLabel(finding.observed_at, false)}</h3>
                    )}
                    <button
                      className={`timeline-card ${selected?.id === finding.id ? 'selected' : ''}`}
                      onClick={() => onSelect(finding)}
                    >
                      <span
                        className="timeline-dot"
                        style={{ background: categoryInfo(finding.category).color }}
                      />
                      <span className="timeline-time">
                        {dateLabel(finding.observed_at).split(', ')[1] ??
                          dateLabel(finding.observed_at)}
                      </span>
                      <span className="timeline-artifact">
                        <strong>{finding.title}</strong>
                        <small>{finding.path}</small>
                      </span>
                      <Tag state={finding.confidence}>{finding.confidence}</Tag>
                    </button>
                  </div>
                );
              })}
          </div>
          <Pager count={ordered.length} page={currentPage} setPage={setPage} />
        </>
      )}
    </section>
  );
}

function Connections({
  caseData,
  visible,
  selected,
  onSelect,
}: {
  caseData: Case;
  visible: Finding[];
  selected: Finding | null;
  onSelect: (finding: Finding) => void;
}) {
  const [page, setPage] = useState(0);
  const index = useMemo(
    () => new Map(caseData.findings.map((finding) => [finding.id, finding])),
    [caseData],
  );
  const visibleIds = useMemo(() => new Set(visible.map((finding) => finding.id)), [visible]);
  const edges = useMemo(
    () =>
      evidenceEdges(caseData.findings).filter(([a, b]) => visibleIds.has(a) || visibleIds.has(b)),
    [caseData, visibleIds],
  );
  useEffect(() => setPage(0), [edges]);
  const currentPage = Math.min(page, Math.max(0, Math.ceil(edges.length / 30) - 1));
  return (
    <section className="panel connections-panel">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">RECORDED CONNECTIONS</span>
          <p>{edges.length} unique links touching the filtered findings</p>
        </div>
        <Tag>Evidence linked</Tag>
      </div>
      {!edges.length ? (
        <Empty title="No recorded connections">
          The analysis engine did not establish a relationship for these findings. Similar-looking
          paths alone do not create a link.
        </Empty>
      ) : (
        <>
          <div className="connection-list">
            {edges.slice(currentPage * 30, currentPage * 30 + 30).map(([a, b]) => {
              const first = index.get(a)!;
              const second = index.get(b)!;
              const identical = first.sha256 !== null && first.sha256 === second.sha256;
              return (
                <div className="connection-card" key={JSON.stringify([a, b])}>
                  <div className="connection-rule">
                    <span className="dot" />
                    {identical ? 'Exact content match' : 'Engine-recorded relationship'}
                    <span>{identical ? 'SHA-256' : 'Inspect evidence'}</span>
                  </div>
                  <div className="connection-pair">
                    {[first, second].map((finding, i) => (
                      <div className="connection-end" key={finding.id}>
                        {i === 1 && (
                          <span className="connection-symbol" aria-hidden="true">
                            ↔
                          </span>
                        )}
                        <button
                          className={selected?.id === finding.id ? 'selected' : ''}
                          onClick={() => onSelect(finding)}
                        >
                          <span
                            className="category-mark"
                            style={{ color: categoryInfo(finding.category).color }}
                          >
                            {categoryInfo(finding.category).short}
                          </span>
                          <span>
                            <strong>{finding.title}</strong>
                            <small title={finding.path}>{finding.path}</small>
                          </span>
                          <Icon name="arrow" size={16} />
                        </button>
                      </div>
                    ))}
                  </div>
                </div>
              );
            })}
          </div>
          <Pager count={edges.length} page={currentPage} setPage={setPage} size={30} />
        </>
      )}
    </section>
  );
}

function Compare({
  caseData,
  comparison,
  onCompare,
  busy,
  selected,
  onSelect,
}: {
  caseData: Case;
  comparison: Comparison | null;
  onCompare: () => void;
  busy: boolean;
  selected: Finding | null;
  onSelect: (finding: Finding, baseline?: boolean) => void;
}) {
  const [change, setChange] = useState<Change>('added');
  const records = useMemo(
    () => (comparison ? diffFindings(comparison.diff, change, comparison.before, caseData) : []),
    [comparison, change, caseData],
  );
  const changes = [
    { id: 'added', label: 'Added', description: 'Present in current only' },
    { id: 'removed', label: 'Removed', description: 'Present in baseline only' },
    { id: 'changed', label: 'Changed', description: 'Different between cases' },
    { id: 'unchanged', label: 'Unchanged', description: 'Same in both cases' },
  ] as const;
  return (
    <div className="compare-view">
      <section className="comparison-sources" aria-label="Comparison sources">
        <div className="panel source-observation current-observation">
          <span className="eyebrow">
            <span className="status-dot" />
            Current observation
          </span>
          <div className="observation-title">
            <Icon name="folder" size={30} />
            <div>
              <h2>{caseData.title}</h2>
              <p>{sourceLabel(caseData.source_kind)}</p>
            </div>
          </div>
          <div className="observation-counts">
            <span>
              <strong>{caseData.findings.length}</strong> findings
            </span>
            <span>
              <strong>{caseData.stats.files_examined}</strong> files
            </span>
          </div>
        </div>
        <span className="comparison-glyph" aria-hidden="true">
          <Icon name="compare" size={25} />
        </span>
        <div className="panel source-observation">
          <span className="eyebrow">
            <span className="status-dot muted-dot" />
            Baseline
          </span>
          <div className="observation-title">
            <Icon name="folder" size={30} />
            <div>
              <h2>{comparison?.before.title ?? 'No case selected'}</h2>
              <p>
                {comparison
                  ? dateLabel(comparison.before.completed_at)
                  : 'Choose a saved case to compare.'}
              </p>
            </div>
          </div>
          <button
            className="button primary"
            disabled={!bridge.native || busy}
            title={!bridge.native ? 'Case selection is available in the desktop app.' : undefined}
            onClick={onCompare}
          >
            <Icon name="open" />
            {comparison ? 'Change baseline' : 'Choose baseline'}
          </button>
        </div>
      </section>
      {comparison ? (
        <>
          <div className="diff-tabs">
            {changes.map((item) => (
              <button
                key={item.id}
                aria-pressed={change === item.id}
                onClick={() => setChange(item.id)}
                className={change === item.id ? 'active' : ''}
              >
                <span>{item.label}</span>
                <strong>{comparison.diff[item.id].length}</strong>
                <small>{item.description}</small>
              </button>
            ))}
          </div>
          <FindingsTable
            findings={records}
            selected={selected}
            onSelect={(finding) => onSelect(finding, change === 'removed')}
            label={`${change} findings`}
          />
          <p className="view-note">
            Comparison describes these snapshots. Source dates, read limits and supported readers
            can differ.
          </p>
        </>
      ) : (
        <section className="panel baseline-empty">
          <div className="baseline-illustration" aria-hidden="true">
            <Icon name="findings" size={54} />
            <Icon name="compare" size={24} />
            <Icon name="findings" size={54} />
          </div>
          <h2>
            {bridge.native
              ? 'Choose your first baseline'
              : 'Comparison is available in the desktop app'}
          </h2>
          <p>
            {bridge.native
              ? 'Choose a saved case to compare it with the current observation.'
              : 'Saved cases are compared by the Rust engine in the desktop app.'}
          </p>
          <div className="change-legend">
            {changes.map((item) => (
              <div key={item.id} className={item.id}>
                <span className="dot" />
                <strong>{item.label}</strong>
                <span>{item.description}</span>
              </div>
            ))}
          </div>
          <p className="view-note">
            <Icon name="shield" size={16} />
            Comparison describes the supplied snapshots. Existing case files stay unchanged.
          </p>
        </section>
      )}
    </div>
  );
}

function Modules({ caseData }: { caseData: Case }) {
  const groups = [
    { state: 'ready', title: 'Implemented modules' },
    { state: 'experimental', title: 'Experimental modules' },
    { state: 'planned', title: 'Planned modules' },
    { state: 'unavailable', title: 'Unavailable for this source' },
  ] as const;
  return (
    <div className="modules-view">
      {groups.map((group) => {
        const items = caseData.modules.filter((module) => module.state === group.state);
        if (!items.length) return null;
        return (
          <section className="module-group" key={group.state}>
            <h2>
              {group.title} <span>({items.length})</span>
            </h2>
            <div className="module-grid">
              {items.map((module, index) => (
                <article className="panel module-card" key={`${module.name}-${index}`}>
                  <div className={`module-symbol ${module.state}`}>
                    <Icon name={group.state === 'planned' ? 'modules' : 'shield'} size={24} />
                  </div>
                  <div className="module-description">
                    <div>
                      <h3>{module.name}</h3>
                      <Tag state={module.state}>{module.state}</Tag>
                    </div>
                    <p>{module.detail}</p>
                  </div>
                </article>
              ))}
            </div>
          </section>
        );
      })}
      <section className="panel coverage-panel">
        <div className="panel-heading">
          <div>
            <h2>
              Inspection notes <span>({caseData.warnings.length})</span>
            </h2>
            <p>
              {caseData.stats.skipped_files} files skipped · coverage depends on the supplied format
              and version.
            </p>
          </div>
          <Icon name="shield" size={20} />
        </div>
        {caseData.warnings.length ? (
          <ul>
            {caseData.warnings.map((warning, index) => (
              <li key={index}>{warning}</li>
            ))}
          </ul>
        ) : (
          <p className="no-warnings">No coverage warnings were reported for this source.</p>
        )}
      </section>
    </div>
  );
}

function Inspector({
  finding,
  source,
  caseData,
  onSelect,
  onCopy,
  onClose,
}: {
  finding: Finding | null;
  source: 'current' | 'baseline';
  caseData: Case | null;
  onSelect: (finding: Finding) => void;
  onCopy: (text: string) => void;
  onClose: () => void;
}) {
  const index = useMemo(
    () => new Map(caseData?.findings.map((item) => [item.id, item]) ?? []),
    [caseData],
  );
  const related = useMemo(
    () =>
      finding
        ? finding.related_ids.flatMap((id) => {
            const other = index.get(id);
            return other ? [other] : [];
          })
        : [],
    [finding, index],
  );
  return (
    <aside
      className={`inspector ${finding ? 'has-selection' : ''}`}
      aria-label="Evidence inspector"
    >
      <div className="inspector-label">
        <h2>Evidence inspector</h2>
        {finding && (
          <button
            className="icon-button mobile-close"
            aria-label="Close inspector"
            onClick={onClose}
          >
            <Icon name="close" />
          </button>
        )}
      </div>
      {finding ? (
        <>
          <div className="inspector-intro">
            <CategoryGlyph category={finding.category} size={30} />
            <div className="inspector-title">
              <h3 title={finding.title}>{findingTitle(finding)}</h3>
              <div className="inspector-tags">
                <Tag state={finding.category}>{categoryInfo(finding.category).label}</Tag>
                <Tag state={finding.confidence}>{finding.confidence}</Tag>
                {source === 'baseline' && <Tag>Baseline</Tag>}
                {finding.sensitive && <Tag state="sensitive">Sensitive metadata</Tag>}
              </div>
            </div>
          </div>
          <div className="inspector-section">
            <span className="eyebrow">ABOUT THIS FINDING</span>
            <p className="classification-summary">{finding.summary}</p>
          </div>
          <div className="inspector-section">
            <span className="eyebrow">SOURCE FILE</span>
            <p className="mono source-path">{finding.path}</p>
            <button
              className="button text small copy-button"
              onClick={() => onCopy(finding.path)}
              disabled={!navigator.clipboard}
              title={
                !navigator.clipboard
                  ? 'Clipboard is unavailable in this environment.'
                  : 'Copy source path'
              }
            >
              <Icon name="copy" size={13} />
              Copy path
            </button>
            <dl className="evidence-facts">
              <dt>Locator</dt>
              <dd>{finding.locator || 'Not supplied'}</dd>
              <dt>Rule</dt>
              <dd className="mono">{finding.rule_id}</dd>
              <dt>Origin</dt>
              <dd>{finding.origin}</dd>
              <dt>Source timestamp</dt>
              <dd>{dateLabel(finding.observed_at)}</dd>
              <dt>Recorded size</dt>
              <dd>{bytesLabel(finding.bytes)}</dd>
              <dt>SHA-256</dt>
              <dd className="mono digest">{finding.sha256 ?? 'Not hashed in this snapshot'}</dd>
            </dl>
          </div>
          {finding.evidence.length > 0 && (
            <div className="inspector-section">
              <span className="eyebrow">RECORDED FACTS</span>
              <dl className="evidence-facts">
                {finding.evidence.map((fact, index) => (
                  <div key={index}>
                    <dt>{fact.label}</dt>
                    <dd>{fact.value}</dd>
                  </div>
                ))}
              </dl>
            </div>
          )}
          <div className="inspector-section related-section">
            <h3>
              Related findings <span>({related.length})</span>
            </h3>
            {related.length ? (
              related.map((other) => (
                <button key={other.id} className="related-item" onClick={() => onSelect(other)}>
                  <CategoryGlyph category={other.category} size={16} />
                  <span title={other.title}>{findingTitle(other)}</span>
                  <Icon name="arrow" size={14} />
                </button>
              ))
            ) : (
              <p className="muted">No related finding recorded in this case.</p>
            )}
          </div>
          <div className="inspector-boundary">
            <Icon name="shield" size={15} />
            <span>Findings describe this source. Candidate labels require review.</span>
          </div>
        </>
      ) : (
        <div className="inspector-empty">
          <Icon name="findings" size={34} />
          <h3>Select a finding</h3>
          <p>Select a finding or a node to inspect its source, rule and recorded evidence.</p>
        </div>
      )}
    </aside>
  );
}

export function App() {
  const [runtime, setRuntime] = useState<RuntimeInfo | null>(null);
  const [caseData, setCaseData] = useState<Case | null>(null);
  const [comparison, setComparison] = useState<Comparison | null>(null);
  const [view, setView] = useState<View>('map');
  const [selected, setSelected] = useState<Finding | null>(null);
  const [selectionSource, setSelectionSource] = useState<'current' | 'baseline'>('current');
  const [filters, setFilters] = useState<Filters>(INITIAL_FILTERS);
  const [busy, setBusy] = useState<string | null>('Preparing the trace desk');
  const [notice, setNotice] = useState<{ kind: 'error' | 'success' | 'info'; text: string } | null>(
    null,
  );
  const filtered = useMemo(
    () => filterFindings(caseData?.findings ?? [], filters),
    [caseData, filters],
  );
  const edges = useMemo(() => evidenceEdges(caseData?.findings ?? []), [caseData]);
  const activeView = VIEWS.find((item) => item.id === view)!;

  function receiveCase(next: Case) {
    setCaseData(next);
    setComparison(null);
    setSelected(next.findings[0] ?? null);
    setSelectionSource('current');
    setFilters(INITIAL_FILTERS);
  }
  function select(finding: Finding, baseline = false) {
    setSelected(finding);
    setSelectionSource(baseline ? 'baseline' : 'current');
  }

  useEffect(() => {
    let active = true;
    void (async () => {
      try {
        const info = await bridge.runtimeInfo();
        const demonstration = await bridge.loadDemo();
        if (!active) return;
        setRuntime(info);
        setView(asView(info.initial_view));
        receiveCase(demonstration);
        if (info.smoke_test)
          requestAnimationFrame(() =>
            requestAnimationFrame(() => {
              setTimeout(() => {
                void bridge.finishSmokeTest().catch((error) => {
                  if (active) setNotice({ kind: 'error', text: String(error) });
                });
              }, 500);
            }),
          );
      } catch (error) {
        if (active) setNotice({ kind: 'error', text: String(error) });
      } finally {
        if (active) setBusy(null);
      }
    })();
    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (notice?.kind !== 'success') return;
    const timeout = setTimeout(() => setNotice(null), 6000);
    return () => clearTimeout(timeout);
  }, [notice]);

  async function changeCase(label: string, action: () => Promise<Case | null>) {
    if (busy) return;
    setBusy(label);
    setNotice(null);
    try {
      const next = await action();
      if (next) {
        receiveCase(next);
        setNotice({
          kind: 'success',
          text: `${sourceLabel(next.source_kind)} opened. ${next.findings.length} findings recorded.`,
        });
      } else setNotice({ kind: 'info', text: 'Selection cancelled. Your open case is unchanged.' });
    } catch (error) {
      setNotice({ kind: 'error', text: String(error) });
    } finally {
      setBusy(null);
    }
  }
  async function save() {
    if (busy || !caseData) return;
    setBusy('Saving a separate case file');
    setNotice(null);
    try {
      const destination = await bridge.saveCase();
      setNotice({
        kind: destination ? 'success' : 'info',
        text: destination ? `Case saved to ${destination}` : 'Save cancelled. No file was written.',
      });
    } catch (error) {
      setNotice({ kind: 'error', text: String(error) });
    } finally {
      setBusy(null);
    }
  }
  async function compare() {
    if (busy || !caseData) return;
    setBusy('Opening the baseline');
    setNotice(null);
    try {
      const result = await bridge.compareCase();
      if (result) {
        setComparison(result);
        setSelected(null);
        setSelectionSource('current');
        setNotice({ kind: 'success', text: 'Baseline compared with the open case.' });
      } else setNotice({ kind: 'info', text: 'Baseline selection cancelled.' });
    } catch (error) {
      setNotice({ kind: 'error', text: String(error) });
    } finally {
      setBusy(null);
    }
  }
  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      setNotice({ kind: 'success', text: 'Source path copied.' });
    } catch {
      setNotice({
        kind: 'error',
        text: 'The clipboard is unavailable. Select and copy the source path manually.',
      });
    }
  }
  function categorySelect(category: Category) {
    setFilters({ ...filters, category });
    const first = caseData?.findings.find((finding) => finding.category === category);
    setSelected(first ?? null);
    setSelectionSource('current');
  }
  const inspectorCase = selectionSource === 'baseline' ? (comparison?.before ?? null) : caseData;
  const nativeTitle = !bridge.native ? 'Available in the ENGRAM desktop application.' : undefined;

  return (
    <div className="app-shell" data-view={view}>
      <aside className="sidebar">
        <div className="brand">
          <img src="/mascot-atlas.png" alt="ENGRAM cobalt lightning creature" />
          <div>
            <span>ENGRAM</span>
            <small>Find what remains.</small>
          </div>
        </div>
        <nav className="main-nav" aria-label="Workspace views">
          {VIEWS.map((item, index) => (
            <button
              key={item.id}
              className={view === item.id ? 'active' : ''}
              aria-current={view === item.id ? 'page' : undefined}
              onClick={() => setView(item.id)}
            >
              <Icon name={item.id} size={22} />
              <span className="nav-label">{item.name}</span>
              <span className="nav-number">{String(index + 1).padStart(2, '0')}</span>
              {item.id === 'findings' && caseData && <small>{caseData.findings.length}</small>}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="source-card">
            <span className="eyebrow">SOURCE</span>
            <strong>{caseData ? sourceLabel(caseData.source_kind) : 'No case open'}</strong>
            <p>
              {caseData?.source_kind === 'demo'
                ? 'Generated test data. No personal files inspected.'
                : caseData
                  ? caseData.title
                  : 'Choose a source explicitly to begin an inspection.'}
            </p>
            <button
              className="button primary"
              disabled={!bridge.native || !!busy}
              title={nativeTitle}
              onClick={() => void changeCase('Inspecting the selected source', bridge.selectSource)}
            >
              <Icon name="folder" size={16} />
              Choose source
            </button>
            <button
              className="demo-link"
              disabled={!!busy}
              onClick={() => void changeCase('Generating a synthetic case', bridge.loadDemo)}
            >
              Load demonstration <span>↗</span>
            </button>
          </div>
          <div className="privacy-stamp">
            <Icon name="shield" size={18} />
            <div>
              <strong>Offline by design</strong>
              <span>Your sources stay on your device.</span>
            </div>
          </div>
          <span className="sidebar-version">
            {bridge.native ? 'Desktop application' : 'Browser demonstration'} ·{' '}
            {runtime?.version ?? '0.4.0'}
          </span>
        </div>
      </aside>
      <div className="workspace">
        <header className="topbar">
          <div className="source-heading">
            <span className="source-heading-icon">
              <Icon name="folder" size={24} />
            </span>
            <div>
              <strong title={caseData?.title}>{caseData?.title ?? 'Trace desk'}</strong>
              <span>
                {bridge.native ? 'Local workspace' : 'Browser demonstration'}
                {caseData?.source_kind === 'demo' ? ' · Synthetic demo' : ''}
              </span>
            </div>
          </div>
          {caseData && (
            <section className="stats-strip" aria-label="Case overview">
              <div>
                <strong>{caseData.findings.length.toLocaleString('en')}</strong>
                <span>findings</span>
              </div>
              <div>
                <strong>{caseData.stats.files_examined.toLocaleString('en')}</strong>
                <span>files</span>
              </div>
              <div>
                <strong>{edges.length.toLocaleString('en')}</strong>
                <span>connections</span>
              </div>
              <div>
                <strong>{caseData.warnings.length.toLocaleString('en')}</strong>
                <span>notes</span>
              </div>
            </section>
          )}

          <div className="top-actions">
            <Tag state={bridge.native ? 'offline' : 'demo'}>
              {bridge.native ? 'Local workspace' : 'Browser demo · synthetic data'}
            </Tag>
            {caseData?.source_kind === 'demo' && bridge.native && (
              <Tag state="demo">Synthetic demo</Tag>
            )}
            <button
              className="button secondary"
              disabled={!bridge.native || !!busy}
              title={nativeTitle}
              onClick={() => void changeCase('Opening a saved case', bridge.openCase)}
            >
              <Icon name="open" size={16} />
              Open case
            </button>
            <button
              className="button secondary"
              disabled={!bridge.native || !!busy || !caseData}
              title={nativeTitle}
              onClick={() => void save()}
            >
              <Icon name="save" size={16} />
              Save case
            </button>
          </div>
        </header>
        <div className="workspace-body">
          <main className="main-content" id="main-content">
            <div
              className={`view-heading ${['map', 'findings', 'timeline', 'connections'].includes(view) ? 'with-filters' : ''}`}
            >
              <div className="view-heading-copy">
                <h1>{activeView.name}</h1>
                <p>{activeView.description}</p>
              </div>
              {caseData &&
                (view === 'map' ||
                  view === 'findings' ||
                  view === 'timeline' ||
                  view === 'connections') && (
                  <FilterBar filters={filters} setFilters={setFilters} />
                )}
            </div>

            {notice && (
              <div
                className={`notice ${notice.kind}`}
                role={notice.kind === 'error' ? 'alert' : 'status'}
              >
                <Icon name={notice.kind === 'success' ? 'check' : 'shield'} size={17} />
                <span>{notice.text}</span>
                <button
                  className="icon-button"
                  aria-label="Dismiss notification"
                  onClick={() => setNotice(null)}
                >
                  <Icon name="close" size={15} />
                </button>
              </div>
            )}
            {busy && (
              <div className="operation-status" role="status" aria-live="polite">
                <span className="spinner" />
                <strong>{busy}</strong>
                <span>Local analysis · source files stay unchanged</span>
              </div>
            )}
            {caseData ? (
              <>
                {view === 'map' && (
                  <>
                    <AtlasTraceMap
                      caseData={caseData}
                      edges={edges}
                      visible={filtered}
                      selected={selected}
                      onSelect={select}
                      onCategory={categorySelect}
                    />
                    <FindingsTable
                      findings={filtered}
                      selected={selected}
                      onSelect={select}
                      compact
                      onViewAll={() => setView('findings')}
                    />
                  </>
                )}
                {view === 'findings' && (
                  <FindingsTable findings={filtered} selected={selected} onSelect={select} />
                )}
                {view === 'timeline' && (
                  <Timeline findings={filtered} selected={selected} onSelect={select} />
                )}
                {view === 'connections' && (
                  <Connections
                    caseData={caseData}
                    visible={filtered}
                    selected={selected}
                    onSelect={select}
                  />
                )}
                {view === 'compare' && (
                  <Compare
                    caseData={caseData}
                    comparison={comparison}
                    onCompare={() => void compare()}
                    busy={!!busy}
                    selected={selected}
                    onSelect={select}
                  />
                )}
                {view === 'modules' && <Modules caseData={caseData} />}
              </>
            ) : (
              !busy && (
                <Empty
                  title="The trace desk is waiting"
                  action={
                    <button
                      className="button primary"
                      onClick={() =>
                        void changeCase('Generating a synthetic case', bridge.loadDemo)
                      }
                    >
                      Load synthetic demonstration
                    </button>
                  }
                >
                  The case could not be loaded. Review the message above, or explicitly load the
                  synthetic demonstration.
                </Empty>
              )
            )}
            <footer className="workspace-footer">
              <span>
                <span className="status-dot" />
                {caseData ? sourceLabel(caseData.source_kind) : 'No source selected'}
              </span>
              <span>
                ENGRAM / case format {caseData?.schema_version ?? runtime?.schema_version ?? 1} /{' '}
                {bridge.native ? 'Offline analysis' : 'Browser demo'}
              </span>
            </footer>
          </main>
          <Inspector
            finding={selected}
            source={selectionSource}
            caseData={inspectorCase}
            onSelect={(finding) => select(finding, selectionSource === 'baseline')}
            onCopy={(text) => void copy(text)}
            onClose={() => setSelected(null)}
          />
        </div>
      </div>
    </div>
  );
}
