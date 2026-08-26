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

// Batch M requirement 3 ("the level of abstraction for animating change in
// map is lookup lines 1, lookup lines 2, and animate 1->2... obviously it's
// not just two things. we can look up n lines in a timerange," user
// direction 2026-08-20, verbatim): BorderLayer below is the one DOM-
// painting consumer of this pure, framework-agnostic module -- see its own
// header comment for the full lookup/overlay/animate factoring.
import { lookup, overlay, animate } from './border-morph.js';

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

        // washPane (fix round 1, BLOCKER B1): the wash's own dedicated pane,
        // z-indexed BETWEEN tilePane (200, Leaflet's own vendored leaflet.css)
        // and overlayPane (400, where BorderLayer's band/line/every other
        // custom SVG layer lives) -- see BorderLayer's own header comment for
        // why `mix-blend-mode` has to live on THIS PANE ELEMENT (app.css's
        // `.atlas-wash-pane` rule), not on the individual wash <path>s inside
        // it, and why that's what makes the blend actually reach the tiles.
        // Created here, before `new BorderLayer()` below, same ordering
        // reason polityLabelsPane already has to exist before BorderLayer's
        // own onAdd runs (it reads both panes back via map.getPane).
        map.createPane('washPane');
        map.getPane('washPane').style.zIndex = 300;
        map.getPane('washPane').style.pointerEvents = 'none';
        map.getPane('washPane').classList.add('atlas-wash-pane');

        // Batch M requirement 4: dotnetRef threaded through so a delta ring's
        // own click can open the popover (mirrors ArrowLayer(dotnetRef) two
        // lines below -- same "this layer owns its own interactive DOM
        // elements, wires them directly" shape).
        polities = new BorderLayer(dotnetRef);
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
        window: null, // Batch H (existence gating): set by setScene from the scene's own `window` (null in scripture mode); applyLabelTier's existenceGatesLabel check reads this
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

    // Batch HOTFIX-2: re-derives every marker's own screen-pixel nudge on
    // every zoom change -- see applyMarkerNudges' own doc comment for why
    // this listener, UNLIKE applyLabelTier's just above, is NOT gated
    // `!mini` (a mini instance still needs its own one-time FitScene zoom
    // change to trigger a recompute against the REAL fit zoom, not
    // whatever stale zoom was current when setScene's own inline call ran).
    map.on('zoomend', () => applyMarkerNudges(instances.get(id)));

    // Batch G1 requirement 3 (click-to-pin/traversal): "clicking elsewhere
    // on the map" closes a pinned card, and Escape does too from anywhere
    // on the page. Both gated !mini -- pinning has no meaning for a 320x240
    // mini-map figure inside a popover (see MiniWorld.razor's own IMapEvents
    // comment). map.click fires on any genuine map-BACKGROUND click (tiles,
    // borders, empty water); a marker or arrow click never reaches it --
    // wireEvents/wireQuietEvents/ArrowLayer's own click handlers each stop
    // propagation FIRST (Leaflet markers bubble mouse events to the map by
    // default, `bubblingMouseEvents: true` -- see each handler's own
    // comment), so a click that pins/traverses never ALSO immediately
    // unpins in the same gesture.
    if (!mini) {
        map.on('click', () => dotnetRef.invokeMethodAsync('OnMapClick'));
        instances.get(id).escapeCleanup = watchEscape(dotnetRef);

        // Batch H fix round 2 (review Critical-1): continuous camera
        // tracking into ViewStateService, mirroring reader.js's own
        // watchScroll -- World.razor's DisposeAsync used to read the
        // camera via an AWAITED getCamera() call, which races a
        // concurrently-mounting new World instance on transitions that
        // dispose one page and mount another in the same navigation
        // (e.g. split-close-reader): Blazor gives no ordering guarantee
        // between an outgoing component's async DisposeAsync continuation
        // and an incoming component's own lifecycle. Firing on EVERY
        // moveend/zoomend (not just user-driven gestures) also naturally
        // covers the very first auto-fit with no separate special case --
        // Leaflet's own setView/fitBounds (used by fitScene/setCamera,
        // both already {animate:false}) trigger moveend/zoomend
        // synchronously, so an auto-fit landing right after init() is
        // captured exactly like a user drag/zoom would be.
        map.on('moveend zoomend', () => {
            const c = map.getCenter();
            dotnetRef.invokeMethodAsync('OnCameraChanged', c.lat, c.lng, map.getZoom());
        });
    }

    return id;
}

// document-level (not map-container-level) Escape listener -- mirrors
// reader.js's own watchShiftRelease: a marker click never moves DOM focus
// anywhere in particular, so a Blazor @onkeydown scoped to some element
// inside the page could easily miss it (keydown targets whatever
// document.activeElement currently is, which after a marker click is
// typically still whatever it was before -- often outside any single
// element a page-level handler could bind to). Returns its own cleanup
// function, stashed on the instance entry and invoked by destroy() below,
// same "per-instance cleanup, not a single module-wide singleton" discipline
// as every other per-map resource in this file (though in practice only one
// non-mini instance is ever alive at a time).
function watchEscape(dotnetRef) {
    const handler = e => {
        if (e.key === 'Escape') {
            dotnetRef.invokeMethodAsync('OnEscapePressed');
        }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
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
// Coordinates are run through applyMarkerNudges (Batch HOTFIX-2 -- see its
// own doc comment; called once markers are all added/updated below, before
// buildLitByName/applyLabelTier/ArrowLayer.setArrows all read positions
// back) before use, both here and, transitively, by ArrowLayer's arrow
// endpoints, which look positions up from this same `markers` map: curated
// places occasionally geocode very close together -- sometimes to the
// EXACT same lat/lon (e.g. Shittim and the "plains of Moab" camp -- both
// real, distinct places per data/curated's own disambiguation comments,
// just resolved identically by the upstream geocoder), sometimes just close
// (Gilgal and Jericho, ~1.8km apart -- accurate geography, Gilgal genuinely
// was that close to Jericho). Two places CLOSE ENOUGH to be the same real
// site instead of merely close neighbors (identical coordinates, or the
// ~100m "same tel" case) are merged server-side into one record before a
// scene is ever built (`atlas_core::merge`) -- ONLY genuinely distinct
// places ever reach this nudge at all.
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
// place that lands within NUDGE_TRIGGER_PX (screen pixels, at the CURRENT
// zoom -- Batch HOTFIX-2; see applyMarkerNudges' own doc comment) of an
// already-placed one (a real pairwise check across the scene, not just an
// exact-match shortcut) keeps each one independently hoverable without
// visibly moving away from its real-world location at any zoom this app uses.
export function setScene(id, sceneJson) {
    const inst = instances.get(id);
    if (!inst) {
        return;
    }

    const scene = typeof sceneJson === 'string' ? JSON.parse(sceneJson) : (sceneJson || {});
    const places = scene.places || [];
    // Batch H (existence gating): the window this scene was composed for --
    // present (time mode) or null (scripture mode, and the field's own
    // absence on a bare {} scene) -- stashed on the instance so
    // applyLabelTier's existenceGatesLabel check (run again on every
    // zoomend, not just here) doesn't need it re-threaded as a parameter.
    inst.window = scene.window || null;
    const seen = new Set();

    for (const p of places) {
        seen.add(p.id);
        const prior = inst.markers.get(p.id);

        if (!prior) {
            // Placed at its TRUE (server-provided, post-merge) position for
            // now -- applyMarkerNudges below (Batch HOTFIX-2) immediately
            // re-derives and, if needed, adjusts every marker's rendered
            // lat/lon in a single pass once every place this call is either
            // added or updated, so a brand new marker never paints at its
            // true position for even one frame if it needs a nudge.
            const marker = L.marker([p.lat, p.lon], { icon: makeIcon(p) });
            wireEvents(marker, inst.dotnetRef, p.id);
            marker.addTo(inst.map);
            // existenceFrom/existenceTo (Batch H): curated once per place id,
            // never re-derived on update below -- a place's own curated
            // established/destroyed bounds never change scene to scene, only
            // the WINDOW being tested against them does (see
            // existenceGatesLabel/applyLabelTier).
            inst.markers.set(p.id, {
                marker, lat: p.lat, lon: p.lon, trueLat: p.lat, trueLon: p.lon,
                brightness: p.brightness, name: p.name,
                existenceFrom: p.existence_from ?? null, existenceTo: p.existence_to ?? null,
            });
            continue;
        }

        if (prior.trueLat !== p.lat || prior.trueLon !== p.lon || prior.brightness !== p.brightness || prior.name !== p.name) {
            prior.marker.setLatLng([p.lat, p.lon]);
            prior.marker.setIcon(makeIcon(p));
            prior.lat = p.lat;
            prior.lon = p.lon;
            prior.trueLat = p.lat;
            prior.trueLon = p.lon;
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
    // removed, unchanged ids left alone. Never nudged (no applyMarkerNudges
    // call, no trueLat/trueLon bookkeeping): quiet dots are furniture, not a
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
                // existenceFrom/existenceTo (Batch H) -- same "set once at
                // creation, never re-derived" reasoning as the lit side above.
                inst.quietMarkers.set(p.id, {
                    marker, lat: p.lat, lon: p.lon, displayName: p.display_name,
                    existenceFrom: p.existence_from ?? null, existenceTo: p.existence_to ?? null,
                });
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

    // Batch HOTFIX-2: derives every marker's final on-screen position from
    // its TRUE lat/lon (just written above) plus a small SCREEN-PIXEL nudge
    // at the map's CURRENT zoom -- see applyMarkerNudges' own doc comment.
    // Before buildLitByName/applyLabelTier/setArrows below, all three of
    // which read a marker's `lat`/`lon` back (label position, arrow
    // endpoints) and must see the FINAL, already-nudged value.
    applyMarkerNudges(inst);

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

// Batch HOTFIX-2: replaces the PRE-existing `nudgeCloseLatLng`, which
// nudged a close marker by a FIXED GEOGRAPHIC delta -- `NUDGE_STEP_DEG`,
// 0.6 degrees (~63-67km depending on latitude), tuned (Batch C2) against
// the exodus scene's own far-zoomed-OUT fit view, where a scene spans
// hundreds of km and 0.6 degrees is a small, barely-noticeable shove. The
// SAME 0.6-degree shove, replayed at a much closer-zoomed scene -- a
// single chapter's own small, tight cluster of places, e.g. Judges 4's
// Hazor/Kedesh-naphtali/Zaanannim (user report 2026-08-20: all three
// rendering in the Mediterranean) -- is easily 80+ SCREEN PIXELS, more
// than enough to cross the coastline into open water. A fixed geographic
// delta simply cannot be tuned to work at every zoom a scene might fit to:
// small enough for a tight scene is invisible for a wide one; large enough
// for a wide scene is a wild, position-breaking jump for a tight one. See
// batch-hotfix2-report.md for the full root-cause chain and before/after
// measurements.
//
// The fix: nudge in SCREEN PIXELS, at the map's CURRENT zoom, recomputed
// fresh every time that zoom changes (setScene and the 'zoomend' listener
// below both call applyMarkerNudges -- the same "recompute on zoom, never
// persist a stale decision" pattern applyLabelTier already established for
// LABEL collision). A pixel amount means the on-screen displacement is the
// same small distance regardless of how close- or wide-zoomed the current
// scene happens to be -- see NUDGE_STEP_PX/NUDGE_TRIGGER_PX below for the
// exact numbers and their own reasoning.
//
// Two lessons from `nudgeCloseLatLng`'s own history survive unchanged in
// the new algorithm, because they were never about DEGREES vs PIXELS in
// the first place:
// (1) compare each candidate's TRUE (pre-nudge) position against every
//     ALREADY-PLACED marker's own TRUE position, never a final/nudged one
//     (fix round 1 / M3's own hard-won lesson: comparing against nudged
//     positions let a nudged point "escape" a cluster's own detection
//     radius, collapsing a 3rd coincident place onto the 2nd's slot instead
//     of getting its own) -- applyMarkerNudges below keeps `trueLat`/
//     `trueLon` on every marker entry specifically so this comparison is
//     always available, on every call, independent of whatever nudge (if
//     any) a PRIOR call already applied.
// (2) spread multiple ties at the SAME spot around the golden angle
//     (phyllotaxis/sunflower-seed packing) rather than a single fixed
//     direction, so a 3rd or 4th place crowding one exact point still lands
//     on its own distinct slot instead of stacking back onto an earlier
//     nudge. Batch HOTFIX-2 additionally requires this for the common
//     TWO-marker case to point AWAY FROM the actual colliding neighbor
//     (brief requirement 2: "spread apart along the axis between the two,
//     not a fixed NW shove" -- the old algorithm's own n=1 case always
//     pointed the same ~137.5 degrees, i.e. always northwest, regardless of
//     where the colliding neighbor actually was) -- the golden angle is now
//     only ever a FALLBACK, for the degenerate case where the true
//     colliding neighbors' own directions cancel out or coincide exactly
//     with the candidate itself (see the `mag > 0.01` branch below).
const GOLDEN_ANGLE_RAD = 2.399963229728653;

// NUDGE_TRIGGER_PX -- "still visually colliding": matches the ember
// marker's own 26x26px hit box (10px core + `::after` inset:-8px padding,
// Batch C2 requirement 0c -- 13px radius; app.css's own `.atlas-marker`
// comment), i.e. two markers are considered close enough to need
// separating exactly when their hit circles overlap at all.
//
// NUDGE_STEP_PX -- brief requirement 2, verbatim: "a few marker-widths max,
// ~12-18px" (the marker's own visual core is ~10px). 16px sits in the
// middle of that range, comfortably inside the brief's own separate ~20px
// sanity ceiling ("a nudge must never move a marker more than ~20px from
// its true position at any zoom") with real margin, applied as ONE fixed
// step per marker regardless of how many neighbors it collides with (never
// compounded/added per colliding neighbor -- see the loop below), so the
// ~20px bound holds unconditionally, not just in the common two-marker
// case. A step this size does not guarantee two very tightly packed
// markers end up hit-box-clear of each other (16px separation is still
// less than the 26px trigger radius) -- that is deliberate, not a gap:
// the brief's own sanity property continues, "if two distinct places are
// so close that 20px cannot separate them at the current zoom, let them
// overlap" -- full independent hoverability in a dense cluster remains
// C3 clustering's job (queued, out of scope here), not this fix's.
const NUDGE_TRIGGER_PX = 26;
const NUDGE_STEP_PX = 16;

// Re-derives every marker's rendered position from its TRUE (server-
// provided, post-merge) lat/lon plus a fresh screen-pixel nudge, at the
// map's CURRENT zoom -- called by setScene (every scene load) and by the
// 'zoomend' listener below (every zoom change, including the very first
// fitScene fit that follows every setScene call -- see setScene's own call
// site comment). Idempotent per call: NEVER reads a marker's own previous
// `lat`/`lon` as input, only `trueLat`/`trueLon`, so a nudge never
// compounds across repeated calls and a collision that resolves at a new
// zoom (zooming in always increases the true on-screen distance between
// two distinct points) cleanly snaps back to the true position with no
// leftover offset.
//
// Runs for MINI instances too (unlike applyLabelTier, which no-ops on
// mini): the pre-existing nudgeCloseLatLng ran unconditionally across both
// kinds of instance, and a mini popover figure can still show two close-
// but-distinct places needing separation. Mini has no user-driven zoom
// (init()'s own `scrollWheelZoom: !mini`), but DOES still receive a
// programmatic zoomend when its own one-time FitScene call changes the
// zoom (Leaflet fires 'zoomend' for a programmatic setView/fitBounds
// regardless of whether interactive zoom controls exist) -- so the
// 'zoomend' listener below is registered unconditionally too, not gated
// `!mini` the way applyLabelTier's own listener is, specifically so a mini
// instance's nudge is recomputed against its REAL fit zoom, not whatever
// stale zoom the map happened to be at when setScene's own inline call ran
// (always BEFORE FitScene -- see World.razor/MiniWorld.razor's own call
// order).
function applyMarkerNudges(inst) {
    if (!inst) {
        return;
    }
    const map = inst.map;

    // Server-sorted-by-id order (mirrors atlas-core::scene::lit_places' own
    // sort) -- NOT necessarily `inst.markers`' own Map-iteration order,
    // which only matches that on a fresh setScene and can drift after an
    // incremental add/remove diff leaves survivors sitting at their
    // ORIGINAL insertion position (plain JS Map semantics). Re-sorting here
    // keeps "the same place among the same neighbors always lands at the
    // same nudge slot" true even across a zoomend that fires with no
    // intervening setScene -- the same determinism nudgeCloseLatLng's own
    // history already established as load-bearing (a stable scene's
    // markers must never jitter on an unrelated refetch/zoom).
    const entries = [...inst.markers.entries()].sort((a, b) => (a[0] < b[0] ? -1 : 1));

    const placedTrue = []; // {x, y} TRUE (unnudged) container points already resolved this pass, in the sorted order above
    for (const [, entry] of entries) {
        const truePt = map.latLngToContainerPoint([entry.trueLat, entry.trueLon]);

        // Repulsion direction: the (normalized) sum of "away from each
        // colliding neighbor's own TRUE point" vectors -- for the common
        // two-marker collision this points exactly away from the one real
        // neighbor (brief requirement 2's own "axis between the two");
        // for a genuine multi-way tie the vectors partially cancel, which
        // is exactly why the golden-angle fallback below still exists.
        let dx = 0, dy = 0, n = 0;
        for (const q of placedTrue) {
            const ddx = truePt.x - q.x, ddy = truePt.y - q.y;
            const dist = Math.sqrt(ddx * ddx + ddy * ddy);
            if (dist < NUDGE_TRIGGER_PX) {
                n++;
                if (dist > 0.01) {
                    dx += ddx / dist;
                    dy += ddy / dist;
                } // else: exactly coincident with this neighbor -- contributes no direction of its own; the golden-angle fallback below covers it
            }
        }
        placedTrue.push(truePt);

        let finalPt = truePt;
        if (n > 0) {
            const mag = Math.sqrt(dx * dx + dy * dy);
            let ux, uy;
            if (mag > 0.01) {
                ux = dx / mag;
                uy = dy / mag;
            } else {
                // Degenerate case: every colliding neighbor's own direction
                // cancelled out (e.g. two colliding neighbors sit on exactly
                // opposite sides) or coincides exactly with this candidate --
                // same golden-angle spread nudgeCloseLatLng always used,
                // keyed off THIS marker's own running collision count `n` so
                // a 3rd/4th place at one exact spot still gets its own
                // distinct slot instead of stacking back onto an earlier one.
                const angle = n * GOLDEN_ANGLE_RAD;
                ux = Math.cos(angle);
                uy = Math.sin(angle);
            }
            finalPt = { x: truePt.x + ux * NUDGE_STEP_PX, y: truePt.y + uy * NUDGE_STEP_PX };
        }

        const ll = map.containerPointToLatLng(finalPt);
        entry.lat = ll.lat;
        entry.lon = ll.lng;
        entry.marker.setLatLng(ll);
    }
}

// Batch G2 decision 6 (multi-select v1, RULED per batch-r-report.md §7):
// Ctrl/Cmd-click on a marker (or its own label -- LABEL-1's own bubbling fix
// means both already reach this SAME handler) toggles tray selection
// INSTEAD of pinning -- never both for one physical click. `e.originalEvent`
// is Leaflet's own wrapped-native-event field (every Leaflet mouse event
// carries it); metaKey covers Cmd on macOS, ctrlKey covers Ctrl everywhere
// else, per the brief's own "Ctrl/Cmd-click" gesture name.
function isToggleSelectClick(e) {
    return !!(e.originalEvent && (e.originalEvent.ctrlKey || e.originalEvent.metaKey));
}

function wireEvents(marker, dotnetRef, placeId) {
    marker.on('mouseover', e => dotnetRef.invokeMethodAsync('OnPlaceHover', placeId, e.containerPoint.x, e.containerPoint.y));
    marker.on('mouseout', () => dotnetRef.invokeMethodAsync('OnPlaceLeave'));
    marker.on('click', e => {
        // Batch G1 requirement 3: a marker click must never ALSO register
        // as a map-background click (Leaflet markers bubble mouse events to
        // the map by default, `bubblingMouseEvents: true`) -- otherwise
        // pinning THIS marker would, in the very same gesture, immediately
        // fire OnMapClick and unpin it again. See init()'s own comment.
        L.DomEvent.stopPropagation(e);
        if (isToggleSelectClick(e)) {
            dotnetRef.invokeMethodAsync('OnPlaceToggleSelect', placeId);
            return;
        }
        dotnetRef.invokeMethodAsync('OnPlaceClick', placeId, e.containerPoint.x, e.containerPoint.y);
    });
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
    // the way mouseover is) -- fires immediately, same as a lit marker's
    // own, including the same Batch G1 stopPropagation (see wireEvents's
    // own comment) so pinning a quiet marker's card doesn't also unpin it
    // via a bubbled map-background click.
    marker.on('click', e => {
        L.DomEvent.stopPropagation(e);
        if (isToggleSelectClick(e)) {
            dotnetRef.invokeMethodAsync('OnPlaceToggleSelect', placeId);
            return;
        }
        dotnetRef.invokeMethodAsync('OnPlaceClick', placeId, e.containerPoint.x, e.containerPoint.y);
    });
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
//     coordinate equality: independently curated datasets (landmarks.toml
//     vs the OpenBible/Theographic-derived place records) describing the
//     same real-world feature are not guaranteed to carry byte-identical
//     lat/lon, so a same-point-only check could miss a real duplicate.
//     Compared against the place's own TRUE position (`p.trueLat`/
//     `p.trueLon`, Batch HOTFIX-2), never its rendered one -- a place's
//     anti-overlap screen-pixel nudge (applyMarkerNudges) is a cosmetic
//     rendering adjustment against OTHER LIT PLACES, not a fact about this
//     place's own real-world identity/position, so it must never leak into
//     an identity/dedupe decision like this one (a place that happens to
//     be nudged this exact zoom, for a reason having nothing to do with
//     ITS landmark twin, must not therefore stop deduping against it).
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
const LANDMARK_DEDUPE_KM = 5; // "same real-world spot" radius for two independently curated datasets describing the same feature -- see the DEDUPE comment above for why this is checked against a place's TRUE position, never its rendered/nudged one
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

// Batch H (existence gating, deferred from E2): true when `window` (`inst.window`,
// set by setScene -- null in scripture mode) falls ENTIRELY outside a
// place's own curated existence bounds -- mirrors atlas-core::history's own
// `existence_gates_label` exactly (inclusive on both ends, either bound
// absent never gates on that side, both absent never gates at all). Scripture
// mode never gates (there is no window to test outside-ness against) --
// `!window` alone covers that, independent of the two bounds. This decides
// whether a place/quiet LABEL even enters applyLabelTier's own candidates
// list at all (see its two call sites below) -- the marker/dot itself is
// never touched here, only ever the label span, per the brief's "the dot
// stays for availability."
function existenceGatesLabel(existenceFrom, existenceTo, window) {
    if (!window) {
        return false;
    }
    if (existenceFrom != null && window.to_year < existenceFrom) {
        return true;
    }
    if (existenceTo != null && window.from_year > existenceTo) {
        return true;
    }
    return false;
}

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
        // Batch H (existence gating): a label whose curated existence bounds
        // don't reach this scene's own window is hidden outright -- same
        // "hidden and skipped, never entered into candidates" treatment the
        // landmark loop below already gives its own tier/dedupe failures.
        // The marker itself (and its hit box) is completely untouched.
        if (existenceGatesLabel(entry.existenceFrom, entry.existenceTo, inst.window)) {
            label.style.display = 'none';
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
        // Batch H: same existence gate as the lit loop above -- this is
        // exactly where it matters most in practice (a quiet dot's whole
        // point is showing regardless of window, so without this its label
        // would happily caption a long-destroyed place with a name it never
        // bore at the window's own moment in time).
        if (existenceGatesLabel(entry.existenceFrom, entry.existenceTo, inst.window)) {
            label.style.display = 'none';
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

// Plain equirectangular-approximation distance in km (not geodesically
// precise) -- more than good enough at the few-km scale isDedupedByLitPlace
// below decides "same real-world feature" at, and far cheaper than a real
// haversine. Batch HOTFIX-2: this is now used ONLY for that landmark/place
// dedupe check -- marker anti-overlap nudging moved to screen-pixel space
// (applyMarkerNudges above) and no longer needs a ground-distance function
// at all.
function approxKm(lat1, lon1, lat2, lon2) {
    const dLat = (lat1 - lat2) * 111.32;
    const dLon = (lon1 - lon2) * 111.32 * Math.cos((lat1 + lat2) / 2 * Math.PI / 180);
    return Math.sqrt(dLat * dLat + dLon * dLon);
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
    // p.trueLat/p.trueLon (Batch HOTFIX-2), not p.lat/p.lon -- see the
    // DEDUPE comment above LANDMARK_DEDUPE_KM's own declaration for why.
    return list.some(p => approxKm(landmark.lat, landmark.lon, p.trueLat, p.trueLon) <= LANDMARK_DEDUPE_KM);
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

// Batch G1 requirement 3 (narrative traversal): recenters on (lat, lon) --
// the adjacent place's own marker -- at the CURRENT zoom (no zoom change,
// "pan" not "fit") and returns its new container point. `animate: false`,
// same reasoning as fitScene's own -- an animated pan updates the map's
// internal center progressively over the transition, so reading
// latLngToContainerPoint immediately afterward would risk an INTERMEDIATE,
// not final, point without a much more complex moveend-await round trip;
// instant keeps this call (and MapInterop.PanToPlace's own C# caller)
// simple and its return value always correct.
export function panToPlace(id, lat, lon) {
    const inst = instances.get(id);
    if (!inst) {
        return { x: 0, y: 0 };
    }

    inst.map.setView([lat, lon], inst.map.getZoom(), { animate: false });
    const pt = inst.map.latLngToContainerPoint([lat, lon]);
    return { x: pt.x, y: pt.y };
}

// Batch H (view-state service): reads the map's own CURRENT center/zoom --
// captured by World.razor's DisposeAsync (both the standalone page and the
// split pane share ONE saved MapViewState, per the brief) so a later
// restore (setCamera below) can put the view back exactly where it was
// left, rather than re-fitting to whatever the restored scene's own markers
// happen to bound. Returns null for an unknown id (defensive -- DisposeAsync
// only ever calls this while its own MapInterop is still live, but a
// disposed-out-from-under-it id costs nothing extra to guard here).
export function getCamera(id) {
    const inst = instances.get(id);
    if (!inst) {
        return null;
    }

    const c = inst.map.getCenter();
    return { lat: c.lat, lng: c.lng, zoom: inst.map.getZoom() };
}

// Batch H: the other half of getCamera above -- sets the map's view
// directly to a previously-captured (lat, lng, zoom), instant (not
// animated, same `{animate:false}` reasoning fitScene/panToPlace already
// use: a restore is a snapshot being put back, not a journey to animate).
export function setCamera(id, lat, lng, zoom) {
    const inst = instances.get(id);
    if (!inst) {
        return;
    }

    inst.map.setView([lat, lng], zoom, { animate: false });
}

// Batch H: test-support-only, read-only introspection -- lets a Playwright
// test (which can reach this same already-loaded module instance via a
// dynamic import, since ES modules are cached per specifier/document) ask
// "what map instance id(s) currently exist" instead of assuming a specific
// number. Needed because `nextId` keeps incrementing across an in-page
// navigation away from and back to /world within ONE test (the document,
// and therefore this module's own state, never resets the way a fresh
// page.goto would) -- exactly VIEWSTATE-1's own round-trip shape, which
// needs EXACT getCamera() values rather than a rendered pixel position
// (fragile: subject to Leaflet's own animated zoom transitions and
// per-marker glow/breathe CSS, neither relevant to what this test actually
// verifies). Never called from production Blazor code -- MapInterop.cs has
// no wrapper for it.
export function debugLiveInstanceIds() {
    return [...instances.keys()];
}

// Batch R requirement 1 ("borders become part of the plate"), test-support-
// only, same treatment as debugLiveInstanceIds above (never called from
// production Blazor code -- MapInterop.cs has no wrapper for it): is (lat,
// lon) inside the CURRENTLY-LOADED land mask? Uses the real SVG
// SVGGeometryElement.isPointInFill() API against BorderLayer's own actual
// clipPath child <path> elements (map.js's own `_landMaskPaths`) -- the
// EXACT geometry `_washGroup`'s own `clip-path: url(#atlas-land-clip)`
// clips against, in the SAME coordinate space (`map.latLngToLayerPoint`,
// the pre-zoomanim-transform space every ring `d` is drawn in -- see
// BorderLayer._redraw's own comment) -- so this is a precise, deterministic
// answer, not a pixel-color heuristic subject to anti-aliasing/compression
// noise a screenshot-based check would carry. Returns null if no land mask
// has loaded yet (so a test can tell "not ready" apart from "false").
export function debugIsPointOnLand(id, lat, lon) {
    const inst = instances.get(id);
    if (!inst || !inst.polities || !inst.polities._landMaskPaths || inst.polities._landMaskPaths.length === 0) {
        return null;
    }

    const pt = inst.map.latLngToLayerPoint([lat, lon]);
    const domPoint = new DOMPoint(pt.x, pt.y);
    return inst.polities._landMaskPaths.some(p => p.isPointInFill(domPoint));
}

// Batch HOTFIX-2, test-support-only (same treatment as debugLiveInstanceIds/
// debugIsPointOnLand above -- never called from production Blazor code,
// MapInterop.cs has no wrapper for it): where a WIRE (lat, lon) -- e.g. a
// scene place's own `lat`/`lon` straight from `GET /api/scene...`, before
// any anti-overlap nudge -- projects to on screen at the map's CURRENT
// zoom/pan, via Leaflet's own real projection (not a reimplementation a
// test would have to keep in sync by hand). Lets a UI test compare a
// marker's ACTUAL rendered position (its own bounding box, measured the
// same way every other test in this suite already does) against where its
// TRUE, un-nudged position would render, and assert the gap stays within
// the brief's own ~20px anti-overlap sanity bound -- see
// applyMarkerNudges' own NUDGE_STEP_PX comment for why that bound always
// holds. Returns null for an unknown id, same defensive shape
// debugIsPointOnLand already uses.
export function debugTrueScreenPoint(id, lat, lon) {
    const inst = instances.get(id);
    if (!inst) {
        return null;
    }
    const pt = inst.map.latLngToContainerPoint([lat, lon]);
    return { x: pt.x, y: pt.y };
}

// Batch M requirement 5, test-support-only (same treatment as the three
// debug exports above -- never called from production Blazor code,
// MapInterop.cs has no wrapper for it): the CURRENT mid-morph `d` path data
// for one polity's own morph-group lines (`[]` if that polity isn't
// currently morphing at all, `null` if no morph engine is even ready).
// Reads `_morphGroups` directly BY POLITY ID -- exact and scoped, unlike a
// bare `.atlas-border-morph-group` CSS-class locator, which sweeps every
// SIMULTANEOUSLY-morphing polity in a wide, multi-polity time window
// (several real curated windows show more than one polity mid-drag at
// once) and so cannot tell one polity's own line apart from another's.
export function debugMorphLineData(id, polityId) {
    const inst = instances.get(id);
    if (!inst || !inst.polities || !inst.polities._morphGroups) {
        return null;
    }
    const group = inst.polities._morphGroups.get(polityId);
    if (!group) {
        return [];
    }
    return group.lines.map(el => el.getAttribute('d'));
}

// Batch M requirement 5, test-support-only (same treatment above): the
// SETTLED `d` path data an era WOULD render, computed via the exact same
// `_ringPathData` projection `_syncMorphGroup`/`_paintSettled` themselves
// call -- for ANY era of a polity in the full `_roster`, whether or not
// that era's own window is the one currently applied. Lets a test capture
// a "snap candidate" shape (e.g. an era just outside the current window)
// with zero extra navigation and zero risk of hand-duplicating the
// projection math -- the SAME real code path production rendering uses,
// not a reimplementation a test would have to keep in sync by hand. `null`
// for an unknown instance/roster, `[]` for a polity/from with no matching
// era (defensive shapes matching debugIsPointOnLand/debugMorphLineData
// above).
export function debugSettledRingPathData(id, polityId, eraFrom) {
    const inst = instances.get(id);
    if (!inst || !inst.polities || !inst.polities._roster) {
        return null;
    }
    const entry = inst.polities._roster.find(e => e.id === polityId && e.from === eraFrom);
    if (!entry) {
        return [];
    }
    return entry.rings.map(ring => inst.polities._ringPathData(ring));
}

// Batch M fix round 1 (C1 regression coverage), test-support-only (same
// treatment as the debug exports above -- never called from production
// Blazor code, MapInterop.cs has no wrapper for it): the FULL set of
// polity ids currently holding a morph group -- i.e. every polity a
// mid-drag frame is actually painting right now, regardless of whether
// its own shape is animating or sitting at n=1 identity. Lets a test
// assert real polity PRESENCE mid-gesture (not just "some path is
// visible somewhere") -- exactly what C1's own regression needed: a
// polity whose own era never crosses the dragged edge must still show up
// here, because the per-frame `lookup` range is supposed to be the FULL
// live window (both handles' current values), not just the swept sliver
// between the dragged handle's pre-drag value and its live probe. `null`
// for an unknown instance.
export function debugMorphingPolityIds(id) {
    const inst = instances.get(id);
    if (!inst || !inst.polities || !inst.polities._morphGroups) {
        return null;
    }
    return [...inst.polities._morphGroups.keys()];
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
export function setPolities(id, politiesJson, from, to) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    const list = typeof politiesJson === 'string' ? JSON.parse(politiesJson) : (politiesJson || []);
    inst.polities.setPolities(list, from, to);
}

// Batch M requirement 3a: fetched ONCE (World.razor's own PushPolitiesRosterIfReady,
// mirroring PushLandMaskIfReady/PushLandmarksIfReady's "fetch once, push once" shape) via
// the SAME /api/polities endpoint, just with the full atlas span as its own window -- every
// era of every polity necessarily intersects [-4004,100], so this is the complete, unfiltered
// roster the morph engine's own `lookup` needs to answer an ARBITRARY drag-time window with
// zero network latency. Never re-fetched -- curated polity data is static for the life of a
// page load.
export function setPolitiesRoster(id, politiesJson) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    const list = typeof politiesJson === 'string' ? JSON.parse(politiesJson) : (politiesJson || []);
    inst.polities.setPolitiesRoster(list);
}

// Batch M requirement 3a: TimeSlider.razor's own OnWindowDrag fires this on every
// pointermove while a handle is down (World.razor's HandleWindowDrag) -- begins a morph
// gesture the first time it's called since the last settle (idempotent otherwise). Fix
// round 1 (C1): no `anchorYear` parameter -- morphFrame below now carries the FULL live
// window on every single call, so there's nothing left for beginMorph itself to remember.
export function beginMorph(id) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    inst.polities.beginMorph();
}

// Batch M requirement 3a: the scrub itself -- called on EVERY drag-move update (never
// itself throttled; BorderLayer.requestMorphFrame is what coalesces however many of these
// land within one animation frame into a single actual evaluate+paint -- see its own doc
// comment for the full CPU evaluator interface / GPU swap seam). A no-op, harmlessly, once
// a morph has already settled (e.g. a straggler call firing after release). Fix round 1
// (C1): `from`/`to` are the FULL, CURRENT live window (both handles' current values,
// exactly what World.razor's own HandleWindowDrag already has as its own `window`
// parameter) -- NOT just the dragged handle's own pre-drag value paired with the probe,
// which silently dropped any polity/era beyond the OTHER, still-committed edge for the
// whole gesture (see the batch report's own "Fix round 1" section). `atYear` is still the
// dragged handle's own live value, the one `animate` sweeps against.
export function morphFrame(id, from, to, atYear) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    inst.polities.requestMorphFrame(from, to, atYear);
}

// Batch M requirement 3a: "on release, settle into the static layered-era presentation."
// Settles INSTANTLY and LOCALLY from the already-loaded roster (zero network wait -- the
// morph engine already has every polity's own full history in memory) using the exact same
// lookup+overlay path the committed-window network response also produces (see
// BorderLayer.settleMorph's own comment) -- World.razor's own separate, debounced network
// fetch still lands a little later and repaints from the server's answer, which is byte-
// identical by construction, so this is a pure UX win (instant settle) with no correctness
// risk from skipping ahead of it.
export function settleMorph(id, from, to) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    inst.polities.settleMorph(from, to);
}

// Batch R requirement 1 ("borders become part of the plate"): pushes the
// curated land mask (fetched once, World.razor's own PushLandMaskIfReady) to
// the border layer's own clipPath -- see BorderLayer.setLandMask's own
// comment. `landMaskJson` is the flat `[[[lat,lon],...],...]` array
// `GET /api/land-mask`'s own `rings` field carries.
export function setLandMask(id, landMaskJson) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    const rings = typeof landMaskJson === 'string' ? JSON.parse(landMaskJson) : (landMaskJson || []);
    inst.polities.setLandMask(rings);
}

// Batch R requirement 5 (place-in-verse hover -> marker blink): toggles the
// `.atlas-blink` class (app.css) on `placeId`'s own marker core -- across
// EVERY currently-live, non-mini map instance (this module's own
// module-level `instances` registry, the same one `debugLiveInstanceIds`
// already exposes read-only) -- so this one call reaches whichever of the
// full `/world` page's own map or a split view's embedded atlas pane's own
// map (or both, or neither) is currently showing this place, with no
// Blazor-side wiring beyond the single interop call itself (see
// MapInterop.BlinkPlace's own comment for why this is a STATIC helper, not
// bound to any one map instance). Looks the id up in BOTH `markers` (lit)
// and `quietMarkers` (quiet) -- a place is always exactly one of the two for
// a live time-mode scene (QUIET-1), so exactly one lookup ever hits.
// Harmlessly a no-op wherever the place isn't currently on the map at all
// (Reader.razor standalone, or a place off the current scene's own window).
export function blinkPlace(placeId, active) {
    for (const inst of instances.values()) {
        if (inst.mini) {
            continue;
        }
        const entry = inst.markers.get(placeId) || inst.quietMarkers.get(placeId);
        if (!entry) {
            continue;
        }
        const el = entry.marker.getElement();
        const core = el && el.querySelector('.atlas-marker, .quiet-marker');
        if (core) {
            core.classList.toggle('atlas-blink', !!active);
        }
    }
}

// Batch N requirement 3 ("map focus sync -- the same graph, seen twice"):
// SAME static-helper shape as blinkPlace directly above, for the identical
// reason (MapInterop.SetNarrativeFocus's own comment) -- loops EVERY
// currently-live, non-mini map instance's own ArrowLayer (mini instances
// never render arrows at all, per ArrowLayer's own pre-existing "mini
// instances never render arrows" precedent this file already documents
// elsewhere) and delegates to ArrowLayer.setFocus below. `activeNarrativeIds`/
// `currentEventIds` are plain arrays (never a Set -- this crosses a JS
// interop boundary from Blazor's own IReadOnlyList<string>, which
// serializes as a JSON array).
export function setNarrativeFocus(activeNarrativeIds, currentEventIds) {
    for (const inst of instances.values()) {
        if (inst.mini || !inst.arrows) {
            continue;
        }
        inst.arrows.setFocus(activeNarrativeIds || [], currentEventIds || []);
    }
}

// HOTFIX batch (batch-hotfix-brief.md requirement 1, "the hover menu can be
// cut off by the top of the screen"): measures a PlaceCard's own just-
// rendered box, ONE-SHOT -- ID-less, not bound to any one map instance,
// same reasoning as blinkPlace's own STATIC-helper comment above (this
// card has no live MapInterop of its own to hang a call off; it just needs
// whatever DOM element Blazor's own @ref already handed it). `cardEl` is
// the `.place-card` root div itself; its DOM parent is ALWAYS the map
// container this card is currently rendered inside (`.world-page`
// standalone, `.split-pane-atlas` embedded -- World.razor renders
// `<PlaceCard>` as a direct child, never through an intermediate wrapper --
// see PlaceCard.razor's own file header), so `clientWidth`/`clientHeight`
// there are exactly the box CardPlacement.Compute (client/CardPlacement.cs)
// needs to clamp against: the SAME box `.world-page`'s own `overflow:hidden`
// actually clips to, in EITHER context, with no separate selector/plumbing
// per page. offsetWidth/offsetHeight (not getBoundingClientRect) -- cheaper,
// and transform (app.css's own `.place-card` translate) never affects
// either: transform is purely a paint-time visual offset, not a layout-box
// size. A detached/gone element (the card closed while an in-flight
// measurement call was still pending -- see PlaceCard.razor's own
// OnAfterRenderAsync comment for the exact race this guards) simply
// measures 0 everywhere, same graceful-degradation shape as panToPlace's
// own `if (!inst)` above -- never throws.
//
// Fix round 1 (review finding, Important): containerHeight added --
// clientWidth alone let CardPlacement.Compute decide the HORIZONTAL clamp
// and the flip-vs-not decision, but never let it check whether a FLIPPED
// card actually fits inside the container's own bottom edge, so a tall
// enough card near the vertical middle of a short viewport could flip
// below and overflow the bottom -- live-reproduced at 1280x720 (122px past
// the bottom, marker "Ai", exodus scene) before this fix.
export function measureCardPlacement(cardEl) {
    if (!cardEl || !cardEl.isConnected) {
        return { width: 0, height: 0, containerWidth: 0, containerHeight: 0 };
    }

    const container = cardEl.parentElement;
    return {
        width: cardEl.offsetWidth,
        height: cardEl.offsetHeight,
        containerWidth: container ? container.clientWidth : 0,
        containerHeight: container ? container.clientHeight : 0,
    };
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

    // Batch G1: the document-level Escape listener (watchEscape, !mini only
    // -- see init()'s own comment) is NOT torn down by map.remove() below,
    // unlike every Leaflet-internal listener (map.on(...), marker/arrow
    // events) -- document.addEventListener is independent of the Leaflet
    // map instance entirely. Must be explicitly cleaned up here or it would
    // keep calling back into a disposed dotnetRef for the rest of the page's
    // life.
    inst.escapeCleanup?.();

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

        // Batch N: same reset, same reasoning, for setFocus's own
        // data-narrative-focus -- a scene change (the time slider dragged
        // while a narrative popover happens to be open) is not itself a
        // popover navigation, so nothing else would otherwise re-sync this
        // attribute for an arrow whose key survives the window change.
        // Cleared to baseline here rather than re-derived (World.razor's
        // own scene reload has no reason to know about the popover's
        // current narrative context, mirroring isolate's own "starts fresh
        // every scene" precedent immediately above) -- the popover's own
        // NEXT navigation (a traversal step, or simply re-opening it) calls
        // MapInterop.SetNarrativeFocus again and restores it.
        for (const entry of this._paths.values()) {
            entry.path.removeAttribute('data-narrative-focus');
            entry.casing.removeAttribute('data-narrative-focus');
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

    // Batch N requirement 3 ("map focus sync -- the same graph, seen
    // twice"): a SEPARATE mechanism from setIsolate above, not a second
    // caller of it -- WORLD-4's own legend isolate is a deliberate, sticky
    // USER toggle (persists until clicked again); this is a transient,
    // POPOVER-driven emphasis that follows whatever narrative context is
    // currently open, and the two must be able to coexist without either
    // silently overwriting the other's own attribute. `data-narrative-focus`
    // is therefore its OWN attribute, never reusing data-faded -- see
    // app.css's own comment on why "recede" (this requirement's own word)
    // is deliberately a gentler dim than isolate's near-invisible .12,
    // and on the deliberate CSS ordering that lets an explicit legend
    // isolate keep winning over this if both ever apply to the same arrow
    // at once.
    //
    // `activeNarrativeIds` empty -> every arrow's own attribute is CLEARED
    // (removed, not set to some "none" string) -- "arrows return to
    // normal" reads correctly off the plain absence of the attribute, the
    // exact same baseline a fresh arrow (never isolated, never focused)
    // already has. Otherwise: an arrow whose own narrative isn't in the
    // active set recedes; one that IS gets "current" (strongest emphasis)
    // when either of its own endpoints (from_event/to_event) is in
    // `currentEventIds`, else "active" (amplified, but less than current).
    setFocus(activeNarrativeIds, currentEventIds) {
        const active = new Set(activeNarrativeIds);
        const current = new Set(currentEventIds);
        for (const entry of this._paths.values()) {
            const a = entry.arrow;
            let state = null;
            if (active.size > 0) {
                state = !active.has(a.narrative) ? 'receded' : (current.has(a.from_event) || current.has(a.to_event)) ? 'current' : 'active';
            }
            if (state) {
                entry.path.setAttribute('data-narrative-focus', state);
                entry.casing.setAttribute('data-narrative-focus', state);
            } else {
                entry.path.removeAttribute('data-narrative-focus');
                entry.casing.removeAttribute('data-narrative-focus');
            }
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
            // Batch G1 requirement 3: same reason wireEvents/wireQuietEvents
            // stop propagation on their own marker clicks -- this is a raw
            // DOM listener (not a Leaflet-wrapped one), so the plain native
            // stopPropagation is what's needed here, but the goal is
            // identical: an arrow click must never ALSO bubble up as a
            // map-background click and unpin a card in the same gesture.
            e.stopPropagation();
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
// unchanged: a custom L.Layer managing TWO plain <svg> elements (see the
// fix round 1 / BLOCKER note just below for why two, not one), added to the
// map BEFORE ArrowLayer in init() so DOM paint order puts the overlayPane
// one below the narrative threads (see init()'s own comment), non-
// interactive (no event wiring; app.css's .atlas-border* rules set
// pointer-events:none), no diffing (a polity swap is a rare, 150ms-
// debounced window-change event, not a hot per-frame path -- replacing the
// layer's content wholesale on every setPolities call is simple and cheap
// enough).
//
// Fix round 1 (BLOCKER B1 -- review finding: shipped `mix-blend-mode:
// multiply` never actually reached the tiles; an A/B pixel-diff against
// `mix-blend-mode:normal` at the same opacity came back 0.257/255 mean
// difference, i.e. pixel-for-pixel indistinguishable, on real textured
// relief). Root cause: Leaflet's own vendored `leaflet.css` gives every
// pane `position:absolute` PLUS an explicit numeric `z-index`
// (`.leaflet-tile-pane{z-index:200}`, `.leaflet-overlay-pane{z-index:400}`)
// -- both of those alone make a pane its OWN CSS stacking context, so
// `mix-blend-mode` set on a path living inside `overlayPane` can only ever
// blend against OTHER content painted inside `overlayPane` (nothing, for a
// lone wash path), never against `tilePane`'s own img elements, which sit
// in a completely different, sibling stacking context. No amount of
// `isolation`/blend-mode tweaking on the PATH itself can cross that
// boundary -- the fix has to move the blending element itself to a
// different point in the pane hierarchy.
//
// Fix: a dedicated `washPane` (created in init(), z-index 300 -- between
// tilePane's 200 and overlayPane's 400, both siblings under Leaflet's own
// `leaflet-map-pane`) holds ONLY the wash <path>s, in their own
// `this._washSvg`. `mix-blend-mode:multiply` is set on the PANE element
// itself (app.css's `.atlas-wash-pane` rule), not on the individual wash
// paths -- because washPane sits at z-index 300, strictly between tilePane
// and overlayPane in the SAME parent stacking context (`leaflet-map-pane`),
// the browser first flattens washPane's own children into one layer
// (ordinary, unblended compositing among the washes themselves, so an
// overlapping older/newer ring pair of one polity still layers normally),
// then blends THAT WHOLE LAYER via multiply against whatever painted before
// it in `leaflet-map-pane`'s own stacking order -- i.e. the actual tile
// imagery. This is the standard, documented way to blend a whole custom
// Leaflet layer against the base map (the pane IS the blending unit, not
// any one shape inside it). Band/line/labels/year-tags stay exactly where
// they always were, in `this._svg` (overlayPane) -- per the fix round's own
// scope, they are NOT blended (`.atlas-border-band`'s own now-inert
// `mix-blend-mode:multiply` is removed too, for the same underlying reason
// it never did anything there either; see app.css's own comment on why the
// band now uses a plain, un-blended translucent stroke instead).
//
// "Printed, not overlaid": every ring still gets THREE stacked <path>
// elements, just no longer all siblings of each other --
//   .atlas-border-wash  the fill, living in `this._washSvg` (washPane) --
//                       blended AS PART OF THE PANE (see above) so the
//                       tint DARKENS the terrain like ink soaking into
//                       paper rather than a flat shape floating on top --
//                       terrain relief shows through at every opacity this
//                       batch uses, by construction of multiply blending.
//   .atlas-border-band  a WIDE, low-opacity, no-fill stroke along the same
//                       ring -- "a soft inner tint band along the border",
//                       living in `this._svg` (overlayPane), UNBLENDED (a
//                       plain translucent stroke -- see app.css). SVG
//                       strokes straddle their path (half in, half out) by
//                       default, so a wide soft stroke reads as a band
//                       hugging the border without any inset-polygon math
//                       (explicitly out of scope -- "do not try to get too
//                       fancy with mathematics").
//   .atlas-border-line  the fine, crisp border LINE itself, in `this._svg`
//                       (overlayPane), in the polity's own hue DARKENED
//                       (POLITY_TINTS_DARK) -- solid for the newest/only
//                       visible era of a polity, dashed or dotted for an
//                       older one still in view (see the "age" tiering
//                       below).
// All three share one ring's own projected `d` (recomputed every redraw,
// same as before, regardless of which SVG element each one's own `<path>`
// lives in); within EACH of the two SVGs, elements are appended in
// oldest-to-newest-per-polity order across every ring (all washes in
// `this._washSvg`; all bands then all lines, in that order, in
// `this._svg`) rather than per-ring, so that even where one polity's
// later, larger ring visually covers an earlier, smaller one's fill, every
// ring's own LINE still paints on top of every band and stays crisply
// visible -- the mechanism "the older dotted ring visible around/inside
// the newer solid one" actually depends on.
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
// Fix round 1 (M1 -- review finding B2: with only 8 hues and 14 curated
// polities, the plain hash-mod-8 assignment collided 11 of 14 polities into
// shared buckets, including geographically/temporally overlapping pairs
// that most needed telling apart -- Judah/Neo-Assyrian-Empire, Judah/
// Achaemenid-Persia, Judah's Herodian era/Rome). Widened 8 -> 16 tints,
// within the design authority's own "12-16" suggestion: entries 0-7 are
// Batch C2's own original 8, byte-identical (so any polity whose id
// already hashed into 0-7 keeps the exact same color it always had);
// entries 8-15 are new. NOT hand-picked/eyeballed (an earlier pass at this
// fix that WAS hand-picked shipped a real, live-screenshot-caught defect --
// see below) -- computed by a small throwaway Python script (farthest-point
// / greedy dispersion: repeatedly add the hue that maximizes its OWN
// minimum angular distance to every hue already placed, starting from the
// 8 fixed originals) so every one of the 16 is as far as achievable from
// its nearest neighbor given the original 8's own uneven, warm-clustered
// spacing (rose/coral/tan/buff all sit within about 40 degrees of each
// other, deliberately, "keep them plate-like on warm terrain" -- fixed,
// unchangeable, see below), at the SAME muted saturation/lightness (S~26%,
// L~58%) as the original 8 so the new hues still read as the same
// hand-tinted plate, not a jarring addition. SERVER-SIDE assignment
// (atlas_etl::polities::assign_color_keys) is what actually guarantees
// collision-freedom -- see that function's own doc comment -- this array
// is just the palette it indexes into; widening it alone would do nothing
// without that algorithm change alongside it.
//
// Live-screenshot-caught defect, fixed by the above (not by hand-tuning
// individual hex values further -- see the fix round's own report for the
// full before/after numbers): an EARLIER version of this array picked its
// 8 new hues by eye, hue-wheel-sorted, then interleaved to avoid nearby
// ARRAY INDICES landing on nearby hues (linear probing tends to assign ids
// with nearby hash seeds to nearby indices). That fixed the index-adjacency
// risk but missed a DIFFERENT failure mode entirely: two hand-picked hues
// (that pass's own "violet" and "indigo") were simply too close to EACH
// OTHER in raw hue terms (30 degrees) regardless of which indices they
// ended up at -- confirmed live when `sumer` and `elam` (this app's own
// real curated roster) landed on exactly that pair and read as visually
// similar in a screenshot. The greedy-dispersion approach here checks
// EVERY pairwise distance among all 16, not just array-adjacent ones, so
// it can't repeat that specific mistake.
//
// Entries 0-7 are Batch C2's own original 8, byte-identical (unchanged,
// deliberately -- any polity whose id already hashed into 0-7 keeps
// exactly the color it always had).
const POLITY_TINTS = [
    '#C98A8A', // 0 rose
    '#C9B37E', // 1 buff
    '#93A98B', // 2 sage
    '#7E99B5', // 3 pale lapis
    '#A18CB0', // 4 lilac
    '#B59B7E', // 5 tan
    '#8FA07A', // 6 moss
    '#C08E7A', // 7 coral
    '#78B09B', // 8 teal-jade
    '#B078A0', // 9 mauve
    '#7B78B0', // 10 indigo
    '#78B082', // 11 jade
    '#78ACB0', // 12 steel-cyan
    '#ACB078', // 13 khaki
    '#AC78B0', // 14 orchid
    '#B0788C', // 15 wine-rose
];

// The SAME 16 hues as POLITY_TINTS, each channel scaled by a flat 0.62 --
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
    '#4A6D60', // 8 teal-jade, darkened
    '#6D4A63', // 9 mauve, darkened
    '#4C4A6D', // 10 indigo, darkened
    '#4A6D51', // 11 jade, darkened
    '#4A6B6D', // 12 steel-cyan, darkened
    '#6B6D4A', // 13 khaki, darkened
    '#6B4A6D', // 14 orchid, darkened
    '#6D4A57', // 15 wine-rose, darkened
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

// --- Batch M requirement 4: delta-ring explorability helpers -------------

// Is `entry`'s own end (`entry.to`) the polity's TRUE chronologically-final
// era, across the FULL roster (not just whatever this narrow window's own
// lookup happens to show)? A narrow window can show only ONE internal era
// of a longer-lived polity, whose own `to` boundary is a transition INTO
// the next era (simply not shown in this window), never a real "fall" --
// `entry.fall`'s own presence already answers this precisely whenever
// curated, but the MINIMAL-popover case (a true final era with no fall
// authored) still needs this roster-wide check to avoid mistakenly
// offering fall-explorability on every merely-newest-in-THIS-window ring.
function isChronologicallyFinalEra(roster, entry) {
    if (!roster || roster.length === 0) {
        return entry.fall !== undefined; // roster not loaded yet -- fall back to what we do know
    }
    let maxTo = -Infinity;
    for (const e of roster) {
        if (e.id === entry.id) {
            maxTo = Math.max(maxTo, e.to);
        }
    }
    return entry.to === maxTo;
}

// The immediately-PRECEDING era of the same polity, by `from` order, in the
// FULL roster -- used only for a transition delta's own TITLE years (the
// year the PREVIOUS era ended is a more precise "when did the change
// happen" than the new era's own `from`, which sits one year later by this
// app's own adjacent-year convention -- see the batch report's own title-
// format note). `null` when `entry` is the polity's own very first era (no
// predecessor).
function previousEra(roster, entry) {
    let best = null;
    for (const e of roster || []) {
        if (e.id === entry.id && e.from < entry.from) {
            if (!best || e.from > best.from) {
                best = e;
            }
        }
    }
    return best;
}

// Which delta (if any) THIS ring is the explorable hit target for, given
// the CURRENTLY APPLIED window -- "every era boundary whose transition
// falls INSIDE the window is explorable," per the brief, verbatim. A ring
// can be the target for at most ONE delta: FALL takes priority on the rare
// occasion both this era's own start AND end boundaries sit inside one
// (necessarily narrow) window simultaneously -- the more climactic moment
// for a polity that's ending, a disclosed tie-break, not an oversight.
// `delta` (the actual `{event, verses, ref_note}` payload) may be
// `undefined` even when a `kind`/title IS returned -- "an uneventful
// boundary stays visible but gets the minimal popover."
function deltaForEntry(roster, entry, from, to) {
    const startInWindow = entry.from >= from && entry.from <= to;
    const endInWindow = entry.to >= from && entry.to <= to;

    if (endInWindow && isChronologicallyFinalEra(roster, entry)) {
        return { kind: 'fall', delta: entry.fall, titleFrom: entry.from, titleTo: entry.to };
    }
    if (startInWindow) {
        const prev = previousEra(roster, entry);
        const titleFrom = prev ? prev.to : entry.from;
        return { kind: 'transition', delta: entry.transition, titleFrom, titleTo: entry.to };
    }
    return null;
}

function deltaAriaLabel(entry, found) {
    const base = found.kind === 'fall' ? `${entry.name}, fall` : `${entry.name}, transition`;
    return found.delta && found.delta.event ? `${base}: ${found.delta.event}` : `${base} (explore)`;
}

// prefers-reduced-motion (req 3a: "no morph -- snap directly"): rounds a
// probe year to whichever bracketing KNOT (border-morph.js's own
// knotYear -- each line's own [from,to] midpoint) it's numerically nearer,
// so `animate(lines, snapYear(...))` returns an UN-interpolated line's own
// rings unchanged -- the identical `animate` function every ordinary
// (motion-allowed) frame also calls, just fed a year that always lands
// exactly ON a knot instead of between two.
function snapYear(lines, atYear) {
    if (lines.length <= 1) {
        return atYear;
    }
    let best = lines[0];
    let bestDist = Infinity;
    for (const line of lines) {
        const knot = (line.from + line.to) / 2;
        const dist = Math.abs(atYear - knot);
        if (dist < bestDist) {
            bestDist = dist;
            best = line;
        }
    }
    return (best.from + best.to) / 2;
}

const BorderLayer = L.Layer.extend({
    initialize(dotnetRef) {
        this._dotnetRef = dotnetRef; // Batch M requirement 4: a delta ring's own click calls back into World.razor (OnPolityDeltaClick)
        this._entries = []; // [{ id, name, from, to, rings, color_key, age, tierCount, transition, fall }]
        this._ringGroups = []; // [{ wash, band, line, ring, entry, ringIndex, hit? }]
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

        // Batch M requirement 3a: the morph engine's own state. `_roster` is
        // the FULL, unfiltered `/api/polities` roster (every era of every
        // polity, fetched once -- setPolitiesRoster below), the one thing
        // `lookup` needs to answer an arbitrary drag-time window without a
        // network round trip. `_morphing`/`_morphGroups` exist only WHILE a
        // drag gesture is in progress -- see beginMorph/morphFrame/settleMorph.
        this._roster = [];
        this._morphing = false;
        this._morphGroups = new Map(); // polityId -> { washes: [], bands: [], lines: [] }
        // The evaluator interface's own rAF-throttling state (requestMorphFrame/
        // _evaluateMorphFrame below): coalesces however many drag-move
        // updates arrive between two animation frames into a single actual
        // evaluate+paint, the standard rAF-throttled-input-handler pattern.
        // `_pendingFrom`/`_pendingTo` (fix round 1, C1): the FULL live window
        // as of the latest drag-move update -- both handles' current values,
        // not just the dragged one's -- also re-read by `_redraw` so an
        // animated zoom/pan mid-drag reprojects the same true window.
        this._rafHandle = null;
        this._pendingFrom = null;
        this._pendingTo = null;
        this._pendingProbeYear = null;
        this._reducedMotion = typeof window !== 'undefined' && window.matchMedia
            ? window.matchMedia('(prefers-reduced-motion: reduce)').matches
            : false;
    },

    onAdd(map) {
        this._map = map;
        this._svg = svgEl('svg', { class: 'atlas-borders' });
        map.getPane('overlayPane').appendChild(this._svg);
        // Fix round 1 (BLOCKER B1): the wash's own SVG, living in washPane
        // (created in init(), before this layer's own addTo(map) --  same
        // ordering this layer already relies on for polityLabelsPane below)
        // instead of overlayPane -- see this layer's own header comment for
        // why the wash needs a separate pane at all. A DIFFERENT class from
        // the main SVG's own `atlas-borders` (`atlas-borders-wash` instead)
        // -- deliberately, so every existing `svg.atlas-borders` selector
        // (BORDERS-4/BORDERS-9's own Playwright locators, both written
        // before this fix and both requiring an unambiguous single-element
        // match) keeps resolving to exactly the one element it always did,
        // with zero test changes forced by this fix; app.css's shared
        // position/overflow/pointer-events rule targets both classes via one
        // comma selector, and `.atlas-wash-pane` (the actual blend-mode
        // rule) targets the PANE, not either SVG -- see this layer's own
        // header comment.
        this._washSvg = svgEl('svg', { class: 'atlas-borders-wash' });
        map.getPane('washPane').appendChild(this._washSvg);

        // Batch R requirement 1 ("borders become part of the plate"): a
        // permanent <defs> holding the land-mask clipPath and the wash
        // feather filter -- SIBLING of `_washGroup` below (never a child of
        // it), so `setPolities`' own wholesale wipe-and-repaint (see its own
        // comment) never has to touch, and can never accidentally delete,
        // this static geometry. `_washGroup` is the ACTUAL wash-paint target
        // from here on (every wash <path> setPolities creates is appended to
        // IT, not to `_washSvg` directly) -- `clip-path` lives on this one
        // group (set once here, static, referencing the clipPath below by
        // id) so it clips the group's WHOLE flattened rendering, including
        // each wash path's own feather blur (CSS Filter Effects/Masking's
        // own documented order: filter first, then clip-path -- so a
        // blurred wash edge bleeding toward open water is still cut off
        // exactly at the coastline, never past it). Feather (feGaussianBlur)
        // itself is applied per-path instead, via app.css's own
        // `.atlas-border-wash { filter: ... }` rule -- each path's own
        // filter region sizes to ITS OWN bounding box, not one shared huge
        // region for the whole plate.
        this._washDefs = svgEl('defs');
        this._washSvg.appendChild(this._washDefs);
        this._clipPath = svgEl('clipPath', { id: 'atlas-land-clip' });
        this._washDefs.appendChild(this._clipPath);
        const feather = svgEl('filter', { id: 'atlas-wash-feather', x: '-40%', y: '-40%', width: '180%', height: '180%' });
        feather.appendChild(svgEl('feGaussianBlur', { stdDeviation: '2.4' }));
        this._washDefs.appendChild(feather);
        this._landMaskRings = [];
        this._landMaskPaths = [];

        this._washGroup = svgEl('g', { class: 'atlas-wash-clip-group' });
        this._washSvg.appendChild(this._washGroup);

        // Batch M requirement 3a: two SIBLING sub-groups inside each of the
        // two ring-painting parents (`_washGroup` for wash fills;
        // `_svg`/overlayPane directly for band/line/hit strokes -- there is
        // no equivalent "clip group" layer to nest under on the overlayPane
        // side, so the morph/settled split happens directly on `_svg`
        // itself there) -- one holds the SETTLED (overlay-combinator)
        // display, one holds the MORPH (animate-combinator) display while a
        // drag is in progress. Both a settled wash sub-group and a morph
        // wash sub-group live INSIDE `_washGroup`, so BOTH inherit its one
        // `clip-path` (the land mask) and both track zoomanim identically --
        // "morphing content stays clipped to Batch R's land mask," per the
        // brief, is true by construction, not a separate wiring concern.
        // O(1) visibility toggling (display:none on one whole group) is
        // what makes beginMorph/settleMorph cheap and avoids ever touching
        // more than the ~14 polities' worth of individual path elements a
        // frame.
        this._washSettledGroup = svgEl('g', { class: 'atlas-wash-settled-group' });
        this._washMorphGroup = svgEl('g', { class: 'atlas-wash-morph-group' });
        this._washMorphGroup.style.display = 'none';
        this._washGroup.appendChild(this._washSettledGroup);
        this._washGroup.appendChild(this._washMorphGroup);

        this._strokeSettledGroup = svgEl('g', { class: 'atlas-border-settled-group' });
        this._strokeMorphGroup = svgEl('g', { class: 'atlas-border-morph-group' });
        this._strokeMorphGroup.style.display = 'none';
        this._svg.appendChild(this._strokeSettledGroup);
        this._svg.appendChild(this._strokeMorphGroup);

        this._labelPane = map.getPane('polityLabelsPane'); // created in init(), before this layer's own addTo(map)
        this._offZoomAnim = attachZoomAnim(map, this._svg);
        // Fix round 1: the wash SVG needs the SAME zoomanim treatment as the
        // main one, independently -- two separate <svg> elements in two
        // separate panes each need their own CSS transform applied during an
        // animated zoom, or the wash would visibly lag/detach from its own
        // ring's band+line mid-zoom (a new regression the original zoom-sync
        // fix, requirement 5, didn't have to account for, since the wash
        // lived in the same SVG as everything else at the time). Batch R:
        // the land-mask clip lives in the SAME `_washSvg`, so it tracks this
        // exact same transform for free -- no separate zoomanim wiring
        // needed for req 1's own "clip must track zoomanim" ask.
        this._offZoomAnimWash = attachZoomAnim(map, this._washSvg);
        return this;
    },

    onRemove() {
        if (this._offZoomAnim) {
            this._offZoomAnim();
            this._offZoomAnim = null;
        }
        if (this._offZoomAnimWash) {
            this._offZoomAnimWash();
            this._offZoomAnimWash = null;
        }
        if (this._rafHandle != null) {
            cancelAnimationFrame(this._rafHandle);
            this._rafHandle = null;
        }
        this._svg.remove();
        this._washSvg.remove();
        this._ringGroups = [];
        // Morph paths live inside _washGroup/_svg (children of the two SVGs
        // just removed above) -- already gone from the DOM at this point;
        // this just drops the now-stale element references.
        this._morphGroups = new Map();
        this._morphing = false;
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

    // `list` is the flat `[{ id, name, from, to, rings, color_key,
    // transition, fall }, ...]` array `GET /api/polities` returns, already
    // ordered by id then from. `from`/`to` are the CURRENTLY APPLIED window
    // (Batch M requirement 4: needed to decide which era boundary, if any,
    // each ring is the explorable delta target for -- "every era boundary
    // whose transition falls INSIDE THE WINDOW is explorable").
    //
    // Batch M requirement 3: routes through `lookup`+`overlay`
    // (border-morph.js) instead of the inline group/sort/age-tag logic this
    // method used to own directly -- the SAME two functions the morph path
    // (settleMorph below) also calls, so the settled display and a
    // released-drag's own instant local settle can never disagree about
    // what "this window's own lines, styled" means. Re-filtering `list`
    // (already server-filtered to this exact window) through `lookup` again
    // is a safe no-op -- `lookup` is idempotent under re-filtering with the
    // SAME window -- and is what makes "the same function, the ONLY
    // function, that resolves era geometry" literally true of this call
    // site too, not just the morph one.
    setPolities(list, from, to) {
        const grouped = lookup(list, from ?? -Infinity, to ?? Infinity);
        const entries = [];
        for (const lines of grouped.values()) {
            entries.push(...overlay(lines));
        }
        this._paintSettled(entries, from, to);
    },

    // Batch M requirement 3a: the FULL, unfiltered roster (every era of
    // every polity) -- see map.js's own exported setPolitiesRoster's
    // comment for why this is fetched once, separately, and stored here.
    setPolitiesRoster(roster) {
        this._roster = roster || [];
    },

    // The settled (overlay-combinator) paint -- shared by setPolities
    // (network-fed) and settleMorph (locally computed from the already-
    // loaded roster, zero network wait). `entries` is already grouped +
    // age-tagged (overlay's own output); this method's ONLY job from here
    // is DOM painting, unchanged in spirit from the pre-Batch-M
    // setPolities this replaces -- see that method's own prior header
    // comment (still accurate) for why paint order alone (oldest-to-newest
    // per polity) is what makes growth/contraction legible with no
    // geometry comparison.
    _paintSettled(entries, from, to) {
        resetZoomAnimTransform(this._svg);
        resetZoomAnimTransform(this._washSvg); // fix round 1: second SVG, same reset (see onAdd's own comment)
        // Batch M requirement 3a: wipes the SETTLED sub-groups only (never
        // the morph ones, never `_washDefs`/`_clipPath` -- see onAdd's own
        // comment on why each of those needs to survive a repaint it isn't
        // itself responsible for).
        this._strokeSettledGroup.replaceChildren();
        this._washSettledGroup.replaceChildren();
        for (const l of this._labels) {
            l.el.remove();
        }
        for (const t of this._yearTags) {
            t.el.remove();
        }

        this._entries = entries;
        this._windowFrom = from;
        this._windowTo = to;

        // Three passes -- ALL washes, then ALL bands, then ALL lines, then
        // ALL delta hit-strokes -- across every ring of every entry (in the
        // entries' own oldest-to-newest-per-polity order, `lookup`'s own
        // guarantee, unchanged by the Batch M refactor).
        this._ringGroups = [];
        const washEls = [], bandEls = [], lineEls = [], hitEls = [];
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
                const g = { wash, band, line, ring, entry, ringIndex };
                this._ringGroups.push(g);

                // Batch M requirement 4: is this ring's own era boundary
                // explorable in the CURRENTLY APPLIED window? See
                // deltaForEntry's own doc comment for the exact rule.
                if (from !== undefined && to !== undefined) {
                    const found = deltaForEntry(this._roster, entry, from, to);
                    if (found) {
                        const hit = this._makeDeltaHit(g, found);
                        hitEls.push(hit);
                        g.hit = hit;
                        g.delta = found;
                    }
                }
            });
        }
        // Fix round 1 (BLOCKER B1): washEls go into the wash SETTLED
        // sub-group (washPane) -- see this layer's own header comment for
        // why the wash needs to live in a different pane for its blend mode
        // to actually reach the tiles; Batch R: `_washSettledGroup` sits
        // INSIDE `_washGroup` (the land-mask-clipped, per-path-feathered
        // group), not `_washSvg` directly -- see onAdd's own comment.
        // bandEls/lineEls/hitEls go into the stroke SETTLED sub-group
        // (overlayPane), oldest-to-newest, bands then lines then hit-
        // strokes (hit-strokes paint LAST/topmost among the three so their
        // own wide invisible stroke always wins hit-testing over a
        // same-ring line/band, never the reverse).
        for (const el of washEls) {
            this._washSettledGroup.appendChild(el);
        }
        for (const el of bandEls) {
            this._strokeSettledGroup.appendChild(el);
        }
        for (const el of lineEls) {
            this._strokeSettledGroup.appendChild(el);
        }
        for (const el of hitEls) {
            this._strokeSettledGroup.appendChild(el);
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
        //
        // Fix round 1 (M2 -- review finding B3: every tag anchored at its
        // own ring's bbox CENTER with only a fixed downward offset, so
        // concentric growth/contraction rings of the SAME polity -- the
        // exact case this feature exists to showcase -- put their tags
        // almost exactly on top of each other; visible in the batch's own
        // egypt-exodus and spanning-1600-900bc screenshots). Fix: group
        // this polity's own tag-needing ring groups first (still in the
        // existing oldest-to-newest order _ringGroups already carries), then
        // give each one an index within ITS OWN polity's group -- fed to
        // _makeYearTag below, which uses it to stagger the tag at a
        // DIFFERENT fractional position around ITS OWN ring's perimeter
        // (see _ringPerimeterPoint's own comment) instead of every tag
        // landing on the shared, near-identical bbox center concentric
        // rings naturally have.
        const tagGroupsById = new Map();
        for (const g of this._ringGroups) {
            if (g.entry.tierCount > 1) {
                if (!tagGroupsById.has(g.entry.id)) {
                    tagGroupsById.set(g.entry.id, []);
                }
                tagGroupsById.get(g.entry.id).push(g);
            }
        }
        this._yearTags = [];
        for (const group of tagGroupsById.values()) {
            group.forEach((g, tagIndex) => {
                this._yearTags.push(this._makeYearTag(g, tagIndex, group.length));
            });
        }

        this.setVisible(true);
        this._redraw();
    },

    // Batch M requirement 4: the wide, transparent hit-stroke a delta-
    // eligible ring gets, ON TOP of (painted after) its own line/band --
    // ">=14px effective hit via a wide transparent hit-stroke," per the
    // brief. `stroke-width` is set (app.css) in the SAME projected-pixel
    // coordinate space `_ringPathData` already draws in
    // (map.latLngToLayerPoint), so 14 CSS px of stroke width IS 14 real
    // screen px of hit target at any zoom, no extra math needed. Hover
    // darkens the ring's own wash+line (ONE-RULE's spirit, adapted for an
    // SVG shape -- see app.css's own `.atlas-border-line[data-delta-hover]`
    // comment for why plain `.explorable` itself, background-color-based,
    // doesn't apply to an SVG path); click/Enter opens the popover via
    // OnPolityDeltaClick, with EVERY field the popover needs already
    // resolved client-side (this._roster already has it all) -- no second
    // fetch, no server-side lookup by id needed.
    _makeDeltaHit(g, found) {
        const hit = svgEl('path', {
            class: 'atlas-border-delta-hit',
            fill: 'none',
            tabindex: '0',
            role: 'button',
            'aria-label': deltaAriaLabel(g.entry, found),
            'data-testid': `polity-delta-${esc(g.entry.id)}-${g.entry.from}-${g.ringIndex}`,
        });
        const setHover = on => {
            if (on) {
                g.line.setAttribute('data-delta-hover', 'true');
                g.wash.setAttribute('data-delta-hover', 'true');
            } else {
                g.line.removeAttribute('data-delta-hover');
                g.wash.removeAttribute('data-delta-hover');
            }
        };
        hit.addEventListener('mouseover', () => setHover(true));
        hit.addEventListener('mouseout', () => setHover(false));
        hit.addEventListener('focus', () => setHover(true));
        hit.addEventListener('blur', () => setHover(false));
        const activate = e => {
            e.stopPropagation(); // never also register as a map-background click (OnMapClick would otherwise close a pinned card underneath)
            const d = found.delta;
            this._dotnetRef.invokeMethodAsync(
                'OnPolityDeltaClick',
                g.entry.name,
                found.kind,
                found.titleFrom,
                found.titleTo,
                d ? d.event : null,
                d ? d.verses : [],
                d ? d.ref_note : null
            );
        };
        hit.addEventListener('click', activate);
        hit.addEventListener('keydown', e => {
            if (e.key === 'Enter') {
                activate(e);
            }
        });
        return hit;
    },

    // --- Batch M requirement 3a: the morph (animate-combinator) path -----

    // Batch M fix round 1 (C1): no `anchorYear` parameter any more -- every
    // `requestMorphFrame` call below now carries the FULL live window
    // (both handles' current values) on its own, every frame, so there is
    // no separate "gesture start" value to remember here at all. Idempotent
    // -- calling again mid-drag (TimeSlider's own OnWindowDrag fires
    // continuously) is a harmless no-op. Swaps which sub-groups are visible
    // (O(1), two `display` toggles) rather than touching any individual
    // ring element -- the settled display's own paths stay fully intact
    // underneath, ready to reappear the instant settleMorph runs on release.
    beginMorph() {
        if (this._morphing) {
            return;
        }
        this._morphing = true;
        this._washSettledGroup.style.display = 'none';
        this._strokeSettledGroup.style.display = 'none';
        this._washMorphGroup.style.display = '';
        this._strokeMorphGroup.style.display = '';
        for (const l of this._labels) {
            l.el.style.display = 'none'; // no per-frame DOM churn beyond path `d` updates -- labels/year-tags simply sit out a morph, restored by the next settle
        }
        for (const t of this._yearTags) {
            t.el.style.display = 'none';
        }
    },

    // Batch M requirement 3a: THE EVALUATOR INTERFACE -- "evaluate on CPU in
    // requestAnimationFrame behind a small evaluator interface... document
    // the GPU/WebGL swap seam." Every drag-move update (World.razor's own
    // HandleWindowDrag, fired on every native pointermove, NOT itself
    // rAF-throttled) lands here and only records the latest probe year;
    // the actual evaluate-and-paint work is scheduled AT MOST ONCE per
    // animation frame, coalescing however many pointer events arrived
    // since the last one -- the standard rAF-throttled-input-handler
    // pattern, not fancy math. `_evaluateMorphFrame` below is the
    // evaluator itself: a CPU implementation of `evaluate(lines, t) ->
    // rings` (via border-morph.js's own `animate`). THE GPU SWAP SEAM: a
    // WebGL evaluator would instead upload posA/posB per-vertex attribute
    // buffers ONCE per correspondence pair (geo.js's own buildCorrespondence
    // output) and vary a single `t` uniform, letting a vertex shader run
    // the identical sin-weighted slerp math geo.js's own slerp() doc
    // comment states, entirely on the GPU -- the CPU side would then update
    // one float per frame instead of recomputing every vertex in JS here.
    // Not built (per the brief's own explicit instruction) -- at this
    // app's own real scale (14 polities, <=80 points/ring, resampled to
    // 128) the CPU path costs microseconds per frame (see the batch
    // report's own measurement), so this seam is documented as a FUTURE
    // swap point, not a present need.
    // Batch M fix round 1 (C1): `from`/`to` are the FULL, CURRENT live
    // window every frame -- exactly the same pair `SettleMorph(from, to)`
    // already correctly receives on release (World.razor's own
    // HandleWindowDrag passes its own `window.From`/`window.To` straight
    // through, unchanged since TimeSlider.razor's OnHandlePointerDown seeds
    // both from the last COMMITTED From/To and only the actively-dragged
    // one ever moves) -- NOT `[the dragged handle's own pre-drag value, the
    // live probe]`, which is what the pre-fix-round code derived `lo`/`hi`
    // from and which silently dropped any polity/era beyond the OTHER,
    // still-committed edge for the whole gesture (see the batch report's
    // own "Fix round 1" section for the live-verified before/after polity
    // counts). `atYear` is still specifically the DRAGGED handle's own live
    // value -- the one `animate` below sweeps against; the other,
    // stationary edge only widens the `lookup` range, it never gets its own
    // `animate` call.
    requestMorphFrame(from, to, atYear) {
        this._pendingFrom = from;
        this._pendingTo = to;
        this._pendingProbeYear = atYear;
        if (this._rafHandle != null) {
            return; // already scheduled -- the eventual frame uses whatever _pending* is LATEST when it actually runs
        }
        this._rafHandle = requestAnimationFrame(() => {
            this._rafHandle = null;
            if (this._morphing) {
                this._evaluateMorphFrame(this._pendingFrom, this._pendingTo, this._pendingProbeYear);
            }
        });
    },

    // One scrub frame's own actual work. `lookup`s the FULL live window
    // (`from`/`to`, both handles' current values -- see requestMorphFrame's
    // own comment for why this must be the full pair, not just the swept
    // sliver), then `animate`s each polity's own sequence to `atYear` (the
    // dragged handle's own live value) to get its CURRENT ring(s) -- "the
    // two modes share everything except the final combinator" is true by
    // construction: this is the EXACT SAME `lookup` `setPolities` above
    // calls, just fed a different (transient, drag-time) window every
    // frame. `prefers-reduced-motion`: SNAPS instead of interpolating
    // (rounds the probe to whichever bracketing knot it's nearer, so
    // `animate` itself returns an UN-interpolated line's own rings -- no
    // separate code path, just a different year fed to the identical
    // function).
    _evaluateMorphFrame(from, to, atYear) {
        if (!this._morphing || !this._map || atYear == null || from == null || to == null) {
            return;
        }
        resetZoomAnimTransform(this._washSvg);
        resetZoomAnimTransform(this._svg);

        const lo = Math.min(from, to);
        const hi = Math.max(from, to);
        const grouped = lookup(this._roster, lo, hi);

        const seen = new Set();
        for (const [polityId, lines] of grouped) {
            const evalYear = this._reducedMotion ? snapYear(lines, atYear) : atYear;
            const rings = animate(lines, evalYear);
            const color = lines[0].color_key;
            this._syncMorphGroup(polityId, color, rings);
            seen.add(polityId);
        }
        // A polity whose OWN full history doesn't reach this instant at all
        // (not yet risen, or -- never happens in this app's real data, but
        // handled -- already gone) shows nothing: drop any leftover morph
        // group for it from a previous frame.
        for (const [polityId, group] of this._morphGroups) {
            if (!seen.has(polityId)) {
                this._removeMorphGroup(polityId, group);
            }
        }
    },

    // Batch M requirement 3a: "on release, settle into the static layered-
    // era presentation" -- INSTANTLY, from the already-loaded roster (zero
    // network wait; see map.js's own exported settleMorph comment for why
    // this never disagrees with the separate, still-in-flight network
    // repaint that also lands a little later). Tears down the morph
    // sub-groups, restores the settled ones, and repaints via the EXACT
    // SAME `_paintSettled` the network-fed `setPolities` above calls.
    settleMorph(from, to) {
        this._morphing = false;
        this._pendingFrom = null;
        this._pendingTo = null;
        this._pendingProbeYear = null;
        if (this._rafHandle != null) {
            cancelAnimationFrame(this._rafHandle);
            this._rafHandle = null;
        }
        this._washMorphGroup.replaceChildren();
        this._strokeMorphGroup.replaceChildren();
        this._washMorphGroup.style.display = 'none';
        this._strokeMorphGroup.style.display = 'none';
        this._morphGroups = new Map();
        this._washSettledGroup.style.display = '';
        this._strokeSettledGroup.style.display = '';

        const grouped = lookup(this._roster, from, to);
        const entries = [];
        for (const lines of grouped.values()) {
            entries.push(...overlay(lines));
        }
        this._paintSettled(entries, from, to);
    },

    // Creates (first call for this polity id) or updates (every later call)
    // ONE wash+band+line triple PER RING `rings` currently has -- ring
    // COUNT is stable frame-to-frame in the overwhelming common case (it
    // only changes at all when `animate` itself snaps across a topology-
    // mismatched boundary, a rare, discrete event -- see border-morph.js's
    // own animate doc comment), so this is "no per-frame DOM churn beyond
    // path `d` updates" for every ordinary frame, with an honest, disclosed
    // exception exactly when the underlying SHAPE genuinely, discretely
    // changes what it even IS.
    _syncMorphGroup(polityId, colorKey, rings) {
        let group = this._morphGroups.get(polityId);
        if (!group) {
            group = { washes: [], bands: [], lines: [] };
            this._morphGroups.set(polityId, group);
        }
        const fillColor = POLITY_TINTS[((colorKey % POLITY_TINTS.length) + POLITY_TINTS.length) % POLITY_TINTS.length];
        const lineColor = POLITY_TINTS_DARK[((colorKey % POLITY_TINTS_DARK.length) + POLITY_TINTS_DARK.length) % POLITY_TINTS_DARK.length];

        while (group.washes.length < rings.length) {
            const wash = svgEl('path', { class: 'atlas-border-wash', 'data-age': 'newest', 'data-morph-state': 'morphing' });
            const band = svgEl('path', { class: 'atlas-border-band', fill: 'none', 'data-age': 'newest', 'data-morph-state': 'morphing' });
            const line = svgEl('path', { class: 'atlas-border-line', fill: 'none', 'data-age': 'newest', 'data-morph-state': 'morphing' });
            this._washMorphGroup.appendChild(wash);
            this._strokeMorphGroup.appendChild(band);
            this._strokeMorphGroup.appendChild(line);
            group.washes.push(wash);
            group.bands.push(band);
            group.lines.push(line);
        }
        while (group.washes.length > rings.length) {
            group.washes.pop().remove();
            group.bands.pop().remove();
            group.lines.pop().remove();
        }

        rings.forEach((ring, i) => {
            const d = this._ringPathData(ring);
            group.washes[i].style.fill = fillColor;
            group.washes[i].setAttribute('d', d);
            group.bands[i].style.stroke = fillColor;
            group.bands[i].setAttribute('d', d);
            group.lines[i].style.stroke = lineColor;
            group.lines[i].setAttribute('d', d);
        });
    },

    _removeMorphGroup(polityId, group) {
        for (const el of [...group.washes, ...group.bands, ...group.lines]) {
            el.remove();
        }
        this._morphGroups.delete(polityId);
    },

    // Batch R requirement 1: sets the land-mask clip geometry ONCE (called
    // by World.razor's own PushLandMaskIfReady the moment BOTH the fetch and
    // this map instance are ready, then never again for the life of the
    // instance -- the mask is static, unlike `setPolities`' own per-window
    // reload). `rings` is the flat `[[[lat,lon],...],...]` array
    // `GET /api/land-mask` returns. Rebuilds `_clipPath`'s own child <path>
    // elements from scratch (cheap -- this runs once) and wires
    // `_washGroup`'s own `clip-path` to reference it: an SVG `<clipPath>`
    // with multiple children UNIONS their filled areas (the spec's own
    // documented behavior), so the mask's own several independent regions
    // (see land-mask.toml's own header comment) need no special handling
    // here beyond "one <path> per ring". A `_redraw()` at the end positions
    // every ring immediately, same as `setPolities` already does for its
    // own rings -- this can run before OR after the first `setPolities`
    // call (whichever of the two async fetches resolves first), and either
    // order produces the same final, fully-clipped result.
    setLandMask(rings) {
        this._landMaskRings = rings || [];
        while (this._clipPath.firstChild) {
            this._clipPath.removeChild(this._clipPath.firstChild);
        }
        this._landMaskPaths = this._landMaskRings.map(() => {
            const p = svgEl('path', {});
            this._clipPath.appendChild(p);
            return p;
        });
        this._washGroup.style.clipPath = 'url(#atlas-land-clip)';
        this._redraw();
    },

    setVisible(visible) {
        this._svg.style.display = visible ? '' : 'none';
        this._washSvg.style.display = visible ? '' : 'none'; // fix round 1: second SVG, same visibility toggle (scripture mode must hide the wash too, not just the line/band SVG)
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
    // happen here, every time. Fix round 1: both SVGs now (this._washSvg
    // too -- see onAdd's own comment on why it needs the identical
    // treatment as this._svg).
    _redraw() {
        if (!this._map) {
            return;
        }
        resetZoomAnimTransform(this._svg);
        resetZoomAnimTransform(this._washSvg);
        for (const g of this._ringGroups) {
            const d = this._ringPathData(g.ring);
            g.wash.setAttribute('d', d);
            g.band.setAttribute('d', d);
            g.line.setAttribute('d', d);
            if (g.hit) {
                g.hit.setAttribute('d', d); // Batch M requirement 4: the delta hit-stroke shares its own ring-group's geometry exactly
            }
        }
        // Batch M requirement 3a: an animated zoom/pan mid-drag (unusual,
        // but not impossible) must keep the MORPH paths in sync too, same
        // "recompute every ring's real d at the now-current zoom" reasoning
        // -- re-evaluates at whatever live window/probe the morph last held
        // (_pendingFrom/_pendingTo/_pendingProbeYear), no new geometry
        // decision, just a reprojection of the same shapes.
        if (this._morphing && this._pendingFrom != null && this._pendingTo != null && this._pendingProbeYear != null) {
            this._evaluateMorphFrame(this._pendingFrom, this._pendingTo, this._pendingProbeYear);
        }
        // Batch R requirement 1: the land-mask clip geometry lives in the
        // SAME `_washSvg` as the wash paths (see onAdd's own comment), so it
        // needs the identical "recompute every ring's real `d` at the
        // now-current zoom" treatment every _redraw -- `_ringPathData` is
        // the exact same helper `setPolities`' own rings already use above,
        // reused verbatim (both are plain `[[lat,lon],...]` ring arrays).
        this._landMaskRings.forEach((ring, i) => {
            if (this._landMaskPaths[i]) {
                this._landMaskPaths[i].setAttribute('d', this._ringPathData(ring));
            }
        });
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

    // Fix round 1 (M2): `tagIndex`/`tagCount` come from setPolities' own
    // per-polity grouping just above -- `tagIndex` is this ring's own
    // position (0-based) among its polity's OWN currently-visible tag-
    // needing rings, `tagCount` how many there are in total. Anchored at a
    // point ALONG the ring's own perimeter (_ringPerimeterPoint below),
    // fanned out by `tagIndex/tagCount`, rather than the ring's bbox
    // center every tag used before this fix -- see that function's own
    // comment for why this is enough to separate concentric rings without
    // any new collision-detection machinery.
    _makeYearTag(ringGroup, tagIndex, tagCount) {
        const frac = tagCount > 1 ? tagIndex / tagCount : 0;
        const [cLat, cLon] = this._ringPerimeterPoint(ringGroup.ring, frac);
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

    // Fix round 1 (M2 -- review finding B3, "stagger same-polity tags at
    // different perimeter fractions of their ring" per the fix round's own
    // brief). Picks the ring's OWN vertex at fractional position `frac`
    // (0 = the ring's first curated point, cycling through its point list)
    // rather than computing a true arc-length fraction -- deliberately
    // array-index based, "no fancy mathematics": every curated ring already
    // carries 30-80 roughly-evenly-spaced hand-placed points (the brief's
    // own recognizability band), so indexing by point-COUNT fraction is a
    // good enough stand-in for genuine arc-length fraction at this app's
    // own scale, and it's a single array lookup, not a geometry
    // computation. Concentric rings of one polity (the common, showcased
    // case -- growth/contraction eras) are independently-authored shapes,
    // so a DIFFERENT ring at the SAME fraction generally lands at a
    // meaningfully different point (verified empirically against the real
    // curated data via the fix round's own re-screenshot of the
    // spanning-1600-900bc window -- see the fix round report); this is a
    // cheap, deterministic improvement over the old shared-bbox-center
    // anchor, not a guarantee for every conceivable ring pair.
    _ringPerimeterPoint(ring, frac) {
        if (!ring || ring.length === 0) {
            return [0, 0];
        }
        const idx = Math.floor(frac * ring.length) % ring.length;
        return ring[idx];
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

    // Year tags sit at their own anchor point (fix round 1: a fractional
    // perimeter point per _ringPerimeterPoint/_makeYearTag above, no longer
    // the ring's bbox center), nudged down a fixed amount so a tag never
    // sits exactly on top of whatever it's anchored near. No offscreen/
    // too-small gating (unlike labels) -- a year tag is small, always
    // wanted whenever its ring is (tierCount>1), and "no fancy math" favors
    // staying visible over a second geometric judgment call the brief never
    // asked for.
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
