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

// Basemap -- Batch C / design-direction.md World THIRD REVISION (supersedes
// the earlier NatGeo_World_Map pick, rejected by user feedback 2026-08-19 as
// "we should not be seeing airports and stuff... don't put literally
// everything at once"): NatGeo's own city/road/country labels were exactly
// the anachronistic clutter the plate has to be free of now -- "the basemap
// contributes ZERO text," full stop, so any labeled service was disqualified
// before comparison even started. Esri's two LABEL-FREE candidates,
// World_Shaded_Relief and World_Terrain_Base, were screenshot-compared at
// all three of this batch's windows (patriarchs -2100/-2085, egypt-exodus
// -1446/-1406, scripture ACT.27 -- Paul's voyage, chosen for its heavy
// Mediterranean coverage) at a 1280x800 viewport, through the app's own
// existing sepia(.12) saturate(.95) tile-pane filter (kept unfiltered would
// only sharpen the same verdict): World_Shaded_Relief reads as ONE warm,
// monotone parchment surface -- land and sea share the same tan relief
// shading, so coastlines read from terrain contrast alone, exactly
// "illuminated atlas plate on a clean canvas." World_Terrain_Base paints
// oceans in a saturated cyan bathymetry gradient that fights the parchment
// concept outright (most visible on the ACT.27 shot, almost entirely sea) --
// it reads as a modern GIS/weather reference layer, not an aged plate.
// World_Shaded_Relief wins. Screenshots + the rest of this reasoning are in
// the batch report. Both candidates' native tiles top out at LOD 13 (see
// tile/{z}/{y}/{x}?f=json on each service) -- lower than NatGeo's 16, so
// TILE_MAX_NATIVE_ZOOM drops to match (Leaflet still lets the MAP zoom past
// this, upscaling tiles -- see maxZoom on the tileLayer below); Carto
// light_nolabels (already label-free) stays the tileerror fallback,
// unchanged.
const TILE_URL = 'https://server.arcgisonline.com/ArcGIS/rest/services/World_Shaded_Relief/MapServer/tile/{z}/{y}/{x}';
const TILE_FALLBACK = 'https://basemaps.cartocdn.com/light_nolabels/{z}/{x}/{y}.png';
// Esri's own copyrightText for the World_Shaded_Relief service, verbatim --
// fetched live from tile/{z}/{y}/{x}?f=json on 2026-08-19 per the standing
// citation-integrity rule (never guess/paraphrase a source's own credit
// line). Keep this string byte-for-byte in sync with the credits panel
// (MainLayout.razor's attribution popover), which quotes the same source.
const TILE_ATTRIBUTION = 'Copyright:(c) 2014 Esri';
const TILE_MAX_NATIVE_ZOOM = 13; // World_Shaded_Relief's own max LOD (see comment above); also the map's maxZoom -- see the tileLayer's own maxZoom option below for why it's never upscaled past this

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

    // minZoom -- REVERSED, Batch C (design-direction.md World THIRD
    // REVISION: "a GENEROUS zoom range... the prior lock was too tight to
    // zoom out," direct user feedback 2026-08-19: "can't zoom out enough").
    // Task 10's original pick used getBoundsZoom(bounds, true) -- the
    // SMALLEST zoom at which the viewport fits ENTIRELY INSIDE the bounds,
    // i.e. the bounds always fill the frame with zero slack -- which is
    // exactly what made zooming OUT past that point impossible: there was
    // no looser zoom level left to go to. inside=false (what fitBounds
    // itself uses) asks the OPPOSITE question -- the LARGEST zoom at which
    // the bounds still fit inside the view WITHOUT CLIPPING -- which at a
    // merely-integer zoom step routinely leaves both axes with slack
    // (measured at this map's 1440x900 dev viewport: zoom 4, where
    // BIBLICAL_WORLD_BOUNDS only fills ~65% of the width/~61% of the
    // height). That slack is no longer a bug to avoid -- it's the literal
    // "plus breathing room" the brief asks for, so this now uses inside=
    // false directly rather than inverting it back to a tight fit. Computed
    // fresh per map instance (not a baked-in constant) so it adapts to any
    // viewport this map didn't happen to be measured at (the quality
    // floor's "nothing breaks between 1024px and ultrawide"); confirmed via
    // a throwaway script at 1024x768, 1280x800 (the brief's own reference
    // viewport), 1440x900, and 1920x1080 -- all four resolve to the same
    // zoom 4, so the whole locked region comfortably fits with room to
    // spare across that entire width range, not just the one viewport.
    map.setMinZoom(map.getBoundsZoom(BIBLICAL_WORLD_BOUNDS, false));

    const tiles = L.tileLayer(TILE_URL, {
        maxNativeZoom: TILE_MAX_NATIVE_ZOOM,
        // maxZoom == maxNativeZoom, deliberately -- NOT upscaled past it.
        // Screenshot review (batch report) tried upscaling to 16 (matching
        // the old NatGeo basemap's own native depth) and found it a visibly
        // blurred, washed-out mess at zoom 14-16 (Leaflet stretches the last
        // real z13 tile up to 8x to fill a z16 grid cell): NatGeo genuinely
        // needed zoom 16 because ITS deep zoom levels carried fine TEXT
        // (small-town labels) that had to stay crisp; this basemap carries
        // no text at all (every label on the plate is ours, rendered as
        // real DOM/SVG elements at their own CSS pixel size, never baked
        // into the raster) -- there is nothing on the tile itself that
        // benefits from deeper zoom, only a texture that degrades. And the
        // brief's actual requirement -- "maxZoom high enough to separate
        // neighboring cities" -- is already massively satisfied at native
        // zoom 13 alone: Jerusalem/Bethlehem (~8km apart) sit roughly 400px
        // apart there already, an order of magnitude past what independent
        // hovering needs.
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
    let polities = null;
    let arrows = null;
    if (!mini) {
        // polityLabelsPane (Batch C, design-direction.md World THIRD
        // REVISION: "polity names rendered from the historical border
        // features"): z-indexed BELOW landmarksPane (500) -- design-
        // direction.md lists "polity names, seas... curated biblical
        // landmarks, and the lit place markers" in that order, least-to-
        // most foreground, and BorderLayer's own polity labels are the
        // background-most text on the plate, one step above the raw
        // border strokes/fill they're paired with (overlayPane, 400).
        // Created here (not inside BorderLayer.onAdd) so it exists before
        // polities.addTo(map) below ever runs. BorderLayer itself reads it
        // back via map.getPane -- see its own onAdd.
        map.createPane('polityLabelsPane');
        map.getPane('polityLabelsPane').style.zIndex = 450;
        map.getPane('polityLabelsPane').style.pointerEvents = 'none';

        polities = new BorderLayer();
        polities.addTo(map);

        arrows = new ArrowLayer(dotnetRef);
        arrows.addTo(map);

        map.createPane('landmarksPane');
        map.getPane('landmarksPane').style.zIndex = 500;
        map.getPane('landmarksPane').style.pointerEvents = 'none';

        // quietPane (Batch E2, the ever-present graph): z-indexed ABOVE
        // landmarksPane (500) but BELOW Leaflet's own default markerPane
        // (600, where the lit ember markers live) -- guarantees a quiet dot
        // can NEVER visually paint over an ember regardless of the two
        // layers' own screen-Y stacking (Leaflet's default per-marker
        // z-index is by screen position WITHIN a pane, not across panes; a
        // separate, lower pane is the only way to make "embers always beat
        // quiet dots" hold unconditionally -- see hoverSafety.ts's own
        // comment on same-pane stacking for why THAT can't be trusted
        // across two different marker kinds). Slots into design-direction.md's
        // existing least-to-most-foreground order right where a "place
        // marker, just unlit" belongs: polity names -> landmarks -> quiet
        // dots -> lit places. Interactive (no pointerEvents override, unlike
        // landmarksPane) -- quiet dots must stay hoverable/clickable.
        map.createPane('quietPane');
        map.getPane('quietPane').style.zIndex = 550;
    }

    instances.set(id, {
        map, dotnetRef, mini, markers: new Map(), arrows, polities, landmarkMarkers: [], litByName: new Map(),
        quietMarkers: new Map(), // Batch E2 -- see setScene's own quiet-places diff for the shape each entry carries
    });

    // Zoom-tiered label density (design-direction.md's declutter rule --
    // see applyLabelTier's own comment for the concrete tiers/thresholds).
    // Markers themselves never respond to zoom (design-direction.md: markers
    // "stay visible at every zoom -- only LABELS tier"); this only ever
    // touches label elements' display. Gated on !mini here (rather than
    // inside applyLabelTier) so a mini instance never pays for a listener
    // it has no labels to update -- applyLabelTier ALSO no-ops on mini as a
    // second, belt-and-suspenders guard, consistent with every other mini
    // exclusion in this file.
    if (!mini) {
        map.on('zoomend', () => applyLabelTier(instances.get(id)));
    }

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
// .atlas-marker's hit box is deliberately tiny (4x4px, unchanged since
// WORLD-1/2 -- Batch C tried growing it and reverted, see app.css's own
// .atlas-marker comment for the real regression that caught) specifically
// so merely-close neighbors already resolve correctly in the COMMON case
// (Rephidim/Mount Sinai and Marah/Elim, both ~10-13km apart, hover fine
// today -- see app.css) -- but once two places sit within a few km of each
// other their 4x4px hit boxes start to overlap, and the browser then
// delivers hover/click to whichever DOM element happens to paint on top,
// not
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

    // Batch E2 (the ever-present graph): quiet dots, diffed by id the exact
    // same way lit markers just were above -- unseen ids added, vanished ids
    // removed, unchanged ids left alone. Never nudged (no nudgeCloseLatLng
    // call, no `placed` bookkeeping): quiet dots are furniture, not a
    // precision-hover surface the way embers are (batch-e2-brief.md's own
    // framing) -- rendering two coincident quiet dots exactly on top of each
    // other, or a quiet dot exactly under an ember, is an acceptable, rare
    // edge case for a background layer, not a bug worth a second nudge
    // system. Skipped ENTIRELY on a mini instance (no quietPane was ever
    // created for one -- see init()) -- `scene.quiet_places` is empty for
    // every scene MiniWorld ever loads anyway (scripture-only), but this
    // guard makes "no quiet dots on mini" a real, defensive guarantee
    // (batch-e2-brief.md Requirement 2) rather than an incidental fact of
    // what MiniWorld happens to fetch today.
    if (!inst.mini) {
        const quietPlaces = scene.quiet_places || [];
        const quietSeen = new Set();
        for (const p of quietPlaces) {
            quietSeen.add(p.id);
            const prior = inst.quietMarkers.get(p.id);

            if (!prior) {
                const marker = L.marker([p.lat, p.lon], { icon: makeQuietIcon(p), pane: 'quietPane' });
                wireQuietEvents(marker, inst.dotnetRef, p.id);
                marker.addTo(inst.map);
                inst.quietMarkers.set(p.id, { marker, lat: p.lat, lon: p.lon, displayName: p.display_name });
                continue;
            }

            if (prior.lat !== p.lat || prior.lon !== p.lon || prior.displayName !== p.display_name) {
                prior.marker.setLatLng([p.lat, p.lon]);
                prior.marker.setIcon(makeQuietIcon(p));
                prior.lat = p.lat;
                prior.lon = p.lon;
                prior.displayName = p.display_name;
            }
        }

        for (const [placeId, entry] of inst.quietMarkers) {
            if (!quietSeen.has(placeId)) {
                inst.map.removeLayer(entry.marker);
                inst.quietMarkers.delete(placeId);
            }
        }
    }

    // Fix round 1 addendum: rebuilt fresh here, every time `markers` just
    // changed (the only thing that can change it) -- see buildLitByName's
    // own comment for why this one Map feeds BOTH applyLabelTier's
    // landmark dedupe AND BorderLayer's polity dedupe. Fed to the border
    // layer BEFORE applyLabelTier below so a same-tick redraw (setLitPlaces
    // calls _redraw() itself) never races the marker-side pass.
    inst.litByName = buildLitByName(inst.markers);
    if (!inst.mini) {
        inst.polities.setLitPlaces(inst.litByName);
    }

    // Re-applied on every scene load, unconditionally (applyLabelTier itself
    // no-ops on mini): a changed/new marker's icon was just rebuilt from
    // scratch by makeIcon above (setIcon replaces the DOM node, so any
    // display:none a PRIOR call left on the old node is gone with it), and
    // even an unchanged marker's own collision-cell claim could now differ
    // if the scene itself didn't change but the zoom or the surrounding
    // candidate set did between loads -- either way, every marker's label
    // needs a fresh collision decision against the map's CURRENT zoom/pan,
    // every time (place/quiet labels no longer have a brightness-vs-tier
    // check to go stale here at all, per this batch's own scope amendment --
    // see applyLabelTier's own comment -- but landmarks still do).
    applyLabelTier(inst);

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
// NUDGE_STEP_DEG -- RE-TUNED, Batch C2 (requirement 0b: "scale
// nudgeCloseLatLng's separation distance with the new marker radius so
// hover targets never overlap"). Picked around the OLD 4x4px marker (2px
// hit-radius, see the pre-Batch-C2 history below); the ember marker's own
// 26x26px hit box (10px core + this batch's `::after` inset:-8px padding,
// requirement 0c -- 13px radius) needed a fresh derivation, not a guess.
//
// A throwaway script rendered the real exodus-window scene (this file's
// own worst-case, richest test window -- WORLD-2/world-hover-text.spec.ts
// both already pick it for exactly that richness) at this app's own
// 1280x800 reference viewport, read every marker's REAL bounding-box
// center via Playwright, and fit a local lat/lon->screen-px affine
// transform from those (Web Mercator is locally linear at this scale, well
// under 4px residual across the tight Levant/Sinai cluster this matters
// for). Replaying nudgeCloseLatLng's own algorithm against that fitted
// projection found NUDGE_STEP_DEG=0.6 (~63-67km, latitude-dependent)
// separates both pairs THIS threshold (CLOSE_THRESHOLD_KM, unchanged --
// see below for why it stays at 5) actually nudges to 29px+ (a real
// margin over the 26px hit-box floor, not the old ~0.3px near-miss the
// pre-Batch-C2 comment below describes): moab-2/shittim (the exact 0km
// coincidence) to 29.2px, Gilgal/Jericho (~1.8km) to 29.3px.
//
// CLOSE_THRESHOLD_KM stays at 5 -- a first attempt at this fix raised it to
// 15 specifically to ALSO reach two other, larger-but-still-real "close
// neighbor" pairs the OLD tiny hit box tolerated (Marah/Elim ~10.5km,
// Rephidim/Mount Sinai ~13km -- see the pre-Batch-C2 history below), and
// the same grid search confirmed that DID separate them cleanly too
// (32-35px). It was reverted after a DIFFERENT real scene caught a
// concrete regression it caused: the apostolic window (AD 46-48) lights
// both "Philippi" and its own port city "Neapolis", genuinely 13.92km
// apart -- INSIDE the raised 15km threshold, OUTSIDE the original 5km one.
// Nudged by the new, much larger NUDGE_STEP_DEG (needed for the exodus
// cluster's OWN much-more-zoomed-OUT view), Philippi jumped ~60-70km from
// its curated position -- at the apostolic scene's own, much CLOSER
// fitScene zoom (a small two-year window, nowhere near as spread out as
// the full exodus scene), that jump pushed its marker (and the hover card
// anchored to it) most of the way off the top of the viewport, breaking
// world-hover-text.spec.ts's own place-card-more test for it (confirmed
// live: `document.elementFromPoint` at the button's own pre-move screen
// position resolved to `place-card-more` before the pointer arrived, and
// to the surrounding `place-card` div after -- the button had moved out
// from under it mid-journey). This is the general problem with a single,
// GLOBAL, zoom-INDEPENDENT threshold/step pair: whether two real places
// need separating depends on how close THEIR OWN scene renders them, which
// varies scene to scene, but nudging is decided once, before any zoom is
// known, and the same result has to hold at every zoom the marker is ever
// viewed at. 5km safely covers only the two pairs confirmed to need it
// (the exact coincidence and its near-exact sibling) without reaching far
// enough to catch a real, ordinary city/port pair like Philippi/Neapolis
// in a DIFFERENT, more tightly-zoomed scene. Marah/Elim and Rephidim/Mount
// Sinai's OWN new collision risk (introduced by the bigger hit target, in
// the exodus scene specifically) is handled the same way the exodus
// scene's other, structurally-unfixable close cluster already has to be --
// see world-map.spec.ts's own WORLD-2 comment for the dynamic, real-
// rendered-pixel filter that keeps the property suite honest about
// exactly this.
//
// Everything below this point is the PRE-Batch-C2 history CLOSE_THRESHOLD_KM
// and NUDGE_STEP_DEG already carried, unchanged in substance and (for
// CLOSE_THRESHOLD_KM) unchanged in its own literal number too:
// CLOSE_THRESHOLD_KM=5 sat deliberately between the one known-broken
// distance (Gilgal/Jericho, ~1.8km -- see setScene's doc comment) and the
// nearest known-working one (Marah/Elim, ~10.5km), so it fixed the former
// without touching (renudging) pairs that already hovered correctly under
// the OLD, much smaller hit box -- still exactly true today, just against
// a bigger hit box needing a bigger NUDGE_STEP_DEG to clear it (see
// above). NUDGE_STEP_DEG (~8.9km, now ~63-67km) was and is deliberately
// LARGER than the threshold that triggers it, so a freshly nudged place
// always clears the distance that flagged it, and was the same order of
// magnitude as this app's already-working close-but-distinct neighbors AT
// THE TIME. Successive collisions against the SAME neighborhood are spread
// around it at the golden angle (same idea phyllotaxis/sunflower-seed
// packing uses) rather than a single fixed direction, so a third or fourth
// place crowding one spot -- none exist today outside WORLD-3's own
// synthetic rig -- still lands at its own distinct spot instead of
// stacking back onto an earlier nudge.
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
const NUDGE_STEP_DEG = 0.6;

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

// Batch E2 fix (self-review finding, not in the original brief): a real bug
// root-caused while verifying this batch against world-hover-text.spec.ts's
// own pre-existing place-card-more/collapse tests, which started failing
// once quiet dots existed. Root cause, confirmed live via
// `document.elementFromPoint` at every step of a real multi-step pointer
// transit (not guessed): the existing hover-persistence design (batch-c2-
// brief.md requirement 0c) makes ANY marker's plain `mouseover` immediately
// adopt it as `_hoverPlace` -- correct and desired for a DELIBERATE hover,
// but with ~200 more small quiet dots now scattered across a typical dense
// scene (the exodus window's own 190), a pointer's ORDINARY straight-line
// travel from an ember marker to that SAME place's own place-card-more
// button (anchored at the CARD's position, not the marker's) has a real,
// observed chance of grazing an unrelated quiet dot's hit-halo along the
// way -- confirmed: hovering "jericho-1"/"timnath-serah" then moving toward
// their card's place-card-more crossed "quiet-marker-caesarea" mid-transit,
// silently swapping the open card to Caesarea (0 events, since it's quiet)
// a split second before the click landed, breaking the "expand THIS
// place's own verses" interaction the user (or a test) was actually doing.
// This is a real UX regression this batch's own quiet-dot feature
// introduces, not a pre-existing issue with lit markers (which were sparse
// enough, and spaced apart enough, that an ordinary marker-to-its-own-card
// transit rarely if ever crossed a DIFFERENT one).
//
// Fix: quiet markers alone (lit markers are completely untouched -- this
// function is never called for them, so every existing CONTRACT-tested lit-
// marker hover timing is byte-for-byte unchanged) get a short "hover
// intent" debounce, a standard, well-understood UX pattern (the same idea
// browser tooltips and libraries like hoverIntent use): a quiet marker only
// actually adopts `_hoverPlace` (fires OnPlaceHover) after the pointer has
// genuinely DWELLED on it for QUIET_HOVER_INTENT_MS -- a fast graze during
// transit to somewhere else never accumulates that dwell time before
// `mouseout` fires and cancels the pending call, so nothing ever happens
// for a pass-through (neither OnPlaceHover NOR OnPlaceLeave fires -- from
// World.razor's own point of view, a graze that never committed is
// completely invisible, not a hover-then-immediate-leave pair). A genuine,
// deliberate hover comfortably clears the delay (150ms is below the
// ~100-200ms threshold most UX research puts at "still feels instant") and
// opens the SAME quiet card exactly as before.
const QUIET_HOVER_INTENT_MS = 150;

function wireQuietEvents(marker, dotnetRef, placeId) {
    let timer = null;
    let committed = false;
    marker.on('mouseover', e => {
        const pt = e.containerPoint;
        clearTimeout(timer);
        timer = setTimeout(() => {
            committed = true;
            dotnetRef.invokeMethodAsync('OnPlaceHover', placeId, pt.x, pt.y);
        }, QUIET_HOVER_INTENT_MS);
    });
    marker.on('mouseout', () => {
        clearTimeout(timer);
        // Only signal a leave if a hover was actually committed -- a graze
        // that never fired OnPlaceHover has nothing to leave (see this
        // function's own header comment for the full race this avoids).
        if (committed) {
            committed = false;
            dotnetRef.invokeMethodAsync('OnPlaceLeave');
        }
    });
    // Click is a discrete, deliberate action (not a continuous transit risk
    // the way mouseover is) -- fires immediately, same as a lit marker's own.
    marker.on('click', e => dotnetRef.invokeMethodAsync('OnPlaceClick', placeId, e.containerPoint.x, e.containerPoint.y));
}

function makeIcon(p) {
    const brightness = Math.min(5, Math.max(1, p.brightness | 0 || 1));
    const html = `<div class="atlas-marker glow-${brightness}" data-testid="marker-${esc(p.id)}"><span class="atlas-label">${esc(p.name)}</span></div>`;
    return L.divIcon({ html, className: 'atlas-marker-icon', iconSize: [0, 0] });
}

// Batch E2 (the ever-present graph): a quiet dot -- no glow-N class at all
// (so none of .atlas-marker's own box-shadow/breathe rules, which key off
// exactly that class, ever apply -- there is nothing here to "turn off" per
// window, it simply never carries the class that turns them on), a small
// muted `.quiet-marker` core instead of `.atlas-marker`'s ember one, and its
// own `.quiet-label` (not `.atlas-label`) so app.css can style the two
// registers independently while keeping the file's one-class-per-rule
// discipline (see app.css's own header comment). Reuses the SAME
// `atlas-marker-icon` wrapper className as makeIcon's ember (the plain
// opacity-only entrance fade, `atlas-fade-in` -- there is no separate
// "quiet-marker-icon" wrapper class because that fade is the only thing the
// wrapper itself contributes, and it's identical for both kinds).
// `p.display_name` is quiet_places' own (and ONLY) name field -- unlike a
// lit ScenePlace, there is no separate `name` key to prefer here (see
// wire.rs's own QuietPlace comment on why the payload stays lean).
function makeQuietIcon(p) {
    const html = `<div class="quiet-marker" data-testid="quiet-marker-${esc(p.id)}"><span class="quiet-label">${esc(p.display_name)}</span></div>`;
    return L.divIcon({ html, className: 'atlas-marker-icon', iconSize: [0, 0] });
}

// Zoom-tiered label DENSITY -- LANDMARK/POLITY ONLY as of Batch E2 (the
// ever-present graph). Originally (Batch C / design-direction.md's declutter
// rule) gated PLACE labels too, keyed off brightness (see BRIGHT_LABEL_MIN,
// removed) -- SUPERSEDED by a direct, mid-batch scope amendment (user
// direction, this batch: "in general, have all biblically relevant names of
// places showing on the map at all times, given the place existed at the
// point in time that we're looking at... zooming in reveals what collision
// dropped"). Place labels (BOTH lit `.atlas-label` and quiet `.quiet-label`)
// now participate in applyLabelTier's shared candidates/collision pass
// UNCONDITIONALLY, every zoom -- COLLISION DAMPING (see (b) below) is the
// ONLY thing that ever hides one now, never the tier alone. Landmarks and
// polity labels are explicitly UNCHANGED by this amendment ("polity/
// landmark rules unchanged") -- they keep exactly the tier gate they always
// had, below:
//   FAR  (< ZOOM_TIER_MID):  polity names (own visibility rule, see
//        BorderLayer) + seas (landmarks kind=water) only.
//   MID  (< ZOOM_TIER_NEAR): + "major" landmarks (water + mountain).
//   NEAR (>= ZOOM_TIER_NEAR): + "dimmer" landmarks (region) too.
// Markers (the lamp/quiet dots themselves) are NEVER touched here -- design-
// direction.md: markers "stay visible at every zoom -- only LABELS tier,"
// and this batch's own amendment doesn't touch dot visibility either, only
// which labels are ALLOWED to compete for a spot at all before collision
// damping decides who wins one.
const ZOOM_TIER_MID = 6;
const ZOOM_TIER_NEAR = 9;

// Fix round 1 (M1 -- review finding: "zoom-tiered label density does not
// prevent local label collision; the new default view is visibly
// cluttered"). Zoom tiering alone only answers "how MANY labels are
// allowed at this zoom" -- it says nothing about whether the survivors
// actually fit without stacking on each other, which is exactly what broke
// on the Gospels-era default view (a dense scene landing in the MID/NEAR
// tier, where "every lit place's label" is allowed at once): Bethsaida,
// Chorazin, Capernaum, Cana, Nazareth, Nain, Sea of Galilee (landmark),
// Galilee and Decapolis (region landmarks) all within a few km of each
// other, PLUS "Mount Hermon" rendering TWICE -- once as a place label
// (DEU.3.8 etc. light it in some windows -- see places.json's "mount-
// hermon") and once as a landmark label (landmarks.toml), both at the
// identical lat/lon. Two independent, ORTHOGONAL passes fix this, both
// re-run every time applyLabelTier runs (zoomend/setScene/setLandmarks --
// "re-bucket on zoom change" falls out of that for free, no new listener):
//
// (a) DEDUPE -- a landmark whose NAME matches a currently-LIT place's name
//     AND whose position is within LANDMARK_DEDUPE_KM of it is the exact
//     same real-world feature described twice by two independent curated
//     datasets. Rule: the PLACE wins, always, while it's lit -- it's the
//     more foreground, more authoritative representation (interactive,
//     glowing, backed by real verse events -- design-direction.md already
//     ranks landmarks "a visual step below place labels so scripture's
//     places stay the foreground"), so the landmark is suppressed outright
//     the whole time its real-world twin is lit, regardless of whether the
//     place's OWN label is tier-visible at this exact zoom (showing the
//     landmark alone the moment the place's label happens to be tier-
//     hidden would just trade one inconsistency for another, less legible
//     one -- a name that's ACTIVE right now reading in the "decorative,
//     never glows" landmark style). LANDMARK_DEDUPE_KM, not exact
//     coordinate equality: nudgeCloseLatLng (this file) can move a place's
//     rendered position up to NUDGE_STEP_DEG away from its curated
//     original when another LIT PLACE (not landmarks -- nudging never
//     looks at landmarks at all) crowds the same spot, which a same-point-
//     only check would miss.
//
// (b) COLLISION DAMPING -- even after dedupe, DIFFERENT labels with no
//     name in common at all (Bethsaida/Chorazin/Cana/Nazareth) can still
//     crowd the same few square pixels at the same tier. Every label that
//     survives its own tier+dedupe check this pass is bucketed into a
//     COLLISION_CELL_PX square grid keyed off its own on-screen anchor
//     point (map.latLngToContainerPoint, so this is naturally zoom/pan-
//     aware already) -- the same coarse-grid-over-exact-geometry
//     philosophy ArrowLayer.setArrows's own clusterKey already uses for
//     crowded arrow bundles, not a real text-bbox intersection test
//     (cheap, approximate, good enough at this label density; two labels
//     can rarely still nudge up against a cell-boundary neighbor -- same
//     known trade-off clusterKey already accepts, see its own comment).
//     Only the HIGHEST-PRIORITY label in a contested cell survives this
//     pass; every other claimant is hidden (never removed -- spread back
//     out by a later zoom/pan, it reclaims its cell on the next call).
//     Priority, highest first: any PLACE label (brighter beats dimmer, via
//     PLACE_PRIORITY_BASE + brightness, so even the dimmest visible place
//     still outranks every landmark) > landmark labels ranked by kind
//     (water > mountain > region -- water names are the coarsest, most
//     orienting labels on the plate, the same reason water is the only
//     landmark kind that survives the FAR tier at all). Polity labels
//     (BorderLayer, its own pane/layer) are deliberately OUTSIDE this pass
//     -- they already have an independent declutter mechanism (size-tier +
//     on-screen-fraction, see BorderLayer._positionLabel) and are
//     inherently sparse (one per historical border FEATURE, not per city);
//     no concrete polity/place or polity/landmark collision was ever
//     observed in review, so this stays scoped to the two label kinds the
//     actual reported clutter (Galilee cluster, Mount Hermon dup) came
//     from rather than a speculative cross-layer rewrite.
const LANDMARK_DEDUPE_KM = 5; // == CLOSE_THRESHOLD_KM's own "same real-world spot" radius (see that constant's own comment) -- a coincidence of purpose, not a shared constant, so the two stay independently tunable
const COLLISION_CELL_PX = 72; // picked by screenshot review (see batch report) against real rendered label widths at .atlas-label's .78rem / .landmark-label's .68rem (app.css)
const PLACE_PRIORITY_BASE = 1000; // + brightness (1-5): unconditionally beats every landmark's own 0-2 priority below
const LANDMARK_KIND_PRIORITY = { water: 2, mountain: 1, region: 0 };
// Batch E2 (the ever-present graph): quiet-dot labels compete at the LOWEST
// priority of any label kind -- strictly below every landmark's own 0-2 (a
// quiet place is furniture, never contests a place or a curated landmark for
// a crowded cell; it only ever wins a cell nothing else claimed). Originally
// ALSO gated to the single highest zoom tier (batch-e2-brief.md's own
// Requirement 2 wording); that tier gate was removed by this batch's own
// mid-flight scope amendment alongside the lit-place one above (see the
// "Zoom-tiered label DENSITY" comment) -- a quiet label now competes for a
// cell at EVERY zoom, same as a lit place's, with only its (lowest, so it
// loses first) priority setting it apart.
const QUIET_LABEL_PRIORITY = -1;

function labelTier(zoom) {
    if (zoom >= ZOOM_TIER_NEAR) {
        return 'near';
    }
    return zoom >= ZOOM_TIER_MID ? 'mid' : 'far';
}

// Every currently-lit place's name/position, keyed by slugify(name) -- the
// same normalization the landmark-{slug}/polity-label-{slug} testids
// already use, not a fresh string-compare, so name pairs differing only in
// case/punctuation still dedupe correctly (case-insensitivity in
// particular falls out of slugify's own lowercasing for free). "Lit" means
// present in `markers` at all (see setScene's own call site), independent
// of whether a place's OWN label is itself currently tier/collision-
// visible. Shared by TWO independent consumers, one source of truth
// instead of two copies: applyLabelTier's own landmark dedupe below, and
// BorderLayer's polity dedupe (fix round 1 addendum -- see
// BorderLayer.setLitPlaces/_isDedupedByLitPlace), fed by setScene's own
// call site every time `markers` changes.
function buildLitByName(markers) {
    const litByName = new Map();
    for (const entry of markers.values()) {
        const slug = slugify(entry.name);
        const list = litByName.get(slug);
        if (list) {
            list.push(entry);
        } else {
            litByName.set(slug, [entry]);
        }
    }
    return litByName;
}

// Applied fresh (not diffed) on every zoomend and every setScene/
// setLandmarks call -- see setScene's own call site for why a fresh pass
// every time is necessary rather than a one-time decision. No-ops entirely
// on a mini instance (mini has no landmarkMarkers/zoomend listener to begin
// with, but this guard makes the function safe to call unconditionally
// regardless).
//
// Fix round 1 (M1): now THREE passes, not one -- see the DEDUPE/COLLISION
// DAMPING comment above this file's LANDMARK_DEDUPE_KM etc. constants for
// the full rule. (1) decide each label's own tier+dedupe visibility (Batch
// E2 scope amendment: PLACE labels -- lit and quiet both -- no longer have a
// tier check here at all, only landmarks still do; see (2) below and the
// "Zoom-tiered label DENSITY" comment above this file's ZOOM_TIER_MID/NEAR
// constants), hiding it immediately (and finally, for this pass) the moment
// it fails whatever check still applies to its kind; (2) collect every
// SURVIVOR -- places, landmarks, and quiet dots together, collision has to
// see all three to damp one kind against another, not just within each kind
// separately -- into one shared `candidates` list; (3) grid-bucket
// `candidates` and hide every cell's non-winners. Markers (the lamp/quiet
// dots themselves) are never touched by any of the three passes -- only
// ever a place's `.atlas-label` child, a quiet dot's `.quiet-label` child,
// or a landmark's own element (which IS just its label; landmarks have no
// separate dot).
//
// Batch E2 (the ever-present graph): a THIRD label source, quiet dots'
// `.quiet-label` children, feeds the same shared `candidates`/collision
// pass at the lowest priority of any kind (QUIET_LABEL_PRIORITY) -- see
// that constant's own comment. Like the other two kinds, only the LABEL is
// ever touched here; a quiet dot's own `.quiet-marker` core is NEVER
// zoom-tiered (unlike a landmark's element, which IS its whole label) --
// screenshot-judged deliberately, not an oversight: the brief's own named
// stress case (primeval era, 2 lit places + 204 quiet dots, this file's own
// worst-case density) was rendered and reviewed at the default whole-world
// fit, and the small muted dots read as atlas-plate texture, not clutter --
// the embers stayed the unmistakable foreground with zero dot-tiering
// needed (see the batch report's own screenshots, both before AND after the
// scope amendment below made every dot's LABEL always-contesting rather
// than tier-gated -- collision damping alone was enough to keep that same
// stress case legible with labels always in play too).
function applyLabelTier(inst) {
    if (!inst || inst.mini) {
        return;
    }

    const tier = labelTier(inst.map.getZoom());

    // inst.litByName (built by buildLitByName, kept fresh by setScene every
    // time inst.markers changes -- see that function's own comment) is the
    // one shared source of truth for "which places are lit, by name" that
    // BOTH this landmark dedupe check AND BorderLayer's own polity dedupe
    // (fix round 1 addendum, BorderLayer.setLitPlaces/_isDedupedByLitPlace)
    // read from -- independent of whether ITS OWN label happens to be
    // tier-visible right now (the place wins outright whenever lit, not
    // only while its own label is currently showing; see the dedupe
    // rule's own comment above the constants). Falls back to an empty Map
    // (never undefined -- initialized alongside every other inst field in
    // init()) if applyLabelTier is ever reached before the first setScene.
    const litByName = inst.litByName;

    // Every label that cleared its own tier+dedupe check (place labels, per
    // the Batch E2 scope amendment above, have no tier check left at all --
    // they always enter this list), still competing for a grid cell:
    // { el, x, y, priority }.
    const candidates = [];

    // Batch E2 scope amendment: place labels compete for a grid cell at
    // EVERY zoom now, never hidden purely for being at a "wrong" tier --
    // "have all biblically relevant names of places showing on the map at
    // all times... zooming in reveals what collision dropped" (user
    // direction, this batch). `entry.brightness` still sets a lit place's
    // own PRIORITY (brighter wins a contested cell over dimmer, unchanged),
    // it just no longer gates whether the label is EVEN OFFERED a chance to
    // win one.
    for (const entry of inst.markers.values()) {
        const el = entry.marker.getElement();
        const label = el && el.querySelector('.atlas-label');
        if (!label) {
            continue;
        }
        const pt = inst.map.latLngToContainerPoint([entry.lat, entry.lon]);
        candidates.push({ el: label, x: pt.x, y: pt.y, priority: PLACE_PRIORITY_BASE + entry.brightness });
    }

    for (const lm of inst.landmarkMarkers) {
        const el = lm.marker.getElement();
        if (!el) {
            continue;
        }
        // Batch C2 (design-direction.md's Atlas plate detail SECOND
        // ADDENDUM): a landmark carrying an explicit `size` hint is a
        // deliberately-curated far-field context label -- "tier-0
        // (visible zoomed out)" -- regardless of what its `kind` alone
        // would otherwise allow at this tier. `size != null` (not
        // `size === 'lg'` specifically) is the gate: EVERY Requirement-2
        // addition needs to clear this bar, not just the empire-scale
        // ones, so tier-0-ness is decoupled from which exact size a label
        // renders at. A landmark with no size hint at all (every entry
        // curated before this batch) falls through to the exact
        // pre-Batch-C2 kind-based rule, unchanged.
        const tierShow = tier === 'near' || lm.kind === 'water' || (tier === 'mid' && lm.kind === 'mountain') || lm.size != null;
        if (!tierShow || isDedupedByLitPlace(lm, litByName)) {
            el.style.display = 'none';
            continue;
        }
        const pt = inst.map.latLngToContainerPoint([lm.lat, lm.lon]);
        candidates.push({ el, x: pt.x, y: pt.y, priority: LANDMARK_KIND_PRIORITY[lm.kind] ?? 0 });
    }

    // Batch E2: quiet-dot labels -- like lit place labels above (scope
    // amendment), NO tier check at all now, always offered a chance to win
    // a cell. No dedupe check either (unlike places/landmarks above): a
    // quiet place sharing a name with a landmark or a lit place is already
    // excluded from ever being quiet in the first place, since `places` and
    // `quiet_places` are disjoint by construction -- see wire.rs's own
    // QUIET-1 comment. Participates in the SAME collision grid as every
    // other label kind, at QUIET_LABEL_PRIORITY (below every landmark's own
    // 0-2) so a quiet label never wins a contested cell against anything
    // else, only an empty one -- in a dense scene (the primeval era stress
    // case: 204 quiet dots) collision damping alone does real, visible work
    // here, exactly the "the plate must still breathe through pruning, not
    // through tiers" the scope amendment asks for.
    for (const entry of inst.quietMarkers.values()) {
        const el = entry.marker.getElement();
        const label = el && el.querySelector('.quiet-label');
        if (!label) {
            continue;
        }
        const pt = inst.map.latLngToContainerPoint([entry.lat, entry.lon]);
        candidates.push({ el: label, x: pt.x, y: pt.y, priority: QUIET_LABEL_PRIORITY });
    }

    // Highest priority first (stable sort -- Array.prototype.sort has been
    // spec-guaranteed stable since ES2019, every engine this app targets --
    // so same-priority candidates keep their original map-iteration order,
    // places before landmarks, rather than reshuffling nondeterministically
    // redraw to redraw); the first claimant of a cell wins it, every later
    // claimant of the SAME cell is hidden for this pass.
    candidates.sort((a, b) => b.priority - a.priority);
    const claimedCells = new Set();
    for (const c of candidates) {
        const cellKey = `${Math.round(c.x / COLLISION_CELL_PX)},${Math.round(c.y / COLLISION_CELL_PX)}`;
        if (claimedCells.has(cellKey)) {
            c.el.style.display = 'none';
        } else {
            claimedCells.add(cellKey);
            c.el.style.display = '';
        }
    }
}

// See applyLabelTier's own DEDUPE comment (above this file's
// LANDMARK_DEDUPE_KM constant) for the full rule. `litByName` is keyed by
// slugify(name) -- the same normalization the landmark-{slug}/polity-
// label-{slug} testids already use, not a fresh string-compare, so name
// pairs differing only in case/punctuation still dedupe correctly.
function isDedupedByLitPlace(landmark, litByName) {
    const list = litByName.get(slugify(landmark.name));
    if (!list) {
        return false;
    }
    return list.some(p => approxKm(landmark.lat, landmark.lon, p.lat, p.lon) <= LANDMARK_DEDUPE_KM);
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
// Batch C2: `data-size` (sm/md/lg) is only written when `l.size` is
// actually set -- every pre-Batch-C2 landmark has none, and omitting the
// attribute entirely (rather than writing a literal "undefined"/"null")
// keeps `.landmark-label:not([data-size])` a meaningful, honest selector
// for "no size hint" if app.css ever wants one.
function makeLandmarkIcon(l) {
    const dir = labelDirection(l.lat, l.lon);
    const sizeAttr = l.size ? ` data-size="${esc(l.size)}"` : '';
    const html = `<span class="landmark-label" data-kind="${esc(l.kind)}" data-dir="${dir}"${sizeAttr} data-testid="landmark-${esc(slugify(l.name))}">${esc(l.name)}</span>`;
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

// Replaces the polity-border layer's data wholesale (Batch B2, "borders v2,
// the cartographer's edition": per-polity timerange borders swapped on
// every window change) and makes sure it's visible -- a fresh window's
// polities are always a reason to show the layer, even if scripture mode
// had it hidden most recently (World.razor's own mode bookkeeping is what
// actually prevents this from being CALLED during scripture mode in the
// first place; this function doesn't need to know about modes at all).
// `politiesJson` is the flat `[{ id, name, from, to, rings, color_key },
// ...]` array `GET /api/polities` returns (already deterministically
// ordered by id then from -- see BorderLayer.setPolities's own comment for
// why that order matters).
export function setPolities(id, politiesJson) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    const list = typeof politiesJson === 'string' ? JSON.parse(politiesJson) : (politiesJson || []);
    inst.polities.setPolities(list);
}

// Shows/hides the polity-border layer without touching its data (Batch B2:
// scripture mode has no time window, so no polities are shown there --
// same "restore on return to time mode" pattern the retired border-tag's
// own visibility toggle used).
export function setPolitiesVisible(id, visible) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    inst.polities.setVisible(!!visible);
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
    for (const { marker } of inst.landmarkMarkers) {
        inst.map.removeLayer(marker);
    }
    // `kind`/`name`/`lat`/`lon` travel alongside the marker (not re-read
    // from the DOM) so applyLabelTier's zoom-tier decision (water/mountain
    // shown from MID, region only from NEAR -- see its own comment) and its
    // dedupe + collision-damping passes (Fix round 1 / M1 -- see the
    // LANDMARK_DEDUPE_KM/COLLISION_CELL_PX comment) don't need a round-trip
    // through the rendered icon's own data-kind attribute or a second
    // Leaflet getLatLng() call every redraw. `size` (Batch C2) travels the
    // same way, for the same reason -- applyLabelTier's tier-0 check reads
    // `lm.size` directly rather than the DOM's data-size attribute.
    inst.landmarkMarkers = landmarks.map(l => {
        const marker = L.marker([l.lat, l.lon], { icon: makeLandmarkIcon(l), interactive: false, pane: 'landmarksPane' });
        marker.addTo(inst.map);
        return { marker, kind: l.kind, name: l.name, lat: l.lat, lon: l.lon, size: l.size ?? null };
    });
    applyLabelTier(inst);
}

export function destroy(id) {
    const inst = instances.get(id);
    if (!inst) {
        return;
    }

    inst.map.remove();
    instances.delete(id);
}

// --- Zoom-animation sync (Batch B2, user: "the narrative arrow graph
// things scale at a different rate than the map does when zooming in/out.
// fix that.") -----------------------------------------------------------
//
// ArrowLayer and BorderLayer each manage their own plain <svg> element,
// positioned/sized via map.latLngToLayerPoint() at the CURRENT zoom every
// time _redraw() runs (on 'zoomend'/'moveend'). That makes them track the
// map correctly once a zoom finishes, but during Leaflet's own ANIMATED
// zoom transition (the ~250ms smooth scale you get from a scroll-wheel
// zoom or a double-click) neither layer repaints a single frame -- Leaflet
// itself only fires 'zoomend' at the very end, and 'zoomstart'/'zoomend'
// bracket the animation, never a per-frame progress event a naive listener
// could redraw against. Leaflet's OWN built-in overlays (L.SVG/L.Canvas,
// which is what draws every ordinary vector layer -- polygons, polylines,
// GeoJSON) don't have this problem because L.Renderer (their common base
// class) listens for the map's own 'zoomanim' event and, for exactly one
// animation frame, applies a CSS transform (translate + scale) to its
// WHOLE container that mirrors the animation in progress -- cheap (one
// transform, no path recomputation) and exactly in sync with the tiles,
// which get the identical transform applied to their own pane by Leaflet's
// core. ArrowLayer/BorderLayer never went through L.Renderer (they own a
// bare <svg> appended straight into overlayPane), so they never got that
// treatment "for free" -- this is the missing piece, added once here and
// shared by both layers rather than duplicated.
//
// attachZoomAnim mirrors Leaflet's own L.Renderer.prototype._onAnimZoom /
// _updateTransform (see leaflet's src/layer/vector/Renderer.js) -- the
// well-established, documented recipe every hand-rolled Leaflet SVG/D3
// overlay plugin uses for this exact problem, not invented projection math:
// on 'zoomanim', compute the scale between the CURRENT zoom and the
// in-progress animated target zoom (map.getZoomScale), find where the
// point that currently sits at the SVG's own origin (the current
// viewport's own north-west corner -- svgEl is always position:absolute;
// top:0;left:0, covering the whole map container, for both layers) will
// land under the animated target zoom/center (map._latLngToNewLayerPoint,
// Leaflet's own semi-public "reproject this latlng at a hypothetical
// zoom/center" helper), and apply that as a single CSS transform via
// L.DomUtil.setTransform -- the same low-level helper Leaflet's own panes
// use, so this produces byte-identical CSS to what a real L.Renderer would.
// Returns a cleanup function that detaches the listener (called from each
// layer's own onRemove).
function attachZoomAnim(map, svgEl) {
    function onAnimZoom(e) {
        const scale = map.getZoomScale(e.zoom, map.getZoom());
        const offset = map._latLngToNewLayerPoint(map.getBounds().getNorthWest(), e.zoom, e.center);
        L.DomUtil.setTransform(svgEl, offset, scale);
    }
    map.on('zoomanim', onAnimZoom);
    return () => map.off('zoomanim', onAnimZoom);
}

// Clears whatever transform the last zoomanim frame left on svgEl. Called
// at the top of both layers' own _redraw() (which recomputes every path's
// real `d` at the NOW-current zoom) so the temporary animation transform
// never compounds with the freshly-recomputed, already-correct geometry --
// _redraw always runs at 'zoomend', i.e. exactly when the animation (and
// therefore the transform it was driving) has just finished.
function resetZoomAnimTransform(svgEl) {
    L.DomUtil.setTransform(svgEl, L.point(0, 0), 1);
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
        this._offZoomAnim = attachZoomAnim(map, this._svg);
        return this;
    },

    onRemove() {
        if (this._offZoomAnim) {
            this._offZoomAnim();
            this._offZoomAnim = null;
        }
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
            entry.casing.setAttribute('data-faded', 'false');
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
                entry = { ...this._createPath(a), arrow: a, parallelIndex: 0 };
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
                entry.casing.remove();
                this._paths.delete(key);
            }
        }

        this._redraw();
    },

    // WORLD-4: null clears every arrow back to "false"; any other value
    // fades every arrow whose narrative doesn't match it. Mirrored onto the
    // casing too (Batch C) purely for visual coherence -- CONTRACT only
    // ever queries the colored path's own data-faded, but an isolated-out
    // arrow whose dark casing stayed at full opacity would leave a
    // conspicuous ghost line where the "faded to .12" colored stroke is
    // meant to all but disappear.
    setIsolate(narrativeId) {
        for (const entry of this._paths.values()) {
            const faded = narrativeId != null && entry.arrow.narrative !== narrativeId;
            entry.path.setAttribute('data-faded', faded ? 'true' : 'false');
            entry.casing.setAttribute('data-faded', faded ? 'true' : 'false');
        }
    },

    // Batch C / design-direction.md World THIRD REVISION: "Colored
    // narrative threads get a dark casing/halo under the colored stroke...
    // so they read on light terrain" -- the light basemap this batch
    // introduces (see the TILE_* comment above) no longer gives a bright
    // narrative color automatic contrast the way the old dusk terrain did.
    // Two stacked <path> elements sharing the exact same `d` (kept in sync
    // every redraw, see _position): a plain dark casing painted FIRST (so
    // it sits visually BEHIND, same DOM-order-is-paint-order rule
    // BorderLayer's own header comment already relies on), then the
    // colored path on top, unchanged in every other way (testid, hover/
    // click wiring, marker-end arrowhead -- "arrowheads keep narrative
    // color" per the brief, which is automatically true since the casing
    // gets no marker-end of its own at all). The casing carries no
    // data-testid -- CONTRACT's arrow-{narrativeId}-{order} testid and the
    // WORLD-3 spec's `path[data-testid^="arrow-"]` count both stay pinned
    // to exactly one element per arrow, unchanged.
    _createPath(a) {
        const casing = svgEl('path', {
            class: 'atlas-arrow-casing',
            fill: 'none',
            'data-faded': 'false',
        });
        this._svg.appendChild(casing);

        const path = svgEl('path', {
            class: 'atlas-arrow',
            'data-testid': `arrow-${a.narrative}-${a.order}`,
            fill: 'none',
            'stroke-width': '3',
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
        return { path, casing };
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
    // Batch B2: clears any transform a mid-flight zoomanim frame left on
    // the SVG before recomputing every path's real `d` at the now-current
    // zoom -- see resetZoomAnimTransform's own comment for why this has to
    // happen here, every time.
    _redraw() {
        if (!this._map) {
            return;
        }
        resetZoomAnimTransform(this._svg);
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

        const d = `M${f.x},${f.y} Q${cx},${cy} ${t.x},${t.y}`;
        entry.path.setAttribute('d', d);
        entry.casing.setAttribute('d', d); // identical curve, painted behind -- see _createPath

        // Keeps the one-shot draw-in animation (app.css: atlas-arrow-in,
        // stroke-dasharray/--arrow-length sized to the path's own length)
        // gap-free after a redraw changes the path's actual pixel length --
        // this only ever mutates a style property on an EXISTING element
        // (never re-creates/re-inserts it), so it can never retrigger the
        // animation, keeping zoom/pan recompute instant per design-direction.
        // Both elements share `d`, so their total lengths are identical --
        // one measurement (off the colored path) is reused for the casing
        // rather than calling getTotalLength() twice per arrow per redraw.
        const len = entry.path.getTotalLength();
        entry.path.style.strokeDasharray = String(len);
        entry.path.style.setProperty('--arrow-length', String(len));
        entry.casing.style.strokeDasharray = String(len);
        entry.casing.style.setProperty('--arrow-length', String(len));
    },
});

// --- BorderLayer: hand-authored per-polity timerange borders -------------
// (Batch B2, "borders v2, the cartographer's edition" -- ground-up redo per
// direct user feedback 2026-08-19: "whatever you did for bordering looks
// absolutely horrendous... if you're overlaying it over the map, don't do
// that... color it in how a cartographer would color a map"). Supersedes
// this layer's ENTIRE Batch B/C/C2 rendering approach (translucent
// snapshot-year GeoJSON polygons) while keeping its structural role
// unchanged: a custom L.Layer managing exactly one <svg class=
// "atlas-borders"> in the map's overlayPane, added to the map BEFORE
// ArrowLayer in init() so DOM paint order puts it below the narrative
// threads (see init()'s own comment), non-interactive (no event wiring;
// app.css's .atlas-border* rules set pointer-events:none), no diffing (a
// polity swap is a rare, 150ms-debounced window-change event, not a hot
// per-frame path -- replacing the layer's content wholesale on every
// setPolities call is simple and cheap enough).
//
// "Printed, not overlaid": every ring gets THREE stacked <path> elements,
// not one --
//   .atlas-border-wash  the fill, mix-blend-mode:multiply (app.css) so the
//                       tint DARKENS the terrain like ink soaking into
//                       paper rather than a flat shape floating on top --
//                       terrain relief shows through at every opacity this
//                       batch uses, by construction of multiply blending.
//   .atlas-border-band  a WIDE, low-opacity, no-fill stroke along the same
//                       ring -- "a soft inner tint band along the border".
//                       Deliberately the SIMPLEST version of this effect
//                       the batch brief sanctions ("a wider, low-opacity
//                       stroke clipped/behind" is the brief's own first
//                       suggestion): SVG strokes straddle their path (half
//                       in, half out) by default, so a wide soft stroke
//                       reads as a band hugging the border without any
//                       inset-polygon math (explicitly out of scope --
//                       "do not try to get too fancy with mathematics").
//   .atlas-border-line  the fine, crisp border LINE itself, in the
//                       polity's own hue DARKENED (POLITY_TINTS_DARK) --
//                       solid for the newest/only visible era of a polity,
//                       dashed or dotted for an older one still in view
//                       (see the "age" tiering below).
// All three share one ring's own projected `d` (recomputed every redraw,
// same as before); they are appended to the SVG in THREE PASSES across
// EVERY ring (all washes, then all bands, then all lines) rather than
// per-ring, so that even where one polity's later, larger ring visually
// covers an earlier, smaller one's fill, every ring's own LINE still
// paints on top of every fill/band and stays crisply visible -- the
// mechanism "the older dotted ring visible around/inside the newer solid
// one" actually depends on.
//
// Multi-era windows (a window spanning a border change -- "the one-up on
// timemap.org"): setPolities groups the incoming flat list by polity id
// and tags each entry with an "age" (oldest/middle/newest) among that
// polity's OWN currently-visible eras -- a single visible era is always
// "newest" (solid line, full wash, no year tag: "single-era window: solid
// ring + wash"); the OLDEST of >=2 visible eras is dotted and lightest;
// any era strictly between the oldest and newest (rare at this app's own
// scale) is dashed at an intermediate opacity. Entries are appended to the
// SVG (and therefore painted) in the SAME oldest-to-newest order the
// server's own deterministic "by id then from" ordering already gives --
// see handlers::polities's own doc comment -- so a GROWING polity's later,
// larger ring paints last/on top (its fuller wash reads over the earlier
// ring's own footprint) while a SHRINKING polity's later, SMALLER ring
// still paints last, sitting visibly inside the earlier, larger, fainter
// one: either way, growth (or its mirror, contraction) is legible without
// this layer ever having to compare two rings' own geometry to decide who
// paints on top -- paint order is just "oldest first", always, and the
// data (drawn to be geographically honest) does the rest. When a polity
// has more than one currently-visible era, each of ITS OWN RINGS gets a
// small mono "c. {year}" year tag (design-direction.md's instrument-face
// convention for years) -- ringed, not per-era, since a single era can
// itself carry more than one disjoint ring (e.g. Ptolemaic Egypt's Nile
// valley + Cyrenaica) and each needs its own tag to stay legible.
//
// Batch C / design-direction.md World THIRD REVISION's own polity NAME
// labels survive unchanged in spirit ("place at the polygon's visual
// center... skip a label when its polygon is mostly outside the viewport
// or too small at current zoom", one plain <span class="polity-label">
// per label positioned into polityLabelsPane) but now key off an ERA
// (id + era name), not a whole snapshot-year feature -- "the polity name
// label renders once per era-name (names may differ across eras -- that
// is the point)": when two eras of the SAME polity are both visible with
// DIFFERENT names (the ordinary case -- a name change usually accompanies
// or follows a border change), BOTH get their own label, each centered on
// its OWN era's own rings.
const POLITY_TINTS = [
    '#C98A8A', // 0 rose
    '#C9B37E', // 1 buff
    '#93A98B', // 2 sage
    '#7E99B5', // 3 pale lapis
    '#A18CB0', // 4 lilac
    '#B59B7E', // 5 tan
    '#8FA07A', // 6 moss
    '#C08E7A', // 7 coral
];

// The SAME 8 hues as POLITY_TINTS, each channel scaled by a flat 0.62 --
// "darkened" per the batch brief's own wording for the border LINE
// ("a fine solid stroke in the polity hue (darkened)"), computed once by
// hand rather than at runtime (no color-space math, "no fancy
// mathematics" per the brief) so .atlas-border-line reads as a real ink
// line against the same-hued, much lighter wash beneath it, at every
// tint. Indices line up 1:1 with POLITY_TINTS -- always indexed by the
// SAME color_key.
const POLITY_TINTS_DARK = [
    '#7D5656', // 0 rose, darkened
    '#7D6F4E', // 1 buff, darkened
    '#5B6956', // 2 sage, darkened
    '#4E5F70', // 3 pale lapis, darkened
    '#64576D', // 4 lilac, darkened
    '#70604E', // 5 tan, darkened
    '#59634C', // 6 moss, darkened
    '#77584C', // 7 coral, darkened
];

// "c. 1500 BC" -- the year tag's own mono-face text (design-direction.md:
// years "set them like instrument readouts"). Mirrors CONTRACT.md's plain
// year format ("1447 BC" / "AD 30") with the cartographer's own "c."
// (circa) prefix the batch brief's example uses verbatim, since a hand-
// drawn era boundary is a curatorial choice, never a precise attested
// date.
function formatYearTag(year) {
    const label = year < 0 ? `${-year} BC` : `AD ${year}`;
    return `c. ${label}`;
}

const BorderLayer = L.Layer.extend({
    initialize() {
        this._entries = []; // [{ id, name, from, to, rings, color_key, age, tierCount }]
        this._ringGroups = []; // [{ wash, band, line, ring, entry, ringIndex }]
        this._labels = []; // [{ el, entry, cLat, cLon, bbox }] -- one per unique (id, era name)
        this._yearTags = []; // [{ el, ringGroup, cLat, cLon }]
        // Fix round 1 addendum (Batch C2): slugify(name) -> [{lat, lon}, ...]
        // of every currently-lit place, fed in by setScene via
        // setLitPlaces below -- defaulted to empty here (not left
        // undefined) so a redraw that happens to run before the first
        // setLitPlaces call (polities can load before the first scene
        // fetch resolves) degrades to "nothing to dedupe against" rather
        // than a null-reference.
        this._litByName = new Map();
    },

    onAdd(map) {
        this._map = map;
        this._svg = svgEl('svg', { class: 'atlas-borders' });
        map.getPane('overlayPane').appendChild(this._svg);
        this._labelPane = map.getPane('polityLabelsPane'); // created in init(), before this layer's own addTo(map)
        this._offZoomAnim = attachZoomAnim(map, this._svg);
        return this;
    },

    onRemove() {
        if (this._offZoomAnim) {
            this._offZoomAnim();
            this._offZoomAnim = null;
        }
        this._svg.remove();
        this._ringGroups = [];
        for (const l of this._labels) {
            l.el.remove();
        }
        this._labels = [];
        for (const t of this._yearTags) {
            t.el.remove();
        }
        this._yearTags = [];
    },

    getEvents() {
        return { zoomend: this._redraw, moveend: this._redraw };
    },

    // `list` is the flat `[{ id, name, from, to, rings, color_key }, ...]`
    // array `GET /api/polities` returns, already ordered by id then from.
    // Groups by id (re-sorting by `from` defensively, even though the
    // server already guarantees that order -- this layer's own painting
    // logic depends on it, so it shouldn't silently trust an upstream
    // invariant it can cheaply re-assert) to tag each entry with its own
    // "age" among its polity's currently-visible eras.
    setPolities(list) {
        resetZoomAnimTransform(this._svg);
        this._svg.replaceChildren();
        for (const l of this._labels) {
            l.el.remove();
        }
        for (const t of this._yearTags) {
            t.el.remove();
        }

        const byId = new Map();
        for (const e of list || []) {
            if (!byId.has(e.id)) {
                byId.set(e.id, []);
            }
            byId.get(e.id).push(e);
        }

        this._entries = [];
        for (const group of byId.values()) {
            group.sort((a, b) => a.from - b.from);
            const tierCount = group.length;
            group.forEach((e, idx) => {
                const age = tierCount === 1 ? 'newest' : idx === 0 ? 'oldest' : idx === tierCount - 1 ? 'newest' : 'middle';
                this._entries.push({ ...e, age, tierCount });
            });
        }

        // Three passes -- ALL washes, then ALL bands, then ALL lines --
        // across every ring of every entry (in the entries' own
        // oldest-to-newest-per-polity order): see this layer's own header
        // comment for why paint order alone, with no geometry comparison,
        // is what makes growth/contraction and the "older ring still
        // visible under/around the newer one" effect both work.
        this._ringGroups = [];
        const washEls = [], bandEls = [], lineEls = [];
        for (const entry of this._entries) {
            const fillColor = POLITY_TINTS[((entry.color_key % POLITY_TINTS.length) + POLITY_TINTS.length) % POLITY_TINTS.length];
            const lineColor = POLITY_TINTS_DARK[((entry.color_key % POLITY_TINTS_DARK.length) + POLITY_TINTS_DARK.length) % POLITY_TINTS_DARK.length];
            (entry.rings || []).forEach((ring, ringIndex) => {
                const wash = svgEl('path', { class: 'atlas-border-wash', 'data-age': entry.age });
                wash.style.fill = fillColor;
                const band = svgEl('path', { class: 'atlas-border-band', fill: 'none', 'data-age': entry.age });
                band.style.stroke = fillColor;
                const line = svgEl('path', {
                    class: 'atlas-border-line',
                    fill: 'none',
                    'data-age': entry.age,
                    'data-testid': `polity-ring-${esc(entry.id)}-${entry.from}-${ringIndex}`,
                });
                line.style.stroke = lineColor;
                washEls.push(wash);
                bandEls.push(band);
                lineEls.push(line);
                this._ringGroups.push({ wash, band, line, ring, entry, ringIndex });
            });
        }
        for (const el of washEls) {
            this._svg.appendChild(el);
        }
        for (const el of bandEls) {
            this._svg.appendChild(el);
        }
        for (const el of lineEls) {
            this._svg.appendChild(el);
        }

        // Labels: once per unique (id, era name) among the currently
        // visible entries -- see this layer's own header comment.
        const seenNameKeys = new Set();
        this._labels = [];
        for (const entry of this._entries) {
            const key = `${entry.id} ${entry.name}`;
            if (seenNameKeys.has(key)) {
                continue;
            }
            seenNameKeys.add(key);
            this._labels.push(this._makeLabel(entry));
        }

        // Year tags: one per RING (not per era), only for a polity with
        // more than one currently-visible era.
        this._yearTags = [];
        for (const g of this._ringGroups) {
            if (g.entry.tierCount > 1) {
                this._yearTags.push(this._makeYearTag(g));
            }
        }

        this.setVisible(true);
        this._redraw();
    },

    setVisible(visible) {
        this._svg.style.display = visible ? '' : 'none';
        for (const l of this._labels) {
            l.el.style.display = visible ? '' : 'none';
        }
        for (const t of this._yearTags) {
            t.el.style.display = visible ? '' : 'none';
        }
    },

    // Fix round 1 addendum (Batch C2): called from setScene (map.js) every
    // time the lit-place set changes, so a polity label's dedupe state
    // (see _isDedupedByLitPlace below) is never more than one scene load
    // stale. Triggers an immediate _redraw() -- unlike a mere zoom/pan, a
    // WINDOW change can flip dedupe state with no zoomend/moveend of its
    // own to ride along on (e.g. "Egypt" the place stops being lit -- the
    // polity label must reappear right away, not wait for the next pan).
    setLitPlaces(litByName) {
        this._litByName = litByName;
        this._redraw();
    },

    // Batch B2: clears any transform a mid-flight zoomanim frame left on
    // the SVG before recomputing every ring's real `d` at the now-current
    // zoom -- see resetZoomAnimTransform's own comment for why this has to
    // happen here, every time.
    _redraw() {
        if (!this._map) {
            return;
        }
        resetZoomAnimTransform(this._svg);
        for (const g of this._ringGroups) {
            const d = this._ringPathData(g.ring);
            g.wash.setAttribute('d', d);
            g.band.setAttribute('d', d);
            g.line.setAttribute('d', d);
        }
        for (const l of this._labels) {
            this._positionLabel(l);
        }
        for (const t of this._yearTags) {
            this._positionYearTag(t);
        }
    },

    // One <path> `d` string per RING: a single "M...Z" subpath, its points
    // in the ring's own curated order. `ring` is a flat `[[lat, lon], ...]`
    // array (Batch B2's own data model -- deliberately [lat, lon], NOT
    // GeoJSON's [lon, lat]; see atlas_core::data::PolityEra's own doc
    // comment) -- no destructure-and-flip needed here, unlike the retired
    // GeoJSON-based renderer this replaces. Leaflet's own default
    // smoothFactor handles path smoothing; nothing here simplifies or
    // otherwise touches the curated points themselves.
    _ringPathData(ring) {
        if (!ring || ring.length === 0) {
            return '';
        }
        const pts = ring.map(([lat, lon]) => this._map.latLngToLayerPoint([lat, lon]));
        return `M${pts.map(p => `${p.x},${p.y}`).join('L')}Z`;
    },

    // Geographic (lat/lon degree) bbox across one or more rings -- "centroid
    // is fine" per the brief, so this uses the plain bbox CENTER as a
    // label/tag's anchor point rather than a true area-weighted centroid,
    // and also drives the size tier below (a polity's on-the-ground
    // extent, not its on-screen pixel size, which changes with zoom).
    _geoBBoxOfRings(rings) {
        let minLat = Infinity, maxLat = -Infinity, minLon = Infinity, maxLon = -Infinity;
        for (const ring of rings || []) {
            for (const [lat, lon] of ring) {
                minLat = Math.min(minLat, lat);
                maxLat = Math.max(maxLat, lat);
                minLon = Math.min(minLon, lon);
                maxLon = Math.max(maxLon, lon);
            }
        }
        return { minLat, maxLat, minLon, maxLon };
    },

    // Size tiers ("sized to the polity's extent"), thresholds picked
    // against the real curated dataset (data/curated/polities/*.toml):
    // max(latSpan, lonSpan) ranges from ~1 deg (Yehud, post-exile Judah)
    // to ~50 deg (Alexander's empire, Achaemenid Persia). SM covers
    // city-state/small-kingdom-scale polities (<5 deg -- Judah, Yehud,
    // Phoenicia, Herodian/Hasmonean Judea, early Assyria); MD covers
    // regional kingdoms (5-16 deg -- Sumer, Elam, Babylonia, every Egypt
    // era, the Hittite Empire); LG covers the true empires (>=16 deg --
    // Neo-Assyria, Rome, the Seleucids/Parthia, Alexander, Persia).
    _sizeTier(bbox) {
        const span = Math.max(bbox.maxLat - bbox.minLat, bbox.maxLon - bbox.minLon);
        if (span >= 16) {
            return 'lg';
        }
        return span >= 5 ? 'md' : 'sm';
    },

    _makeLabel(entry) {
        const bbox = this._geoBBoxOfRings(entry.rings);
        const cLat = (bbox.minLat + bbox.maxLat) / 2;
        const cLon = (bbox.minLon + bbox.maxLon) / 2;
        const el = document.createElement('span');
        el.className = 'polity-label';
        el.dataset.size = this._sizeTier(bbox);
        el.setAttribute('data-testid', `polity-label-${slugify(entry.name)}`);
        el.textContent = entry.name;
        this._labelPane.appendChild(el);
        return { el, entry, cLat, cLon, bbox };
    },

    _makeYearTag(ringGroup) {
        const bbox = this._geoBBoxOfRings([ringGroup.ring]);
        const cLat = (bbox.minLat + bbox.maxLat) / 2;
        const cLon = (bbox.minLon + bbox.maxLon) / 2;
        const el = document.createElement('span');
        el.className = 'polity-year-tag';
        el.setAttribute(
            'data-testid',
            `polity-year-tag-${esc(ringGroup.entry.id)}-${ringGroup.entry.from}-${ringGroup.ringIndex}`
        );
        el.textContent = formatYearTag(ringGroup.entry.from);
        this._labelPane.appendChild(el);
        return { el, ringGroup, cLat, cLon };
    },

    // Fix round 1 addendum (Batch C2, coordinator follow-up to M1's own
    // landmark dedupe): a polity label whose NAME matches a currently-lit
    // place's name (case-insensitively -- slugify already lowercases, the
    // same normalization applyLabelTier's own landmark dedupe and the
    // landmark-{slug}/polity-label-{slug} testids all use) AND whose
    // on-screen anchor sits within COLLISION_CELL_PX of that place's own
    // on-screen position is describing the identical real-world thing
    // twice -- confirmed live in the egypt-exodus scene: the polity
    // "Egypt" and the lit place "EGYPT" label the same point. Same rule as
    // the landmark dedupe: the place wins, the polity label is suppressed
    // for as long as its same-named place is lit, and returns the moment
    // that place drops out of the scene (setLitPlaces re-triggers a
    // redraw for exactly this reason). Deliberately NARROW -- this is NOT
    // polity labels joining applyLabelTier's general collision pass: a
    // DIFFERENT-named polity sitting near a city (e.g. "Roman Empire" near
    // "Jerusalem") is legitimate period cartography, not a duplicate, and
    // must never be suppressed for proximity alone.
    _isDedupedByLitPlace(l) {
        const list = this._litByName.get(slugify(l.entry.name));
        if (!list) {
            return false;
        }
        const center = this._map.latLngToContainerPoint([l.cLat, l.cLon]);
        return list.some(p => {
            const pt = this._map.latLngToContainerPoint([p.lat, p.lon]);
            return Math.hypot(pt.x - center.x, pt.y - center.y) <= COLLISION_CELL_PX;
        });
    },

    // Positions the label at its era's own bbox centroid and decides
    // visibility for THIS redraw: "skip a label when its polygon is mostly
    // outside the viewport or too small at current zoom" (re-evaluated
    // fresh every zoomend/moveend, since both depend on the current
    // projection), PLUS the lit-place dedupe above. POLITY_LABEL_MIN_PX /
    // POLITY_LABEL_MIN_ONSCREEN_FRAC: unchanged constants/reasoning from
    // Batch C -- see below.
    _positionLabel(l) {
        const POLITY_LABEL_MIN_PX = 50;
        const POLITY_LABEL_MIN_ONSCREEN_FRAC = 0.35;

        const size = this._map.getSize();
        let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
        for (const ring of l.entry.rings || []) {
            for (const [lat, lon] of ring) {
                const p = this._map.latLngToContainerPoint([lat, lon]);
                minX = Math.min(minX, p.x);
                maxX = Math.max(maxX, p.x);
                minY = Math.min(minY, p.y);
                maxY = Math.max(maxY, p.y);
            }
        }

        const bboxW = Math.max(maxX - minX, 1);
        const bboxH = Math.max(maxY - minY, 1);
        const onscreenW = Math.max(0, Math.min(maxX, size.x) - Math.max(minX, 0));
        const onscreenH = Math.max(0, Math.min(maxY, size.y) - Math.max(minY, 0));
        const onscreenFrac = (onscreenW * onscreenH) / (bboxW * bboxH);
        const tooSmall = Math.max(bboxW, bboxH) < POLITY_LABEL_MIN_PX;
        const mostlyOffscreen = onscreenFrac < POLITY_LABEL_MIN_ONSCREEN_FRAC;
        const dedupedByLitPlace = this._isDedupedByLitPlace(l);

        l.el.style.display = (tooSmall || mostlyOffscreen || dedupedByLitPlace) ? 'none' : '';
        const center = this._map.latLngToLayerPoint([l.cLat, l.cLon]);
        l.el.style.transform = `translate(${center.x}px, ${center.y}px) translate(-50%, -50%)`;
    },

    // Year tags sit at their own RING's centroid, nudged down a fixed
    // amount so a single-ring era's tag never sits exactly on top of that
    // same era's own polity-label (both anchor near the same point in the
    // common case). No offscreen/too-small gating (unlike labels) -- a
    // year tag is small, always wanted whenever its ring is (tierCount>1),
    // and "no fancy math" favors staying visible over a second geometric
    // judgment call the brief never asked for.
    _positionYearTag(t) {
        const YEAR_TAG_OFFSET_PX = 14;
        const center = this._map.latLngToLayerPoint([t.cLat, t.cLon]);
        t.el.style.transform = `translate(${center.x}px, ${center.y + YEAR_TAG_OFFSET_PX}px) translate(-50%, -50%)`;
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
