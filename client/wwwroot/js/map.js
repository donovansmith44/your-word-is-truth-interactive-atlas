// Leaflet interop for the World map (design-direction.md: "the world by
// lamplight"). One ES module instance backs every `MapInterop` on the C#
// side (client/MapInterop.cs), keyed by a small integer id so the same
// module can host more than one Leaflet map at once -- the full-size world
// map, and (Task 15) any number of 320x240 mini-maps inside popovers,
// concurrently. Mini instances (opts.mini) render the base tile layer and
// markers only: no zoom/pan controls (unchanged since Task 11), and no
// border overlay / landmark labels / narrative arrows at all -- init()
// below skips creating those three layers entirely for a mini instance
// rather than creating-then-never-feeding them, and setScene()'s own arrow
// push (arrows travel bundled inside every Scene payload, unlike borders/
// landmarks which only ever arrive through their own SetBorders/
// SetLandmarks calls that MiniWorld.razor simply never makes) is gated on
// the same flag.
//
// Scene data crosses the JS interop boundary as a pre-serialized JSON
// string (see MapInterop.SetScene's comment for why: IJSObjectReference
// arguments are serialized with System.Text.Json's *default* options, not
// Wire.Options, so passing the Scene DTO directly would rename every
// snake_case field). `setScene` JSON.parses it back into a plain object
// whose shape matches atlas-server's wire JSON exactly.

// Basemap (design-direction.md "World -- REVISED": "an illuminated atlas
// plate... terrain shading, coastlines, water contrast, country borders,
// and reference city labels"). NatGeo_World_Map won a side-by-side against
// the alternative (World_Physical_Map base + World_Boundaries_and_Places
// overlay), screenshot-compared at all three WINDOWS world-arrows.spec.ts
// uses (exodus/patriarchs/paul): NatGeo reads as a single coherent vintage
// atlas plate (letterspaced serif country names, italic sea names, elevation
// callouts, one tile layer/one failure path) and its native tiles run to
// zoom 16, so it stays crisp at every zoom this app reaches; the
// alternative's World_Physical_Map base tops out at native zoom 8 and was
// visibly soft/smeared once fitScene zoomed in for the patriarchs window's
// smaller extent (confirmed by screenshot, not just LOD metadata). See
// tile/{z}/{y}/{x}?f=json on each service for the LOD lists this compares.
const TILE_URL = 'https://server.arcgisonline.com/ArcGIS/rest/services/NatGeo_World_Map/MapServer/tile/{z}/{y}/{x}';
const TILE_FALLBACK = 'https://basemaps.cartocdn.com/light_nolabels/{z}/{x}/{y}.png';
// Esri's own copyrightText for the NatGeo_World_Map service, verbatim (order
// and all): a fix-round review found this had drifted (Esri/National
// Geographic swapped, METI silently dropped, "increment P Corp." abbreviated
// to "iPC") — keep this string byte-for-byte in sync with the credits panel
// (MainLayout.razor's attribution popover), which quotes the same source.
const TILE_ATTRIBUTION =
    'Tiles &copy; National Geographic, Esri, Garmin, HERE, UNEP-WCMC, USGS, NASA, ESA, METI, NRCAN, GEBCO, NOAA, increment P Corp.';
const TILE_MAX_NATIVE_ZOOM = 16; // NatGeo_World_Map's own max LOD (see comment above)

// Roughly centers the Fertile Crescent / Levant before the first real
// scene arrives; fitScene() (called on the first successful scene fetch)
// immediately replaces this with a bounds-fit view, so it only shows for a
// moment.
const DEFAULT_CENTER = [31.5, 35.0];
const DEFAULT_ZOOM = 5;

// Region lock (design-direction.md "World -- REVISED": "The map is locked
// to the biblical world" -- user feedback 2026-08-18 rejected an
// unconstrained globe: "we don't need the whole world, just the Biblical
// world"). Computed ONCE (not derived at runtime) from every place's lat/lon
// in data/compiled/places.json (1375 places) via a throwaway node script:
// min/max over every {lat, lon}, which came out to
//   lat  11.595   (punt / Punt)        .. 44.94278 (ashkenaz / Ashkenaz)
//   lon  -6.944167 (tarshish-1 / Tarshish 1) .. 67.4308 (india / India)
// then padded by a flat 4 degrees on every side (within the brief's
// "~3-5 degrees" range) and rounded to 1 decimal place:
//   south = 11.595 - 4  =  7.595  -> 7.6
//   north = 44.94278 + 4 = 48.94278 -> 48.9
//   west  = -6.944167 - 4 = -10.944167 -> -10.9
//   east  = 67.4308 + 4 = 71.4308 -> 71.4
// This is intentionally the full extent of every place the compiled data
// set ever positions (Table-of-Nations entries like Punt/Tarshish/India
// included), not just whichever handful of narratives happen to be lit in
// any one scene -- scripture mode can jump to any verse in the canon, so
// the lock has to cover everywhere a real place marker could ever land.
// The result is still a firmly regional box (Atlantic-adjacent Morocco to
// the Indus, just north of the Horn of Africa to the Caucasus) -- nowhere
// close to "the whole world" and nowhere close to the globe.
const BIBLICAL_WORLD_BOUNDS = [[7.6, -10.9], [48.9, 71.4]];

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
        maxBounds: BIBLICAL_WORLD_BOUNDS,
        maxBoundsViscosity: 1.0,
    }).setView(DEFAULT_CENTER, DEFAULT_ZOOM);

    // minZoom chosen empirically FROM THIS MAP'S OWN real container size --
    // Leaflet's getBoundsZoom(bounds, inside) has two modes and only ONE of
    // them is "fills the frame": plain getBoundsZoom(bounds) (inside=false,
    // what fitBounds uses) returns the LARGEST zoom at which the bounds still
    // fit inside the view WITHOUT CLIPPING, which at a merely-integer zoom
    // step routinely leaves both axes with slack (measured: at this map's
    // 1440x900 dev viewport it returned zoom 4, where BIBLICAL_WORLD_BOUNDS
    // only filled ~65% of the width and ~61% of the height -- Portugal and
    // India both visible beyond the "biblical world," the exact bug this
    // region lock exists to prevent). inside=true asks the OPPOSITE
    // question -- the SMALLEST zoom at which the viewport fits ENTIRELY
    // INSIDE the bounds, i.e. no area outside BIBLICAL_WORLD_BOUNDS is ever
    // visible -- which is what "fills the frame" actually means; confirmed
    // against the same 1440x900 viewport (zoom 5, both axes fully covered).
    // Computed fresh per map instance rather than a baked-in constant so it
    // adapts to any viewport this map didn't happen to be measured at (the
    // quality floor's "nothing breaks between 1024px and ultrawide").
    map.setMinZoom(map.getBoundsZoom(BIBLICAL_WORLD_BOUNDS, true));

    const tiles = L.tileLayer(TILE_URL, {
        maxNativeZoom: TILE_MAX_NATIVE_ZOOM,
        maxZoom: TILE_MAX_NATIVE_ZOOM,
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

    // Borders/arrows/landmarks (Task 15 controller ruling): a mini instance
    // gets NONE of the three -- not created, never populated -- rather than
    // created-but-fed-nothing. Borders are added to the map BEFORE arrows
    // so their shared overlayPane's DOM paint order puts them BELOW the
    // narrative threads (design-direction.md: borders are period
    // cartography; narrative threads read on top of it) -- see
    // BorderLayer's own header comment. Landmarks get their own pane,
    // z-indexed between overlayPane (400: borders/arrows) and markerPane
    // (600: scripture's places) -- design-direction.md: landmark labels
    // sit "a visual step below place labels so scripture's places stay the
    // foreground." A dedicated pane makes that ordering independent of DOM
    // insertion timing: landmarks are fetched/set once, asynchronously,
    // from World.razor (see setLandmarks below), while place markers come
    // and go on every window change -- relying on relative insertion order
    // between the two would be racy.
    let borders = null;
    let arrows = null;
    if (!mini) {
        borders = new BorderLayer();
        borders.addTo(map);

        arrows = new ArrowLayer(dotnetRef);
        arrows.addTo(map);

        map.createPane('landmarksPane');
        map.getPane('landmarksPane').style.zIndex = 500;
        map.getPane('landmarksPane').style.pointerEvents = 'none';
    }

    instances.set(id, { map, dotnetRef, mini, markers: new Map(), arrows, borders, landmarkMarkers: [] });
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
//
// Coordinates are run through nudgeCloseLatLng before use (both here and,
// transitively, by ArrowLayer's arrow endpoints, which look positions up
// from this same `markers` map): curated places occasionally geocode very
// close together -- sometimes to the EXACT same lat/lon (e.g. Shittim and
// the "plains of Moab" camp -- both real, distinct places per
// data/curated's own disambiguation comments, just resolved identically by
// the upstream geocoder), sometimes just close (Gilgal and Jericho, ~1.8km
// apart -- accurate geography, Gilgal genuinely was that close to Jericho).
// .atlas-marker's hit box is deliberately tiny specifically so merely-close
// neighbors already resolve correctly in the COMMON case (Rephidim/Mount
// Sinai and Marah/Elim, both ~10-13km apart, hover fine today -- see
// app.css) -- but once two places sit within a few km of each other their
// 4x4px hit boxes start to overlap, and the browser then delivers
// hover/click to whichever DOM element happens to paint on top, not
// deterministically to either place (caught by WORLD-2 and the arrow-hover
// test once Task 16's narratives introduced both the exact Shittim/Moab
// collision and the close-but-distinct Gilgal/Jericho pair). Nudging every
// place that lands within CLOSE_THRESHOLD_KM of an already-placed one (a
// real pairwise check across the scene, not just an exact-match shortcut)
// keeps each one independently hoverable without visibly moving away from
// its real-world location at any zoom this app uses.
export function setScene(id, sceneJson) {
    const inst = instances.get(id);
    if (!inst) {
        return;
    }

    const scene = typeof sceneJson === 'string' ? JSON.parse(sceneJson) : (sceneJson || {});
    const places = scene.places || [];
    const seen = new Set();
    const placed = []; // {lat, lon, origLat, origLon} already resolved this call, in scene order -- nudgeCloseLatLng checks each new candidate's ORIGINAL coords against these (fix round 1 / M3: not the nudged lat/lon, see nudgeCloseLatLng's own comment)

    for (const p of places) {
        seen.add(p.id);
        const [lat, lon] = nudgeCloseLatLng(p, placed);
        placed.push({ lat, lon, origLat: p.lat, origLon: p.lon });
        const prior = inst.markers.get(p.id);

        if (!prior) {
            const marker = L.marker([lat, lon], { icon: makeIcon(p) });
            wireEvents(marker, inst.dotnetRef, p.id);
            marker.addTo(inst.map);
            inst.markers.set(p.id, { marker, lat, lon, brightness: p.brightness, name: p.name });
            continue;
        }

        if (prior.lat !== lat || prior.lon !== lon || prior.brightness !== p.brightness || prior.name !== p.name) {
            prior.marker.setLatLng([lat, lon]);
            prior.marker.setIcon(makeIcon(p));
            prior.lat = lat;
            prior.lon = lon;
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

    // Mini instances never render arrows at all (Task 15 controller ruling
    // -- see init()'s own comment); guard rather than call setArrows on a
    // null inst.arrows.
    if (!inst.mini) {
        inst.arrows.setArrows(scene.arrows || [], inst.markers);
    }
}

// See setScene's own doc comment for why this exists. Distance is a plain
// equirectangular approximation (not geodesically precise) -- more than
// good enough to decide "too close to hover independently" at the few-km
// scale this operates at, and far cheaper than a real haversine. `placed`
// is fresh per setScene call but `places` arrives in a stable, deterministic
// order (server-sorted by place id -- see atlas-core::scene::lit_places),
// so the SAME place among the SAME neighbors always lands at the SAME
// nudge index call to call, keeping a stable scene's markers from jittering
// on every refetch.
//
// CLOSE_THRESHOLD_KM=5 sits deliberately between the one known-broken
// distance (Gilgal/Jericho, ~1.8km -- see setScene's doc comment) and the
// nearest known-working one (Marah/Elim, ~10.5km), so this fixes the
// former without touching (renudging) pairs that already hover correctly
// today. NUDGE_STEP_DEG (~8.9km) is deliberately LARGER than the threshold
// that triggers it, so a freshly nudged place always clears the distance
// that flagged it, and is the same order of magnitude as this app's
// already-working close-but-distinct neighbors. Successive collisions
// against the SAME neighborhood are spread around it at the golden angle
// (same idea phyllotaxis/sunflower-seed packing uses) rather than a single
// fixed direction, so a third or fourth place crowding one spot -- none
// exist today, but nothing here assumes exactly two -- would still land at
// its own distinct spot instead of stacking back onto an earlier nudge.
//
// Fix round 1 (M3): "nothing here assumes exactly two" was false until this
// fix -- `placed` entries are compared by their ORIGINAL (pre-nudge)
// coordinates (origLat/origLon), not their final nudged ones. Comparing
// against final positions was the actual bug: NUDGE_STEP_DEG is deliberately
// LARGER than CLOSE_THRESHOLD_KM (see above), so a nudged point always moves
// itself outside CLOSE_THRESHOLD_KM of the cluster it came from -- meaning a
// THIRD place at the same original spot would only ever count the still-
// unmoved first point as "close" (the second point, already nudged away,
// drops out of range), landing it on the exact same golden-angle slot as
// the second place instead of a fresh one. Original coordinates never move,
// so counting against those gives every coincident place in a cluster of
// any size its own distinct running count (1, 2, 3, ...) and hence its own
// slot, and preserves the existing call-to-call determinism (same place
// among the same neighbors, in the same server-sorted order, always sees
// the same count).
const GOLDEN_ANGLE_RAD = 2.399963229728653;
const CLOSE_THRESHOLD_KM = 5;
const NUDGE_STEP_DEG = 0.08;

function approxKm(lat1, lon1, lat2, lon2) {
    const dLat = (lat1 - lat2) * 111.32;
    const dLon = (lon1 - lon2) * 111.32 * Math.cos((lat1 + lat2) / 2 * Math.PI / 180);
    return Math.sqrt(dLat * dLat + dLon * dLon);
}

function nudgeCloseLatLng(p, placed) {
    let n = 0;
    for (const q of placed) {
        if (approxKm(p.lat, p.lon, q.origLat, q.origLon) < CLOSE_THRESHOLD_KM) {
            n++;
        }
    }
    if (n === 0) {
        return [p.lat, p.lon];
    }
    const angle = n * GOLDEN_ANGLE_RAD;
    return [p.lat + NUDGE_STEP_DEG * Math.sin(angle), p.lon + NUDGE_STEP_DEG * Math.cos(angle)];
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

// Lowercase-kebab-case of a landmark's name, matching the CONTRACT's
// landmark-{slug} testid exactly ("Mount Sinai" -> "mount-sinai"; "The
// Great Sea" -> "the-great-sea") -- mirrors atlas-etl's geo::kebab (same
// "runs of non-alphanumeric characters become one dash, no leading/
// trailing dash" rule), independently reimplemented here since map.js has
// no access to that Rust code.
function slugify(name) {
    return String(name).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
}

// Deterministically picks one of 4 diagonal quadrants to offset a
// landmark's label into, from its OWN coordinates (not its name) --
// screenshot review (fix round 1) found several curated points only a few
// real-world km apart (Mount Sinai/Mount Horeb/Wilderness of Sinai, all in
// the same corner of the Sinai peninsula) whose labels overlapped almost
// completely at overview zoom when every label used the same fixed offset
// direction. Hashing the coordinates (not the name) decorrelates well even
// for near-duplicate names like "Mount Sinai"/"Mount Horeb" -- verified
// against the real curated set (see the batch report) that this spreads
// that specific cluster across 3 different quadrants; it isn't a general
// collision-detection algorithm (two points could still coincidentally
// hash to the same quadrant), just a cheap, deterministic improvement over
// one fixed direction for everyone.
function labelDirection(lat, lon) {
    const dirs = ['ne', 'nw', 'se', 'sw'];
    const h = Math.abs(Math.round(lat * 9973 + lon * 7919));
    return dirs[h % dirs.length];
}

// Landmarks (design-direction.md's Atlas plate detail: "always visible,
// non-interactive, classic atlas typography"). No dot/glow -- just the
// label itself, offset from the point per labelDirection above (unlike
// makeIcon's marker+offset-label pair, there is no separate "capital" to
// offset from, so map.js decides the direction here instead of app.css
// assuming a single fixed one). Styling by kind (italic water names vs
// small-caps mountain/region names) is entirely app.css's job, driven by
// the data-kind attribute -- map.js only decides placement and the testid.
function makeLandmarkIcon(l) {
    const dir = labelDirection(l.lat, l.lon);
    const html = `<span class="landmark-label" data-kind="${esc(l.kind)}" data-dir="${dir}" data-testid="landmark-${esc(slugify(l.name))}">${esc(l.name)}</span>`;
    return L.divIcon({ html, className: 'landmark-label-icon', iconSize: [0, 0] });
}

// Fits the map's viewport to whatever markers are CURRENTLY on it. This
// module has no notion of "mode" or "first scene" -- World.razor decides
// entirely on its own when to call this (see its _lastFitMode field and
// the comments on DebouncedLoadScene/DebouncedLoadScriptureScene/
// OnAfterRenderAsync for the actual policy: the very first scene of the
// page's life, every scripture-mode load, and any load whose mode differs
// from the last one actually fit -- which is what makes returning to time
// mode after a scripture visit re-fit correctly). The one deliberately
// EXCLUDED case, unchanged since Task 10: an ordinary same-mode time-window
// change (slider drag, era click, readout Enter) does NOT re-fit -- panning
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
    if (!inst || !inst.arrows) {
        return;
    }

    inst.arrows.setIsolate(narrativeId ?? null);
}

// Replaces the border layer's geojson wholesale (Task batch-B: "time
// period accurate borders... swapped to the snapshot nearest the selected
// window") and makes sure it's visible -- a fresh snapshot arriving is
// always a reason to show it, even if scripture mode had it hidden most
// recently (World.razor's own mode bookkeeping is what actually prevents
// this from being CALLED during scripture mode in the first place; this
// function doesn't need to know about modes at all).
export function setBorders(id, geojsonString) {
    const inst = instances.get(id);
    if (!inst || !inst.borders) {
        return;
    }

    const fc = typeof geojsonString === 'string' ? JSON.parse(geojsonString) : (geojsonString || {});
    inst.borders.setData(fc);
}

// Shows/hides the border layer without touching its data (Task batch-B:
// "Scripture mode: hide the border layer AND the tag... restore on return
// to time mode").
export function setBordersVisible(id, visible) {
    const inst = instances.get(id);
    if (!inst || !inst.borders) {
        return;
    }

    inst.borders.setVisible(!!visible);
}

// Renders the curated landmark list (Task batch-B: "always-visible
// landmark labels"). Called once per map instance (World.razor fetches
// landmarks once on init) -- clears any prior markers first so a second
// call is still idempotent rather than duplicating labels.
export function setLandmarks(id, landmarksJson) {
    const inst = instances.get(id);
    if (!inst || inst.mini) {
        return;
    }

    const landmarks = typeof landmarksJson === 'string' ? JSON.parse(landmarksJson) : (landmarksJson || []);
    for (const marker of inst.landmarkMarkers) {
        inst.map.removeLayer(marker);
    }
    inst.landmarkMarkers = landmarks.map(l => {
        const marker = L.marker([l.lat, l.lon], { icon: makeLandmarkIcon(l), interactive: false, pane: 'landmarksPane' });
        marker.addTo(inst.map);
        return marker;
    });
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
        // World.razor resets its isolate state (`_isolated`) to null on
        // every new scene, but that alone only fixes the LEGEND's
        // aria-pressed -- an arrow whose key (narrative:order) survives the
        // window change goes through the `else` branch below, which never
        // touched data-faded, so it would otherwise keep whatever
        // "true"/"false" a PRIOR setIsolate call left it at (review fix
        // round 1: isolate narrative A, then change to a window that still
        // has one of B/C's same-keyed arrows -- it stayed faded at opacity
        // .12 with no legend button pressed to explain why). Resetting
        // every EXISTING path unconditionally here, before the diff below,
        // makes "a fresh scene starts unisolated" true for arrows exactly
        // the same way it's already true for newly-created ones (which get
        // data-faded="false" from _createPath regardless) -- one JS-side
        // source of truth, no second SetIsolate(null) interop round-trip
        // needed from World.razor.
        for (const entry of this._paths.values()) {
            entry.path.setAttribute('data-faded', 'false');
        }

        this._placesById = placesById;
        const list = arrows || [];

        // parallelIndex = position among arrows whose curves would land in
        // the same crowded neighborhood, centered 0, +1, -1, +2, ... --
        // grouped fresh on every call (which arrows are crowded together can
        // change scene to scene) in the scene's own array order, which is
        // the narrative/order the server emits arrows in (stable and
        // deterministic). Grouped by a COARSE ROUNDED MIDPOINT rather than
        // an exact from/to place-id match: two arrows between the exact same
        // two places obviously share a midpoint (the original "this
        // narrative walks A-B then later B-A again" case this always
        // handled) -- but Task 16 surfaced a second case a bare place-id
        // match misses entirely: two DIFFERENT narratives' arrows on
        // DIFFERENT place pairs that still sit in the same real-world
        // corner (conquest's Shittim->Gilgal and exodus's Moab->Jericho are
        // both part of the single crowded "plains of Moab" staging area
        // opposite Jericho -- their short, near-coincident bowed curves
        // otherwise overlap enough that a hover square-in-the-middle of one
        // lands on the OTHER instead; confirmed via a live DOM probe, not
        // guessed). Rounding each arrow's own from/to midpoint to a coarse
        // grid catches both cases under the one existing mechanism, since a
        // shared OR merely nearby midpoint rounds to the identical cell
        // either way.
        const CLUSTER_GRID_DEG = 0.05; // ~5km at these latitudes
        const clusterKey = a => {
            const from = placesById.get(a.from_place);
            const to = placesById.get(a.to_place);
            if (!from || !to) {
                return [a.from_place, a.to_place].slice().sort().join('|'); // ARROW-1 guarantees this never happens for a real scene; still a valid, stable key if it somehow did
            }
            const midLat = (from.lat + to.lat) / 2;
            const midLon = (from.lon + to.lon) / 2;
            return `${Math.round(midLat / CLUSTER_GRID_DEG)},${Math.round(midLon / CLUSTER_GRID_DEG)}`;
        };
        const clusterTotal = new Map(); // clusterKey -> total members in this scene
        for (const a of list) {
            const key = clusterKey(a);
            clusterTotal.set(key, (clusterTotal.get(key) ?? 0) + 1);
        }
        const clusterSeen = new Map(); // clusterKey -> count so far
        const parallelIndexByKey = new Map(); // "{narrative}:{order}" -> centered index
        for (const a of list) {
            const key = clusterKey(a);
            const k = clusterSeen.get(key) ?? 0;
            clusterSeen.set(key, k + 1);
            // A multi-member cluster shifts every member's index up by one
            // (1, -1, 2, -2, ... -- never 0), so EVERY arrow in a crowded
            // spot gets a nonzero push, not just the second-and-later ones.
            // A lone unboosted member (index 0, the pre-Task-16 behavior)
            // sitting in the shared base region keeps failing to separate
            // from a boosted neighbor no matter how far that neighbor bows,
            // because a quadratic bezier's bbox always includes its own f/t
            // endpoints -- for a crowded cluster those already overlap, so
            // the "straight" member's tiny bbox stays inside the bowed
            // member's ever-larger one regardless of magnitude (verified
            // numerically, not assumed, before landing on this fix). A
            // single-member cluster (the overwhelming majority of arrows)
            // is completely unaffected: index 0, no offset, identical to
            // the pre-existing behavior.
            const total = clusterTotal.get(key) ?? 1;
            parallelIndexByKey.set(arrowKey(a), centeredIndex(total > 1 ? k + 1 : k));
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
        // MIN_BOW_PX (Task 16 finding): two real, geographically-close places
        // (e.g. Shittim/Gilgal, ~7px apart at this app's typical whole-scene
        // zoom) can put `dist` itself down in single-digit pixels, making the
        // proportional-only `0.18 * dist` bow negligible -- the resulting
        // curve (plus its arrowhead decoration) is so compact that BOTH
        // endpoint markers sit inside its bounding box, and since
        // `pointer-events: bounding-box` makes that whole box the arrow's
        // hover target while markerPane still paints above overlayPane, the
        // box's CENTER (exactly where a plain .hover() lands) can fall right
        // on top of one marker's own tiny 4x4px hit area -- confirmed via
        // `document.elementFromPoint` at that exact point returning the
        // marker div, not the arrow path (root-caused, not guessed: this is
        // what silently broke the arrow-hover tooltip once a real narrative
        // leg made this the first arrow in a scene). A floor well above any
        // marker's hit box guarantees the curve bulges out past it regardless
        // of how close the two endpoints are; only already-very-short arrows
        // are affected; the proportional term still wins for any normal-length
        // arrow (0.18 * dist alone exceeds this floor past ~100px).
        //
        // PARALLEL_STEP_PX (raised from 14, also a Task 16 finding): a
        // quadratic bezier's bounding box always includes its own f/t
        // endpoints, so for a crowded multi-arrow cluster (see
        // setArrows/clusterKey's own comment) the step needs to be large
        // enough that a cluster member's bbox CENTER clears every other
        // member's bbox entirely, not merely grow its own tail -- verified
        // numerically against the real Shittim/Gilgal vs Moab/Jericho
        // cluster (a Node simulation of this exact bbox math, not a guess)
        // before landing on this value.
        const MIN_BOW_PX = 30;
        const PARALLEL_STEP_PX = 40;
        const mag = Math.max(0.18 * dist, MIN_BOW_PX) + PARALLEL_STEP_PX * entry.parallelIndex;
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

// --- BorderLayer: period-accurate country/culture-region strokes ---------
// (Task batch-B, design-direction.md's "Atlas plate detail: the plate
// carries period cartography").
//
// A custom L.Layer managing exactly one <svg class="atlas-borders"> in the
// map's overlayPane, created and added to the map BEFORE ArrowLayer in
// init() so its DOM node lands first: overlayPane assigns no z-index
// between layers sharing it, so paint order is plain DOM order, and
// borders must render BELOW the narrative threads (see init()'s own
// comment). Both already sit below markerPane (places) via Leaflet's own
// per-pane z-indices (400 vs 600), independent of DOM order.
//
// Deliberately non-interactive -- no event wiring at all, unlike
// ArrowLayer: borders are decoration, never a hover/click target. app.css's
// .atlas-borders rule also sets pointer-events:none explicitly (Leaflet's
// own default `.leaflet-pane > svg path` rule already implies it, but
// relying on a rule documented as being about ARROWS -- see that rule's own
// comment in app.css -- would be less clear here).
//
// No diffing (unlike ArrowLayer.setArrows/setScene's marker diff): a
// border-snapshot swap is a rare, deliberate window-change event driven by
// a 150ms-debounced fetch, not a hot per-frame path, so replacing the
// layer's content wholesale on every setData call is simple and cheap
// enough.
const BorderLayer = L.Layer.extend({
    initialize() {
        this._paths = []; // [{ path, feature }]
    },

    onAdd(map) {
        this._map = map;
        this._svg = svgEl('svg', { class: 'atlas-borders' });
        map.getPane('overlayPane').appendChild(this._svg);
        return this;
    },

    onRemove() {
        this._svg.remove();
        this._paths = [];
    },

    getEvents() {
        return { zoomend: this._redraw, moveend: this._redraw };
    },

    setData(featureCollection) {
        this._svg.replaceChildren();
        const features = (featureCollection && featureCollection.features) || [];
        this._paths = features.map(feature => {
            const path = svgEl('path', { class: 'atlas-border' });
            this._svg.appendChild(path);
            return { path, feature };
        });
        this.setVisible(true);
        this._redraw();
    },

    setVisible(visible) {
        this._svg.style.display = visible ? '' : 'none';
    },

    _redraw() {
        if (!this._map) {
            return;
        }
        for (const entry of this._paths) {
            entry.path.setAttribute('d', this._pathData(entry.feature));
        }
    },

    // One <path> per feature, one "M...Z" subpath per ring of every
    // polygon in its geometry (atlas-etl always compiles MultiPolygon --
    // see atlas-etl/src/borders.rs -- but this reads plain
    // geometry.coordinates structurally rather than assuming that, so it
    // degrades harmlessly on any array-of-rings-of-points shape). A single
    // SVG path element can represent a multi-ring/multi-polygon shape this
    // way, exactly how Leaflet's own SVG renderer draws multi-ring
    // polygons (holes and all -- fill-rule defaults to nonzero, which
    // reads holes correctly given GeoJSON's right-hand-rule ring winding).
    _pathData(feature) {
        const polygons = (feature.geometry && feature.geometry.coordinates) || [];
        let d = '';
        for (const polygon of polygons) {
            for (const ring of polygon) {
                if (!ring || ring.length === 0) {
                    continue;
                }
                const pts = ring.map(([lon, lat]) => this._map.latLngToLayerPoint([lat, lon]));
                d += `M${pts.map(p => `${p.x},${p.y}`).join('L')}Z`;
            }
        }
        return d;
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
