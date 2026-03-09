import type { TreemapRect } from "./treemapLayout";
import type { RGB } from "./colorMap";
import { getColorRGB } from "./colorMap";

// --- Cushion Shading Parameters (WinDirStat defaults) ---

export interface CushionParams {
  brightness: number;
  cushionHeight: number;
  scaleFactor: number;
  ambientLight: number;
  lightDir: { x: number; y: number; z: number };
}

export const DEFAULT_CUSHION_PARAMS: CushionParams = {
  brightness: 0.84,
  cushionHeight: 0.40,
  scaleFactor: 0.90,
  ambientLight: 0.15,
  lightDir: (() => {
    const inv = 1 / Math.sqrt(3);
    return { x: -inv, y: -inv, z: inv };
  })(),
};

// --- Cushion Coefficients ---

/**
 * Pre-computed linear coefficients for the cushion surface partial derivatives.
 *
 * For each axis, the partial derivative of the surface is a linear function of the pixel coordinate:
 *   dF/dx = bx - 2 * ax * px
 *   dF/dy = by - 2 * ay * py
 *
 * The normal vector at pixel (px, py) is then:
 *   N = (-(bx - 2*ax*px), -(by - 2*ay*py), 1)
 * which simplifies to:
 *   N = (2*ax*px - bx, 2*ay*py - by, 1)
 */
export interface CushionCoeffs {
  ax: number;
  bx: number;
  ay: number;
  by: number;
}

export function computeCushionCoeffs(
  rect: TreemapRect,
  params: CushionParams = DEFAULT_CUSHION_PARAMS,
): CushionCoeffs {
  const { cushionHeight, scaleFactor } = params;
  let ax = 0;
  let bx = 0;
  let ay = 0;
  let by = 0;

  // Accumulate contributions from each ancestor level and the leaf itself
  const allRects = [
    ...rect.ancestors.map((a) => ({
      x0: a.x0,
      y0: a.y0,
      x1: a.x1,
      y1: a.y1,
      depth: a.depth,
    })),
    { x0: rect.x0, y0: rect.y0, x1: rect.x1, y1: rect.y1, depth: rect.depth },
  ];

  for (const r of allRects) {
    const h = cushionHeight * Math.pow(scaleFactor, r.depth);

    const dx = r.x1 - r.x0;
    if (dx > 0) {
      const coeff = h * 4 / (dx * dx);
      ax += coeff;
      bx += coeff * (r.x1 + r.x0);
    }

    const dy = r.y1 - r.y0;
    if (dy > 0) {
      const coeff = h * 4 / (dy * dy);
      ay += coeff;
      by += coeff * (r.y1 + r.y0);
    }
  }

  return { ax, bx, ay, by };
}

// --- Intensity Computation ---

export function computeIntensity(
  nx: number,
  ny: number,
  lightDir: { x: number; y: number; z: number },
  ambientLight: number,
): number {
  // Normal vector: (nx, ny, 1) - not normalized for performance,
  // we normalize inline
  const len = Math.sqrt(nx * nx + ny * ny + 1);
  const dot = (nx * lightDir.x + ny * lightDir.y + lightDir.z) / len;
  return ambientLight + (1 - ambientLight) * Math.max(0, dot);
}

// --- ImageData Rendering ---

const MIN_AREA_FOR_CUSHION = 4;

export function renderCushionTreemap(
  rects: TreemapRect[],
  width: number,
  height: number,
  params: CushionParams = DEFAULT_CUSHION_PARAMS,
): ImageData {
  const imageData = new ImageData(width, height);
  const pixels = imageData.data;
  const { brightness, ambientLight, lightDir } = params;

  for (const rect of rects) {
    const rw = rect.x1 - rect.x0;
    const rh = rect.y1 - rect.y0;
    if (rw < 1 || rh < 1) continue;

    const baseColor = getColorRGB(rect.data.extension, rect.data.is_dir);
    const area = rw * rh;

    const x0i = Math.max(0, Math.floor(rect.x0));
    const y0i = Math.max(0, Math.floor(rect.y0));
    const x1i = Math.min(width, Math.ceil(rect.x1));
    const y1i = Math.min(height, Math.ceil(rect.y1));

    if (area < MIN_AREA_FOR_CUSHION) {
      // Small rects: flat color with reduced brightness
      const flatIntensity = brightness * (ambientLight + (1 - ambientLight) * 0.5);
      const fr = Math.round(baseColor.r * flatIntensity);
      const fg = Math.round(baseColor.g * flatIntensity);
      const fb = Math.round(baseColor.b * flatIntensity);

      for (let py = y0i; py < y1i; py++) {
        for (let px = x0i; px < x1i; px++) {
          const idx = (py * width + px) * 4;
          pixels[idx] = fr;
          pixels[idx + 1] = fg;
          pixels[idx + 2] = fb;
          pixels[idx + 3] = 255;
        }
      }
      continue;
    }

    const coeffs = computeCushionCoeffs(rect, params);
    const br = baseColor.r * brightness;
    const bg = baseColor.g * brightness;
    const bb = baseColor.b * brightness;

    for (let py = y0i; py < y1i; py++) {
      const ny = 2 * coeffs.ay * py - coeffs.by;
      const rowOffset = py * width;

      for (let px = x0i; px < x1i; px++) {
        const nx = 2 * coeffs.ax * px - coeffs.bx;
        const intensity = computeIntensity(nx, ny, lightDir, ambientLight);

        const idx = (rowOffset + px) * 4;
        pixels[idx] = br * intensity;
        pixels[idx + 1] = bg * intensity;
        pixels[idx + 2] = bb * intensity;
        pixels[idx + 3] = 255;
      }
    }
  }

  return imageData;
}
