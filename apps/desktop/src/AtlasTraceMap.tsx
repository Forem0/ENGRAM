import {
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
} from 'react';
import { atlasFindingLabel, buildAtlasMap, type AtlasMapNode } from './atlas-map-layout';
import {
  ATLAS_MAX_ZOOM,
  ATLAS_MIN_ZOOM,
  ATLAS_VIEW_HEIGHT,
  ATLAS_VIEW_WIDTH,
  fitAtlasSelection,
  panAtlasViewport,
  projectAtlasPoint,
  resetAtlasViewport,
  revealAtlasBounds,
  zoomAtlasViewport,
  type AtlasBounds,
  type AtlasPoint,
  type AtlasViewport,
} from './atlas-viewport';
import { categoryInfo, type Case, type Category, type Finding } from './model';
import './atlas-trace-map.css';

interface AtlasTraceMapProps {
  caseData: Case;
  edges: [string, string][];
  visible: Finding[];
  selected: Finding | null;
  onSelect: (finding: Finding) => void;
  onCategory: (category: Category) => void;
}

const ICONS: Record<Category, string[]> = {
  media: ['M4 4h16v16H4zM5 16l5-6 4 4 3-3 3 4', 'M16 8h.01'],
  secret: ['M15 3a6 6 0 1 1-4 10l-6 6H2v-3l6-6a6 6 0 0 1 7-7Z', 'M16 7h.01'],
  document: ['M6 3h8l4 4v14H6zM14 3v5h4M9 12h6M9 16h6'],
  browser: ['M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM3 12h18', 'M12 3c-5 5-5 13 0 18 5-5 5-13 0-18Z'],
  message: ['M4 4h16v13H9l-5 4V4ZM8 8h8M8 12h5'],
  location: ['M19 10c0 5-7 11-7 11S5 15 5 10a7 7 0 1 1 14 0ZM15 10a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z'],
  database: ['M20 6c0 2-16 2-16 0s16-2 16 0ZM4 6v12c0 3 16 3 16 0V6M4 12c0 3 16 3 16 0'],
};

function NodeIcon({ category, x, y }: { category: Category; x: number; y: number }) {
  return (
    <svg x={x - 13} y={y - 13} width="26" height="26" viewBox="0 0 24 24" aria-hidden="true">
      {ICONS[category].map((path, index) => (
        <path
          key={index}
          d={path}
          fill="none"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      ))}
    </svg>
  );
}

function truncate(value: string, length: number): string {
  return value.length <= length ? value : `${value.slice(0, length - 1)}…`;
}

function nodeBounds(node: AtlasMapNode): AtlasBounds {
  return {
    x: node.selected ? node.x - 120 : node.x - 25,
    y: node.y - (node.selected ? 30 : 27),
    width: node.selected ? 240 : 220,
    height: node.selected ? 60 : 54,
  };
}

function canvasPoint(svg: SVGSVGElement, clientX: number, clientY: number): AtlasPoint | null {
  const bounds = svg.getBoundingClientRect();
  if (!bounds.width || !bounds.height) return null;
  return {
    x: ((clientX - bounds.left) * ATLAS_VIEW_WIDTH) / bounds.width,
    y: ((clientY - bounds.top) * ATLAS_VIEW_HEIGHT) / bounds.height,
  };
}

export function AtlasTraceMap({
  caseData,
  edges,
  visible,
  selected,
  onSelect,
  onCategory,
}: AtlasTraceMapProps) {
  const pattern = `atlas-dots-${useId().replace(/:/g, '')}`;
  const svgRef = useRef<SVGSVGElement | null>(null);
  const selectedRef = useRef<SVGGElement | null>(null);
  const dragRef = useRef<{ pointerId: number; start: AtlasPoint; viewport: AtlasViewport } | null>(
    null,
  );
  const [viewport, setViewport] = useState(resetAtlasViewport);
  const [dragging, setDragging] = useState(false);
  const layout = useMemo(
    () => buildAtlasMap(caseData.findings, edges, visible, selected?.id ?? null),
    [caseData.findings, edges, visible, selected?.id],
  );
  const pointIndex = useMemo(
    () => new Map(layout.nodes.map((node) => [node.finding.id, node])),
    [layout.nodes],
  );
  const activeNode = layout.nodes.find((node) => node.selected);
  const zoomPercent = Math.round(viewport.zoom * 100);

  useEffect(() => {
    setViewport(resetAtlasViewport());
    const drag = dragRef.current;
    dragRef.current = null;
    setDragging(false);
    if (drag && svgRef.current?.hasPointerCapture(drag.pointerId))
      svgRef.current.releasePointerCapture(drag.pointerId);
  }, [caseData.id]);

  useEffect(() => {
    if (activeNode) setViewport((current) => revealAtlasBounds(current, nodeBounds(activeNode)));
  }, [activeNode]);

  useEffect(() => {
    const svg = svgRef.current;
    if (!svg) return;
    const wheel = (event: WheelEvent) => {
      // Ordinary scrolling belongs to the page; only an explicit modifier zooms.
      if ((!event.ctrlKey && !event.metaKey) || !Number.isFinite(event.deltaY)) return;
      const anchor = canvasPoint(svg, event.clientX, event.clientY);
      if (!anchor) return;
      event.preventDefault();
      event.stopPropagation();
      const unit =
        event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? svg.getBoundingClientRect().height : 1;
      const delta = Math.max(-600, Math.min(600, event.deltaY * unit));
      setViewport((current) =>
        zoomAtlasViewport(current, current.zoom * Math.exp(-delta * 0.002), anchor),
      );
    };
    svg.addEventListener('wheel', wheel, { passive: false });
    return () => svg.removeEventListener('wheel', wheel);
  }, []);

  function startDrag(event: PointerEvent<SVGSVGElement>) {
    if (event.button !== 0 || !event.isPrimary) return;
    if (event.target instanceof Element && event.target.closest('[data-atlas-interactive]')) return;
    const start = canvasPoint(event.currentTarget, event.clientX, event.clientY);
    if (!start) return;
    dragRef.current = { pointerId: event.pointerId, start, viewport };
    event.currentTarget.setPointerCapture(event.pointerId);
    event.currentTarget.focus({ preventScroll: true });
    setDragging(true);
    event.preventDefault();
  }

  function moveDrag(event: PointerEvent<SVGSVGElement>) {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    const point = canvasPoint(event.currentTarget, event.clientX, event.clientY);
    if (!point) return;
    setViewport(
      panAtlasViewport(drag.viewport, { x: point.x - drag.start.x, y: point.y - drag.start.y }),
    );
  }

  function endDrag(event: PointerEvent<SVGSVGElement>) {
    if (dragRef.current?.pointerId !== event.pointerId) return;
    dragRef.current = null;
    setDragging(false);
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId);
  }

  function navigateMap(event: KeyboardEvent<SVGSVGElement>) {
    if (event.altKey || event.ctrlKey || event.metaKey || event.nativeEvent.isComposing) return;
    const delta: Record<string, AtlasPoint> = {
      ArrowLeft: { x: 45, y: 0 },
      ArrowRight: { x: -45, y: 0 },
      ArrowUp: { x: 0, y: 45 },
      ArrowDown: { x: 0, y: -45 },
    };
    if (event.key === '0' || event.key === 'Home') {
      event.preventDefault();
      setViewport(resetAtlasViewport());
    } else if (delta[event.key]) {
      event.preventDefault();
      setViewport((current) => panAtlasViewport(current, delta[event.key]));
    } else if (event.key === '+' || event.key === '=' || event.key === '-') {
      event.preventDefault();
      const id =
        event.target instanceof Element
          ? event.target.closest('[data-atlas-node]')?.getAttribute('data-atlas-node')
          : null;
      const node = id ? pointIndex.get(id) : undefined;
      setViewport((current) =>
        zoomAtlasViewport(
          current,
          current.zoom * (event.key === '-' ? 1 / 1.25 : 1.25),
          node ? projectAtlasPoint(current, node) : undefined,
        ),
      );
    }
    // Enter and Space remain the existing category / evidence selection actions.
  }

  function fitSelected() {
    if (!activeNode) return;
    setViewport(fitAtlasSelection(nodeBounds(activeNode)));
    selectedRef.current?.focus({ preventScroll: true });
  }

  return (
    <section className="atlas-map-panel" aria-label="Evidence trace map">
      <div className="atlas-map-toolbar">
        <span className="atlas-map-toolbar-label">Evidence canvas</span>
        <div className="atlas-map-tools" role="toolbar" aria-label="Map view controls">
          <button
            type="button"
            className="atlas-tool-button"
            aria-label="Zoom out"
            title="Zoom out (−)"
            disabled={viewport.zoom <= ATLAS_MIN_ZOOM + 0.001}
            onClick={() =>
              setViewport((current) => zoomAtlasViewport(current, current.zoom / 1.25))
            }
          >
            −
          </button>
          <button
            type="button"
            className="atlas-tool-button atlas-zoom-reset"
            aria-label={`Reset map view to 100 percent; current zoom ${zoomPercent} percent`}
            title="Reset map view (0)"
            onClick={() => setViewport(resetAtlasViewport())}
          >
            {zoomPercent}%
          </button>
          <button
            type="button"
            className="atlas-tool-button"
            aria-label="Zoom in"
            title="Zoom in (+)"
            disabled={viewport.zoom >= ATLAS_MAX_ZOOM - 0.001}
            onClick={() =>
              setViewport((current) => zoomAtlasViewport(current, current.zoom * 1.25))
            }
          >
            +
          </button>
          <span className="atlas-toolbar-divider" aria-hidden="true" />
          <button
            type="button"
            className="atlas-tool-button atlas-fit-selection"
            disabled={!activeNode}
            onClick={fitSelected}
            title={
              activeNode ? `Focus ${activeNode.finding.title}` : 'Select a finding on the map first'
            }
          >
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              aria-hidden="true"
            >
              <path d="M4 9V4h5M15 4h5v5M20 15v5h-5M9 20H4v-5M8 12h8M12 8v8" />
            </svg>
            Fit selection
          </button>
        </div>
      </div>
      <div className="atlas-map-canvas">
        <svg
          id={`${pattern}-canvas`}
          ref={svgRef}
          className={`atlas-map ${dragging ? 'is-dragging' : ''}`}
          viewBox={`0 0 ${ATLAS_VIEW_WIDTH} ${ATLAS_VIEW_HEIGHT}`}
          role="group"
          tabIndex={0}
          aria-describedby={`${pattern}-help`}
          aria-label={`Trace map showing ${layout.sampledMatches} representative matches from ${layout.matchingCount} findings. Category counts include the complete case.`}
          onPointerDown={startDrag}
          onPointerMove={moveDrag}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
          onLostPointerCapture={endDrag}
          onKeyDown={navigateMap}
        >
          <defs>
            <pattern id={pattern} x="0" y="0" width="22" height="22" patternUnits="userSpaceOnUse">
              <circle cx="1" cy="1" r="0.8" fill="#3b5068" opacity=".58" />
            </pattern>
          </defs>
          <rect width="1000" height="500" fill={`url(#${pattern})`} />
          <g
            className="atlas-map-layer"
            transform={`translate(${viewport.x} ${viewport.y}) scale(${viewport.zoom})`}
          >
            <g className="atlas-map-groups">
              {layout.groups
                .filter((group) => group.total > 0 || group.category !== 'location')
                .map((group) => {
                  const category = categoryInfo(group.category);
                  return (
                    <g
                      key={group.category}
                      className={`atlas-map-group ${group.matching ? '' : 'is-muted'}`}
                    >
                      <rect
                        className="atlas-group-surface"
                        x={group.x}
                        y={group.y + 14}
                        width={group.width}
                        height="142"
                        rx="16"
                      />
                      <g
                        role="button"
                        tabIndex={0}
                        data-atlas-interactive="category"
                        className="atlas-group-label"
                        aria-label={`${category.label}: ${group.total} findings, ${group.matching} match the current filters. Filter category.`}
                        onClick={() => onCategory(group.category)}
                        onFocus={() =>
                          setViewport((current) =>
                            revealAtlasBounds(current, {
                              x: group.x + 8,
                              y: group.y,
                              width: 190,
                              height: 42,
                            }),
                          )
                        }
                        onKeyDown={(event) => {
                          if (event.key === 'Enter' || event.key === ' ') {
                            event.preventDefault();
                            onCategory(group.category);
                          }
                        }}
                      >
                        <title>{`${category.label}: ${group.total} total, ${group.matching} matching`}</title>
                        <rect
                          className="atlas-group-heading"
                          x={group.x + 8}
                          y={group.y}
                          width="190"
                          height="42"
                          rx="10"
                        />
                        <circle cx={group.x + 30} cy={group.y + 21} r="6" fill={category.color} />
                        <text x={group.x + 46} y={group.y + 27}>
                          {category.label}
                          <tspan className="atlas-group-count" dx="10">
                            {group.total}
                          </tspan>
                        </text>
                      </g>
                      {group.sampled === 0 && (
                        <text className="atlas-group-empty" x={group.x + 24} y={group.y + 99}>
                          {activeNode?.finding.category === group.category
                            ? 'Selected at center'
                            : group.total
                              ? 'No matching findings'
                              : 'No findings recorded'}
                        </text>
                      )}
                    </g>
                  );
                })}
            </g>
            <g className="atlas-map-lines" aria-hidden="true">
              {layout.edges.map(([firstId, secondId]) => {
                const first = pointIndex.get(firstId)!;
                const second = pointIndex.get(secondId)!;
                const active = first.selected || second.selected;
                const color = categoryInfo(
                  first.selected ? second.finding.category : first.finding.category,
                ).color;
                const mid = (first.x + second.x) / 2;
                const sameColumn = Math.abs(first.x - second.x) < 50;
                const curve = sameColumn
                  ? `M ${first.x} ${first.y} C ${first.x - 55} ${first.y}, ${second.x - 55} ${second.y}, ${second.x} ${second.y}`
                  : `M ${first.x} ${first.y} C ${mid} ${first.y}, ${mid} ${second.y}, ${second.x} ${second.y}`;
                return (
                  <path
                    key={JSON.stringify([firstId, secondId])}
                    d={curve}
                    stroke={color}
                    className={active ? 'is-active' : ''}
                  />
                );
              })}
            </g>
            {layout.nodes.map((node) => {
              const category = categoryInfo(node.finding.category);
              const label = atlasFindingLabel(node.finding);
              const textX = node.selected ? node.x - 35 : node.x + 36;
              const iconX = node.selected ? node.x - 77 : node.x;
              return (
                <g
                  key={node.finding.id}
                  ref={node.selected ? selectedRef : undefined}
                  role="button"
                  tabIndex={0}
                  data-atlas-interactive="finding"
                  data-atlas-node={node.finding.id}
                  className={`atlas-map-node ${node.selected ? 'is-selected' : ''} ${node.matching ? '' : 'is-context'}`}
                  aria-label={`Inspect ${node.finding.title}${node.matching ? '' : ', selected finding outside the current filters'}`}
                  aria-pressed={node.selected}
                  style={{ color: node.selected ? '#3991ff' : category.color }}
                  onClick={() => onSelect(node.finding)}
                  onFocus={() =>
                    setViewport((current) => revealAtlasBounds(current, nodeBounds(node)))
                  }
                  onKeyDown={(event) => {
                    if (event.key === 'Enter' || event.key === ' ') {
                      event.preventDefault();
                      onSelect(node.finding);
                    }
                  }}
                >
                  <title>{`${node.finding.title}\n${node.finding.path}${node.matching ? '' : '\nSelected finding is outside the current filters'}`}</title>
                  <rect
                    className="atlas-node-hit"
                    x={node.selected ? node.x - 120 : node.x - 25}
                    y={node.y - (node.selected ? 30 : 27)}
                    width={node.selected ? 240 : 220}
                    height={node.selected ? 60 : 54}
                    rx="12"
                  />
                  <rect
                    className="atlas-node-icon"
                    x={iconX - 23}
                    y={node.y - 23}
                    width="46"
                    height="46"
                    rx="12"
                  />
                  <NodeIcon category={node.finding.category} x={iconX} y={node.y} />
                  <text className="atlas-node-name" x={textX} y={node.y - 2}>
                    {truncate(label, node.selected ? 17 : 21)}
                  </text>
                  <text className="atlas-node-kind" x={textX} y={node.y + 20}>
                    {node.matching ? category.label : 'Outside current filters'}
                  </text>
                </g>
              );
            })}
            {activeNode &&
              layout.edges.every(
                ([first, second]) =>
                  first !== activeNode.finding.id && second !== activeNode.finding.id,
              ) && (
                <text className="atlas-no-links" x="500" y="313" textAnchor="middle">
                  No sampled links for this finding
                </text>
              )}
            {!layout.matchingCount && (
              <text className="atlas-no-matches" x="500" y="385" textAnchor="middle">
                No findings match the current filters
              </text>
            )}
          </g>
        </svg>
      </div>
      <div className="atlas-map-footer">
        <span>
          <strong>{layout.sampledMatches}</strong> of {layout.matchingCount.toLocaleString('en')}{' '}
          matching findings shown
          {layout.selectedOutsideFilter ? ' · selected finding retained for context' : ''}
        </span>
        <span>
          {layout.edges.length} of {layout.totalLinks.toLocaleString('en')} recorded links shown
        </span>
      </div>
      <p className="atlas-map-note">
        Select a file to inspect its evidence. Category counts include the complete case; the map
        shows a bounded sample.
      </p>
      <p className="atlas-map-help" id={`${pattern}-help`}>
        Drag the background to pan. Ctrl/⌘ + scroll to zoom. With the map focused: + / − zoom,
        arrows pan, 0 resets.
      </p>
    </section>
  );
}
