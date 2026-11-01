// The look the user picks: an accent colour and an optional background picture. The CSS reads
// everything from custom properties on :root, so applying a theme is setting a handful of them.

/** The preset accents. The first is QuickTranslate's own. */
export const ACCENTS = [
  '#6fe0c3', // mint
  '#d4d8df', // silver
  '#6aa8ff', // blue
  '#9d8cff', // violet
  '#ff7eb6', // pink
  '#ff6b6b', // red
  '#ffa65c', // orange
  '#ffd166', // yellow
  '#5ee08b', // green
];
export const DEFAULT_ACCENT = ACCENTS[0];

type Rgb = [number, number, number];

function parse(hex: string): Rgb | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) return null;
  const n = parseInt(m[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

const toHex = (c: Rgb) => '#' + c.map((v) => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, '0')).join('');
const mix = (a: Rgb, b: Rgb, t: number): Rgb => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];

/** WCAG relative luminance, 0 (black) to 1 (white). */
function luminance([r, g, b]: Rgb) {
  const lin = (v: number) => ((v /= 255) <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

/** An accent that reads on the near-black surfaces: a too-dark pick is lifted until it does. */
function readable(c: Rgb): Rgb {
  let out = c;
  for (let i = 0; i < 10 && luminance(out) < 0.18; i++) out = mix(out, [255, 255, 255], 0.15);
  return out;
}

export interface Look {
  accent: string;
  /** A data URL, or '' for none. */
  background: string;
  blur: boolean;
  /** 0 to 0.9: how much the picture is darkened under the interface. */
  dim: number;
  /** Take the accent from the picture instead. */
  accentFromBackground: boolean;
}

/** Applies a look to the page. Returns the accent in use (the picture's when that is chosen). */
export async function applyLook(look: Look): Promise<string> {
  let accent = parse(look.accent) ?? parse(DEFAULT_ACCENT)!;
  if (look.background && look.accentFromBackground) accent = (await pictureAccent(look.background)) ?? accent;
  accent = readable(accent);
  const root = document.documentElement.style;
  root.setProperty('--accent', toHex(accent));
  root.setProperty('--accent-hover', toHex(mix(accent, [255, 255, 255], 0.22)));
  root.setProperty('--accent-pressed', toHex(mix(accent, [0, 0, 0], 0.16)));
  root.setProperty('--accent-rgb', accent.map(Math.round).join(' '));
  // Dark text on a light accent, white on a dark one: whichever has the higher contrast.
  root.setProperty('--on-accent', luminance(accent) > 0.36 ? '#0b0c0d' : '#ffffff');
  if (look.background) {
    root.setProperty('--bg-image', `url("${look.background}")`);
    root.setProperty('--bg-blur', look.blur ? '28px' : '0px');
    root.setProperty('--bg-dim', String(Math.min(0.9, Math.max(0, look.dim))));
  } else {
    root.removeProperty('--bg-image');
  }
  document.documentElement.classList.toggle('has-picture', !!look.background);
  return toHex(accent);
}

/** The picture's most colourful hue: saturated, bright pixels vote for one of 36 hue buckets. */
export async function pictureAccent(src: string): Promise<Rgb | null> {
  const img = new Image();
  img.src = src;
  await img.decode().catch(() => undefined);
  if (!img.naturalWidth) return null;
  const size = 48;
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = size;
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  if (!ctx) return null;
  ctx.drawImage(img, 0, 0, size, size);
  const data = ctx.getImageData(0, 0, size, size).data;
  const buckets = Array.from({ length: 36 }, () => ({ weight: 0, sum: [0, 0, 0] as Rgb }));
  for (let i = 0; i < data.length; i += 4) {
    const r = data[i], g = data[i + 1], b = data[i + 2];
    const max = Math.max(r, g, b), min = Math.min(r, g, b);
    const s = max ? (max - min) / max : 0;
    const v = max / 255;
    if (s < 0.25 || v < 0.25) continue; // greys and shadows say nothing about the picture's colour
    const d = max - min;
    let h = max === r ? (g - b) / d : max === g ? 2 + (b - r) / d : 4 + (r - g) / d;
    h = ((h * 60) + 360) % 360;
    const bucket = buckets[Math.floor(h / 10) % 36];
    const w = s * v;
    bucket.weight += w;
    bucket.sum[0] += r * w; bucket.sum[1] += g * w; bucket.sum[2] += b * w;
  }
  const best = buckets.reduce((a, b) => (b.weight > a.weight ? b : a));
  if (best.weight === 0) return null;
  return best.sum.map((v) => v / best.weight) as Rgb;
}

/** Reads a picture the user picked and shrinks it to at most 2560 px: a few hundred KB of JPEG.
 *  Read as a data: URL, not a blob: one, because the page's CSP allows images only from data: (and itself). */
export async function loadPicture(file: File): Promise<string> {
  const source = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
  const img = new Image();
  img.src = source;
  await img.decode();
  const scale = Math.min(1, 2560 / Math.max(img.naturalWidth, img.naturalHeight));
  const canvas = document.createElement('canvas');
  canvas.width = Math.round(img.naturalWidth * scale);
  canvas.height = Math.round(img.naturalHeight * scale);
  canvas.getContext('2d')!.drawImage(img, 0, 0, canvas.width, canvas.height);
  return canvas.toDataURL('image/jpeg', 0.88);
}
