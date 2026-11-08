import { FontInfo } from "./types";

/**
 * Extra HTML for a finding, built from its metadata.
 * Returns [html, suppressMessage]; renderers here never suppress the message.
 */
type Renderer = (metadata: any, font: FontInfo) => [string, boolean];
export const CheckSpecificRendering: Record<string, Renderer> = {};

type GlyphProblem = {
  glyph_name: string;
  glyph_id: number;
  position?: [number, number];
  message: string;
};

const MAX_GLYPHS = 24;

/** Metadata is serialized as [{ "GlyphProblem": {...} }, ...]. */
function glyphProblems(metadata: any): GlyphProblem[] {
  if (!Array.isArray(metadata)) return [];
  return metadata
    .map((m) => m?.GlyphProblem)
    .filter((p): p is GlyphProblem => p && typeof p.glyph_id === "number");
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]!);
}

/** Bounding box of every coordinate in an SVG path (control points included). */
function pathBounds(d: string, extra: [number, number][]): [number, number, number, number] | null {
  const nums = (d.match(/-?\d+(?:\.\d+)?/g) || []).map(Number);
  const xs = extra.map((p) => p[0]);
  const ys = extra.map((p) => p[1]);
  for (let i = 0; i + 1 < nums.length; i += 2) {
    xs.push(nums[i]);
    ys.push(nums[i + 1]);
  }
  if (!xs.length) return null;
  return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
}

function glyphSvg(font: FontInfo, gid: number, points: [number, number][]): string {
  let d = "";
  try {
    d = font.font.glyphToPath(gid) || "";
  } catch {
    return "";
  }
  const b = pathBounds(d, points);
  if (!b) return "";
  const size = Math.max(b[2] - b[0], b[3] - b[1], 1);
  const pad = size * 0.08;
  const [x0, y0, w, h] = [b[0] - pad, -b[3] - pad, b[2] - b[0] + 2 * pad, b[3] - b[1] + 2 * pad];
  const r = size * 0.035;
  const marks = points
    .map(([x, y]) => `<circle cx="${x}" cy="${y}" r="${r}" class="gp-mark"/>`)
    .join("");
  return `<svg viewBox="${x0} ${y0} ${w} ${h}" class="gp-svg"><g transform="scale(1,-1)">` +
    `<path d="${d}" class="gp-ink"/>${marks}</g></svg>`;
}

/** A gallery of the affected glyphs, with the reported points circled. */
const glyphGallery: Renderer = (metadata, font) => {
  const problems = glyphProblems(metadata);
  if (!problems.length || !font?.font) return ["", false];
  const byGlyph = new Map<number, { name: string; points: [number, number][]; notes: Set<string> }>();
  for (const p of problems) {
    if (!byGlyph.has(p.glyph_id)) byGlyph.set(p.glyph_id, { name: p.glyph_name, points: [], notes: new Set() });
    const g = byGlyph.get(p.glyph_id)!;
    if (p.position) g.points.push(p.position);
    if (p.message) g.notes.add(p.message);
  }
  const entries = [...byGlyph.entries()];
  const cards = entries.slice(0, MAX_GLYPHS).map(([gid, g]) => {
    const svg = glyphSvg(font, gid, g.points);
    if (!svg) return "";
    const title = escapeHtml([...g.notes].join("; "));
    const count = g.points.length > 1 ? ` <span class="gp-count">×${g.points.length}</span>` : "";
    return `<figure class="gp-card" title="${title}">${svg}<figcaption>${escapeHtml(g.name)}${count}</figcaption></figure>`;
  });
  if (!cards.some(Boolean)) return ["", false];
  const more = entries.length > MAX_GLYPHS ? `<p class="gp-more">… and ${entries.length - MAX_GLYPHS} more glyphs</p>` : "";
  return [`<div class="gp-gallery">${cards.join("")}</div>${more}`, false];
};

for (const id of [
  "outline_alignment_miss",
  "outline_colinear_vectors",
  "outline_direction",
  "overlapping_path_segments",
  "outline_jaggy_segments",
  "outline_semi_vertical",
  "outline_short_segments",
  "opentype/points_out_of_bounds",
]) {
  CheckSpecificRendering[id] = glyphGallery;
}
