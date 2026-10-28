import { useId, useMemo } from 'react';
import { atlasFindingLabel, buildAtlasMap } from './atlas-map-layout';
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

export function AtlasTraceMap({
  caseData,
  edges,
  visible,
  selected,
  onSelect,
  onCategory,
}: AtlasTraceMapProps) {
  const pattern = `atlas-dots-${useId().replace(/:/g, '')}`;
  const layout = useMemo(
    () => buildAtlasMap(caseData.findings, edges, visible, selected?.id ?? null),
    [caseData.findings, edges, visible, selected?.id],
  );
  const pointIndex = useMemo(
    () => new Map(layout.nodes.map((node) => [node.finding.id, node])),
    [layout.nodes],
  );
  const activeNode = layout.nodes.find((node) => node.selected);

  return (
    <section className="atlas-map-panel" aria-label="Evidence trace map">
      <div className="atlas-map-canvas">
        <svg
          className="atlas-map"
          viewBox="0 0 1000 500"
          role="group"
          aria-label={`Trace map showing ${layout.sampledMatches} representative matches from ${layout.matchingCount} findings. Category counts include the complete case.`}
        >
          <defs>
            <pattern id={pattern} x="0" y="0" width="22" height="22" patternUnits="userSpaceOnUse">
              <circle cx="1" cy="1" r="0.8" fill="#3b5068" opacity=".58" />
            </pattern>
          </defs>
          <rect width="1000" height="500" fill={`url(#${pattern})`} />
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
                      className="atlas-group-label"
                      aria-label={`${category.label}: ${group.total} findings, ${group.matching} match the current filters. Filter category.`}
                      onClick={() => onCategory(group.category)}
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
                role="button"
                tabIndex={0}
                className={`atlas-map-node ${node.selected ? 'is-selected' : ''} ${node.matching ? '' : 'is-context'}`}
                aria-label={`Inspect ${node.finding.title}${node.matching ? '' : ', selected finding outside the current filters'}`}
                aria-pressed={node.selected}
                style={{ color: node.selected ? '#3991ff' : category.color }}
                onClick={() => onSelect(node.finding)}
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
    </section>
  );
}
