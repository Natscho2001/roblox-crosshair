import {test} from 'node:test';
import assert from 'node:assert/strict';
import {actorPosition, computeLayout, isRobloxIds, parseColor} from '../geometry.js';

// Alpha-free "rasteriser": centre of mass of all covered pixels of a layout.
function centroid({box, rects, dot}) {
    let sx = 0, sy = 0, n = 0;
    for (let y = 0; y < box; y++) {
        for (let x = 0; x < box; x++) {
            const [px, py] = [x + 0.5, y + 0.5];
            const hit = dot
                ? (px - dot.c) ** 2 + (py - dot.c) ** 2 <= dot.r ** 2
                : rects.some(r => px >= r.x && px < r.x + r.w && py >= r.y && py < r.y + r.h);
            if (hit) {
                sx += px;
                sy += py;
                n++;
            }
        }
    }
    return [sx / n, sy / n, n];
}

test('every shape is centred in its actor for many sizes', () => {
    for (const shape of ['dot', 'cross', 'cross-gap']) {
        for (const size of [8, 15, 20, 21, 33]) {
            for (const thickness of [1, 2, 3, 4]) {
                const l = computeLayout({shape, size, thickness, gap: 6});
                const [cx, cy, n] = centroid(l);
                assert.ok(n > 0, `${shape} ${size}/${thickness} draws something`);
                assert.ok(Math.abs(cx - l.box / 2) <= 0.5, `${shape} ${size}/${thickness}: cx=${cx} box=${l.box}`);
                assert.ok(Math.abs(cy - l.box / 2) <= 0.5, `${shape} ${size}/${thickness}: cy=${cy}`);
            }
        }
    }
});

test('the outline fits inside the actor', () => {
    for (const shape of ['dot', 'cross', 'cross-gap']) {
        const l = computeLayout({shape, size: 20, thickness: 2, outlineThickness: 3});
        for (const r of l.rects) {
            assert.ok(r.x - l.outline >= 0 && r.y - l.outline >= 0, shape);
            assert.ok(r.x + r.w + l.outline <= l.box && r.y + r.h + l.outline <= l.box, shape);
        }
        if (l.dot)
            assert.ok(l.dot.r + l.outline <= l.box / 2, 'dot + outline inside');
    }
});

test('cross-gap leaves the middle empty, cross does not', () => {
    const gapped = computeLayout({shape: 'cross-gap', size: 20, thickness: 2, gap: 6});
    const cross = computeLayout({shape: 'cross', size: 20, thickness: 2});
    const covers = (l, x, y) => l.rects.some(r => x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h);
    assert.equal(gapped.rects.length, 4);
    assert.equal(covers(gapped, gapped.box / 2, gapped.box / 2), false);
    assert.equal(covers(cross, cross.box / 2, cross.box / 2), true);
});

test('no outline when switched off', () => {
    assert.equal(computeLayout({outline: false}).outline, 0);
    assert.equal(computeLayout({outline: true, outlineThickness: 0}).outline, 0);
});

test('actor position puts the box centre on the target', () => {
    const [x, y] = actorPosition(960, 540, 24);
    assert.deepEqual([x + 12, y + 12], [960, 540]);
    const [x2] = actorPosition(1280 + 40, 0, 24);
    assert.equal(x2 + 12, 1320);
});

test('colours parse with and without #, bad input falls back', () => {
    assert.deepEqual(parseColor('#ff0000'), [1, 0, 0]);
    assert.deepEqual(parseColor('00ff00'), [0, 1, 0]);
    assert.deepEqual(parseColor('nope', [9, 9, 9]), [9, 9, 9]);
});

test('Roblox (Sober) windows are recognised by app id or wm class', () => {
    assert.ok(isRobloxIds(['org.vinegarhq.Sober']));
    assert.ok(isRobloxIds([null, undefined, 'sober']));
    assert.ok(isRobloxIds(['RobloxPlayerBeta.exe']));
    assert.equal(isRobloxIds(['org.mozilla.firefox', 'firefox', null]), false);
    assert.equal(isRobloxIds(['org.gnome.Ptyxis']), false);
});
