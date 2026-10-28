import { CATEGORIES, type Category, type Finding } from './model';

// A readable map is a bounded view of the case, never another analysis pass.
// Category totals still describe the complete case; the table contains all matches.
export const ATLAS_CATEGORY_SAMPLE = 2;

const GROUP_POSITIONS: Record<Category, [number, number, number]> = {
  media: [45, 20, 285],
  secret: [670, 20, 285],
  document: [45, 178, 285],
  message: [670, 178, 285],
  browser: [45, 336, 285],
  database: [670, 336, 285],
  location: [365, 20, 270],
};

export interface AtlasMapNode {
  finding: Finding;
  x: number;
  y: number;
  selected: boolean;
  matching: boolean;
}

export interface AtlasMapGroup {
  category: Category;
  x: number;
  y: number;
  width: number;
  total: number;
  matching: number;
  sampled: number;
}

export interface AtlasMapLayout {
  nodes: AtlasMapNode[];
  groups: AtlasMapGroup[];
  edges: [string, string][];
  matchingCount: number;
  sampledMatches: number;
  totalLinks: number;
  selectedOutsideFilter: boolean;
}

export function buildAtlasMap(
  findings: Finding[],
  recordedEdges: [string, string][],
  visible: Finding[],
  selectedId: string | null,
): AtlasMapLayout {
  const index = new Map(findings.map((finding) => [finding.id, finding]));
  const matching = new Set(visible.filter((finding) => index.has(finding.id)).map((row) => row.id));
  const selected = selectedId ? index.get(selectedId) : undefined;
  const uniqueEdges = new Map<string, [string, string]>();
  const connected = new Set<string>();
  const selectedNeighbors = new Set<string>();

  for (const [first, second] of recordedEdges) {
    if (first === second || !index.has(first) || !index.has(second)) continue;
    const pair: [string, string] = first < second ? [first, second] : [second, first];
    uniqueEdges.set(JSON.stringify(pair), pair);
    connected.add(first);
    connected.add(second);
    if (selected?.id === first) selectedNeighbors.add(second);
    if (selected?.id === second) selectedNeighbors.add(first);
  }

  const rank = (finding: Finding) =>
    finding.id === selected?.id
      ? 0
      : selectedNeighbors.has(finding.id)
        ? 1
        : connected.has(finding.id)
          ? 2
          : 3;

  const nodes: AtlasMapNode[] = [];
  const groups = CATEGORIES.map((category): AtlasMapGroup => {
    const rows = findings.filter((finding) => finding.category === category.id);
    const eligible = rows.filter(
      (finding) => matching.has(finding.id) || finding.id === selected?.id,
    );
    // Sort copies only. Equal ranks retain the case's stable source order.
    const sample = eligible
      .sort((first, second) => rank(first) - rank(second))
      .slice(0, ATLAS_CATEGORY_SAMPLE);
    const [x, y, width] = GROUP_POSITIONS[category.id];
    let row = 0;
    for (const finding of sample) {
      const active = finding.id === selected?.id;
      nodes.push({
        finding,
        x: active ? 500 : x + 45,
        y: active ? 250 : y + 70 + row++ * 58,
        selected: active,
        matching: matching.has(finding.id),
      });
    }
    return {
      category: category.id,
      x,
      y,
      width,
      total: rows.length,
      matching: rows.filter((finding) => matching.has(finding.id)).length,
      sampled: sample.filter((finding) => finding.id !== selected?.id).length,
    };
  });
  const nodeIds = new Set(nodes.map((node) => node.finding.id));
  return {
    nodes,
    groups,
    edges: [...uniqueEdges.values()].filter(
      ([first, second]) => nodeIds.has(first) && nodeIds.has(second),
    ),
    matchingCount: matching.size,
    sampledMatches: nodes.filter((node) => node.matching).length,
    totalLinks: uniqueEdges.size,
    selectedOutsideFilter: selected !== undefined && !matching.has(selected.id),
  };
}

export function atlasFindingLabel(finding: Finding): string {
  const basename = finding.path.split(/[\\/]/).filter(Boolean).pop();
  return basename || finding.title;
}
