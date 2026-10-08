// How an image sits inside a fixed box (the Overview header, image cards): it always
// covers the box, `zoom` enlarges it further, and `x`/`y` pick which part is visible
// (0 = left/top edge, 1 = right/bottom edge).

export interface Framing {
  x: number;
  y: number;
  /** 1 = just covers the box; up to MAX_ZOOM. */
  zoom: number;
}

export const DEFAULT_FRAMING: Framing = { x: 0.5, y: 0.5, zoom: 1 };
export const MAX_ZOOM = 3;

const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, Number.isFinite(n) ? n : lo));

export function cleanFraming(f: Partial<Framing> | null | undefined): Framing {
  return {
    x: clamp(f?.x ?? 0.5, 0, 1),
    y: clamp(f?.y ?? 0.5, 0, 1),
    zoom: clamp(f?.zoom ?? 1, 1, MAX_ZOOM),
  };
}

/** Where to draw an image of natural size `nw`×`nh` in a `w`×`h` box. */
export function placeImage(nw: number, nh: number, w: number, h: number, f: Framing) {
  const scale = Math.max(w / nw, h / nh) * f.zoom;
  const width = nw * scale;
  const height = nh * scale;
  return { width, height, left: -(width - w) * f.x, top: -(height - h) * f.y, overflowX: width - w, overflowY: height - h };
}

const NICE_RATIOS: [number, number][] = [
  [1, 1], [4, 3], [3, 2], [16, 9], [2, 1], [5, 2], [3, 1], [4, 1], [5, 1], [6, 1],
  [3, 4], [2, 3], [9, 16], [1, 2], [1, 3],
];

/** A friendly recommendation for a `w`×`h` box, e.g. { ratio: "4:1", size: "2400×600" }. */
export function recommendFor(w: number, h: number, longSide: number) {
  const r = w / Math.max(1, h);
  const [a, b] = NICE_RATIOS.reduce((best, cur) =>
    Math.abs(Math.log(cur[0] / cur[1] / r)) < Math.abs(Math.log(best[0] / best[1] / r)) ? cur : best,
  );
  const width = r >= 1 ? longSide : Math.round(longSide * r);
  const height = r >= 1 ? Math.round(longSide / r) : longSide;
  return { ratio: `${a}:${b}`, size: `${width}×${height}` };
}
