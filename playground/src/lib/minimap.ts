import type { MapMetadata } from "./wasm";
import { slotColor } from "./format";

/** Canonical mmp coordinate space */
export const MMP_SIZE = 256;

export type IconType = 0 | 1 | 2 | number;

export interface OverlayIcon {
  icon_type: IconType;
  x: number;
  y: number;
  /** CSS color */
  color: string;
  label?: string;
}

function rgbaCss(c: number[] | undefined, fallback = "#ffffff"): string {
  if (!c || c.length < 3) return fallback;
  const [r, g, b, a = 255] = c;
  return `rgba(${r},${g},${b},${(a / 255).toFixed(3)})`;
}

/**
 * Build overlay icons from `war3map.mmp`.
 * Falls back to w3i player start locations projected into 256-space when mmp has no starts.
 */
export function buildOverlayIcons(data: MapMetadata): OverlayIcon[] {
  const icons: OverlayIcon[] = [];
  const mmp = data.minimap_icons ?? [];

  for (const ic of mmp) {
    icons.push({
      icon_type: ic.icon_type,
      x: ic.x,
      y: ic.y,
      color: rgbaCss(ic.color as unknown as number[]),
    });
  }

  const hasStarts = icons.some((i) => i.icon_type === 2);
  if (!hasStarts && data.map_info) {
    for (const p of data.map_info.players ?? []) {
      const [wx, wy] = p.start_location ?? [0, 0];
      const [x, y] = worldToMinimap(wx, wy, data.map_info);
      icons.push({
        icon_type: 2,
        x,
        y,
        color: slotColor(p.id),
        label: String(p.id),
      });
    }
  }

  return icons;
}

/** Project world coords → 256×256 minimap pixels using camera bounds + complements. */
export function worldToMinimap(
  wx: number,
  wy: number,
  info: {
    camera_bounds?: number[] | null;
    camera_bounds_complements?: number[] | null;
  },
): [number, number] {
  const cb = info.camera_bounds ?? [-8192, -8192, 8192, 8192];
  const comp = info.camera_bounds_complements ?? [0, 0, 0, 0];
  // complements: left, right, bottom, top (tiles of 128)
  const left = (cb[0] ?? -8192) - (comp[0] ?? 0) * 128;
  const bottom = (cb[1] ?? -8192) - (comp[2] ?? 0) * 128;
  const right = (cb[2] ?? 8192) + (comp[1] ?? 0) * 128;
  const top = (cb[3] ?? 8192) + (comp[3] ?? 0) * 128;
  const w = Math.max(1, right - left);
  const h = Math.max(1, top - bottom);
  const x = ((wx - left) / w) * MMP_SIZE;
  const y = ((top - wy) / h) * MMP_SIZE;
  return [x, y];
}

export function isMinimapImage(filename: string): boolean {
  const f = filename.toLowerCase().replace(/\\/g, "/");
  return f.includes("war3mapmap");
}

/**
 * Render minimap + icons onto a canvas. Resolves when the image has painted.
 */
export function paintMinimapCover(
  canvas: HTMLCanvasElement,
  imageUrl: string,
  icons: OverlayIcon[],
  displaySize = 360,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      const size = displaySize;
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.round(size * dpr);
      canvas.height = Math.round(size * dpr);
      canvas.style.width = `${size}px`;
      canvas.style.height = `${size}px`;
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        reject(new Error("2d context unavailable"));
        return;
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.imageSmoothingEnabled = false;
      ctx.clearRect(0, 0, size, size);
      ctx.drawImage(img, 0, 0, size, size);

      const sx = size / MMP_SIZE;
      const sy = size / MMP_SIZE;

      // gold mines → houses → player starts (on top)
      for (const t of [0, 1, 2]) {
        for (const ic of icons) {
          if (ic.icon_type !== t) continue;
          drawIcon(ctx, ic.icon_type, ic.x * sx, ic.y * sy, ic.color, size);
        }
      }
      resolve();
    };
    img.onerror = () => reject(new Error("failed to load minimap image"));
    img.src = imageUrl;
  });
}

function drawIcon(
  ctx: CanvasRenderingContext2D,
  type: IconType,
  x: number,
  y: number,
  color: string,
  canvasSize: number,
): void {
  const unit = Math.max(7, canvasSize * 0.032);

  ctx.save();
  ctx.translate(x, y);

  if (type === 0) {
    // Gold mine — gold disc
    const r = unit * 0.55;
    ctx.beginPath();
    ctx.arc(0, 0, r, 0, Math.PI * 2);
    ctx.fillStyle = "#e8b923";
    ctx.fill();
    ctx.lineWidth = Math.max(1.2, unit * 0.12);
    ctx.strokeStyle = "rgba(40, 28, 4, 0.85)";
    ctx.stroke();
    // inner highlight
    ctx.beginPath();
    ctx.arc(-r * 0.25, -r * 0.25, r * 0.28, 0, Math.PI * 2);
    ctx.fillStyle = "rgba(255, 240, 160, 0.65)";
    ctx.fill();
  } else if (type === 1) {
    // Neutral building / house
    const w = unit * 0.95;
    const h = unit * 0.75;
    ctx.fillStyle = color && color !== "rgba(255,255,255,1.000)" ? color : "#d8dde6";
    ctx.strokeStyle = "rgba(0,0,0,0.75)";
    ctx.lineWidth = Math.max(1, unit * 0.1);
    // body
    ctx.beginPath();
    ctx.rect(-w * 0.4, -h * 0.05, w * 0.8, h * 0.55);
    ctx.fill();
    ctx.stroke();
    // roof
    ctx.beginPath();
    ctx.moveTo(-w * 0.5, -h * 0.05);
    ctx.lineTo(0, -h * 0.55);
    ctx.lineTo(w * 0.5, -h * 0.05);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();
  } else if (type === 2) {
    // Player start — colored X (classic minimap style)
    const arm = unit * 0.62;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    // dark outline
    ctx.strokeStyle = "rgba(0,0,0,0.85)";
    ctx.lineWidth = Math.max(3.2, unit * 0.38);
    strokeX(ctx, arm);
    // colored stroke
    ctx.strokeStyle = color;
    ctx.lineWidth = Math.max(2, unit * 0.24);
    strokeX(ctx, arm);
  } else {
    // Unknown — small diamond
    const r = unit * 0.4;
    ctx.beginPath();
    ctx.moveTo(0, -r);
    ctx.lineTo(r, 0);
    ctx.lineTo(0, r);
    ctx.lineTo(-r, 0);
    ctx.closePath();
    ctx.fillStyle = color;
    ctx.fill();
    ctx.strokeStyle = "rgba(0,0,0,0.7)";
    ctx.lineWidth = 1;
    ctx.stroke();
  }

  ctx.restore();
}

function strokeX(ctx: CanvasRenderingContext2D, arm: number): void {
  ctx.beginPath();
  ctx.moveTo(-arm, -arm);
  ctx.lineTo(arm, arm);
  ctx.moveTo(arm, -arm);
  ctx.lineTo(-arm, arm);
  ctx.stroke();
}

export function iconTypeLabel(t: number): string {
  if (t === 0) return "Gold mine";
  if (t === 1) return "Building";
  if (t === 2) return "Player start";
  return `Type ${t}`;
}

export function countByType(icons: OverlayIcon[]): Record<string, number> {
  const out: Record<string, number> = {};
  for (const ic of icons) {
    const k = iconTypeLabel(ic.icon_type);
    out[k] = (out[k] ?? 0) + 1;
  }
  return out;
}
