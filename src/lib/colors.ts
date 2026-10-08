// Soft, friendly palette. Colors are stored as 0xRRGGBB integers in the database.
export const PALETTE: { name: string; value: number }[] = [
  { name: 'Lavender', value: 0x7c74ff },
  { name: 'Sky', value: 0x4aa8ff },
  { name: 'Mint', value: 0x34c38f },
  { name: 'Lemon', value: 0xf5b83d },
  { name: 'Peach', value: 0xff8a5c },
  { name: 'Rose', value: 0xf2668b },
  { name: 'Grape', value: 0xb164e8 },
  { name: 'Teal', value: 0x22b5bf },
  { name: 'Slate', value: 0x7d8597 },
];

export const DEFAULT_COLOR = PALETTE[0].value;

export const hex = (n: number) => '#' + (n >>> 0).toString(16).padStart(6, '0').slice(-6);
export const fromHex = (s: string) => parseInt(s.replace('#', ''), 16);

/** Stable color for an arbitrary name (apps in charts). */
export function colorForName(name: string): string {
  let h = 5381;
  for (let i = 0; i < name.length; i++) h = ((h * 33) ^ name.charCodeAt(i)) >>> 0;
  return hex(PALETTE[h % PALETTE.length].value);
}

/** Readable text color (dark or white) on a solid block of color `n`. */
export function textOn(n: number): string {
  const r = (n >> 16) & 255;
  const g = (n >> 8) & 255;
  const b = n & 255;
  return (r * 299 + g * 587 + b * 114) / 1000 > 165 ? '#1e2232' : '#ffffff';
}
