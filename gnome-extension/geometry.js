// Pure crosshair geometry (no GNOME imports) — shared by the extension and the tests.
// Mirrors the Rust overlay: whole pixels, every part centred on the same point
// (all lengths share the parity of the line thickness).

/** @typedef {{x:number,y:number,w:number,h:number}} Rect */

const px = len => Math.max(1, Math.round(len));

/** Start of a run of `len` whole pixels centred on `c`. */
const centred = (c, len) => Math.round(c - len / 2);

export const DEFAULTS = Object.freeze({
    shape: 'cross-gap',
    size: 20,
    thickness: 2,
    gap: 6,
    outline: true,
    outlineThickness: 1,
});

/**
 * Layout in an actor of side `box` (logical px); the crosshair centre is at box/2.
 * @returns {{box:number, rects:Rect[], dot:{c:number,r:number}|null, outline:number}}
 */
export function computeLayout(cfg) {
    const c = {...DEFAULTS, ...cfg};
    const outline = c.outline && c.outlineThickness > 0 ? px(c.outlineThickness) : 0;
    const thick = px(c.thickness);
    const sameParity = len => ((len - thick) % 2 === 0 ? len : len + 1);
    const size = sameParity(Math.max(px(c.size), thick));
    // Actor side: crosshair + outline + 1px margin each side, same parity as size.
    const box = size + 2 * outline + 2;
    const mid = box / 2;
    const rects = [];
    let dot = null;

    if (c.shape === 'dot') {
        dot = {c: mid, r: Math.max(0.5, c.size / 2)};
    } else if (c.shape === 'cross') {
        rects.push({x: centred(mid, size), y: centred(mid, thick), w: size, h: thick});
        rects.push({x: centred(mid, thick), y: centred(mid, size), w: thick, h: size});
    } else {
        let gap = c.gap > 0 ? sameParity(Math.round(c.gap)) : 0;
        gap = Math.min(gap, size - 2);
        const x0 = centred(mid, size);
        const x1 = x0 + size;
        const g0 = centred(mid, gap);
        const g1 = g0 + gap;
        const t0 = centred(mid, thick);
        rects.push({x: x0, y: t0, w: g0 - x0, h: thick}); // left
        rects.push({x: g1, y: t0, w: x1 - g1, h: thick}); // right
        rects.push({x: t0, y: x0, w: thick, h: g0 - x0}); // top
        rects.push({x: t0, y: g1, w: thick, h: x1 - g1}); // bottom
    }
    return {box, rects: rects.filter(r => r.w > 0 && r.h > 0), dot, outline};
}

/** Top-left position for the actor so that its centre lands on (cx, cy). */
export function actorPosition(cx, cy, box) {
    return [Math.round(cx - box / 2), Math.round(cy - box / 2)];
}

/** '#39ff14' → [r, g, b] in 0..1; invalid → fallback. */
export function parseColor(s, fallback = [0x39 / 255, 1, 0x14 / 255]) {
    const m = /^#?([0-9a-f]{6})$/i.exec(String(s ?? '').trim());
    if (!m)
        return fallback;
    const v = parseInt(m[1], 16);
    return [(v >> 16) / 255, ((v >> 8) & 255) / 255, (v & 255) / 255];
}

/** Is this window Roblox (Sober)? `ids` = app id / wm class strings of the window. */
export function isRobloxIds(ids) {
    return ids.some(id => typeof id === 'string' && /(^|\.)sober$|vinegarhq|roblox/i.test(id));
}
