import { useEffect, useMemo, useState, type ReactNode } from 'react';
import * as bridge from './bridge';
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
  map: ['M13 2 5 13h6l-1 9 9-13h-7l1-7Z'],
  findings: ['M5 4h14v16H5zM8 8h8M8 12h8M8 16h5'],
  timeline: ['M6 3v18M3 6h6M3 12h6M3 18h6M12 6h8M12 12h5M12 18h8'],
  connections: [
    'M5 6h.01M19 6h.01M12 18h.01M6 7l5 9M18 7l-5 9M7 6h10',
    'M7 6a2 2 0 1 0-4 0 2 2 0 0 0 4 0ZM21 6a2 2 0 1 0-4 0 2 2 0 0 0 4 0ZM14 18a2 2 0 1 0-4 0 2 2 0 0 0 4 0Z',
  ],
  compare: ['M3 5h7v14H3zM14 5h7v14h-7zM6 9h1M17 9h1M6 13h1M17 13h1'],
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
            placeholder="Search a name, path or rule…"
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

const clusterPositions: Record<Category, [number, number, number, number]> = {
  media: [150, 145, 150, 78],
  secret: [550, 145, 550, 78],
  document: [150, 345, 150, 414],
  browser: [350, 365, 350, 434],
  message: [550, 345, 550, 414],
  location: [350, 105, 350, 38],
  database: [350, 245, 350, 311],
};

function jitter(id: string, salt: number): number {
  let value = salt;
  for (let index = 0; index < id.length; index += 1)
    value = ((value << 5) - value + id.charCodeAt(index)) | 0;
  return ((value >>> 0) % 1000) / 1000;
}

function TraceMap({
  caseData,
  edges,
  visible,
  selected,
  onSelect,
  onCategory,
}: {
  caseData: Case;
  edges: [string, string][];
  visible: Finding[];
  selected: Finding | null;
  onSelect: (finding: Finding) => void;
  onCategory: (category: Category) => void;
}) {
  const data = useMemo(() => {
    const points = new Map<string, { x: number; y: number; finding: Finding }>();
    const groups = CATEGORIES.map((category) => {
      const findings = caseData.findings.filter((finding) => finding.category === category.id);
      const [x, y] = clusterPositions[category.id];
      const sampled = findings.slice(0, 9);
      if (
        selected?.category === category.id &&
        !sampled.some((finding) => finding.id === selected.id) &&
        findings.some((finding) => finding.id === selected.id)
      )
        sampled[sampled.length - 1] = selected;
      sampled.forEach((finding, index) => {
        const angle = index * 2.39996;
        const radius = index === 0 ? 0 : 12 + 13 * Math.sqrt(index);
        points.set(finding.id, {
          finding,
          x: x + Math.cos(angle) * radius + jitter(finding.id, 13) * 6,
          y: y + Math.sin(angle) * radius + jitter(finding.id, 17) * 6,
        });
      });
      return { ...category, findings };
    });
    return { points, groups };
  }, [caseData, selected]);
  const visibleIds = useMemo(() => new Set(visible.map((finding) => finding.id)), [visible]);
  const visibleEdges = edges.filter(([a, b]) => data.points.has(a) && data.points.has(b));
  const selectedEdges = selected ? new Set(selected.related_ids) : new Set<string>();
  return (
    <section className="panel map-panel" aria-label="Evidence trace map">
      <div className="panel-heading">
        <div>
          <span className="eyebrow">TRACE MAP / LOCAL EVIDENCE</span>
          <p>Seven categories. Follow their recorded connections.</p>
        </div>
        <span className="map-status">
          <span className="status-dot" />
          Evidence mapped
        </span>
      </div>
      <svg
        className="trace-map"
        viewBox="0 0 700 485"
        role="group"
        aria-label={`Interactive map of ${caseData.findings.length} findings. Select a category or a finding.`}
      >
        <defs>
          <pattern id="map-grid" x="0" y="0" width="24" height="24" patternUnits="userSpaceOnUse">
            <path d="M1 0v4M0 1h4" stroke="#bcc2cc" strokeWidth=".6" opacity=".4" />
          </pattern>
        </defs>
        <rect x="5" y="5" width="690" height="475" fill="url(#map-grid)" />
        <g aria-hidden="true" className="signal-guides">
          <path className="signal-bolt" d="M386 56 266 245h73l-45 185 149-220h-78l21-154Z" />
          <path className="signal-rule" d="M28 145h644M28 345h644M150 64v360M550 64v360" />
          <path
            className="signal-register"
            d="M48 35H23v25M652 35h25v25M23 421v25h25M677 421v25h-25"
          />
          <text className="signal-reference" x="28" y="240">
            SOURCE
          </text>
          <text className="signal-reference" x="672" y="240" textAnchor="end">
            LOCAL / 01
          </text>
        </g>
        <g aria-hidden="true">
          {visibleEdges.map(([a, b]) => {
            const first = data.points.get(a)!;
            const second = data.points.get(b)!;
            const active = selected?.id === a || selected?.id === b;
            return (
              <path
                key={JSON.stringify([a, b])}
                className={`evidence-line ${active ? 'selected' : ''}`}
                d={`M${first.x},${first.y}H${(first.x + second.x) / 2}V${second.y}H${second.x}`}
              />
            );
          })}
        </g>
        {data.groups.map((category) => {
          const [x, y, labelX, labelY] = clusterPositions[category.id];
          const active = category.findings.some((finding) => visibleIds.has(finding.id));
          return (
            <g key={category.id} className={`map-cluster ${active ? '' : 'dimmed'}`}>
              <rect
                className="cluster-halo"
                x={x - 47}
                y={y - 44}
                width="94"
                height="88"
                fill={category.color}
              />
              <g
                role="button"
                tabIndex={0}
                className="category-map-label"
                aria-label={`${category.label}, ${category.findings.length} findings. Filter category.`}
                onClick={() => onCategory(category.id)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    onCategory(category.id);
                  }
                }}
              >
                <rect
                  x={labelX - 58}
                  y={labelY - 13}
                  width="116"
                  height="32"
                  rx="0"
                  fill="transparent"
                />
                <text x={labelX} y={labelY} textAnchor="middle">
                  {category.label}
                  <tspan className="map-count" dx="6">
                    {category.findings.length}
                  </tspan>
                </text>
                <line
                  x1={labelX}
                  y1={labelY + 8}
                  x2={labelX}
                  y2={labelY + 15}
                  stroke={category.color}
                />
              </g>
            </g>
          );
        })}
        {[...data.points.entries()].map(([id, point]) => {
          const info = categoryInfo(point.finding.category);
          const active = selected?.id === id;
          const linked = selectedEdges.has(id);
          return (
            <g
              key={id}
              role="button"
              tabIndex={0}
              className={`map-node ${active ? 'selected' : ''} ${visibleIds.has(id) ? '' : 'dimmed'}`}
              aria-label={`Inspect ${point.finding.title}`}
              aria-pressed={active}
              onClick={() => onSelect(point.finding)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  onSelect(point.finding);
                }
              }}
            >
              <title>{point.finding.title}</title>
              <circle className="node-hit" cx={point.x} cy={point.y} r="11" fill="transparent" />
              {(active || linked) && (
                <rect
                  x={point.x - (active ? 9 : 7)}
                  y={point.y - (active ? 9 : 7)}
                  width={active ? 18 : 14}
                  height={active ? 18 : 14}
                  transform={`rotate(45 ${point.x} ${point.y})`}
                  fill="none"
                  stroke={active ? '#246BFE' : info.color}
                  strokeWidth="1.5"
                />
              )}
              <rect
                className="node-core"
                x={point.x - 3.5}
                y={point.y - 3.5}
                width="7"
                height="7"
                transform={`rotate(45 ${point.x} ${point.y})`}
                fill={info.color}
              />
            </g>
          );
        })}
        <text x="350" y="475" textAnchor="middle" className="map-caption">
          OBSERVED ARTIFACTS · RECORDED RELATIONSHIPS
        </text>
      </svg>
      <div className="map-foot">
        <span>
          <strong>{visible.length.toLocaleString('en')}</strong> findings match your filters
        </span>
        <span>
          {data.points.size} representative nodes · {edges.length} total links
        </span>
      </div>
      <p className="map-note">
        Select a node to inspect its evidence. Large sources show a bounded sample; category counts
        include every finding.
      </p>
    </section>
  );
}

function FindingsTable({
  findings,
  selected,
  onSelect,
  label = 'Findings',
}: {
  findings: Finding[];
  selected: Finding | null;
  onSelect: (finding: Finding) => void;
  label?: string;
}) {
  const [page, setPage] = useState(0);
  useEffect(() => setPage(0), [findings]);
  const currentPage = Math.min(page, Math.max(0, Math.ceil(findings.length / 50) - 1));
  if (!findings.length)
    return (
      <Empty title="No findings match">
        Try another category, confidence label or search term. Missing results describe this view,
        not complete device coverage.
      </Empty>
    );
  return (
    <section className="panel findings-panel" aria-label={label}>
      <div className="table-heading">
        <span className="eyebrow">{label.toUpperCase()}</span>
        <span>{findings.length.toLocaleString('en')} records</span>
      </div>
      <div className="finding-table" role="table" aria-label={label}>
        <div className="finding-table-head" role="row">
          <span role="columnheader">Artifact / source</span>
          <span role="columnheader">Category</span>
          <span role="columnheader">Confidence</span>
        </div>
        {findings.slice(currentPage * 50, currentPage * 50 + 50).map((finding) => (
          <button
            role="row"
            key={finding.id}
            className={`finding-row ${selected?.id === finding.id ? 'selected' : ''}`}
            onClick={() => onSelect(finding)}
            aria-label={`Inspect ${finding.title}`}
          >
            <span role="cell" className="artifact-cell">
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
            </span>
            <span role="cell" className="category-cell">
              {categoryInfo(finding.category).label}
            </span>
            <span role="cell">
              <Tag state={finding.confidence}>{finding.confidence}</Tag>
            </span>
          </button>
        ))}
      </div>
      <Pager count={findings.length} page={currentPage} setPage={setPage} />
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
  return (
    <div className="compare-view">
      <section className="panel comparison-intro">
        <div className="comparison-glyph">
          <Icon name="compare" size={30} />
        </div>
        <div>
          <span className="eyebrow">TWO OBSERVATIONS</span>
          <h2>A before. An after. The evidence between.</h2>
          <p>
            Choose a saved case as the baseline. The open case is the current observation. Existing
            case files stay unchanged.
          </p>
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
      </section>
      {comparison ? (
        <>
          <div className="comparison-sources">
            <div>
              <span className="eyebrow">BASELINE</span>
              <strong>{comparison.before.title}</strong>
              <small>{dateLabel(comparison.before.completed_at)}</small>
            </div>
            <span aria-hidden="true">→</span>
            <div>
              <span className="eyebrow">CURRENT</span>
              <strong>{caseData.title}</strong>
              <small>{dateLabel(caseData.completed_at)}</small>
            </div>
          </div>
          <div className="diff-tabs">
            {(['added', 'removed', 'changed', 'unchanged'] as const).map((kind) => (
              <button
                key={kind}
                aria-pressed={change === kind}
                onClick={() => setChange(kind)}
                className={change === kind ? 'active' : ''}
              >
                <span>{kind}</span>
                <strong>{comparison.diff[kind].length}</strong>
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
            Absent from a snapshot does not establish secure deletion. Scan bounds, source dates and
            reader coverage can differ.
          </p>
        </>
      ) : (
        <Empty
          title={
            bridge.native
              ? 'Choose your first baseline'
              : 'Comparison is available in the desktop app'
          }
        >
          Saved cases are compared by the Rust engine. No comparison result is simulated in this
          browser preview.
        </Empty>
      )}
    </div>
  );
}

function Modules({ caseData }: { caseData: Case }) {
  return (
    <div className="modules-view">
      <section className="coverage-summary">
        <Icon name="shield" size={28} />
        <div>
          <span className="eyebrow">COVERAGE, MADE VISIBLE</span>
          <h2>Every source has its limits.</h2>
          <p>
            Ready describes the implemented reader. Experimental coverage depends on the supplied
            format and version.
          </p>
        </div>
      </section>
      <div className="module-grid">
        {caseData.modules.map((module, index) => (
          <article className="panel module-card" key={`${module.name}-${index}`}>
            <div>
              <span className="module-index">{String(index + 1).padStart(2, '0')}</span>
              <Tag state={module.state}>{module.state}</Tag>
            </div>
            <h3>{module.name}</h3>
            <p>{module.detail}</p>
          </article>
        ))}
      </div>
      <section className="panel coverage-panel">
        <div className="panel-heading">
          <div>
            <span className="eyebrow">INSPECTION NOTES</span>
            <p>
              {caseData.stats.skipped_files} files skipped · {caseData.warnings.length} coverage
              notes
            </p>
          </div>
        </div>
        {caseData.warnings.length ? (
          <ul>
            {caseData.warnings.map((warning, index) => (
              <li key={index}>{warning}</li>
            ))}
          </ul>
        ) : (
          <p className="no-warnings">
            No coverage warnings were reported for this run. This does not establish complete device
            coverage.
          </p>
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
        <span className="eyebrow">EVIDENCE INSPECTOR</span>
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
            <div className="inspector-tags">
              <Tag state={finding.category}>{categoryInfo(finding.category).label}</Tag>
              {source === 'baseline' && <Tag>Baseline</Tag>}
            </div>
            <h2>{finding.title}</h2>
            <p>{finding.summary}</p>
          </div>
          <div className="inspector-section">
            <span className="eyebrow">CLASSIFICATION</span>
            <div className="classification">
              <Tag state={finding.confidence}>{finding.confidence}</Tag>
              {finding.sensitive && <Tag state="sensitive">Sensitive metadata</Tag>}
            </div>
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
            <span className="eyebrow">RELATED TRACES / {finding.related_ids.length}</span>
            {related.length ? (
              related.map((other) => (
                <button key={other.id} className="related-item" onClick={() => onSelect(other)}>
                  <span
                    className="dot"
                    style={{ background: categoryInfo(other.category).color }}
                  />
                  <span>{other.title}</span>
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
          <h3>Every trace has a story.</h3>
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
          <img src="/mascot.png" alt="ENGRAM cobalt lightning creature" />
          <span>ENGRAM</span>
          <small>LOCAL TRACES / {runtime?.version ?? '0.2.0'}</small>
        </div>
        <nav className="main-nav" aria-label="Workspace views">
          {VIEWS.map((item, index) => (
            <button
              key={item.id}
              className={view === item.id ? 'active' : ''}
              aria-current={view === item.id ? 'page' : undefined}
              onClick={() => setView(item.id)}
            >
              <span className="nav-number">{String(index + 1).padStart(2, '0')}</span>
              <Icon name={item.id} size={17} />
              <span>{item.name}</span>
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
          <span className="sidebar-version">DEVELOPER PREVIEW</span>
        </div>
      </aside>
      <div className="workspace">
        <header className="topbar">
          <div className="breadcrumb">
            <span>ENGRAM</span>
            <i>/</i>
            <span>Trace desk</span>
          </div>
          <div className="top-actions">
            <Tag state={bridge.native ? 'offline' : 'preview'}>
              {bridge.native ? 'Local workspace' : 'Browser preview · synthetic data'}
            </Tag>
            {caseData?.source_kind === 'demo' && bridge.native && (
              <Tag state="preview">Synthetic demo</Tag>
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
            <div className="view-heading">
              <div>
                <span className="eyebrow">YOUR DIGITAL EVIDENCE</span>
                <h1>{activeView.subtitle}</h1>
                <p>{activeView.description}</p>
              </div>
              <span className="view-number">
                {String(VIEWS.findIndex((item) => item.id === view) + 1).padStart(2, '0')}
                <small>/ 06</small>
              </span>
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
                <section className="stats-strip" aria-label="Case overview">
                  <div>
                    <span className="eyebrow">TRACES RECORDED</span>
                    <strong>{caseData.findings.length.toLocaleString('en')}</strong>
                    <small>
                      Across{' '}
                      {
                        CATEGORIES.filter((category) =>
                          caseData.findings.some((finding) => finding.category === category.id),
                        ).length
                      }{' '}
                      categories
                    </small>
                  </div>
                  <div>
                    <span className="eyebrow">FILES EXAMINED</span>
                    <strong>{caseData.stats.files_examined.toLocaleString('en')}</strong>
                    <small>{bytesLabel(caseData.stats.bytes_examined)} inspected</small>
                  </div>
                  <div>
                    <span className="eyebrow">CONNECTIONS</span>
                    <strong>{edges.length.toLocaleString('en')}</strong>
                    <small>Unique evidence links</small>
                  </div>
                  <div>
                    <span className="eyebrow">COVERAGE NOTES</span>
                    <strong>{caseData.warnings.length.toLocaleString('en')}</strong>
                    <small>{caseData.stats.skipped_files} files skipped</small>
                  </div>
                </section>
                {(view === 'map' ||
                  view === 'findings' ||
                  view === 'timeline' ||
                  view === 'connections') && (
                  <FilterBar filters={filters} setFilters={setFilters} />
                )}
                {view === 'map' && (
                  <>
                    <TraceMap
                      caseData={caseData}
                      edges={edges}
                      visible={filtered}
                      selected={selected}
                      onSelect={select}
                      onCategory={categorySelect}
                    />
                    <div className="desk-bottom">
                      <div>
                        <span className="eyebrow">FIELD NOTE / 001</span>
                        <p>A remaining copy is a finding. Its history still needs evidence.</p>
                      </div>
                      <button className="button text" onClick={() => setView('modules')}>
                        Inspect coverage <Icon name="arrow" size={15} />
                      </button>
                    </div>
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
                ENGRAM / schema {caseData?.schema_version ?? runtime?.schema_version ?? 1} /{' '}
                {bridge.native ? 'Rust + Tauri' : 'React preview'}
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
