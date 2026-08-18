// Leaflet interop for the World map (design-direction.md: "the world by
// lamplight"). One ES module instance backs every `MapInterop` on the C#
// side (client/MapInterop.cs), keyed by a small integer id so the same
// module can host more than one Leaflet map at once (the full-size world
// map now; mini-maps inside popovers are a later task).
//
// Scene data crosses the JS interop boundary as a pre-serialized JSON
// string (see MapInterop.SetScene's comment for why: IJSObjectReference
// arguments are serialized with System.Text.Json's *default* options, not
// Wire.Options, so passing the Scene DTO directly would rename every
// snake_case field). `setScene` JSON.parses it back into a plain object
// whose shape matches atlas-server's wire JSON exactly.

const TILE_URL = 'https://server.arcgisonline.com/ArcGIS/rest/services/World_Shaded_Relief/MapServer/tile/{z}/{y}/{x}';
const TILE_FALLBACK = 'https://basemaps.cartocdn.com/light_nolabels/{z}/{x}/{y}.png';
const TILE_ATTRIBUTION = 'Tiles &copy; Esri &mdash; Source: Esri, DeLorme, NAVTEQ';

// Roughly centers the Fertile Crescent / Levant before the first real
// scene arrives; fitScene() (called on the first successful scene fetch)
// immediately replaces this with a bounds-fit view, so it only shows for a
// moment.
const DEFAULT_CENTER = [31.5, 35.0];
const DEFAULT_ZOOM = 5;

const SVG_NS = 'http://www.w3.org/2000/svg';

let nextId = 1;
const instances = new Map();

export function init(el, dotnetRef, opts) {
    const id = nextId++;
    const mini = !!(opts && opts.mini);

    const map = L.map(el, {
        zoomControl: !mini,
        attributionControl: !mini,
        scrollWheelZoom: !mini,
    }).setView(DEFAULT_CENTER, DEFAULT_ZOOM);

    const tiles = L.tileLayer(TILE_URL, {
        maxNativeZoom: 13,
        maxZoom: 13,
        attribution: TILE_ATTRIBUTION,
    }).addTo(map);

    // Esri's shaded-relief service occasionally 404s individual tiles at
    // the edges of its coverage; fall back to a plain light basemap rather
    // than leaving permanent holes. Switches once -- repeatedly calling
    // setUrl on every failed tile (there can be many in one bad batch)
    // would just restart the same fallback load over and over.
    let felBack = false;
    tiles.on('tileerror', () => {
        if (felBack) {
            return;
        }
        felBack = true;
        tiles.setUrl(TILE_FALLBACK);
    });

    const arrows = new ArrowLayer(dotnetRef);
    arrows.addTo(map);

    instances.set(id, { map, dotnetRef, markers: new Map(), arrows });
    return id;
}

// Diffs the incoming place list against the markers already on the map,
// keyed by place id: unseen ids are added, vanished ids are removed, and
// ids present in both are left alone unless their visible fields actually
// changed (name/position/brightness) -- so a scene refetch that returns
// the same places doesn't tear down and re-animate markers the user might
// currently be hovering. Arrows (see ArrowLayer below) are diffed the same
// way, keyed by "{narrative}:{order}", and read place positions straight
// out of the `markers` map this loop maintains -- ArrowLayer.setArrows is
// called after it so every place an arrow can reference is already there.
export function setScene(id, sceneJson) {
    const inst = instances.get(id);
    if (!inst) {
        return;
    }

    const scene = typeof sceneJson === 'string' ? JSON.parse(sceneJson) : (sceneJson || {});
    const places = scene.places || [];
    const seen = new Set();

    for (const p of places) {
        seen.add(p.id);
        const prior = inst.markers.get(p.id);

        if (!prior) {
            const marker = L.marker([p.lat, p.lon], { icon: makeIcon(p) });
            wireEvents(marker, inst.dotnetRef, p.id);
            marker.addTo(inst.map);
            inst.markers.set(p.id, { marker, lat: p.lat, lon: p.lon, brightness: p.brightness, name: p.name });
            continue;
        }

        if (prior.lat !== p.lat || prior.lon !== p.lon || prior.brightness !== p.brightness || prior.name !== p.name) {
            prior.marker.setLatLng([p.lat, p.lon]);
            prior.marker.setIcon(makeIcon(p));
            prior.lat = p.lat;
            prior.lon = p.lon;
            prior.brightness = p.brightness;
            prior.name = p.name;
        }
    }

    for (const [placeId, entry] of inst.markers) {
        if (!seen.has(placeId)) {
            inst.map.removeLayer(entry.marker);
            inst.markers.delete(placeId);
        }
    }

    inst.arrows.setArrows(scene.arrows || [], inst.markers);
}

function wireEvents(marker, dotnetRef, placeId) {
    marker.on('mouseover', e => dotnetRef.invokeMethodAsync('OnPlaceHover', placeId, e.containerPoint.x, e.containerPoint.y));
    marker.on('mouseout', () => dotnetRef.invokeMethodAsync('OnPlaceLeave'));
    marker.on('click', e => dotnetRef.invokeMethodAsync('OnPlaceClick', placeId, e.containerPoint.x, e.containerPoint.y));
}

function makeIcon(p) {
    const brightness = Math.min(5, Math.max(1, p.brightness | 0 || 1));
    const html = `<div class="atlas-marker glow-${brightness}" data-testid="marker-${esc(p.id)}"><span class="atlas-label">${esc(p.name)}</span></div>`;
    return L.divIcon({ html, className: 'atlas-marker-icon', iconSize: [0, 0] });
}

function esc(value) {
    return String(value).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

// Called once, after the first scene of the page's life loads (World.razor
// decides "first"; this module just fits whatever markers are currently
// on the map). Later scene changes intentionally do NOT re-fit -- panning
// off to look at a place shouldn't get yanked back by the next slider tick.
export function fitScene(id) {
    const inst = instances.get(id);
    if (!inst || inst.markers.size === 0) {
        return;
    }

    const latlngs = [...inst.markers.values()].map(entry => [entry.lat, entry.lon]);
    inst.map.fitBounds(L.latLngBounds(latlngs), { padding: [48, 48], maxZoom: 8, animate: false });
}

// Sets/clears the legend's isolate filter (Task 12, WORLD-4): `narrativeId`
// null clears every arrow back to data-faded="false"; any other value fades
// every arrow whose narrative doesn't match it.
export function setIsolate(id, narrativeId) {
    const inst = instances.get(id);
    if (!inst) {
        return;
    }

    inst.arrows.setIsolate(narrativeId ?? null);
}

export function destroy(id) {
    const inst = instances.get(id);
    if (!inst) {
        return;
    }

    inst.map.remove();
    instances.delete(id);
}

// --- ArrowLayer: narrative story-threads (Task 12, WORLD-3/WORLD-4) -------
//
// A custom L.Layer managing exactly one <svg data-testid="arrows-svg"> in
// the map's overlayPane (design-direction.md: "narratives stitch between
// [places] as colored threads"). overlayPane sits inside the same
// CSS-transformed pane hierarchy tilePane/markerPane do, so -- exactly like
// L.Marker -- positioning path coordinates with `map.latLngToLayerPoint`
// keeps arrows glued to their markers through a live drag for free; only
// zoomend/moveend (the brief's exact recompute triggers, matching
// Leaflet's own internal renderers) need to touch coordinates again, and
// that recompute is a plain, instant attribute set -- no CSS transition
// ever targets `d`, so panning/zooming never animates.
//
// Arrow identity is `"{narrative}:{order}"`, matching the
// OnArrowHover/OnArrowClick key contract and the arrow-{narrative}-{order}
// testid; setArrows diffs the incoming list against `_paths` the same way
// setScene above diffs markers, so a scene refresh that keeps an arrow
// updates it in place (color/position) rather than tearing down and
// re-animating a path the user might be hovering.
const ArrowLayer = L.Layer.extend({
    initialize(dotnetRef) {
        this._dotnetRef = dotnetRef;
        this._paths = new Map(); // "{narrative}:{order}" -> { path, arrow, parallelIndex }
        this._markerIds = new Set(); // colorhex already present as a <marker> in defs
    },

    onAdd(map) {
        this._map = map;
        this._svg = svgEl('svg', { class: 'atlas-arrows', 'data-testid': 'arrows-svg' });
        this._defs = svgEl('defs');
        this._svg.appendChild(this._defs);
        map.getPane('overlayPane').appendChild(this._svg);
        return this;
    },

    onRemove() {
        this._svg.remove();
        this._paths.clear();
        this._markerIds.clear();
    },

    // Leaflet calls these with `this` bound to the layer (the documented
    // `map.on(eventMap, context)` form) -- see L.Layer's own _layerAdd.
    getEvents() {
        return { zoomend: this._redraw, moveend: this._redraw };
    },

    // `placesById` is map.js's own `inst.markers` (placeId -> {lat, lon,
    // ...}), reused as-is rather than rebuilding a second parallel lookup:
    // setScene above always finishes diffing markers before calling this,
    // so every place an arrow in `arrows` can reference (ARROW-1: arrows
    // only ever reference lit places) is already present in it.
    setArrows(arrows, placesById) {
        this._placesById = placesById;
        const list = arrows || [];

        // parallelIndex = position among arrows sharing the same UNORDERED
        // place pair, centered 0, +1, -1, +2, ... -- grouped fresh on every
        // call (which arrows share a pair can change scene to scene) in
        // the scene's own array order, which is the narrative/order the
        // server emits arrows in (stable and deterministic).
        const pairSeen = new Map(); // pairKey -> count so far
        const parallelIndexByKey = new Map(); // "{narrative}:{order}" -> centered index
        for (const a of list) {
            const pairKey = [a.from_place, a.to_place].slice().sort().join('|');
            const k = pairSeen.get(pairKey) ?? 0;
            pairSeen.set(pairKey, k + 1);
            parallelIndexByKey.set(arrowKey(a), centeredIndex(k));
        }

        const seen = new Set();
        for (const a of list) {
            const key = arrowKey(a);
            seen.add(key);
            let entry = this._paths.get(key);
            if (!entry) {
                entry = { path: this._createPath(a), arrow: a, parallelIndex: 0 };
                this._paths.set(key, entry);
            } else {
                entry.arrow = a;
                this._syncColor(entry.path, a);
            }
            entry.parallelIndex = parallelIndexByKey.get(key) ?? 0;
        }

        for (const [key, entry] of this._paths) {
            if (!seen.has(key)) {
                entry.path.remove();
                this._paths.delete(key);
            }
        }

        this._redraw();
    },

    // WORLD-4: null clears every arrow back to "false"; any other value
    // fades every arrow whose narrative doesn't match it.
    setIsolate(narrativeId) {
        for (const entry of this._paths.values()) {
            const faded = narrativeId != null && entry.arrow.narrative !== narrativeId;
            entry.path.setAttribute('data-faded', faded ? 'true' : 'false');
        }
    },

    _createPath(a) {
        const path = svgEl('path', {
            class: 'atlas-arrow',
            'data-testid': `arrow-${a.narrative}-${a.order}`,
            fill: 'none',
            'stroke-width': '2.5',
            'data-faded': 'false',
        });
        this._syncColor(path, a);
        path.addEventListener('mouseover', e => {
            const pt = this._map.mouseEventToContainerPoint(e);
            this._dotnetRef.invokeMethodAsync('OnArrowHover', arrowKey(a), pt.x, pt.y);
        });
        path.addEventListener('mouseout', () => this._dotnetRef.invokeMethodAsync('OnArrowLeave'));
        path.addEventListener('click', e => {
            const pt = this._map.mouseEventToContainerPoint(e);
            this._dotnetRef.invokeMethodAsync('OnArrowClick', arrowKey(a), pt.x, pt.y);
        });
        this._svg.appendChild(path);
        return path;
    },

    // `stroke` is set to the EXACT data string (never re-derived/restyled --
    // design-direction.md: narrative colors "are never restyled"); the
    // per-color <marker> arrowhead is created once per distinct color and
    // reused by every path sharing it.
    _syncColor(path, a) {
        if (path.getAttribute('stroke') === a.color) {
            return;
        }
        const colorhex = String(a.color).replace(/^#/, '');
        this._ensureMarker(a.color, colorhex);
        path.setAttribute('stroke', a.color);
        path.setAttribute('marker-end', `url(#ah-${colorhex})`);
    },

    _ensureMarker(color, colorhex) {
        if (this._markerIds.has(colorhex)) {
            return;
        }
        this._markerIds.add(colorhex);
        const marker = svgEl('marker', {
            id: `ah-${colorhex}`,
            viewBox: '0 0 10 10',
            refX: '8.5',
            refY: '5',
            markerWidth: '6',
            markerHeight: '6',
            orient: 'auto',
        });
        marker.appendChild(svgEl('path', { d: 'M0,0 L10,5 L0,10 z', fill: color }));
        this._defs.appendChild(marker);
    },

    // Recomputes every path's `d` from the CURRENT map projection --
    // called on zoomend/moveend (instant, no transition on `d` ever) and
    // once synchronously at the end of every setArrows (so a brand new
    // scene's arrows are positioned immediately, without waiting for a
    // zoom/pan event that may never come).
    _redraw() {
        if (!this._map) {
            return;
        }
        for (const entry of this._paths.values()) {
            this._position(entry);
        }
    },

    _position(entry) {
        const from = this._placesById && this._placesById.get(entry.arrow.from_place);
        const to = this._placesById && this._placesById.get(entry.arrow.to_place);
        if (!from || !to) {
            return; // ARROW-1 guarantees this never happens for real scenes
        }

        const f = this._map.latLngToLayerPoint([from.lat, from.lon]);
        const t = this._map.latLngToLayerPoint([to.lat, to.lon]);
        const dx = t.x - f.x;
        const dy = t.y - f.y;
        const dist = Math.hypot(dx, dy) || 1;
        const nx = -dy / dist;
        const ny = dx / dist;
        const mag = 0.18 * dist + 14 * entry.parallelIndex;
        const cx = (f.x + t.x) / 2 + nx * mag;
        const cy = (f.y + t.y) / 2 + ny * mag;

        entry.path.setAttribute('d', `M${f.x},${f.y} Q${cx},${cy} ${t.x},${t.y}`);

        // Keeps the one-shot draw-in animation (app.css: atlas-arrow-in,
        // stroke-dasharray/--arrow-length sized to the path's own length)
        // gap-free after a redraw changes the path's actual pixel length --
        // this only ever mutates a style property on an EXISTING element
        // (never re-creates/re-inserts it), so it can never retrigger the
        // animation, keeping zoom/pan recompute instant per design-direction.
        const len = entry.path.getTotalLength();
        entry.path.style.strokeDasharray = String(len);
        entry.path.style.setProperty('--arrow-length', String(len));
    },
});

function arrowKey(a) {
    return `${a.narrative}:${a.order}`;
}

// 0, +1, -1, +2, -2, ... for k = 0, 1, 2, 3, 4, ...
function centeredIndex(k) {
    if (k === 0) {
        return 0;
    }
    const half = Math.ceil(k / 2);
    return k % 2 === 1 ? half : -half;
}

function svgEl(name, attrs) {
    const el = document.createElementNS(SVG_NS, name);
    if (attrs) {
        for (const k in attrs) {
            el.setAttribute(k, attrs[k]);
        }
    }
    return el;
}
