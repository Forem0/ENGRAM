import { describe, expect, it } from 'vitest';
import { ATLAS_CATEGORY_SAMPLE, buildAtlasMap } from './atlas-map-layout';
import { CATEGORIES, type Finding } from './model';

function finding(id: string, overrides: Partial<Finding> = {}): Finding {
  return {
    id,
    category: 'document',
    title: 'Archive notes',
    path: 'documents/archive.txt',
    origin: 'selected source',
    summary: 'Observed text document',
    locator: 'file',
    observed_at: null,
    bytes: 40,
    sha256: null,
    confidence: 'observed',
    rule_id: 'inventory.document',
    sensitive: false,
    evidence: [],
    related_ids: [],
    ...overrides,
  };
}

describe('Atlas map evidence boundaries', () => {
  it('shows only supplied real edge pairs, ignoring dangling, duplicate and self references', () => {
    const rows = [finding('a', { related_ids: ['b', 'c'] }), finding('b'), finding('c')];
    const layout = buildAtlasMap(
      rows,
      [
        ['b', 'a'],
        ['a', 'b'],
        ['a', 'missing'],
        ['a', 'a'],
      ],
      rows,
      'a',
    );
    expect(layout.edges).toEqual([['a', 'b']]);
    expect(layout.totalLinks).toBe(1);
    // Equal filenames and related_ids are not additional evidence supplied by this view.
    expect(buildAtlasMap(rows, [], rows, 'a').edges).toEqual([]);
  });

  it('keeps a deep selected finding and its recorded neighbor within a deterministic category budget', () => {
    const rows = Array.from({ length: 150 }, (_, index) => finding(`item-${index}`));
    const before = rows.map((row) => row.id);
    const layout = buildAtlasMap(rows, [['item-149', 'item-120']], rows, 'item-149');
    expect(layout.nodes.map((node) => node.finding.id)).toEqual(['item-149', 'item-120']);
    expect(layout.nodes).toHaveLength(ATLAS_CATEGORY_SAMPLE);
    expect(layout.edges).toEqual([['item-120', 'item-149']]);
    expect(layout.groups.find((group) => group.category === 'document')?.total).toBe(150);
    expect(buildAtlasMap(rows, [['item-149', 'item-120']], rows, 'item-149')).toEqual(layout);
    expect(rows.map((row) => row.id)).toEqual(before);
  });

  it('filters the sampled evidence while retaining an explicitly identified selected context item', () => {
    const selected = finding('selected');
    const match = finding('matching', { category: 'media' });
    const excluded = finding('excluded', { category: 'media' });
    const rows = [selected, match, excluded];
    const layout = buildAtlasMap(
      rows,
      [
        ['selected', 'matching'],
        ['matching', 'excluded'],
      ],
      [match],
      'selected',
    );
    expect(layout.nodes.map((node) => node.finding.id)).toEqual(['matching', 'selected']);
    expect(layout.nodes.find((node) => node.selected)?.matching).toBe(false);
    expect(layout.selectedOutsideFilter).toBe(true);
    expect(layout.matchingCount).toBe(1);
    expect(layout.sampledMatches).toBe(1);
    expect(layout.edges).toEqual([['matching', 'selected']]);
    expect(layout.totalLinks).toBe(2);
    expect(layout.groups.find((group) => group.category === 'media')?.total).toBe(2);
    expect(layout.groups.find((group) => group.category === 'media')?.matching).toBe(1);
  });

  it('never imports a selected finding or a filter result from outside the open case', () => {
    const rows = [finding('local')];
    const layout = buildAtlasMap(rows, [['local', 'foreign']], [finding('foreign')], 'foreign');
    expect(layout.nodes).toEqual([]);
    expect(layout.edges).toEqual([]);
    expect(layout.matchingCount).toBe(0);
    expect(layout.selectedOutsideFilter).toBe(false);
  });

  it('bounds a large multi-category source without making omitted relationships into visible ones', () => {
    const rows = CATEGORIES.flatMap((category) =>
      Array.from({ length: 50 }, (_, index) =>
        finding(`${category.id}-${index}`, { category: category.id }),
      ),
    );
    const edges: [string, string][] = [
      ['media-49', 'secret-49'],
      ['browser-49', 'message-49'],
    ];
    const layout = buildAtlasMap(rows, edges, rows, 'database-49');
    expect(layout.nodes.length).toBeLessThanOrEqual(CATEGORIES.length * ATLAS_CATEGORY_SAMPLE);
    expect(layout.nodes.some((node) => node.finding.id === 'database-49' && node.selected)).toBe(
      true,
    );
    expect(layout.groups.every((group) => group.total === 50)).toBe(true);
    const nodeIds = new Set(layout.nodes.map((node) => node.finding.id));
    expect(layout.edges.every(([first, second]) => nodeIds.has(first) && nodeIds.has(second))).toBe(
      true,
    );
    expect(layout.edges).toEqual(edges);
  });
});
