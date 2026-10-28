import { describe, expect, it } from 'vitest';
import {
  ATLAS_MAX_ZOOM,
  ATLAS_MIN_ZOOM,
  ATLAS_VIEW_HEIGHT,
  ATLAS_VIEW_WIDTH,
  clampAtlasViewport,
  fitAtlasSelection,
  panAtlasViewport,
  projectAtlasPoint,
  resetAtlasViewport,
  revealAtlasBounds,
  zoomAtlasViewport,
} from './atlas-viewport';

describe('Atlas viewport navigation', () => {
  it('keeps the evidence beneath the cursor anchored while zooming when bounds permit', () => {
    const before = { zoom: 1.2, x: -100, y: -40 };
    const anchor = { x: 350, y: 180 };
    const evidence = {
      x: (anchor.x - before.x) / before.zoom,
      y: (anchor.y - before.y) / before.zoom,
    };
    const after = zoomAtlasViewport(before, 2, anchor);
    expect(projectAtlasPoint(after, evidence).x).toBeCloseTo(anchor.x);
    expect(projectAtlasPoint(after, evidence).y).toBeCloseTo(anchor.y);
    expect(before).toEqual({ zoom: 1.2, x: -100, y: -40 });
  });

  it('clamps extreme zoom and dragging so the canvas cannot be panned out of reach', () => {
    const maximum = zoomAtlasViewport(resetAtlasViewport(), 100);
    expect(maximum.zoom).toBe(ATLAS_MAX_ZOOM);
    const leftTop = panAtlasViewport(maximum, { x: 1e8, y: 1e8 });
    const rightBottom = panAtlasViewport(maximum, { x: -1e8, y: -1e8 });
    expect(leftTop.x).toBe(0);
    expect(leftTop.y).toBe(0);
    expect(projectAtlasPoint(rightBottom, { x: ATLAS_VIEW_WIDTH, y: ATLAS_VIEW_HEIGHT })).toEqual({
      x: ATLAS_VIEW_WIDTH,
      y: ATLAS_VIEW_HEIGHT,
    });
    const minimum = zoomAtlasViewport(maximum, 0);
    expect(minimum.zoom).toBe(ATLAS_MIN_ZOOM);
    for (const delta of [
      { x: 1e8, y: -1e8 },
      { x: -1e8, y: 1e8 },
    ]) {
      const moved = panAtlasViewport(minimum, delta);
      const start = projectAtlasPoint(moved, { x: 0, y: 0 });
      const end = projectAtlasPoint(moved, { x: ATLAS_VIEW_WIDTH, y: ATLAS_VIEW_HEIGHT });
      expect(start.x).toBeGreaterThanOrEqual(0);
      expect(start.y).toBeGreaterThanOrEqual(0);
      expect(end.x).toBeLessThanOrEqual(ATLAS_VIEW_WIDTH);
      expect(end.y).toBeLessThanOrEqual(ATLAS_VIEW_HEIGHT);
    }
  });

  it('fits the selected evidence card and centers it without changing evidence coordinates', () => {
    const selected = { x: 380, y: 220, width: 240, height: 60 };
    const fitted = fitAtlasSelection(selected);
    expect(fitted.zoom).toBe(ATLAS_MAX_ZOOM);
    const center = projectAtlasPoint(fitted, {
      x: selected.x + selected.width / 2,
      y: selected.y + selected.height / 2,
    });
    expect(center).toEqual({ x: ATLAS_VIEW_WIDTH / 2, y: ATLAS_VIEW_HEIGHT / 2 });
    const first = projectAtlasPoint(fitted, selected);
    const last = projectAtlasPoint(fitted, {
      x: selected.x + selected.width,
      y: selected.y + selected.height,
    });
    expect(first.x).toBeGreaterThanOrEqual(48);
    expect(first.y).toBeGreaterThanOrEqual(48);
    expect(last.x).toBeLessThanOrEqual(ATLAS_VIEW_WIDTH - 48);
    expect(last.y).toBeLessThanOrEqual(ATLAS_VIEW_HEIGHT - 48);
    expect(selected).toEqual({ x: 380, y: 220, width: 240, height: 60 });
  });

  it('reveals a keyboard-focused evidence card even after it has been clipped by zoom and pan', () => {
    const before = { zoom: 2, x: -1000, y: -500 };
    const card = { x: 65, y: 63, width: 220, height: 54 };
    const after = revealAtlasBounds(before, card);
    expect(after.zoom).toBe(before.zoom);
    const start = projectAtlasPoint(after, card);
    const end = projectAtlasPoint(after, { x: card.x + card.width, y: card.y + card.height });
    expect(start.x).toBeGreaterThanOrEqual(24);
    expect(start.y).toBeGreaterThanOrEqual(24);
    expect(end.x).toBeLessThanOrEqual(ATLAS_VIEW_WIDTH - 24);
    expect(end.y).toBeLessThanOrEqual(ATLAS_VIEW_HEIGHT - 24);
  });

  it('returns to the complete canvas on reset or an unavailable selection', () => {
    const before = panAtlasViewport(zoomAtlasViewport(resetAtlasViewport(), 2), { x: -200, y: 30 });
    const reset = resetAtlasViewport();
    expect(reset).toEqual({ zoom: 1, x: 0, y: 0 });
    expect(projectAtlasPoint(reset, { x: ATLAS_VIEW_WIDTH, y: ATLAS_VIEW_HEIGHT })).toEqual({
      x: ATLAS_VIEW_WIDTH,
      y: ATLAS_VIEW_HEIGHT,
    });
    expect(before).not.toEqual(reset);
    expect(fitAtlasSelection(null)).toEqual(reset);
    expect(fitAtlasSelection({ x: 5, y: 10, width: 0, height: 60 })).toEqual(reset);
    expect(resetAtlasViewport()).not.toBe(reset);
  });

  it('normalizes nonfinite navigation input instead of producing an unusable SVG transform', () => {
    expect(
      clampAtlasViewport({ zoom: Number.NaN, x: Number.POSITIVE_INFINITY, y: Number.NaN }),
    ).toEqual(resetAtlasViewport());
    expect(
      panAtlasViewport(resetAtlasViewport(), { x: Number.NaN, y: Number.NEGATIVE_INFINITY }),
    ).toEqual(resetAtlasViewport());
    expect(zoomAtlasViewport(resetAtlasViewport(), Number.NaN)).toEqual(resetAtlasViewport());
  });
});
