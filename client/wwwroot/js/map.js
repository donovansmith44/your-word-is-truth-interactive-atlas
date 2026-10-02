const TILE_URL = 'https://server.arcgisonline.com/ArcGIS/rest/services/World_Shaded_Relief/MapServer/tile/{z}/{y}/{x}';
const TILE_FALLBACK = 'https://basemaps.cartocdn.com/light_nolabels/{z}/{x}/{y}.png';
const TILE_ATTRIBUTION = 'Copyright:(c) 2014 Esri';
const TILE_MAX_NATIVE_ZOOM = 13;

const DEFAULT_CENTER = [31.5, 35.0];
const DEFAULT_ZOOM = 5;

const BIBLICAL_WORLD_BOUNDS = [[7.6, -10.9], [48.9, 71.4]];

const SVG_NS = 'http://www.w3.org/2000/svg';

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

    map.setMinZoom(map.getBoundsZoom(BIBLICAL_WORLD_BOUNDS, false));

    const tiles = L.tileLayer(TILE_URL, {
        maxNativeZoom: TILE_MAX_NATIVE_ZOOM,
        maxZoom: TILE_MAX_NATIVE_ZOOM,
        attribution: TILE_ATTRIBUTION,
    }).addTo(map);

    let felBack = false;
    tiles.on('tileerror', () => {
        if (felBack) {
            return;
        }
        felBack = true;
        tiles.setUrl(TILE_FALLBACK);
    });

    let polities = null;
    let arrows = null;
    if (!mini) {
        map.createPane('polityLabelsPane');
        map.getPane('polityLabelsPane').style.zIndex = 450;
        map.getPane('polityLabelsPane').style.pointerEvents = 'none';

        // mix-blend-mode has to live on this pane element (app.css's .atlas-wash-pane), not on
        // individual wash paths, for the blend to actually reach the tiles -- see BorderLayer's header.
        map.createPane('washPane');
        map.getPane('washPane').style.zIndex = 300;
        map.getPane('washPane').style.pointerEvents = 'none';
        map.getPane('washPane').classList.add('atlas-wash-pane');

        polities = new BorderLayer(dotnetRef);
        polities.addTo(map);

        arrows = new ArrowLayer(dotnetRef);
        arrows.addTo(map);

        map.createPane('landmarksPane');
        map.getPane('landmarksPane').style.zIndex = 500;
        map.getPane('landmarksPane').style.pointerEvents = 'none';

        // Below Leaflet's default markerPane (600) so a quiet dot can never paint over an ember,
        // since Leaflet's per-marker z-index is by screen position within a pane, not across panes.
        map.createPane('quietPane');
        map.getPane('quietPane').style.zIndex = 550;
    }

    instances.set(id, {
        map, dotnetRef, mini, markers: new Map(), arrows, polities, landmarkMarkers: [], litByName: new Map(),
        quietMarkers: new Map(),
        window: null,
        clusterMarkers: [], clusteredIds: new Set(),
    });

    if (!mini) {
        map.on('zoomend', () => applyLabelTier(instances.get(id)));
    }

    map.on('zoomend', () => applyMarkerClusters(instances.get(id)));

    map.on('zoomend', () => applyMarkerNudges(instances.get(id)));

    if (!mini) {
        map.on('click', e => {
            const polityId = polities.polityAt(e.latlng);
            if (polityId) {
                dotnetRef.invokeMethodAsync('OnPolityClick', polityId);
            } else {
                dotnetRef.invokeMethodAsync('OnMapClick');
            }
        });
        instances.get(id).escapeCleanup = watchEscape(dotnetRef);

        map.on('moveend zoomend', () => {
            const c = map.getCenter();
            dotnetRef.invokeMethodAsync('OnCameraChanged', c.lat, c.lng, map.getZoom());
        });
    }

    return id;
}

function watchEscape(dotnetRef) {
    const handler = e => {
        if (e.key === 'Escape') {
            dotnetRef.invokeMethodAsync('OnEscapePressed');
        }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
}

// Two places can be genuinely distinct but a few km (or, after server-side merge decisions,
// occasionally the exact same lat/lon) apart, and this app's marker hit box is deliberately
// tiny -- so any place landing within NUDGE_TRIGGER_PX of an already-placed one gets nudged,
// keeping both independently hoverable without visibly moving away from their real location.
export function setScene(id, sceneJson) {
    const inst = instances.get(id);
    if (!inst) {
        return;
    }

    const scene = typeof sceneJson === 'string' ? JSON.parse(sceneJson) : (sceneJson || {});
    const places = scene.places || [];
    inst.window = scene.window || null;
    const seen = new Set();

    for (const p of places) {
        seen.add(p.id);
        const prior = inst.markers.get(p.id);

        if (!prior) {
            const marker = L.marker([p.lat, p.lon], { icon: makeIcon(p) });
            wireEvents(inst, marker, p.id);
            marker.addTo(inst.map);
            inst.markers.set(p.id, {
                marker, node: p.node.id, lat: p.lat, lon: p.lon, trueLat: p.lat, trueLon: p.lon,
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

    if (!inst.mini) {
        const quietPlaces = scene.quiet_places || [];
        const quietSeen = new Set();
        for (const p of quietPlaces) {
            quietSeen.add(p.id);
            const prior = inst.quietMarkers.get(p.id);

            if (!prior) {
                const marker = L.marker([p.lat, p.lon], { icon: makeQuietIcon(p), pane: 'quietPane' });
                wireQuietEvents(inst, marker, p.id);
                marker.addTo(inst.map);
                inst.quietMarkers.set(p.id, {
                    marker, node: p.node.id, lat: p.lat, lon: p.lon, displayName: p.display_name,
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

    applyMarkerClusters(inst);

    applyMarkerNudges(inst);

    inst.litByName = buildLitByName(inst.markers);
    if (!inst.mini) {
        inst.polities.setLitPlaces(inst.litByName);
    }

    applyLabelTier(inst);

    if (!inst.mini) {
        inst.arrows.setArrows(scene.arrows || [], inst.markers);
    }

    if (inst.emphasisSite) {
        applySiteEmphasis(inst);
    }
}

const GOLDEN_ANGLE_RAD = 2.399963229728653;

// NUDGE_TRIGGER_PX matches the ember marker's own ~26px hit box (two markers count as
// colliding once their hit circles overlap at all). NUDGE_STEP_PX (~16px, one fixed step
// regardless of how many neighbors collide) stays within a ~20px sanity ceiling; if two
// places are too close for 20px to separate them at the current zoom, they are allowed to
// still overlap -- full disambiguation in a dense cluster is applyMarkerClusters' job instead.
const NUDGE_TRIGGER_PX = 26;
const NUDGE_STEP_PX = 16;

const HIT_RADIUS_PX = NUDGE_TRIGGER_PX;

// Radius within which the nearest candidate's tie is treated as a genuine ambiguity (a chooser)
// rather than resolved directly -- tight enough to catch literal coincidences (identical
// lat/lon) without catching real, merely-nearby distinct places.
const AMBIGUITY_RADIUS_PX = 3;

// Screen-pixel radius within which lit markers (far/mid label tier only) collapse into one
// cluster glyph instead of each fighting for a nudge slot.
const CLUSTER_D_PX = 18;

function applyMarkerNudges(inst) {
    if (!inst) {
        return;
    }
    const map = inst.map;

    const entries = [...inst.markers.entries()].sort((a, b) => (a[0] < b[0] ? -1 : 1));

    const placedTrue = [];
    for (const [, entry] of entries) {
        const truePt = map.latLngToContainerPoint([entry.trueLat, entry.trueLon]);

        let dx = 0, dy = 0, n = 0;
        for (const q of placedTrue) {
            const ddx = truePt.x - q.x, ddy = truePt.y - q.y;
            const dist = Math.sqrt(ddx * ddx + ddy * ddy);
            if (dist < NUDGE_TRIGGER_PX) {
                n++;
                if (dist > 0.01) {
                    dx += ddx / dist;
                    dy += ddy / dist;
                }
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

function applyMarkerClusters(inst) {
    if (!inst) {
        return;
    }
    const map = inst.map;

    for (const c of inst.clusterMarkers) {
        map.removeLayer(c.marker);
    }
    inst.clusterMarkers = [];
    inst.clusteredIds = new Set();

    if (!inst.mini && labelTier(map.getZoom()) !== 'near') {
        const entries = [...inst.markers.entries()].sort((a, b) => (a[0] < b[0] ? -1 : 1));
        const points = entries.map(([id, entry]) => ({ id, pt: map.latLngToContainerPoint([entry.trueLat, entry.trueLon]) }));

        const parent = new Map(points.map(p => [p.id, p.id]));
        const find = id => {
            while (parent.get(id) !== id) {
                parent.set(id, parent.get(parent.get(id)));
                id = parent.get(id);
            }
            return id;
        };
        const union = (a, b) => {
            const ra = find(a), rb = find(b);
            if (ra !== rb) {
                const [lo, hi] = ra < rb ? [ra, rb] : [rb, ra];
                parent.set(hi, lo);
            }
        };
        for (let i = 0; i < points.length; i++) {
            for (let j = i + 1; j < points.length; j++) {
                const dist = Math.hypot(points[i].pt.x - points[j].pt.x, points[i].pt.y - points[j].pt.y);
                if (dist <= CLUSTER_D_PX) {
                    union(points[i].id, points[j].id);
                }
            }
        }

        const groups = new Map();
        for (const p of points) {
            const root = find(p.id);
            const list = groups.get(root);
            if (list) {
                list.push(p);
            } else {
                groups.set(root, [p]);
            }
        }

        for (const members of groups.values()) {
            if (members.length < 2) {
                continue;
            }
            members.sort((a, b) => (a.id < b.id ? -1 : 1));
            const cx = members.reduce((s, m) => s + m.pt.x, 0) / members.length;
            const cy = members.reduce((s, m) => s + m.pt.y, 0) / members.length;
            const center = map.containerPointToLatLng({ x: cx, y: cy });
            const memberIds = members.map(m => m.id);
            const marker = L.marker(center, { icon: makeClusterIcon(memberIds.length) });
            wireClusterEvents(inst, marker, memberIds, center);
            marker.addTo(map);
            inst.clusterMarkers.push({ marker, memberIds, lat: center.lat, lon: center.lng });
            for (const id of memberIds) {
                inst.clusteredIds.add(id);
            }
        }
    }

    for (const [id, entry] of inst.markers) {
        const el = entry.marker.getElement();
        if (el) {
            el.style.display = inst.clusteredIds.has(id) ? 'none' : '';
        }
    }
}

function makeClusterIcon(count) {
    const html = `<div class="atlas-cluster" data-testid="marker-cluster-${esc(count)}"><span class="atlas-cluster-count">${esc(count)}</span></div>`;
    return L.divIcon({ html, className: 'atlas-marker-icon', iconSize: [0, 0] });
}

// A cluster glyph shares wireQuietEvents' own QUIET_HOVER_INTENT_MS dwell debounce, so a fast
// pointer transit grazing it en route to somewhere else (e.g. toward a card's own button)
// never swaps the open card out from under the click mid-transit.
function wireClusterEvents(inst, marker, memberIds, latlng) {
    let timer = null;
    let committed = false;
    marker.on('mouseover', e => {
        const pt = e.containerPoint;
        clearTimeout(timer);
        timer = setTimeout(() => {
            committed = true;
            inst.dotnetRef.invokeMethodAsync('OnPlaceHoverAmbiguous', memberIds, pt.x, pt.y);
        }, QUIET_HOVER_INTENT_MS);
    });
    marker.on('mouseout', () => {
        clearTimeout(timer);
        if (committed) {
            committed = false;
            inst.dotnetRef.invokeMethodAsync('OnPlaceLeave');
        }
    });
    marker.on('click', e => {
        L.DomEvent.stopPropagation(e);
        const map = inst.map;
        if (map.getZoom() >= map.getMaxZoom()) {
            inst.dotnetRef.invokeMethodAsync('OnPlaceHoverAmbiguous', memberIds, e.containerPoint.x, e.containerPoint.y);
            return;
        }
        map.setZoomAround(latlng, map.getZoom() + 1);
    });
}

// Every place-marker/quiet-marker/cluster-glyph mouseover or click funnels through this,
// given the pointer's real containerPoint, rather than trusting which DOM element the
// browser routed the raw event to. Returns:
//   null                     -- nothing within HIT_RADIUS_PX of the point
//   { type: 'single', id }   -- one clear winner, resolve directly
//   { type: 'chooser', ids } -- a cluster glyph's own members, or 2+ individual candidates
//                               genuinely coincident at this zoom (AMBIGUITY_RADIUS_PX)
// `ids` is always ascending-sorted by place id.
function collectHoverCandidates(inst) {
    const list = [];
    for (const [id, entry] of inst.markers) {
        if (inst.clusteredIds.has(id)) {
            continue;
        }
        list.push({ id, kind: 'place', lat: entry.lat, lon: entry.lon, trueLat: entry.trueLat, trueLon: entry.trueLon });
    }
    for (const [id, entry] of inst.quietMarkers) {
        list.push({ id, kind: 'quiet', lat: entry.lat, lon: entry.lon, trueLat: entry.lat, trueLon: entry.lon });
    }
    for (const c of inst.clusterMarkers) {
        list.push({ id: null, kind: 'cluster', lat: c.lat, lon: c.lon, trueLat: c.lat, trueLon: c.lon, memberIds: c.memberIds });
    }
    return list;
}

// `fallbackId` (optional): the place id of whichever DOM element actually raised the
// underlying native event, used only when no candidate's true position is within
// HIT_RADIUS_PX of `pt` -- a marker's rendered hit-circle (nudge + own hit-radius) can
// reach a pixel just past HIT_RADIUS_PX of its own true center, so falling back to
// whichever element genuinely raised the event is still a legitimate, unambiguous answer.
function resolveHoverTarget(inst, pt, fallbackId) {
    const map = inst.map;
    const candidates = collectHoverCandidates(inst)
        .map(c => {
            const p = map.latLngToContainerPoint([c.lat, c.lon]);
            const t = map.latLngToContainerPoint([c.trueLat, c.trueLon]);
            return Object.assign(c, { sx: p.x, sy: p.y, tx: t.x, ty: t.y, dist: Math.hypot(p.x - pt.x, p.y - pt.y) });
        });
    const routed = fallbackId ? candidates.find(c => c.id === fallbackId) : undefined;
    const within = candidates.filter(c => c.dist <= HIT_RADIUS_PX);

    if (!routed && within.length === 0) {
        return fallbackId ? { type: 'single', id: fallbackId } : null;
    }

    // Nearest wins; a tied distance breaks first by kind (a lit place or cluster always
    // wins over a quiet dot -- some quiet furniture shares an exact lat/lon with a lit
    // neighbor, and id-only sorting could otherwise hand the hover to the wrong one), then by id.
    const KIND_RANK = { place: 0, cluster: 0, quiet: 1 };
    within.sort((a, b) => a.dist - b.dist || KIND_RANK[a.kind] - KIND_RANK[b.kind] || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
    const winner = routed ?? within[0];

    if (winner.kind === 'cluster') {
        return { type: 'chooser', ids: [...winner.memberIds].sort() };
    }

    // A tie only ever forms among candidates of the same kind as the winner -- a lit place
    // always wins outright over any quiet neighbor regardless of distance.
    const tied = candidates.filter(c =>
        c !== winner && c.kind === winner.kind &&
        Math.hypot(c.tx - winner.tx, c.ty - winner.ty) <= AMBIGUITY_RADIUS_PX);
    if (tied.length > 0) {
        return { type: 'chooser', ids: [winner.id, ...tied.map(c => c.id)].sort() };
    }

    return { type: 'single', id: winner.id };
}

// This suite also fires events programmatically (element.dispatchEvent(new Event('click')))
// to target one specific element unambiguously; such an event's clientX/clientY default to
// 0, which can genuinely land close to some unrelated candidate near the map's own screen
// origin -- so a synthetic event must skip position-based arbitration entirely rather than
// merely fall back on an empty result. Event.isTrusted is the DOM's own signal for this.
function isTrustedPointerEvent(e) {
    return !!(e.originalEvent && e.originalEvent.isTrusted);
}

function dispatchResolvedHover(inst, pt, trusted, fallbackId) {
    if (!trusted) {
        if (fallbackId) {
            inst.dotnetRef.invokeMethodAsync('OnPlaceHover', fallbackId, pt.x, pt.y);
        }
        return;
    }
    const resolved = resolveHoverTarget(inst, pt, fallbackId);
    if (!resolved) {
        return;
    }
    if (resolved.type === 'chooser') {
        inst.dotnetRef.invokeMethodAsync('OnPlaceHoverAmbiguous', resolved.ids, pt.x, pt.y);
    } else {
        inst.dotnetRef.invokeMethodAsync('OnPlaceHover', resolved.id, pt.x, pt.y);
    }
}

function dispatchResolvedClick(inst, e, fallbackId) {
    L.DomEvent.stopPropagation(e);

    if (!isTrustedPointerEvent(e)) {
        if (isToggleSelectClick(e)) {
            inst.dotnetRef.invokeMethodAsync('OnPlaceToggleSelect', fallbackId);
        } else if (fallbackId) {
            inst.dotnetRef.invokeMethodAsync('OnPlaceClick', fallbackId, e.containerPoint.x, e.containerPoint.y);
        }
        return;
    }

    const resolved = resolveHoverTarget(inst, e.containerPoint, fallbackId);
    if (isToggleSelectClick(e)) {
        const id = resolved ? (resolved.type === 'chooser' ? resolved.ids[0] : resolved.id) : fallbackId;
        inst.dotnetRef.invokeMethodAsync('OnPlaceToggleSelect', id);
        return;
    }
    if (!resolved) {
        return;
    }
    if (resolved.type === 'chooser') {
        inst.dotnetRef.invokeMethodAsync('OnPlaceHoverAmbiguous', resolved.ids, e.containerPoint.x, e.containerPoint.y);
        return;
    }
    inst.dotnetRef.invokeMethodAsync('OnPlaceClick', resolved.id, e.containerPoint.x, e.containerPoint.y);
}

function isToggleSelectClick(e) {
    return !!(e.originalEvent && (e.originalEvent.ctrlKey || e.originalEvent.metaKey));
}

function wireEvents(inst, marker, placeId) {
    marker.on('mouseover', e => dispatchResolvedHover(inst, e.containerPoint, isTrustedPointerEvent(e), placeId));
    marker.on('mouseout', () => inst.dotnetRef.invokeMethodAsync('OnPlaceLeave'));
    marker.on('click', e => dispatchResolvedClick(inst, e, placeId));
}

// A quiet marker's mouseover only actually adopts the hover (fires OnPlaceHover) after the
// pointer has dwelled on it for QUIET_HOVER_INTENT_MS: with many small quiet dots scattered
// across a dense scene, an ordinary straight-line pointer transit toward some other target
// can graze one along the way, and an immediate-fire hover would silently swap the open
// card out from under a click mid-transit. Lit markers are unaffected -- this function is
// never called for them.
const QUIET_HOVER_INTENT_MS = 150;

function wireQuietEvents(inst, marker, placeId) {
    let timer = null;
    let committed = false;
    marker.on('mouseover', e => {
        const pt = e.containerPoint;
        const trusted = isTrustedPointerEvent(e);
        clearTimeout(timer);
        timer = setTimeout(() => {
            committed = true;
            dispatchResolvedHover(inst, pt, trusted, placeId);
        }, QUIET_HOVER_INTENT_MS);
    });
    marker.on('mouseout', () => {
        clearTimeout(timer);
        if (committed) {
            committed = false;
            inst.dotnetRef.invokeMethodAsync('OnPlaceLeave');
        }
    });
    marker.on('click', e => dispatchResolvedClick(inst, e, placeId));
}

function makeIcon(p) {
    const brightness = Math.min(5, Math.max(1, p.brightness | 0 || 1));
    const html = `<div class="atlas-marker glow-${brightness}" data-testid="marker-${esc(p.id)}"><span class="atlas-label">${esc(p.name)}</span></div>`;
    return L.divIcon({ html, className: 'atlas-marker-icon', iconSize: [0, 0] });
}

function makeQuietIcon(p) {
    const html = `<div class="quiet-marker" data-testid="quiet-marker-${esc(p.id)}"><span class="quiet-label">${esc(p.display_name)}</span></div>`;
    return L.divIcon({ html, className: 'atlas-marker-icon', iconSize: [0, 0] });
}

// Label density tiers, landmark/polity only -- place labels (lit and quiet) compete for a
// grid cell at every zoom, unconditionally; only collision damping ever hides one.
//   FAR  (< ZOOM_TIER_MID):  polity names + seas (landmarks kind=water) only.
//   MID  (< ZOOM_TIER_NEAR): + "major" landmarks (water + mountain).
//   NEAR (>= ZOOM_TIER_NEAR): + "dimmer" landmarks (region) too.
// Markers themselves are never tiered -- only labels.
const ZOOM_TIER_MID = 6;
const ZOOM_TIER_NEAR = 9;

// Two orthogonal passes keep dense labels legible, re-run every zoomend/setScene/setLandmarks:
//
// DEDUPE -- a landmark whose name matches a currently-lit place's name and sits within
// LANDMARK_DEDUPE_KM of it (not exact coordinate equality -- independently curated datasets
// aren't guaranteed byte-identical lat/lon) describes the same real-world feature twice; the
// place always wins while it's lit, regardless of whether its own label is tier-visible right
// now. Compared against the place's TRUE position, never its rendered/nudged one, since the
// anti-overlap nudge is cosmetic and must never leak into an identity/dedupe decision.
//
// COLLISION DAMPING -- every label surviving its own tier+dedupe check is bucketed into a
// COLLISION_CELL_PX grid keyed by its on-screen anchor point; only the highest-priority label
// in a contested cell survives, others are hidden (not removed -- they reclaim their cell once
// it's no longer contested). Priority: any place label (brighter beats dimmer) > landmark by
// kind (water > mountain > region) > quiet-dot label (always lowest, never contests a place or
// landmark, only wins an empty cell). Polity labels have their own separate declutter mechanism
// and are deliberately outside this pass.
const LANDMARK_DEDUPE_KM = 5;
const COLLISION_CELL_PX = 72;
const PLACE_PRIORITY_BASE = 1000;
const LANDMARK_KIND_PRIORITY = { water: 2, mountain: 1, region: 0 };
const QUIET_LABEL_PRIORITY = -1;

// Mirrors atlas-core::history's existence_gates_label exactly (inclusive both ends, an
// absent bound never gates on that side). Only ever hides a label, never the marker/dot
// itself, which stays for availability regardless of window.
function existenceGatesLabel(existenceFrom, existenceTo, window) {
    if (!window) {
        return false;
    }
    if (existenceFrom != null && window.to.value < existenceFrom.value) {
        return true;
    }
    if (existenceTo != null && window.from.value > existenceTo.value) {
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

// Every currently-lit place's name/position, keyed by slugify(name), shared by this file's own
// landmark dedupe and BorderLayer's polity dedupe -- one source of truth instead of two copies.
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

function applyLabelTier(inst) {
    if (!inst || inst.mini) {
        return;
    }

    const tier = labelTier(inst.map.getZoom());
    const litByName = inst.litByName;
    const candidates = [];

    for (const [id, entry] of inst.markers) {
        if (inst.clusteredIds.has(id)) {
            continue;
        }
        const el = entry.marker.getElement();
        const label = el && el.querySelector('.atlas-label');
        if (!label) {
            continue;
        }
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
        const tierShow = tier === 'near' || lm.kind === 'water' || (tier === 'mid' && lm.kind === 'mountain') || lm.size != null;
        if (!tierShow || isDedupedByLitPlace(lm, litByName)) {
            el.style.display = 'none';
            continue;
        }
        const pt = inst.map.latLngToContainerPoint([lm.lat, lm.lon]);
        candidates.push({ el, x: pt.x, y: pt.y, priority: LANDMARK_KIND_PRIORITY[lm.kind] ?? 0 });
    }

    for (const entry of inst.quietMarkers.values()) {
        const el = entry.marker.getElement();
        const label = el && el.querySelector('.quiet-label');
        if (!label) {
            continue;
        }
        if (existenceGatesLabel(entry.existenceFrom, entry.existenceTo, inst.window)) {
            label.style.display = 'none';
            continue;
        }
        const pt = inst.map.latLngToContainerPoint([entry.lat, entry.lon]);
        candidates.push({ el: label, x: pt.x, y: pt.y, priority: QUIET_LABEL_PRIORITY });
    }

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

function approxKm(lat1, lon1, lat2, lon2) {
    const dLat = (lat1 - lat2) * 111.32;
    const dLon = (lon1 - lon2) * 111.32 * Math.cos((lat1 + lat2) / 2 * Math.PI / 180);
    return Math.sqrt(dLat * dLat + dLon * dLon);
}

function isDedupedByLitPlace(landmark, litByName) {
    const list = litByName.get(slugify(landmark.name));
    if (!list) {
        return false;
    }
    return list.some(p => approxKm(landmark.lat, landmark.lon, p.trueLat, p.trueLon) <= LANDMARK_DEDUPE_KM);
}

function esc(value) {
    return String(value).replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

// Mirrors atlas-etl's geo::kebab -- independently reimplemented here since map.js has no
// access to that Rust code, and both must agree on the CONTRACT's landmark-{slug} testid.
function slugify(name) {
    return String(name).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
}

// Hashes coordinates (not the name) to pick one of 4 diagonal offset quadrants, so
// near-duplicate names in one small area (e.g. Mount Sinai/Mount Horeb) still decorrelate
// rather than all offsetting the same fixed direction and overlapping.
function labelDirection(lat, lon) {
    const dirs = ['ne', 'nw', 'se', 'sw'];
    const h = Math.abs(Math.round(lat * 9973 + lon * 7919));
    return dirs[h % dirs.length];
}

function makeLandmarkIcon(l) {
    const dir = labelDirection(l.lat, l.lon);
    const sizeAttr = l.size ? ` data-size="${esc(l.size)}"` : '';
    const html = `<span class="landmark-label" data-kind="${esc(l.kind)}" data-dir="${dir}"${sizeAttr} data-testid="landmark-${esc(slugify(l.name))}">${esc(l.name)}</span>`;
    return L.divIcon({ html, className: 'landmark-label-icon', iconSize: [0, 0] });
}

export function fitScene(id) {
    const inst = instances.get(id);
    if (!inst || inst.markers.size === 0) {
        return;
    }

    const latlngs = [...inst.markers.values()].map(entry => [entry.lat, entry.lon]);
    inst.map.fitBounds(L.latLngBounds(latlngs), { padding: [48, 48], maxZoom: 8, animate: false });
    if (inst.emphasisAt) {
        inst.map.setView(inst.emphasisAt, inst.map.getZoom(), { animate: false });
    }
}

// animate: false -- an animated pan updates the map's internal center progressively, so
// reading latLngToContainerPoint immediately after would risk an intermediate, not final, point.
export function panToPlace(id, lat, lon) {
    const inst = instances.get(id);
    if (!inst) {
        return { x: 0, y: 0 };
    }

    inst.map.setView([lat, lon], inst.map.getZoom(), { animate: false });
    const pt = inst.map.latLngToContainerPoint([lat, lon]);
    return { x: pt.x, y: pt.y };
}

export function setEmphasis(id, emphasis) {
    const inst = instances.get(id);
    if (!inst) {
        return;
    }

    const next = emphasis || {};
    const site = next.site ?? null;
    const siteChanged = site !== inst.emphasisSite;
    inst.emphasisSite = site;
    inst.emphasisAt = site === null ? null : [next.lat, next.lon];
    if (siteChanged && site !== null) {
        inst.map.setView(inst.emphasisAt, inst.map.getZoom(), { animate: false });
    }
    applySiteEmphasis(inst);
    if (inst.polities) {
        inst.polities.setEmphasis(next.polity ?? null);
    }
}

function markerOfNode(inst, node) {
    for (const entries of [inst.markers, inst.quietMarkers]) {
        for (const entry of entries.values()) {
            if (entry.node === node) {
                return entry;
            }
        }
    }
    return null;
}

function applySiteEmphasis(inst) {
    for (const ring of inst.map.getContainer().querySelectorAll('.atlas-emphasis-ring')) {
        ring.remove();
    }
    if (inst.emphasisSite === null) {
        return;
    }

    const entry = markerOfNode(inst, inst.emphasisSite);
    if (!entry) {
        return;
    }

    const el = entry.marker.getElement();
    if (el) {
        const ring = document.createElement('span');
        ring.className = 'atlas-emphasis-ring';
        ring.setAttribute('data-testid', 'world-emphasis-site');
        el.firstElementChild.appendChild(ring);
    }
}

export function debugClickMap(id, lat, lon) {
    const inst = instances.get(id);
    if (!inst) {
        return false;
    }
    inst.map.fire('click', { latlng: L.latLng(lat, lon) });
    return true;
}

export function debugRecordSink(id) {
    const inst = instances.get(id);
    if (!inst) {
        return false;
    }
    inst.sinkCalls = [];
    const original = inst.dotnetRef.invokeMethodAsync.bind(inst.dotnetRef);
    inst.dotnetRef.invokeMethodAsync = (method, ...args) => {
        inst.sinkCalls.push([method, ...args]);
        return original(method, ...args);
    };
    return true;
}

export function debugSinkCalls(id) {
    const inst = instances.get(id);
    return inst && inst.sinkCalls ? inst.sinkCalls : [];
}

export function getCamera(id) {
    const inst = instances.get(id);
    if (!inst) {
        return null;
    }

    const c = inst.map.getCenter();
    return { lat: c.lat, lng: c.lng, zoom: inst.map.getZoom() };
}

export function setCamera(id, lat, lng, zoom) {
    const inst = instances.get(id);
    if (!inst) {
        return;
    }

    inst.map.setView([lat, lng], zoom, { animate: false });
}

export function debugLiveInstanceIds() {
    return [...instances.keys()];
}

export function debugIsPointOnLand(id, lat, lon) {
    const inst = instances.get(id);
    if (!inst || !inst.polities || !inst.polities._landMaskPaths || inst.polities._landMaskPaths.length === 0) {
        return null;
    }

    const pt = inst.map.latLngToLayerPoint([lat, lon]);
    const domPoint = new DOMPoint(pt.x, pt.y);
    return inst.polities._landMaskPaths.some(p => p.isPointInFill(domPoint));
}

export function debugTrueScreenPoint(id, lat, lon) {
    const inst = instances.get(id);
    if (!inst) {
        return null;
    }
    const pt = inst.map.latLngToContainerPoint([lat, lon]);
    return { x: pt.x, y: pt.y };
}

// Also recenters the map at the target's true lat/lon rather than a bare setZoom (which
// zooms around the current center): confirmed live that a bare setZoom drifts a target
// increasingly far off-screen, zoom step after zoom step, when the target isn't near center.
export function debugSetZoom(id, zoom, lat, lon) {
    const inst = instances.get(id);
    if (!inst) {
        return false;
    }
    if (lat != null && lon != null) {
        inst.map.setView([lat, lon], zoom);
    } else {
        inst.map.setZoom(zoom);
    }
    return true;
}

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

export function debugMorphingPolityIds(id) {
    const inst = instances.get(id);
    if (!inst || !inst.polities || !inst.polities._morphGroups) {
        return null;
    }
    return [...inst.polities._morphGroups.keys()];
}

export function setIsolate(id, narrativeId) {
    const inst = instances.get(id);
    if (!inst || !inst.arrows) {
        return;
    }

    inst.arrows.setIsolate(narrativeId ?? null);
}

export function setPolities(id, politiesJson, from, to) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    const list = typeof politiesJson === 'string' ? JSON.parse(politiesJson) : (politiesJson || []);
    inst.polities.setPolities(list, from, to);
}

// The full, unfiltered roster (every era of every polity), fetched once -- every era
// necessarily intersects the full atlas span, so this is what the morph engine's own
// `lookup` needs to answer an arbitrary drag-time window with zero network latency.
export function setPolitiesRoster(id, politiesJson) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    const list = typeof politiesJson === 'string' ? JSON.parse(politiesJson) : (politiesJson || []);
    inst.polities.setPolitiesRoster(list);
}

export function beginMorph(id) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    inst.polities.beginMorph();
}

export function morphFrame(id, from, to, atYear) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    inst.polities.requestMorphFrame(from, to, atYear);
}

export function settleMorph(id, from, to) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    inst.polities.settleMorph(from, to);
}

export function setLandMask(id, landMaskJson) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    const rings = typeof landMaskJson === 'string' ? JSON.parse(landMaskJson) : (landMaskJson || []);
    inst.polities.setLandMask(rings);
}

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

export function setNarrativeFocus(activeNarrativeIds, currentEventIds) {
    for (const inst of instances.values()) {
        if (inst.mini || !inst.arrows) {
            continue;
        }
        inst.arrows.setFocus(activeNarrativeIds || [], currentEventIds || []);
    }
}

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

export function setPolitiesVisible(id, visible) {
    const inst = instances.get(id);
    if (!inst || !inst.polities) {
        return;
    }

    inst.polities.setVisible(!!visible);
}

export function setLandmarks(id, landmarksJson) {
    const inst = instances.get(id);
    if (!inst || inst.mini) {
        return;
    }

    const landmarks = typeof landmarksJson === 'string' ? JSON.parse(landmarksJson) : (landmarksJson || []);
    for (const { marker } of inst.landmarkMarkers) {
        inst.map.removeLayer(marker);
    }
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

    // document.addEventListener is independent of the Leaflet map instance, so map.remove()
    // below does not tear this down on its own -- it must be cleaned up explicitly.
    inst.escapeCleanup?.();

    inst.map.remove();
    instances.delete(id);
}

// ArrowLayer/BorderLayer each own a bare <svg> in overlayPane rather than going through
// L.Renderer, so they don't get Leaflet's built-in 'zoomanim' CSS-transform sync for free
// during an animated zoom transition (during which neither would otherwise repaint a single
// frame, since Leaflet only fires 'zoomend' at the very end). attachZoomAnim mirrors
// L.Renderer's own _onAnimZoom/_updateTransform recipe to give them the same treatment.
function attachZoomAnim(map, svgEl) {
    function onAnimZoom(e) {
        const scale = map.getZoomScale(e.zoom, map.getZoom());
        const offset = map._latLngToNewLayerPoint(map.getBounds().getNorthWest(), e.zoom, e.center);
        L.DomUtil.setTransform(svgEl, offset, scale);
    }
    map.on('zoomanim', onAnimZoom);
    return () => map.off('zoomanim', onAnimZoom);
}

function resetZoomAnimTransform(svgEl) {
    L.DomUtil.setTransform(svgEl, L.point(0, 0), 1);
}

const ArrowLayer = L.Layer.extend({
    initialize(dotnetRef) {
        this._dotnetRef = dotnetRef;
        this._paths = new Map();
        this._markerIds = new Set();
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

    getEvents() {
        return { zoomend: this._redraw, moveend: this._redraw };
    },

    // `placesById` is map.js's own `inst.markers` (placeId -> {lat, lon,
    // ...}), reused as-is rather than rebuilding a second parallel lookup:
    // setScene above always finishes diffing markers before calling this,
    // so every place an arrow in `arrows` can reference (ARROW-1: arrows
    // only ever reference lit places) is already present in it.
    setArrows(arrows, placesById) {
        // Reset here so a fresh scene starts unisolated/unfocused even for an arrow whose
        // key survives the window change (the else branch below never touches these
        // attributes on an existing entry, so a stale isolate/focus state would otherwise
        // persist with no legend/popover control still pointing at it).
        for (const entry of this._paths.values()) {
            entry.path.setAttribute('data-faded', 'false');
            entry.casing.setAttribute('data-faded', 'false');
        }

        for (const entry of this._paths.values()) {
            entry.path.removeAttribute('data-narrative-focus');
            entry.casing.removeAttribute('data-narrative-focus');
        }

        this._placesById = placesById;
        const list = arrows || [];

        // parallelIndex spreads arrows whose curves would otherwise land in the same crowded
        // neighborhood (0, +1, -1, +2, ...), grouped by a coarse rounded midpoint rather than
        // an exact from/to place match, so two different narratives' arrows that happen to
        // sit in the same real-world corner still separate, not just a repeated A-B/B-A pair.
        const CLUSTER_GRID_DEG = 0.05;
        const clusterKey = a => {
            const from = placesById.get(a.from_place);
            const to = placesById.get(a.to_place);
            if (!from || !to) {
                return [a.from_place, a.to_place].slice().sort().join('|');
            }
            const midLat = (from.lat + to.lat) / 2;
            const midLon = (from.lon + to.lon) / 2;
            return `${Math.round(midLat / CLUSTER_GRID_DEG)},${Math.round(midLon / CLUSTER_GRID_DEG)}`;
        };
        const clusterTotal = new Map();
        for (const a of list) {
            const key = clusterKey(a);
            clusterTotal.set(key, (clusterTotal.get(key) ?? 0) + 1);
        }
        const clusterSeen = new Map();
        const parallelIndexByKey = new Map();
        for (const a of list) {
            const key = clusterKey(a);
            const k = clusterSeen.get(key) ?? 0;
            clusterSeen.set(key, k + 1);
            // Every member of a multi-member cluster gets a nonzero push (1, -1, 2, -2, ...),
            // not just the second-and-later ones: a quadratic bezier's bbox always includes
            // its own endpoints, so an unboosted member's tiny bbox stays inside a boosted
            // neighbor's larger one regardless of how far that neighbor bows.
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

    setIsolate(narrativeId) {
        for (const entry of this._paths.values()) {
            const faded = narrativeId != null && entry.arrow.narrative !== narrativeId;
            entry.path.setAttribute('data-faded', faded ? 'true' : 'false');
            entry.casing.setAttribute('data-faded', faded ? 'true' : 'false');
        }
    },

    // A separate mechanism from setIsolate above: that is a sticky user toggle, this is a
    // transient popover-driven emphasis, and the two must coexist without overwriting each
    // other's attribute -- hence data-narrative-focus, never reusing data-faded.
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
            e.stopPropagation();
            const pt = this._map.mouseEventToContainerPoint(e);
            this._dotnetRef.invokeMethodAsync('OnArrowClick', arrowKey(a), pt.x, pt.y);
        });
        this._svg.appendChild(path);
        return { path, casing };
    },

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
            return;
        }

        const f = this._map.latLngToLayerPoint([from.lat, from.lon]);
        const t = this._map.latLngToLayerPoint([to.lat, to.lon]);
        const dx = t.x - f.x;
        const dy = t.y - f.y;
        const dist = Math.hypot(dx, dy) || 1;
        const nx = -dy / dist;
        const ny = dx / dist;
        // MIN_BOW_PX: two geographically-close places can put `dist` down in single-digit
        // pixels, making the proportional-only bow negligible -- a curve that compact puts
        // both endpoint markers inside its own hover bounding box, stealing the arrow's hover
        // from directly on top of a marker. This floor guarantees the curve bulges out past
        // it regardless of how close the endpoints are.
        // PARALLEL_STEP_PX: large enough that a crowded cluster member's bbox center clears
        // every other member's bbox entirely (a quadratic bezier's bbox always includes its
        // own endpoints), not merely grow its own tail.
        const MIN_BOW_PX = 30;
        const PARALLEL_STEP_PX = 40;
        const mag = Math.max(0.18 * dist, MIN_BOW_PX) + PARALLEL_STEP_PX * entry.parallelIndex;
        const cx = (f.x + t.x) / 2 + nx * mag;
        const cy = (f.y + t.y) / 2 + ny * mag;

        const d = `M${f.x},${f.y} Q${cx},${cy} ${t.x},${t.y}`;
        entry.path.setAttribute('d', d);
        entry.casing.setAttribute('d', d);

        // Mutating an existing element's style (never re-creating/re-inserting it) can never
        // retrigger the one-shot draw-in animation, so this stays gap-free after a redraw
        // changes the path's actual pixel length.
        const len = entry.path.getTotalLength();
        entry.path.style.strokeDasharray = String(len);
        entry.path.style.setProperty('--arrow-length', String(len));
        entry.casing.style.strokeDasharray = String(len);
        entry.casing.style.setProperty('--arrow-length', String(len));
    },
});

// BorderLayer manages two plain <svg> elements (see the washPane note below for why two, not
// one), added to the map before ArrowLayer so DOM paint order puts borders below narrative
// threads. Non-interactive; a polity swap replaces the layer's content wholesale rather than
// diffing, since it's a rare, debounced window-change event, not a hot per-frame path.
//
// mix-blend-mode on a path inside overlayPane never reaches the tiles: Leaflet's vendored
// leaflet.css gives every pane position:absolute plus an explicit z-index, which makes each
// pane its own CSS stacking context -- so a blend-mode set on a shape inside one pane can only
// ever blend against other content in that SAME pane, never a sibling pane's tiles. The fix is
// a dedicated washPane (z-index 300, between tilePane's 200 and overlayPane's 400) holding only
// the wash <path>s in their own SVG, with mix-blend-mode:multiply set on the PANE element
// itself (app.css's .atlas-wash-pane) -- the browser flattens the pane's children into one
// layer first, then blends that whole layer against whatever painted before it in the shared
// parent stacking context, which is the actual tile imagery. The pane is the blending unit,
// not any one shape inside it.
//
// Every ring gets three stacked <path> elements, sharing one projected `d`, appended in
// oldest-to-newest-per-polity order across the whole SVG (not per-ring): a growing polity's
// later, larger ring paints last/on top; a shrinking polity's later, smaller ring still paints
// last, sitting visibly inside the earlier, larger, fainter one -- either way growth or
// contraction reads correctly without ever comparing two rings' geometry. Within a polity
// with more than one currently-visible era, the oldest is dotted and lightest, any era
// strictly between oldest and newest is dashed at an intermediate opacity, and each of its own
// rings gets a small "c. {year}" tag (rings, not eras, since one era can carry more than one
// disjoint ring).
//
// Entries 0-7 of POLITY_TINTS/POLITY_TINTS_DARK are the original 8 tints, byte-identical --
// any polity whose id already hashed into 0-7 must keep exactly the color it always had.
// Entries 8-15 were added later, chosen by greedy hue dispersion so every one is as far as
// achievable from its nearest neighbor. The two arrays' indices must stay aligned 1:1 (same
// color_key indexes both) -- POLITY_TINTS_DARK is each hue scaled by a flat 0.62.
const POLITY_TINTS = [
    '#C98A8A', '#C9B37E', '#93A98B', '#7E99B5',
    '#A18CB0', '#B59B7E', '#8FA07A', '#C08E7A',
    '#78B09B', '#B078A0', '#7B78B0', '#78B082',
    '#78ACB0', '#ACB078', '#AC78B0', '#B0788C',
];

const POLITY_TINTS_DARK = [
    '#7D5656', '#7D6F4E', '#5B6956', '#4E5F70',
    '#64576D', '#70604E', '#59634C', '#77584C',
    '#4A6D60', '#6D4A63', '#4C4A6D', '#4A6D51',
    '#4A6B6D', '#6B6D4A', '#6B4A6D', '#6D4A57',
];

function formatYearTag(year) {
    return `c. ${year.label}`;
}

// A narrow window can show only one internal era of a longer-lived polity, whose `to`
// boundary is a transition into the next era (not shown in this window), never a real fall --
// this roster-wide check avoids offering fall-explorability on every merely-newest-in-window ring.
function isChronologicallyFinalEra(roster, entry) {
    if (!roster || roster.length === 0) {
        return entry.fall !== undefined;
    }
    let maxTo = -Infinity;
    for (const e of roster) {
        if (e.id === entry.id) {
            maxTo = Math.max(maxTo, e.to);
        }
    }
    return entry.to === maxTo;
}

// The previous era's own `to` year is a more precise "when did the change happen" for a
// transition's title than the new era's `from`, which sits one year later by this app's
// adjacent-year convention.
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

// A ring can be the explorable target for at most one delta: fall takes priority on the rare
// occasion both this era's start and end boundaries sit inside one (necessarily narrow)
// window simultaneously, as the more climactic moment for a polity that's ending.
function deltaForEntry(roster, entry, from, to) {
    const startInWindow = entry.from >= from && entry.from <= to;
    const endInWindow = entry.to >= from && entry.to <= to;

    if (endInWindow && isChronologicallyFinalEra(roster, entry)) {
        return { kind: 'fall', delta: entry.fall, titleFrom: entry.reign.from, titleTo: entry.reign.to };
    }
    if (startInWindow) {
        const prev = previousEra(roster, entry);
        const titleFrom = prev ? prev.reign.to : entry.reign.from;
        return { kind: 'transition', delta: entry.transition, titleFrom, titleTo: entry.reign.to };
    }
    return null;
}

function deltaAriaLabel(entry, found) {
    const base = found.kind === 'fall' ? `${entry.name}, fall` : `${entry.name}, transition`;
    return found.delta && found.delta.event ? `${base}: ${found.delta.event}` : `${base} (explore)`;
}

// prefers-reduced-motion: rounds a probe year to whichever bracketing knot it's nearer, so
// animate() returns an un-interpolated line's own rings rather than a smoothed intermediate.
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

function ringContains(ring, lat, lon) {
    let inside = false;
    for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
        const [latI, lonI] = ring[i];
        const [latJ, lonJ] = ring[j];
        if ((lonI > lon) !== (lonJ > lon) && lat < ((latJ - latI) * (lon - lonI)) / (lonJ - lonI) + latI) {
            inside = !inside;
        }
    }
    return inside;
}

const BorderLayer = L.Layer.extend({
    initialize(dotnetRef) {
        this._dotnetRef = dotnetRef;
        this._entries = [];
        this._ringGroups = [];
        this._labels = [];
        this._yearTags = [];
        this._emphasis = null;
        // Defaulted to empty (not left undefined) so a redraw that happens to run before the
        // first setLitPlaces call degrades to "nothing to dedupe against" rather than a
        // null-reference.
        this._litByName = new Map();

        // `_roster` is the full, unfiltered polities roster (setPolitiesRoster below) --
        // what `lookup` needs to answer an arbitrary drag-time window without a network round
        // trip. `_morphing`/`_morphGroups` exist only while a drag gesture is in progress.
        this._roster = [];
        this._morphing = false;
        this._morphGroups = new Map();
        // rAF-throttling state: coalesces however many drag-move updates arrive between two
        // frames into a single evaluate+paint. `_pendingFrom`/`_pendingTo` are the full live
        // window as of the latest update (both handles' current values, not just the dragged
        // one), also re-read by `_redraw` so an animated zoom/pan mid-drag reprojects correctly.
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
        this._washSvg = svgEl('svg', { class: 'atlas-borders-wash' });
        map.getPane('washPane').appendChild(this._washSvg);

        // Sibling of _washGroup, never a child of it, so setPolities' wholesale wipe-and-
        // repaint never touches this static geometry. Filter (feather blur) is applied
        // per-path in app.css, while clip-path lives on _washGroup itself: CSS Filter
        // Effects applies filter before clip-path, so a blurred wash edge is still cut off
        // exactly at the coastline.
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

        // Settled and morph sub-groups: both wash sub-groups live inside _washGroup so both
        // inherit its clip-path and track zoomanim identically. O(1) visibility toggling
        // (display:none on one whole group) is what makes beginMorph/settleMorph cheap.
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

        this._labelPane = map.getPane('polityLabelsPane');
        this._offZoomAnim = attachZoomAnim(map, this._svg);
        // The wash SVG lives in a separate pane, so it needs its own independent zoomanim
        // transform or it would visibly lag/detach from its own ring's band+line mid-zoom.
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

    // Routes through lookup+overlay (border-morph.js) -- the same two functions the morph
    // path (settleMorph below) also calls, so the settled display and a released-drag's
    // instant local settle can never disagree about what "this window's own lines, styled" means.
    setPolities(list, from, to) {
        const grouped = lookup(list, from ?? -Infinity, to ?? Infinity);
        const entries = [];
        for (const lines of grouped.values()) {
            entries.push(...overlay(lines));
        }
        this._paintSettled(entries, from, to);
    },

    setPolitiesRoster(roster) {
        this._roster = roster || [];
    },

    setEmphasis(polityId) {
        if (polityId === this._emphasis) {
            return;
        }
        this._emphasis = polityId;
        if (!this._morphing && this._entries.length > 0) {
            this._paintSettled(this._entries, this._windowFrom, this._windowTo);
        }
    },

    _emphasisedEntries() {
        const ofPolity = this._entries.filter(entry => entry.node.id === this._emphasis);
        const drawn = ofPolity.filter(entry => this._windowTo === undefined || (entry.from <= this._windowTo && entry.to >= this._windowTo));
        return new Set(drawn.length > 0 ? drawn : ofPolity);
    },

    polityAt(latlng) {
        if (this._morphing || this._svg.style.display === 'none') {
            return null;
        }
        for (let i = this._ringGroups.length - 1; i >= 0; i--) {
            if (ringContains(this._ringGroups[i].ring, latlng.lat, latlng.lng)) {
                return this._ringGroups[i].entry.id;
            }
        }
        return null;
    },

    // The settled (overlay-combinator) paint, shared by setPolities (network-fed) and
    // settleMorph (locally computed). Paint order alone (oldest-to-newest per polity) is
    // what makes growth/contraction legible with no geometry comparison.
    _paintSettled(entries, from, to) {
        resetZoomAnimTransform(this._svg);
        resetZoomAnimTransform(this._washSvg);
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

        this._ringGroups = [];
        const washEls = [], bandEls = [], lineEls = [], hitEls = [], outlineEls = [], raisedWashEls = [];
        const emphasised = this._emphasisedEntries();
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
                (emphasised.has(entry) ? raisedWashEls : washEls).push(wash);
                bandEls.push(band);
                lineEls.push(line);
                const g = { wash, band, line, ring, entry, ringIndex };
                if (emphasised.has(entry)) {
                    g.outline = svgEl('path', {
                        class: 'atlas-border-emphasis',
                        fill: 'none',
                        'data-testid': 'world-emphasis-territory',
                    });
                    outlineEls.push(g.outline);
                }
                this._ringGroups.push(g);

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
        // Bands then lines then hit-strokes -- hit-strokes paint last/topmost so their own
        // wide invisible stroke always wins hit-testing over a same-ring line/band.
        for (const el of [...washEls, ...raisedWashEls]) {
            this._washSettledGroup.appendChild(el);
        }
        for (const el of bandEls) {
            this._strokeSettledGroup.appendChild(el);
        }
        for (const el of lineEls) {
            this._strokeSettledGroup.appendChild(el);
        }
        for (const el of outlineEls) {
            this._strokeSettledGroup.appendChild(el);
        }
        for (const el of hitEls) {
            this._strokeSettledGroup.appendChild(el);
        }

        const seenNameKeys = new Set();
        this._labels = [];
        for (const entry of this._entries) {
            const key = `${entry.id} ${entry.name}`;
            if (seenNameKeys.has(key)) {
                continue;
            }
            seenNameKeys.add(key);
            this._labels.push(this._makeLabel(entry));
        }

        // One year tag per ring (not per era), only for a polity with more than one
        // currently-visible era, fanned across each polity's own tag-needing rings so
        // concentric growth/contraction rings don't all anchor at the same bbox center.
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

    // The wide, transparent hit-stroke a delta-eligible ring gets, painted after its own
    // line/band. Every field the popover needs is already resolved client-side from the
    // roster -- no second fetch or server-side lookup.
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
            e.stopPropagation();
            const d = found.delta;
            this._dotnetRef.invokeMethodAsync(
                'OnPolityDeltaClick',
                g.entry.id,
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

    // Swaps which sub-groups are visible (O(1)) rather than touching any individual ring
    // element -- the settled display's paths stay intact underneath, ready to reappear the
    // instant settleMorph runs on release. Idempotent.
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
            l.el.style.display = 'none';
        }
        for (const t of this._yearTags) {
            t.el.style.display = 'none';
        }
    },

    // Every drag-move update records the latest probe year; the actual evaluate-and-paint
    // work is scheduled at most once per animation frame, coalescing however many pointer
    // events arrived since the last one.
    requestMorphFrame(from, to, atYear) {
        this._pendingFrom = from;
        this._pendingTo = to;
        this._pendingProbeYear = atYear;
        if (this._rafHandle != null) {
            return;
        }
        this._rafHandle = requestAnimationFrame(() => {
            this._rafHandle = null;
            if (this._morphing) {
                this._evaluateMorphFrame(this._pendingFrom, this._pendingTo, this._pendingProbeYear);
            }
        });
    },

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
        for (const [polityId, group] of this._morphGroups) {
            if (!seen.has(polityId)) {
                this._removeMorphGroup(polityId, group);
            }
        }
    },

    // Settles instantly and locally from the already-loaded roster (zero network wait) --
    // World.razor's separate network fetch still lands a little later and repaints from the
    // server's answer, byte-identical by construction, so this is a pure UX win with no risk.
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

    // An SVG clipPath with multiple children unions their filled areas, so the mask's several
    // independent regions need no special handling beyond one <path> per ring. Can run before
    // or after the first setPolities call -- either order produces the same clipped result.
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
        this._washSvg.style.display = visible ? '' : 'none';
        for (const l of this._labels) {
            l.el.style.display = visible ? '' : 'none';
        }
        for (const t of this._yearTags) {
            t.el.style.display = visible ? '' : 'none';
        }
    },

    // Triggers an immediate redraw: unlike a mere zoom/pan, a window change can flip dedupe
    // state with no zoomend/moveend of its own to ride along on.
    setLitPlaces(litByName) {
        this._litByName = litByName;
        this._redraw();
    },

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
            if (g.outline) {
                g.outline.setAttribute('d', d);
            }
            if (g.hit) {
                g.hit.setAttribute('d', d);
            }
        }
        if (this._morphing && this._pendingFrom != null && this._pendingTo != null && this._pendingProbeYear != null) {
            this._evaluateMorphFrame(this._pendingFrom, this._pendingTo, this._pendingProbeYear);
        }
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

    // `ring` is deliberately [lat, lon], not GeoJSON's [lon, lat] -- matching
    // atlas_core::data::PolityEra's own convention, so no destructure-and-flip is needed here.
    _ringPathData(ring) {
        if (!ring || ring.length === 0) {
            return '';
        }
        const pts = ring.map(([lat, lon]) => this._map.latLngToLayerPoint([lat, lon]));
        return `M${pts.map(p => `${p.x},${p.y}`).join('L')}Z`;
    },

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

    _makeYearTag(ringGroup, tagIndex, tagCount) {
        const frac = tagCount > 1 ? tagIndex / tagCount : 0;
        const [cLat, cLon] = this._ringPerimeterPoint(ringGroup.ring, frac);
        const el = document.createElement('span');
        el.className = 'polity-year-tag';
        el.setAttribute(
            'data-testid',
            `polity-year-tag-${esc(ringGroup.entry.id)}-${ringGroup.entry.from}-${ringGroup.ringIndex}`
        );
        el.textContent = formatYearTag(ringGroup.entry.reign.from);
        this._labelPane.appendChild(el);
        return { el, ringGroup, cLat, cLon };
    },

    // Picks the ring's own vertex at fractional position `frac` (array-index based, not a
    // true arc-length fraction) -- every curated ring already carries enough roughly-evenly-
    // spaced points that this is a good enough stand-in, and cheap.
    _ringPerimeterPoint(ring, frac) {
        if (!ring || ring.length === 0) {
            return [0, 0];
        }
        const idx = Math.floor(frac * ring.length) % ring.length;
        return ring[idx];
    },

    // A polity label whose name matches a currently-lit place's name AND sits within
    // COLLISION_CELL_PX of it is describing the same real-world thing twice -- the place
    // always wins. Deliberately narrow: a different-named polity sitting near a city is
    // legitimate period cartography, not a duplicate, and must never be suppressed by proximity.
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
