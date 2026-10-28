export const ATLAS_VIEW_WIDTH = 1000;
export const ATLAS_VIEW_HEIGHT = 500;
export const ATLAS_MIN_ZOOM = 0.6;
export const ATLAS_MAX_ZOOM = 2.5;

export interface AtlasPoint {
  x: number;
  y: number;
}
export interface AtlasBounds extends AtlasPoint {
  width: number;
  height: number;
}
export interface AtlasViewport extends AtlasPoint {
  zoom: number;
}

const finite = (value: number, fallback: number) => (Number.isFinite(value) ? value : fallback);
const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, value));

export function resetAtlasViewport(): AtlasViewport {
  return { zoom: 1, x: 0, y: 0 };
}

export function clampAtlasViewport(viewport: AtlasViewport): AtlasViewport {
  const zoom = clamp(finite(viewport.zoom, 1), ATLAS_MIN_ZOOM, ATLAS_MAX_ZOOM);
  const horizontal = ATLAS_VIEW_WIDTH * (1 - zoom);
  const vertical = ATLAS_VIEW_HEIGHT * (1 - zoom);
  // At high zoom the canvas always covers the viewport. At low zoom it stays
  // wholly inside the viewport, so dragging can never lose the evidence canvas.
  return {
    zoom,
    x: clamp(finite(viewport.x, 0), Math.min(0, horizontal), Math.max(0, horizontal)),
    y: clamp(finite(viewport.y, 0), Math.min(0, vertical), Math.max(0, vertical)),
  };
}

export function projectAtlasPoint(viewport: AtlasViewport, point: AtlasPoint): AtlasPoint {
  return { x: point.x * viewport.zoom + viewport.x, y: point.y * viewport.zoom + viewport.y };
}

export function zoomAtlasViewport(
  viewport: AtlasViewport,
  requestedZoom: number,
  anchor: AtlasPoint = { x: ATLAS_VIEW_WIDTH / 2, y: ATLAS_VIEW_HEIGHT / 2 },
): AtlasViewport {
  const previous = clampAtlasViewport(viewport);
  const zoom = clamp(finite(requestedZoom, previous.zoom), ATLAS_MIN_ZOOM, ATLAS_MAX_ZOOM);
  const x = clamp(finite(anchor.x, ATLAS_VIEW_WIDTH / 2), 0, ATLAS_VIEW_WIDTH);
  const y = clamp(finite(anchor.y, ATLAS_VIEW_HEIGHT / 2), 0, ATLAS_VIEW_HEIGHT);
  const ratio = zoom / previous.zoom;
  return clampAtlasViewport({
    zoom,
    x: x - (x - previous.x) * ratio,
    y: y - (y - previous.y) * ratio,
  });
}

export function panAtlasViewport(viewport: AtlasViewport, delta: AtlasPoint): AtlasViewport {
  const previous = clampAtlasViewport(viewport);
  return clampAtlasViewport({
    ...previous,
    x: previous.x + finite(delta.x, 0),
    y: previous.y + finite(delta.y, 0),
  });
}

function validBounds(bounds: AtlasBounds): boolean {
  return (
    [bounds.x, bounds.y, bounds.width, bounds.height].every(Number.isFinite) &&
    bounds.width > 0 &&
    bounds.height > 0
  );
}

export function fitAtlasSelection(bounds: AtlasBounds | null, padding = 48): AtlasViewport {
  if (!bounds || !validBounds(bounds)) return resetAtlasViewport();
  const inset = clamp(finite(padding, 48), 0, ATLAS_VIEW_HEIGHT / 3);
  const zoom = clamp(
    Math.min(
      (ATLAS_VIEW_WIDTH - 2 * inset) / bounds.width,
      (ATLAS_VIEW_HEIGHT - 2 * inset) / bounds.height,
    ),
    ATLAS_MIN_ZOOM,
    ATLAS_MAX_ZOOM,
  );
  return clampAtlasViewport({
    zoom,
    x: ATLAS_VIEW_WIDTH / 2 - (bounds.x + bounds.width / 2) * zoom,
    y: ATLAS_VIEW_HEIGHT / 2 - (bounds.y + bounds.height / 2) * zoom,
  });
}

export function revealAtlasBounds(
  viewport: AtlasViewport,
  bounds: AtlasBounds,
  padding = 24,
): AtlasViewport {
  const previous = clampAtlasViewport(viewport);
  if (!validBounds(bounds)) return previous;
  const inset = clamp(finite(padding, 24), 0, ATLAS_VIEW_HEIGHT / 3);
  const topLeft = projectAtlasPoint(previous, bounds);
  const width = bounds.width * previous.zoom;
  const height = bounds.height * previous.zoom;
  const adjustment = (start: number, size: number, frame: number) => {
    if (size > frame - 2 * inset) return frame / 2 - (start + size / 2);
    if (start < inset) return inset - start;
    if (start + size > frame - inset) return frame - inset - start - size;
    return 0;
  };
  return panAtlasViewport(previous, {
    x: adjustment(topLeft.x, width, ATLAS_VIEW_WIDTH),
    y: adjustment(topLeft.y, height, ATLAS_VIEW_HEIGHT),
  });
}
