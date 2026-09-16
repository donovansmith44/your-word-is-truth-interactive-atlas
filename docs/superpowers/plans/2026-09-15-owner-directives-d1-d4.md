# Owner Directives D1–D4 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the owner's five 2026-09-15 directives: (D1) one scripture selector while following; (D3) the Small Catechism and Book of Concord become genuinely explorable with a symmetric verse ↔ catechism ↔ Concord traversal and a real Concord containment tree; (D4) table-of-contents navigation as an expandable tree derived from the graph's containers; (D2) any two of {reader, world} side by side, including the same view twice, with a follow toggle where following makes sense; (D5) person cards that show life, events and family instead of every verse.

**Architecture:** Every affordance is a queried edge (project law), so D3 is mostly *traversal* of edges that already exist (`catechism-link` is already symmetric in the graph) plus one structural fix in the compiler (Concord documents contain articles as `Container` rows, mirroring Bible books ⊃ chapters). D4 reads that containment forest through a new `/api/contents/{corpus}` endpoint and renders it with one generic tree component. D1 is a guard plus a source-scan law. D2 replaces five hardcoded host→guest pairings with a generic pairing over the view registry and introduces pane-scoped state so the same view can appear twice.

**Tech Stack:** Rust (axum, graph-types port), Blazor WebAssembly (C#/Razor), xunit, Playwright + fast-check (`tests/ux`), Gherkin/Reqnroll contract suites (`contracts/`).

**Spec:** No standalone spec — the owner ruled these are to be implemented directly ("you know how to do them"). The binding authorities are the owner's verbatim directives (memory `owner-queue-2026-09-15.md`, reproduced in Global Constraints), the graph design spec `docs/superpowers/specs/2026-08-22-explorable-graph-design.md` (affordance honesty, containers), the frontend contract spec `docs/superpowers/specs/2026-08-26-frontend-backend-contract-design.md` (§4 views/composition, §5 hatches), and the code survey at `.superpowers/sdd/2026-09-15-owner-directives-survey.md` (read it — every file:line below comes from it).

## Global Constraints

- **Owner's words (binding):** D1 "if we're following scripture on the map, there is not an additional place to select scripture (the box at the top right) on the map side of the screen" — a LAW, tested. D2 "window any two things side by side: multiple maps; multiple readers … (corpus x corpus) cartesian product, where we can toggle whether they are 'following' each other or not where it makes sense. for now we'll only do this for the map and reader." D3 "the book of concord is not fully explorable … bijective mapping across corpora … paragraphs within the BoC are pointlessly clickable … start by making the s.c. explorable … a symmetric edge tying the pieces of scripture from the brain-fuel repository … to the actual small catechism sections … new abstractions that are implementations of our container abstraction to group things in the BoC in an analogous way to how we did the Bible." D4 "contents is a tree. clicking on any given item in the tree expands and shows that nodes children … toggles … stop at the level of article, or topic in the small catechism. pages are not a meaningful way of thinking about things."
- **Project laws:** every UI affordance corresponds to a real edge (no dead clicks); conditional presence (a section/affordance appears only when content exists); `graph-types` is owner-approved, extend-only, zero-dependency; laws stay in the compiler; the 25 `scene_byte_identity` hashes never change; never loosen a gate; the KJV text is never edited.
- **Contract policy (CDC-1, `contracts/CHANGELOG.md`):** additive wire change = PATCH bump of the affected suite with fixtures re-recorded through the recorder (`ATLAS_BLESS_PACT=1 cargo test --test contract_pact` from `server/`); breaking = MINOR. Nothing in this plan is breaking.
- **Test commands:** Rust from `server/`: `cargo test --workspace` then `cargo test -p atlas-graph-types` (and, after CONTENTION-1 lands, `bash ../scripts/timing-gates.sh`). Client: `dotnet test client.Tests` and `dotnet test client.ContractTests` from the repo root — NEVER while the client dev server is running. UX: `cd tests/ux; npx playwright test <spec>` — its harness starts the API on port **8000** and the client on 5000. **On this machine port 8000 is held by an unrelated app (Hyperbeam).** If `curl -s http://localhost:8000/health` does not return `ok` and the port is busy, Playwright cannot run: report exactly which specs were NOT executed rather than working around it; do not kill the other process.
- **Untracked files you must not touch or stage:** `client.Tests/FrontierMatrixConformanceTests.cs`, `client.Tests/FrontierMatrixRustParityTests.cs`, `tests/ux/frontier-matrix.spec.ts`, `publish-apphome/`. They belong to another in-flight batch. If a task must edit the frontier matrix mirror, edit the TRACKED files only and say so in the report.
- **Attribution trailers** at the end of every commit message:
  `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and
  `Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W`.
- Shell: PowerShell for dotnet/cargo per README (`$env:Path = "$env:Path;$env:LOCALAPPDATA\Microsoft\dotnet;$env:USERPROFILE\.cargo\bin"; $env:DOTNET_ROOT = "$env:LOCALAPPDATA\Microsoft\dotnet"`), Git Bash for scripts.
- Testid registry: every new `data-testid` is added to `tests/ux/CONTRACT.md`'s inventory (from line ~880) in the same commit that introduces it.

---

## File Structure

| Path | Responsibility (new or changed) |
|---|---|
| `client/Pages/World.razor` | D1: picker mount guarded by the follow predicate |
| `client/Pages/Kretzmann.razor` | D1: same guard for the kretzmann+reader pair |
| `client.Tests/State/ConformanceTests.cs` | D1: source-scan law — every `ScripturePicker` mount outside `Reader.razor` sits inside a follow guard |
| `tests/ux/split-view.spec.ts`, `tests/ux/kretzmann.spec.ts`, `tests/ux/CONTRACT.md` | D1: behavioural assertions + inventory prose |
| `server/atlas-graph/src/concord_adapter.rs` | D3: document ⊃ article as `ContainerContent::Container` rows |
| `server/atlas-graph/tests/concord_containers_real_data.rs` (NEW) | D3: the Concord containment forest has the Bible's shape |
| `data/compiled/graph.bin` (+ `report.txt`, exports if they move) | D3: regenerated artifact |
| `server/atlas-server/src/graph_handlers.rs` | D3: `TextUnitOut.edge_summary` (additive) |
| `contracts/atlas-query-contract/{VERSION,CHANGELOG.md,fixtures/*}` | D3: PATCH bump, re-recorded text-window fixtures |
| `graph-types/src/frontier.rs` | D3: fill the declared-empty `ConcordUnit × CatechismLink` and `CatechismItem × CatechismLink` cells |
| `client/Explore/CatechismNode.cs`, `client/Explore/ConcordUnitNode.cs`, `client/Explore/PopoverSectionProviders.cs` | D3: real frontiers via `/api/node/{id}/edges?kind=catechism-link`; chips both ways |
| `client/Pages/Concord.razor`, `client/Explore/ConcordToc.cs` | D3: clickability from `edge_summary`; `?ref=` deep link |
| `client/Explore/FrontierMatrix*.cs` (TRACKED mirror files only) | D3: mirror the two filled cells |
| `server/atlas-server/src/contents.rs` (NEW), `app.rs` | D4: `GET /api/contents/{corpus}` from Container/Contains via the port |
| `client/Dtos.cs`, `client/AtlasClient.cs` | D4: `ContentsTreeOut` DTOs + client call |
| `client/Components/ContentsTree.razor` (NEW), `client/Components/ContentsPanel.razor` (NEW) | D4: generic expandable tree; the panel hosting it |
| `client/Pages/Reader.razor`, `client/Pages/Concord.razor`, `client/Layout/MainLayout.razor` | D4: open the panel from the chapter head and the hamburger; Concord sidebar becomes the tree |
| `client.Tests/ContentsTreeModelTests.cs` (NEW), `tests/ux/contents-tree.spec.ts` (NEW) | D4 tests |
| `client/State/PaneScope.cs` (NEW), `client/State/ViewArrangement.cs`, `client/Views/ViewRegistrySetup.cs`, `client/Views/EnterSplitHatch.cs`, `client/Views/SplitUrlContract.cs`, `client/Components/CompositionSplit.razor` | D2: pane-scoped atoms, generic pairing, guest locus in the URL |
| `client.Tests/State/PaneScopeTests.cs` (NEW), `client.Tests/State/ViewRegistryConformanceTests.cs`, `tests/ux/split-pairs.spec.ts` (NEW) | D2 tests |

Order of tasks: D1 (Task 1) → D3 (Tasks 2–4) → D4 (Tasks 5–6) → D2 (Tasks 7–8). D4's Concord tree needs D3's document ⊃ article rows; D2 is the largest and touches the state core, so it goes last.

---

### Task 1: D1 — one scripture selector while following (World and Kretzmann)

**Files:**
- Modify: `client/Pages/World.razor:78–80` (the `<ScripturePicker … />` mount), header comment `:60–77`
- Modify: `client/Pages/Kretzmann.razor:145` (its `<ScripturePicker … />` mount)
- Modify: `client.Tests/State/ConformanceTests.cs` (add one source-scan law)
- Modify: `tests/ux/split-view.spec.ts`, `tests/ux/kretzmann.spec.ts`, `tests/ux/CONTRACT.md:1403–1409` and the `picker` inventory rows

**Interfaces:**
- Consumes: `World.razor`'s existing `_follow` (`:472–473`) and `SplitMode`; `Kretzmann.razor`'s `Following` (`:369`) and `SplitMode`.
- Produces: the law "a `ScripturePicker` is mounted outside `Reader.razor` only inside a `!(SplitMode && <follow>)` guard", which Task 8 must keep when it generalizes pairing.

- [ ] **Step 1: Write the failing source-scan law**

Append to `client.Tests/State/ConformanceTests.cs`, following the file's existing source-scan pattern (it already locates the client source root for `InitialDividerFraction_ReferencedFromExactlyOneProductionSite` — reuse that helper; if the helper is private, call it the same way the existing test does):

```csharp
    /// <summary>
    /// D1 (owner, 2026-09-15): "if we're following scripture on the map, there is
    /// not an additional place to select scripture on the map side of the screen."
    /// The Reader is the canonical locus writer and keeps its picker unconditionally;
    /// every OTHER page that mounts a ScripturePicker must do so inside a guard that
    /// hides it while following. A source scan, because a runtime test can only
    /// prove one page at one moment; the law is about every mount that exists.
    /// </summary>
    [Fact]
    public void OneScriptureSelectorLaw_EveryNonReaderPickerMountIsFollowGuarded()
    {
        var pages = Directory.GetFiles(Path.Combine(ClientRoot, "Pages"), "*.razor");
        var violations = new List<string>();
        foreach (var page in pages)
        {
            if (Path.GetFileName(page) == "Reader.razor") continue;
            var lines = File.ReadAllLines(page);
            for (var i = 0; i < lines.Length; i++)
            {
                if (!lines[i].Contains("<ScripturePicker", StringComparison.Ordinal)) continue;
                // The guard must be an @if whose condition contains `!(SplitMode &&` within
                // the 6 lines above the mount (the mount may span several lines).
                var window = string.Join("\n", lines.Skip(Math.Max(0, i - 6)).Take(6));
                if (!window.Contains("@if", StringComparison.Ordinal) ||
                    !window.Contains("!(SplitMode &&", StringComparison.Ordinal))
                {
                    violations.Add($"{Path.GetFileName(page)}:{i + 1}");
                }
            }
        }
        Assert.True(violations.Count == 0,
            "ScripturePicker mounted without a follow guard at: " + string.Join(", ", violations));
    }
```

If `ClientRoot` is not the name the file uses for the client source directory, use the file's own name for it. Do not add a new helper if one exists.

- [ ] **Step 2: Run it to verify it fails**

```powershell
dotnet test client.Tests --filter "FullyQualifiedName~OneScriptureSelectorLaw" 
```

Expected: FAIL naming `World.razor:78` and `Kretzmann.razor:145` (line numbers approximate).

- [ ] **Step 3: Guard both mounts**

In `client/Pages/World.razor`, wrap the mount (currently at `:78–80`) exactly as the `mode-chip` is guarded at `:132`:

```razor
@* D1 LAW (owner, 2026-09-15): while the atlas FOLLOWS the reader there is ONE
   scripture selector on screen -- the reader's. The picker returns the moment
   follow is released. Same predicate as the mode-chip guard below. *@
@if (!(SplitMode && _follow))
{
    <ScripturePicker OnApply="ApplyScriptureRef" ThemeClass="picker-dusk"
                     CurrentBook="@_locus.Value.Book" CurrentChapter="@_locus.Value.Chapter"
                     SyncToken="@_pickerSyncToken" />
}
```

Keep the existing attributes verbatim (copy them from the file; the list above is illustrative of shape, not authority). Add one sentence to the header comment at `:60–77` noting the D1 guard. Leave `_pickerSyncToken++` at `:982` in place (it still matters for the follow OFF transition, when the picker re-mounts).

In `client/Pages/Kretzmann.razor`, wrap the mount at `:145` the same way using that page's follow predicate: `@if (!(SplitMode && Following))` — check `:369` for the exact identifier and use it.

- [ ] **Step 4: Run the law and the client suite**

```powershell
dotnet test client.Tests
```

Expected: the new law passes; no other test changes (report the total).

- [ ] **Step 5: Behavioural assertions in Playwright**

In `tests/ux/split-view.spec.ts`, next to the existing follow-chip assertions (`:208–227` region), add one test:

```ts
test('D1 law: while following, the atlas pane has no scripture picker; releasing follow restores it', async ({ page }) => {
  await page.goto('/read/JHN/3?split=world&follow=1');
  const atlas = page.locator('.split-pane-guest');
  await expect(atlas.getByTestId('follow-chip')).toBeVisible();
  await expect(atlas.getByTestId('picker')).toHaveCount(0);
  // the reader's own picker is untouched
  await expect(page.locator('.split-view').getByTestId('picker')).toHaveCount(1);
  await atlas.getByTestId('follow-chip').click();
  await expect(atlas.getByTestId('picker')).toBeVisible();
});
```

Adapt the URL and locators to the spec file's existing helpers (it already navigates into split mode at `:54`, `:88` — reuse the same helper rather than a raw `goto` if one exists). Add the mirror test to `tests/ux/kretzmann.spec.ts` for `/kretzmann?split=reader&follow=1` using `kretzmann-follow-chip`.

In `tests/ux/CONTRACT.md`, replace the prose at `:1403–1409` ("World's OWN picker Apply is UNCHANGED… still works with follow off") with: "D1 LAW (2026-09-15): the world and kretzmann pickers are NOT MOUNTED while following (`!(SplitMode && follow)`); they return on follow release. The reader's picker is always mounted. Enforced by `ConformanceTests.OneScriptureSelectorLaw_EveryNonReaderPickerMountIsFollowGuarded` and the two split specs." Update the `picker` inventory rows' "where" column accordingly.

- [ ] **Step 6: Run the two specs (if port 8000 is free)**

```powershell
cd tests/ux; npx playwright test split-view kretzmann
```

Expected: green. If 8000 is busy (see Global Constraints), record "not executed: split-view.spec.ts, kretzmann.spec.ts — port 8000 held by another process" in the report.

- [ ] **Step 7: Commit**

```bash
git add client/Pages/World.razor client/Pages/Kretzmann.razor client.Tests/State/ConformanceTests.cs tests/ux/split-view.spec.ts tests/ux/kretzmann.spec.ts tests/ux/CONTRACT.md
git commit -F - <<'EOF'
feat(client): D1 -- one scripture selector while following, as a law

The world and kretzmann pages mount their ScripturePicker only while NOT
following (!(SplitMode && follow)); the reader's picker is the one
selector on screen in follow mode. A source-scan law in
ConformanceTests fails if any non-Reader page ever mounts a picker
without that guard; the split specs assert the behaviour.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
EOF
```

---

### Task 2: D3a — Concord documents contain articles (compiler + artifact)

**Files:**
- Modify: `server/atlas-graph/src/concord_adapter.rs:150–185` (the container rows; module comment `:26–34`)
- Create: `server/atlas-graph/tests/concord_containers_real_data.rs`
- Regenerate: `data/compiled/graph.bin`, `data/compiled/report.txt` (and `data/exports/*` if the compile rewrites them byte-differently — it should not)

**Interfaces:**
- Consumes: `bible_container_adapter.rs`'s shape as the model — book ⊃ chapter as `ContainerContent::Container(child)` one row per child; chapter ⊃ verses as `ContainerContent::Loci`. Ids `concord-doc-{key}` (`:106`) and `concord-art-{key}-{article}` (`:110`) stay unchanged.
- Produces: for every Concord document container, one `Contains<ConcordTag>` row per article with `content = ContainerContent::Container(article_container_id)`; article rows unchanged (`Loci` of paragraph loci). Task 5's `/api/contents/concord` walks exactly these rows.

- [ ] **Step 1: Write the failing real-data test**

```rust
//! D3 (owner, 2026-09-15): "new abstractions that are implementations of our
//! container abstraction to group things in the BoC in an analogous way to how
//! we did the Bible." The Bible's shape: book ⊃ chapter as Container rows (one
//! per child), chapter ⊃ verses as Loci. This test pins the same shape for the
//! Concord: document ⊃ article as Container rows, article ⊃ paragraphs as Loci.
//! Before this batch the document tier was a FLAT union of paragraph loci
//! (concord_adapter.rs's own module comment disclosed it).

use atlas_graph_types::edge::ContainerContent;
use atlas_graph_types::id::{AnyNodeId, NodeKind};

// Build the real graph the way the other *_real_data tests do -- copy the
// helper the nearest sibling uses (e.g. tests/concord_sc_overlap_real_data.rs)
// rather than inventing a new one.
fn real_graph() -> atlas_graph_types::graph::Graph { /* same helper as the sibling test */ }

#[test]
fn every_concord_document_contains_its_articles_as_container_rows() {
    let g = real_graph();
    let doc_rows: Vec<_> = g.contains_concord.iter()
        .filter(|r| r.container.0.raw.starts_with("concord-doc-"))
        .collect();
    assert!(!doc_rows.is_empty(), "no document-level Contains rows at all");
    for r in &doc_rows {
        match &r.content {
            ContainerContent::Container(child) => assert!(
                child.0.raw.starts_with("concord-art-"),
                "document {} contains a non-article container {}", r.container.0.raw, child.0.raw),
            ContainerContent::Loci(_) => panic!(
                "document {} still has a flat Loci row -- the Bible shape is Container rows only",
                r.container.0.raw),
        }
    }
}

#[test]
fn every_concord_article_container_is_contained_by_exactly_one_document() {
    let g = real_graph();
    let articles: Vec<&AnyNodeId> = g.nodes.keys()
        .filter(|id| id.kind == NodeKind::Container && id.raw.starts_with("concord-art-"))
        .collect();
    assert!(!articles.is_empty());
    for a in articles {
        let parents = g.contains_concord.iter().filter(|r| matches!(&r.content,
            ContainerContent::Container(c) if c.0 == *a)).count();
        assert_eq!(parents, 1, "article {} has {parents} document parents", a.raw);
    }
}

#[test]
fn article_containers_keep_their_paragraph_loci() {
    let g = real_graph();
    let art_rows = g.contains_concord.iter()
        .filter(|r| r.container.0.raw.starts_with("concord-art-"));
    let mut n = 0;
    for r in art_rows {
        n += 1;
        assert!(matches!(r.content, ContainerContent::Loci(_)),
            "article {} must contain paragraph loci", r.container.0.raw);
    }
    assert!(n > 0);
}
```

Use the exact field/constructor names from `graph-types/src/edge.rs:189–210` and the sibling test's graph-building helper; the snippet shows intent and the assertions, not import paths you must copy blindly.

- [ ] **Step 2: Run it to verify it fails**

```powershell
cd server; cargo test -p atlas-graph --test concord_containers_real_data
```

Expected: `every_concord_document_contains_its_articles_as_container_rows` FAILS with "still has a flat Loci row".

- [ ] **Step 3: Mint document ⊃ article Container rows**

In `concord_adapter.rs`, where the document row is pushed (`:177–180`), replace the single flat `Loci` row with one `Container` row per article, in article order, mirroring `bible_container_adapter.rs`'s book ⊃ chapter loop (read that loop first and copy its provenance/justification handling):

```rust
// D3: document ⊃ article as Container rows -- the Bible's book ⊃ chapter shape.
// One row per article, in article order (order is load-bearing: contents trees
// and `member-of` pages read it). The FLAT paragraph-locus union the document
// tier used to carry is gone: paragraphs are reachable through their article,
// exactly as verses are reachable through their chapter.
for article in &doc.articles {
    if is_skipped_article(/* same predicate the article loop uses */) { continue; }
    bundle.contains.push(Contains {
        container: ContainerNodeId(AnyNodeId { kind: NodeKind::Container, raw: doc_container_id(&doc.key) }),
        content: ContainerContent::Container(ContainerNodeId(AnyNodeId {
            kind: NodeKind::Container, raw: article_container_id(&doc.key, article.number) })),
        provenance: /* same provenance id the document row used */,
        justification: /* same as the document row used */,
    });
}
```

Update the module comment at `:26–34` to describe the new two-tier shape and delete the "FLAT LocusSet" disclosure. Do not rename ids.

- [ ] **Step 4: Run the adapter's tests and the law suite**

```powershell
cd server; cargo test -p atlas-graph --test concord_containers_real_data; cargo test -p atlas-graph
```

Expected: the three new tests pass; `law_check::container_containment_is_a_forest` still passes (documents have no parent, articles exactly one); every other `atlas-graph` test passes. If any existing test asserted the flat document row (grep `concord-doc-` in `server/atlas-graph/tests` and `server/atlas-server/tests`), update its expectation to the Container shape and say so in the report.

- [ ] **Step 5: Regenerate the artifact**

```powershell
cd server
cargo run -p atlas-etl
cargo run -p atlas-graph --release --bin atlas-graph-compile -- --data-dir ../data/compiled --out ../data/compiled/graph.bin
git status --short ../data
```

Expected: both ADMISSION passes print; `graph.bin` changes; `report.txt` may change; `data/exports/*.json` should be byte-identical (they derive from nodes, and no node changed) — if they differ, stop and report the diff before committing. Record the new `graph.bin` size against 104,857,600 bytes.

- [ ] **Step 6: Prove nothing observable moved**

```powershell
cd server; cargo test -p atlas-server --test scene_byte_identity; cargo test --workspace
```

Expected: 25 scene hashes unchanged; workspace green. The version root printed at load (`atlas-graph: … graph version …`) is unchanged, because only edge rows changed — state the value in the report.

- [ ] **Step 7: Commit**

```bash
git add server/atlas-graph/src/concord_adapter.rs server/atlas-graph/tests/concord_containers_real_data.rs data/compiled/graph.bin data/compiled/report.txt
git commit -F - <<'EOF'
feat(graph): D3 -- Concord documents contain articles as Container rows

The Book of Concord's document tier was a flat union of paragraph loci;
it is now document ⊃ article Container rows, one per article, mirroring
the Bible's book ⊃ chapter shape. Articles keep their paragraph loci.
The containment forest law holds (documents rootless, articles
single-parent). graph.bin regenerated; 25 scene hashes and the version
root unchanged.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
EOF
```

---

### Task 3: D3b — text-window units carry their edge summary (additive wire change)

**Files:**
- Modify: `server/atlas-server/src/graph_handlers.rs:254–268` (`TextUnitOut`, `TextWindowOut`) and the `text_window` handler that fills them
- Modify: `server/atlas-server/tests/graph_api.rs` (one test)
- Modify: `contracts/atlas-query-contract/VERSION` (0.1.0 → 0.1.1), `contracts/atlas-query-contract/CHANGELOG.md`, re-recorded fixtures under `contracts/atlas-query-contract/fixtures/` and `contracts/pacts/*.json`

**Interfaces:**
- Consumes: `GraphQuery::edge_summary(&Position)` (graph-types `store.rs:28`) and the existing `EdgeSummaryEntryOut { kind, count }` used by `node_card` (`graph_handlers.rs:52–53`).
- Produces: each `TextUnitOut` gains `edge_summary: Vec<EdgeSummaryEntryOut>` (inhabited kinds only, sorted by kind label). Task 4's Concord page decides clickability from it; the Reader ignores it.

- [ ] **Step 1: Write the failing API test**

In `server/atlas-server/tests/graph_api.rs`, next to the existing text-window tests:

```rust
/// D3: a text-window unit says what is on its frontier, so a reader page can
/// make ONLY units with edges clickable (no dead clicks) without an N+1 of
/// node-card calls. Additive field; the reader may ignore it.
#[tokio::test]
async fn text_window_units_carry_their_edge_summary() {
    let app = /* the same app-builder the sibling text-window test uses */;
    let body = get_json(&app, "/api/text?ref=concord/7.1.1&n=3&corpus=concord").await;
    let units = body["units"].as_array().expect("units");
    assert!(!units.is_empty());
    for u in units {
        let es = u["edge_summary"].as_array().expect("edge_summary present on every unit");
        for e in es {
            assert!(e["kind"].is_string() && e["count"].as_u64().unwrap_or(0) > 0,
                "only inhabited kinds are listed");
        }
    }
    // A verse we know has cross-references and a catechism link:
    let jhn = get_json(&app, "/api/text?ref=JHN.3.16&n=1").await;
    let kinds: Vec<&str> = jhn["units"][0]["edge_summary"].as_array().unwrap()
        .iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"cites"), "JHN.3.16 cites others: {kinds:?}");
}
```

Use the helper names the file already has for building the app and fetching JSON; the ref grammar for `/api/text` is in the handler's doc comment — match it.

- [ ] **Step 2: Run it to verify it fails**

```powershell
cd server; cargo test -p atlas-server --test graph_api text_window_units_carry_their_edge_summary
```

Expected: FAIL — `edge_summary present on every unit` (field missing).

- [ ] **Step 3: Add the field**

In `graph_handlers.rs`, add `pub edge_summary: Vec<EdgeSummaryEntryOut>` to `TextUnitOut` (after `words_of_christ`), and in the `text_window` handler, for each unit id, compute `snapshot.edge_summary(&Position::Node(id.clone()))`, map to `EdgeSummaryEntryOut { kind: <the same label function node_card uses>, count }`, sort by `kind`, and fill the field. Reuse the exact conversion `node_card` uses for its `edge_summary` (do not write a second label mapping).

- [ ] **Step 4: Run the test, then the recorder**

```powershell
cd server; cargo test -p atlas-server --test graph_api
$env:ATLAS_BLESS_PACT = "1"; cargo test -p atlas-server --test contract_pact; Remove-Item Env:ATLAS_BLESS_PACT
git status --short ../contracts
```

Expected: `graph_api` green; the recorder rewrites the text-window fixtures under `contracts/atlas-query-contract/fixtures/` and `contracts/pacts/` with the new field. No `.feature` text changes.

- [ ] **Step 5: PATCH bump under the CDC-1 policy**

Set `contracts/atlas-query-contract/VERSION` to `0.1.1`; add a `## 0.1.1` entry to its `CHANGELOG.md`: "additive: text-window units carry `edge_summary` (kind, count) — D3, 2026-09-15; fixtures re-recorded". Run the gate and self-test:

```bash
bash scripts/contract-semver-gate.sh && bash scripts/contract-gate.sh --base HEAD
```

Expected: both exit 0 (additive graded PATCH). If the gate refuses, read its reason; the fix is in what you changed, never in the gate.

- [ ] **Step 6: Client contract tests still pass**

```powershell
dotnet test client.ContractTests
```

Expected: 45/45 (Reqnroll reads the re-recorded fixtures; additive fields are ignored by existing steps).

- [ ] **Step 7: Commit**

```bash
git add server/atlas-server/src/graph_handlers.rs server/atlas-server/tests/graph_api.rs contracts/atlas-query-contract contracts/pacts
git commit -F - <<'EOF'
feat(server): D3 -- text-window units carry their edge summary (AQC 0.1.1, additive)

Each unit in /api/text now lists its inhabited frontier kinds and counts,
so a corpus page can make only units with edges clickable without an
N+1 of node-card calls. Fixtures re-recorded through the one recorder;
PATCH bump under the CDC-1 policy.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
EOF
```

---

### Task 4: D3c — the catechism traversal, both directions, and no dead clicks

**Files:**
- Modify: `graph-types/src/frontier.rs:115, :234, :377, :577–582` (fill two declared-empty cells)
- Modify: `client/Explore/CatechismNode.cs:45–46`, `client/Explore/ConcordUnitNode.cs:44–57`, `client/Explore/PopoverSectionProviders.cs` (two new section providers)
- Modify: `client/Pages/Concord.razor:180–206` (clickability), `:1` route + a `[SupplyParameterFromQuery] string? Ref` deep link; `client/Explore/ConcordToc.cs:90`
- Modify: the TRACKED frontier-matrix mirror in `client/Explore/` (grep `FrontierMatrix` in tracked files; leave the untracked test files alone)
- Modify: `tests/ux/concord.spec.ts`, `tests/ux/CONTRACT.md`; `client.Tests/PopoverSectionRegistryTests.cs`

**Interfaces:**
- Consumes: `/api/node/{id}/edges?kind=catechism-link&limit=…` (`app.rs:167`, `graph_handlers::node_edges`, wire `EdgePageOut { kind, entries: [{ edge, node: { id, kind, label } }], next }`); node id grammar `graph_wire.rs:84,112,154` (`"CatechismItem:{id}"`, `"text-unit:BoC {p}.{a}.{par}"` — read the file for the exact canonical spellings and use them); Task 3's `edge_summary` on text-window units.
- Produces: `CatechismNode.ExploreAsync` → two explorations: **"Scripture (n)"** (targets that are Bible text units, pushing the existing verse/passage node kind the popover already uses for verse targets) and **"In the Book of Concord (n)"** (targets that are Concord text units, pushing `ConcordUnitNode`s); `ConcordUnitNode.ExploreAsync` → **"Small Catechism (n)"** (targets that are `CatechismItem` nodes, pushing `CatechismNode`). A Concord unit renders as clickable iff its `edge_summary` is non-empty. `/concord?ref=7.2.1` scrolls to and highlights that unit.

- [ ] **Step 1: Fill the frontier matrix cells (Rust first, it is the authority)**

In `graph-types/src/frontier.rs`, the matrix declares which edge kinds each focus kind offers. Add `EK::Symmetric(S::CatechismLink)` to the `ConcordUnit` row (`:115` area; the note at `:234` about ConcordUnit and the group at `:577–582` must be updated so the cell is no longer listed as empty) and make sure the `CatechismItem` row offers `CatechismLink` (`:377` already maps `CatechismLink → CatechismItem` for the target side; the FOCUS side for `CatechismItem` must list the kind too). Run:

```powershell
cd server; cargo test -p atlas-graph-types
```

Expected: the matrix's own law tests pass (they assert every declared cell resolves to a served kind). If a law names the cell as deliberately empty, update the law's expectation and cite the owner's directive in the doc comment.

- [ ] **Step 2: Mirror in the tracked C# matrix**

Find the tracked mirror: `git ls-files client | Select-String FrontierMatrix`. Add the same two cells. Run `dotnet test client.Tests --filter FrontierMatrix` — the TRACKED parity tests must pass; if the only parity tests are the untracked files, run them locally but do not stage them, and say so.

- [ ] **Step 3: Write the failing popover-registry test**

In `client.Tests/PopoverSectionRegistryTests.cs`, following its existing pattern for asserting which providers claim a node kind:

```csharp
[Fact]
public void CatechismItem_AndConcordUnit_HaveSymmetricCatechismLinkSections()
{
    var registry = /* the same registry construction the file's other tests use */;
    Assert.Contains(registry.ProvidersFor("CatechismItem"), p => p.Id == "catechism-in-concord");
    Assert.Contains(registry.ProvidersFor("CatechismItem"), p => p.Id == "catechism-scripture-edges");
    Assert.Contains(registry.ProvidersFor("ConcordUnit"), p => p.Id == "concord-small-catechism");
}
```

Use the file's real accessor names (`ProvidersFor`/`Id` are illustrative; match the registry's API).

- [ ] **Step 4: Run it to verify it fails**

```powershell
dotnet test client.Tests --filter CatechismItem_AndConcordUnit_HaveSymmetricCatechismLinkSections
```

Expected: FAIL (no such providers).

- [ ] **Step 5: Implement the traversal**

In `client/Explore/CatechismNode.cs`, replace `ExploreAsync`'s `Array.Empty<Exploration>()` (`:45–46`) with a fetch of `/api/node/CatechismItem:{Id}/edges?kind=catechism-link&limit=200` through `IExplorableClient` (the same client the other nodes use — find how `VerseNode`/`PassageNode` call edges and copy the call shape), then partition entries by target kind:

```csharp
public override async Task<IReadOnlyList<Exploration>> ExploreAsync(IExplorableClient client, CancellationToken ct)
{
    var page = await client.EdgesAsync(NodeId, "catechism-link", cursor: null, limit: 200, ct);
    var scripture = page.Entries.Where(e => e.Node.Id.StartsWith("text-unit:", StringComparison.Ordinal)
                                          && !IsConcord(e.Node.Id)).ToList();
    var concord   = page.Entries.Where(e => IsConcord(e.Node.Id)).ToList();
    var out = new List<Exploration>();
    if (scripture.Count > 0)
        out.Add(new Exploration($"Scripture ({scripture.Count})", () => scripture.Select(e => (ExplorableNode)VerseTargetNode.From(e.Node)).ToList()));
    if (concord.Count > 0)
        out.Add(new Exploration($"In the Book of Concord ({concord.Count})", () => concord.Select(e => (ExplorableNode)ConcordUnitNode.FromNodeId(e.Node.Id, e.Node.Label)).ToList()));
    return out;
}
static bool IsConcord(string id) => id.Contains("concord", StringComparison.OrdinalIgnoreCase);
```

`Exploration`, `ExplorableNode`, `EdgesAsync` and the verse-target node type are whatever `client/Explore/` already defines — use those exact names (read `Exploration.cs`/`IExplorableClient.cs`); the snippet fixes the BEHAVIOUR: two chips, labelled with counts, only when inhabited. Add `ConcordUnitNode.FromNodeId(string nodeId, string label)` parsing `part.article.paragraph` from the wire id (see `Concord.razor:321–330 ParseRef` for the grammar).

In `client/Explore/ConcordUnitNode.cs`, replace the empty `ExploreAsync` (`:44–45`) with the same fetch on the unit's node id, producing one chip **"Small Catechism (n)"** whose targets are `new CatechismNode(id, label)` for entries whose `Node.Kind == "CatechismItem"`. Keep `BodyAsync`.

Register the three section providers in `PopoverSectionProviders.cs` (ids exactly `catechism-scripture-edges`, `catechism-in-concord`, `concord-small-catechism`), obeying POPOVER-LAW-1 (render only when the exploration is inhabited; the existing `CatechismScripturesSection` at `:1061` renders verses from the bespoke item payload — keep it, and make the new `catechism-scripture-edges` section render ONLY when its target set differs from what `CatechismScripturesSection` already shows, otherwise return nothing; state in the report which it was on real data).

- [ ] **Step 6: Clickability from edge_summary, and the deep link**

In `client/Pages/Concord.razor:180–206`: a unit gets `class="concord-unit explorable" role="button" tabindex="0" @onclick=…` iff `unit.EdgeSummary is { Count: > 0 }` (Task 3's field, surfaced through the client's text-window DTO — add `EdgeSummary` to that record in `client/Dtos.cs` as a `List<EdgeSummaryEntry>` with `Kind`/`Count`); otherwise plain markup with NO role/tabindex/handler. Remove `ConcordToc.IsExplorablePart` from the clickability decision (keep the class if other code uses it; the report says what remains of it). Add `[SupplyParameterFromQuery] public string? Ref { get; set; }` and, after the window loads, if `Ref` parses via `ParseRef`, scroll the matching `concord-unit-{slug}` into view and add class `concord-unit-target` (add the CSS rule next to the unit rules in `app.css`). Wire `CatechismNode`'s Concord chip targets so their card offers "Open in the Book of Concord" navigating to `/concord?ref={p}.{a}.{par}` (an escape hatch per §5 of the frontend spec — use the existing hatch component the other cards use for navigation).

- [ ] **Step 7: Run the client suites**

```powershell
dotnet test client.Tests; dotnet test client.ContractTests
```

Expected: green, including Step 3's test and the frontier-matrix parity.

- [ ] **Step 8: Playwright**

Add to `tests/ux/concord.spec.ts`:

```ts
test('D3: a Small Catechism paragraph is clickable only when it has edges, and its card reaches the catechism item', async ({ page }) => {
  await page.goto('/concord?ref=7.1.1');
  const target = page.locator('.concord-unit-target');
  await expect(target).toBeVisible();
  const clickable = page.locator('.concord-unit[role="button"]');
  const inert = page.locator('.concord-unit:not([role="button"])');
  expect(await clickable.count()).toBeGreaterThan(0);
  // every clickable unit opens a card with the Small Catechism section
  await clickable.first().click();
  await expect(page.getByTestId('popover')).toBeVisible();
  await expect(page.getByText(/^Small Catechism \(\d+\)$/)).toBeVisible();
  // and an inert unit has no button semantics at all
  if (await inert.count() > 0) await expect(inert.first()).not.toHaveAttribute('tabindex');
});

test('D3: from Genesis 1:1 the catechism card reaches the Book of Concord and back', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('verse-1').click();           // use the spec file's existing verse-click helper
  await page.getByTestId(/^catechism-item-/).first().click();
  await expect(page.getByText(/^In the Book of Concord \(\d+\)$/)).toBeVisible();
});
```

Adapt testids to CONTRACT.md's inventory (`popover`, `verse-N`, `catechism-item-{id}` — check the exact names). Register any new testid (`concord-unit-target` is a class, not a testid; if you add testids, inventory them). Run `npx playwright test concord` if 8000 is free; otherwise report not-executed.

- [ ] **Step 9: Commit**

```bash
git add graph-types/src/frontier.rs client/Explore client/Pages/Concord.razor client/Dtos.cs client/wwwroot/css/app.css client.Tests/PopoverSectionRegistryTests.cs tests/ux/concord.spec.ts tests/ux/CONTRACT.md
git status --short   # confirm the untracked FrontierMatrix test files are NOT staged
git commit -F - <<'EOF'
feat(explore): D3 -- the catechism traversal both ways, and no dead Concord clicks

CatechismItem cards now explore their symmetric catechism-link edges:
"Scripture (n)" and "In the Book of Concord (n)". Concord paragraphs
explore "Small Catechism (n)" back to the item. A Concord unit is
clickable iff its text-window edge_summary is inhabited; /concord?ref=
deep-links to a paragraph. The frontier matrix fills the two cells it
had declared empty, in Rust and its C# mirror.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
EOF
```

---

### Task 5: D4a — `/api/contents/{corpus}` from the containment forest

**Files:**
- Create: `server/atlas-server/src/contents.rs`
- Modify: `server/atlas-server/src/app.rs:132–168` (one route), `server/atlas-server/src/lib.rs` or `main.rs` (`mod contents;`)
- Modify: `server/atlas-server/tests/graph_api.rs` (two tests)

**Interfaces:**
- Consumes: `GraphQuery::nodes_of_kind` is NOT available yet (DB-3), so roots are found by scanning `GraphService`'s snapshot for `Container` nodes with no `member-of` edge — use `edge_summary` on each Container id from the graph's node map (`GraphService` exposes the graph; read `service.rs` for the accessor the other handlers use). Children: `edges(&Position::Node(container), EdgeQuery { kind: EdgeKind::Directed(RelationId::Contains, Direction::Forward), cursor: None, limit: 400 })`. Depth stop: children that are themselves Containers are listed; their own children are NOT expanded (Bible: book → chapter; Concord: document → article — the owner's "stop at the level of article").
- Produces:

```json
GET /api/contents/bible   |  GET /api/contents/concord
{ "corpus": "bible", "version": "<root>",
  "roots": [ { "id": "Container:bible-book-GEN", "title": "Genesis", "kind": "book", "group": "OT",
               "ref": "GEN.1", "children": [ { "id": "Container:bible-chapter-GEN-1", "title": "1", "kind": "chapter", "ref": "GEN.1", "count": 31 } ] } ] }
```

`kind` ∈ `book|chapter|document|article`; `group` is `OT`/`NT` for Bible books (from `data.canon.books`' testament) and absent for Concord; `ref` is the navigation target (Bible chapter ref `GEN.1`; for a Concord article, the ref of its FIRST paragraph locus, e.g. `7.2.1`); `count` is the number of leaf loci under the child (verses in the chapter; paragraphs in the article). Roots and children in canonical order (canon order for books; `ord` of the Contains rows for the rest). Unknown corpus → 404 with body `{"error":"unknown corpus"}`.

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn contents_bible_is_books_then_chapters_and_stops_there() {
    let app = /* app builder */;
    let body = get_json(&app, "/api/contents/bible").await;
    let roots = body["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 66);
    assert_eq!(roots[0]["title"], "Genesis"); assert_eq!(roots[0]["group"], "OT");
    let gen_ch = roots[0]["children"].as_array().unwrap();
    assert_eq!(gen_ch.len(), 50);
    assert_eq!(gen_ch[0]["ref"], "GEN.1"); assert_eq!(gen_ch[0]["count"], 31);
    assert!(gen_ch[0].get("children").is_none(), "depth stops at chapter");
    assert_eq!(roots[39]["group"], "NT");
}

#[tokio::test]
async fn contents_concord_is_documents_then_articles() {
    let app = /* app builder */;
    let body = get_json(&app, "/api/contents/concord").await;
    let roots = body["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 10, "ten Concord documents");
    for d in roots {
        assert_eq!(d["kind"], "document");
        let arts = d["children"].as_array().unwrap();
        assert!(!arts.is_empty(), "{} has articles", d["title"]);
        for a in arts { assert_eq!(a["kind"], "article"); assert!(a["ref"].as_str().unwrap().contains('.')); }
    }
    let r = get(&app, "/api/contents/nope").await;
    assert_eq!(r.status(), 404);
}
```

- [ ] **Step 2: Run to verify they fail** — `cargo test -p atlas-server --test graph_api contents_` → FAIL (404 for both; route missing).

- [ ] **Step 3: Implement `contents.rs`**

One handler `contents(Path(corpus), State(app))`. Steps: (1) map corpus → container id prefix (`bible-book-` / `concord-doc-`) and child kind names; (2) collect root Container ids from the graph's node map with that prefix, order Bible books by canon order (`data.canon.books` code order), Concord docs by the `DOCUMENTS` order (`ord` of first Contains row is fine); (3) for each root, page `contains` edges; for each child container, page ITS `contains` edges once only to compute `count` (Loci children) and `ref` (first locus, formatted with the same ref formatter `text_window` uses for unit refs); (4) build the DTOs (`ContentsOut`, `ContentsRootOut`, `ContentsChildOut`, `#[derive(Serialize)]`, snake_case like every other wire type). Register `.route("/api/contents/{corpus}", get(contents::contents))` in `app.rs` next to `/api/books`.

- [ ] **Step 4: Run the tests and the workspace** — `cargo test -p atlas-server` green; then `cargo test --workspace` green. This is a NEW route, not in any contract suite yet: add it to `contracts/atlas-query-contract` as an additive scenario ONLY if that suite's `glossary.md` says new read routes must be pinned; otherwise note in the report that the route is unpinned and why (the CDC-1 CHANGELOG policy text is the authority — quote the line you relied on).

- [ ] **Step 5: Commit**

```bash
git add server/atlas-server/src/contents.rs server/atlas-server/src/app.rs server/atlas-server/src/lib.rs server/atlas-server/src/main.rs server/atlas-server/tests/graph_api.rs
git commit -F - <<'EOF'
feat(server): D4 -- GET /api/contents/{corpus}: the containment forest, two levels deep

Books then chapters for the Bible; documents then articles for the
Concord (owner: "stop at the level of article"). Read through the
GraphQuery port from Container nodes and contains edges -- the tree IS
the graph's containment forest, not a hand-maintained list.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
EOF
```

---

### Task 6: D4b — the contents tree in the client

**Files:**
- Create: `client/Components/ContentsTree.razor`, `client/Components/ContentsPanel.razor`, `client/Components/ContentsTreeModel.cs`
- Modify: `client/Dtos.cs` (`ContentsOut`, `ContentsRootOut`, `ContentsChildOut` records), `client/AtlasClient.cs` (`Contents(string corpus)`)
- Modify: `client/Pages/Reader.razor:176–196` (chapter head opens the panel; keep the dwell peek), `client/Layout/MainLayout.razor:64–90` (hamburger "Contents" entry), `client/Pages/Concord.razor:118–135` (sidebar becomes the tree), `client/Explore/ConcordToc.cs:38–50` (hardcoded list retired or reduced to `StartRef`), `client/wwwroot/css/app.css`
- Create: `client.Tests/ContentsTreeModelTests.cs`, `tests/ux/contents-tree.spec.ts`; modify `tests/ux/CONTRACT.md`, `client.Tests/ConcordTocTests.cs` (if it pins the hardcoded list)

**Interfaces:**
- Consumes: Task 5's JSON via `AtlasClient.Contents(corpus)`.
- Produces: `ContentsTreeModel` (pure C#, testable): `IReadOnlyList<Node> Roots`; `Node { Id, Title, Kind, Group, Ref, Count, Children, Expanded }`; `Toggle(id)`; `ExpandPathTo(ref)`; `Flatten()` → the visible rows in order. `ContentsTree` renders the model with `role="tree"`/`role="treeitem"`/`aria-expanded`, keyboard (Enter/Space toggles, ArrowRight expands, ArrowLeft collapses), and calls `OnNavigate(ref)` for leaves. `ContentsPanel` wraps it in a slide-in panel with a corpus header, `data-testid="contents-panel"`, close button `contents-close`, and persists expanded ids per corpus under localStorage key `contents-expanded-v1:{corpus}` through `LocalStore` (fail-soft). Testids: `contents-open` (the chapter-head/hamburger trigger), `contents-panel`, `contents-node-{id-slug}` (each row), `contents-toggle-{id-slug}`, `contents-current`.

- [ ] **Step 1: Write the failing model tests**

```csharp
public class ContentsTreeModelTests
{
    static ContentsTreeModel Sample() => ContentsTreeModel.From(new ContentsOut("bible", "v", new[] {
        new ContentsRootOut("Container:bible-book-GEN", "Genesis", "book", "OT", "GEN.1", new[] {
            new ContentsChildOut("Container:bible-chapter-GEN-1", "1", "chapter", "GEN.1", 31),
            new ContentsChildOut("Container:bible-chapter-GEN-2", "2", "chapter", "GEN.2", 25) }),
        new ContentsRootOut("Container:bible-book-EXO", "Exodus", "book", "OT", "EXO.1", new[] {
            new ContentsChildOut("Container:bible-chapter-EXO-1", "1", "chapter", "EXO.1", 22) }) }));

    [Fact] public void Collapsed_by_default_shows_only_roots()
        => Assert.Equal(new[] { "Genesis", "Exodus" }, Sample().Flatten().Select(r => r.Title));

    [Fact] public void Toggle_expands_then_collapses_children()
    {
        var m = Sample(); m.Toggle("Container:bible-book-GEN");
        Assert.Equal(new[] { "Genesis", "1", "2", "Exodus" }, m.Flatten().Select(r => r.Title));
        m.Toggle("Container:bible-book-GEN");
        Assert.Equal(new[] { "Genesis", "Exodus" }, m.Flatten().Select(r => r.Title));
    }

    [Fact] public void Toggle_is_an_involution_for_any_id_sequence()
    {   // property: toggling the same id twice restores the visible rows
        var m = Sample(); var before = m.Flatten().Select(r => r.Id).ToList();
        foreach (var id in new[] { "Container:bible-book-EXO", "Container:bible-book-GEN" }) { m.Toggle(id); m.Toggle(id); }
        Assert.Equal(before, m.Flatten().Select(r => r.Id));
    }

    [Fact] public void ExpandPathTo_opens_exactly_the_ancestors_of_the_current_ref()
    {
        var m = Sample(); m.ExpandPathTo("EXO.1");
        Assert.Equal(new[] { "Genesis", "Exodus", "1" }, m.Flatten().Select(r => r.Title));
        Assert.Equal("Container:bible-chapter-EXO-1", m.CurrentId);
    }

    [Fact] public void Leaves_never_expand()
    {
        var m = Sample(); m.Toggle("Container:bible-chapter-GEN-1");
        Assert.Equal(new[] { "Genesis", "Exodus" }, m.Flatten().Select(r => r.Title));
    }
}
```

- [ ] **Step 2: Run to verify they fail** — `dotnet test client.Tests --filter ContentsTreeModelTests` → compile errors / FAIL.

- [ ] **Step 3: Implement the model and DTOs**, then the components. `ContentsTree.razor` renders `Model.Flatten()` as `<ul role="tree">` rows with indentation by depth, a toggle button on expandable rows (`aria-expanded`), a navigate button on leaves; the panel loads `Contents(corpus)` once per corpus (cache in a `Dictionary<string, ContentsTreeModel>` on the panel), calls `ExpandPathTo(currentRef)` when opened, restores/saves expanded ids via `LocalStore`, and closes on Escape or the close button. Style it with the reader's parchment tokens and the Concord page's sidebar tokens (both already in `app.css`); no new colours.

- [ ] **Step 4: Wire it in**: Reader chapter head (`Reader.razor:176–180`) gains a `contents-open` trigger (click opens the panel with `corpus="bible"`, `currentRef=$"{Book}.{Chapter}"`; the dwell peek stays as is); hamburger (`MainLayout.razor:64–90`) gains "Contents" opening the panel for the current page's corpus (bible on reader/world/kretzmann, concord on /concord); `Concord.razor:118–135` replaces the flat `<ul class="concord-toc-list">` with `<ContentsTree Corpus="concord" CurrentRef=@CurrentRef OnNavigate=… />` inside the existing sidebar (keep `concord-sidebar-search`); leaf navigation for Concord goes to `/concord?ref={ref}` (Task 4's deep link), for the Bible to `/read/{Book}/{Chapter}`. Retire `ConcordToc`'s hardcoded document list if nothing else reads it; update `ConcordTocTests.cs` accordingly.

- [ ] **Step 5: Client suites** — `dotnet test client.Tests` green (model tests + updated ConcordToc tests).

- [ ] **Step 6: Playwright** — `tests/ux/contents-tree.spec.ts`:

```ts
import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { RUNS_UI } from './lib/fc';   // the suite's shared runs knob

test('contents tree: roots collapsed, toggle shows children, toggle again hides them, chapter leaf navigates', async ({ page }) => {
  await page.goto('/read/JHN/3');
  await page.getByTestId('contents-open').click();
  const panel = page.getByTestId('contents-panel');
  await expect(panel).toBeVisible();
  await expect(panel.getByTestId('contents-current')).toContainText('3');   // current chapter auto-expanded
  const gen = panel.getByTestId('contents-toggle-Container-bible-book-GEN');
  await gen.click();
  await expect(panel.getByTestId('contents-node-Container-bible-chapter-GEN-1')).toBeVisible();
  await gen.click();
  await expect(panel.getByTestId('contents-node-Container-bible-chapter-GEN-1')).toHaveCount(0);
  await gen.click();
  await panel.getByTestId('contents-node-Container-bible-chapter-GEN-2').click();
  await expect(page).toHaveURL(/\/read\/GEN\/2/);
});

test('contents tree property: any toggle sequence applied twice restores the visible rows', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('contents-open').click();
  const panel = page.getByTestId('contents-panel');
  const books = ['GEN', 'EXO', 'PSA', 'ISA', 'MAT', 'JHN', 'ROM', 'REV'];
  await fc.assert(fc.asyncProperty(fc.array(fc.constantFrom(...books), { maxLength: 6 }), async seq => {
    const before = await panel.locator('[role=treeitem]').allInnerTexts();
    for (const pass of [0, 1]) for (const b of seq) await panel.getByTestId(`contents-toggle-Container-bible-book-${b}`).click();
    expect(await panel.locator('[role=treeitem]').allInnerTexts()).toEqual(before);
  }), { numRuns: RUNS_UI });
});

test('concord contents: documents then articles, and an article navigates to its first paragraph', async ({ page }) => {
  await page.goto('/concord');
  const tree = page.locator('.concord-sidebar [role=tree]');
  await expect(tree.locator('[role=treeitem]')).toHaveCount(10);
  await tree.getByTestId(/contents-toggle-Container-concord-doc-/).first().click();
  const article = tree.getByTestId(/contents-node-Container-concord-art-/).first();
  await expect(article).toBeVisible();
  await article.click();
  await expect(page).toHaveURL(/\/concord\?ref=\d+\.\d+\.\d+/);
});
```

Slugs: `contents-*-{id-slug}` where the slug replaces `:` and `.` with `-` (state the rule in CONTRACT.md). Run `npx playwright test contents-tree concord` if 8000 is free; otherwise report not-executed.

- [ ] **Step 7: Commit**

```bash
git add client/Components/ContentsTree.razor client/Components/ContentsPanel.razor client/Components/ContentsTreeModel.cs client/Dtos.cs client/AtlasClient.cs client/Pages/Reader.razor client/Pages/Concord.razor client/Layout/MainLayout.razor client/Explore/ConcordToc.cs client/wwwroot/css/app.css client.Tests/ContentsTreeModelTests.cs client.Tests/ConcordTocTests.cs tests/ux/contents-tree.spec.ts tests/ux/CONTRACT.md
git commit -F - <<'EOF'
feat(client): D4 -- table of contents as an expandable tree over the containment forest

One ContentsTree component (role=tree, keyboard, aria-expanded) renders
/api/contents/{corpus}: books then chapters; Concord documents then
articles. Click toggles children; leaves navigate. Opened from the
chapter head and the hamburger; the Concord sidebar's flat list is
replaced. Expanded state is a per-viewer convenience in localStorage.
"Pages are not a meaningful way of thinking about things."

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
EOF
```

---

### Task 7: D2a — pane-scoped state and generic pairing (the state core)

**Files:**
- Create: `client/State/PaneScope.cs`
- Modify: `client/State/ViewArrangement.cs:36–62, :143` (extend-only: `EnterSplit` gains an optional `GuestRef`), `client/Views/SplitUrlContract.cs` (`GuestParam = "guest"`), `client/Views/EnterSplitHatch.cs` (`PartnerView` → `PartnerViews: IReadOnlyList<string>`, keeping a `PartnerView` property that returns the first for compatibility), `client/Views/ViewRegistrySetup.cs:44–103, :168–223` (generic `EnterSplit(host, guest)`), `client/Components/CompositionSplit.razor:263–289, :481–534, :553` (provide `CascadingValue<PaneId>`; guest mounts under pane 1; `SyncSplitUrl` writes `guest=`)
- Modify: `client/Program.cs` (register `PaneScopes`), the pages that read `StateAtom<Locus>`/`StateAtom<TimeWindow>` directly (Reader, World, Kretzmann, Concord) to resolve them through `PaneScopes.For(PaneId)`
- Create: `client.Tests/State/PaneScopeTests.cs`; modify `client.Tests/State/ViewArrangementTests.cs`, `client.Tests/State/ViewRegistryConformanceTests.cs`

**Interfaces:**
- Consumes: `StateAtom<T>` (`client/State/StateAtom.cs`), `StateLinkRunner` (`:163`), `ViewCapabilities`.
- Produces:

```csharp
public readonly record struct PaneId(int Index) { public static readonly PaneId Host = new(0), Guest = new(1); }

/// Per-pane atoms. Pane 0 (the host) OWNS the singleton atoms already registered in DI, so
/// every existing single-pane behaviour and URL is unchanged. Pane 1 gets its own atoms.
public sealed class PaneScopes
{
    public PaneAtoms For(PaneId pane);            // Host → the DI singletons; Guest → a lazily created private set
    public void ResetGuest();                      // called when the split closes
    public IDisposable Link(PaneId from, PaneId to, ViewCapabilities via); // follow: Locus→Locus and/or Window→Window one-way link via StateLinkRunner; disposing unlinks
}
public sealed record PaneAtoms(StateAtom<Locus> Locus, StateAtom<TimeWindow> TimeWindow, SelectionAtom Selection);
```

`ViewArrangement` gains `string? GuestRef` (extend-only; `null` today); `EnterSplit(Host, Guest, DefaultFollow, DefaultDividerFraction, GuestRef = null)`. `ViewRegistrySetup` replaces the five closures with `EnterSplitAny(hostName, guestName)` that: dispatches `EnterSplit`, seeds the guest pane's `Locus` from `GuestRef` (or copies the host's locus when the guest `BearsLocus` and follow is on), and links panes when `Follow` is true and both bear a compatible capability (`BearsLocus`→`BearsLocus`/`BearsWindow` link Locus; `BearsWindow`→`BearsWindow` link TimeWindow). Every pair (host, guest) over the registered views is legal, INCLUDING `host == guest`; the allowed guest MENU is decided in Task 8 (owner: map and reader for now).

- [ ] **Step 1: Write the failing law tests**

```csharp
public class PaneScopeTests
{
    [Fact] public void Host_pane_atoms_are_the_DI_singletons()
    {   var (scopes, locus, window, sel) = Build();
        Assert.Same(locus, scopes.For(PaneId.Host).Locus); Assert.Same(window, scopes.For(PaneId.Host).TimeWindow); }

    [Fact] public void Guest_pane_atoms_are_distinct_and_stable()
    {   var (scopes, locus, _, _) = Build();
        var g1 = scopes.For(PaneId.Guest); var g2 = scopes.For(PaneId.Guest);
        Assert.NotSame(locus, g1.Locus); Assert.Same(g1.Locus, g2.Locus); }

    [Fact] public void Two_panes_hold_two_different_loci_without_follow()
    {   var (scopes, _, _, _) = Build();
        scopes.For(PaneId.Host).Locus.Dispatch(new SetLocus(Locus.Parse("GEN.1")));
        scopes.For(PaneId.Guest).Locus.Dispatch(new SetLocus(Locus.Parse("EXO.20")));
        Assert.Equal("GEN.1", scopes.For(PaneId.Host).Locus.Value.Ref);
        Assert.Equal("EXO.20", scopes.For(PaneId.Guest).Locus.Value.Ref); }

    [Fact] public void Follow_link_mirrors_host_locus_into_guest_one_way_and_unlinks_on_dispose()
    {   var (scopes, _, _, _) = Build();
        using (scopes.Link(PaneId.Host, PaneId.Guest, ViewCapabilities.BearsLocus))
        {   scopes.For(PaneId.Host).Locus.Dispatch(new SetLocus(Locus.Parse("JHN.3")));
            Assert.Equal("JHN.3", scopes.For(PaneId.Guest).Locus.Value.Ref);
            scopes.For(PaneId.Guest).Locus.Dispatch(new SetLocus(Locus.Parse("ROM.8")));
            Assert.Equal("JHN.3", scopes.For(PaneId.Host).Locus.Value.Ref, "guest never writes back into host"); }
        scopes.For(PaneId.Host).Locus.Dispatch(new SetLocus(Locus.Parse("PSA.23")));
        Assert.Equal("ROM.8", scopes.For(PaneId.Guest).Locus.Value.Ref, "unlinked after dispose"); }

    [Fact] public void ResetGuest_forgets_the_guest_pane()
    {   var (scopes, _, _, _) = Build(); var g = scopes.For(PaneId.Guest); scopes.ResetGuest(); Assert.NotSame(g, scopes.For(PaneId.Guest)); }
}
```

`Build()` constructs the three real atoms and a `PaneScopes` over them, using whatever intent/record names `Locus.cs`/`TimeWindow.cs` really expose (`SetLocus`, `Locus.Parse` are illustrative — match the real API; `ViewArrangementTests.cs` shows how atoms are built in tests). In `ViewRegistryConformanceTests.cs` add: "every registered view can enter a split with every registered view, itself included" (iterate the registry, call the generic `EnterSplitAny`, assert the arrangement's `Members == [host, guest]`).

- [ ] **Step 2: Run to verify they fail** — `dotnet test client.Tests --filter "PaneScopeTests|ViewRegistryConformanceTests"` → compile errors / FAIL.

- [ ] **Step 3: Implement** `PaneScopes`, the `ViewArrangement` extension, the URL `guest=` param (`SplitUrlContract`), the generic pairing in `ViewRegistrySetup` (delete the five closures once every registration goes through `EnterSplitAny`), the `CascadingValue<PaneId>` in `CompositionSplit` (host content under `PaneId.Host`, the guest mount under `PaneId.Guest`), and switch the four pages' atom resolution to `PaneScopes.For(PaneId)` via `[CascadingParameter] PaneId Pane { get; set; } = PaneId.Host` (single-pane pages default to Host, so nothing changes for them). `SyncSplitUrl` writes `guest={guestLocusRef}` when the guest bears a locus and is not following. Keep every existing URL working (`?split=world&follow=1` unchanged).

- [ ] **Step 4: Full client suites** — `dotnet test client.Tests; dotnet test client.ContractTests` green. The `ConformanceTests` source-scan tripwires (e.g. `InitialDividerFraction_ReferencedFromExactlyOneProductionSite`) must still pass — if one trips because you moved a reference, restore the single production site rather than editing the tripwire.

- [ ] **Step 5: Existing UX specs unchanged** — `npx playwright test split-view state-sync state-window kretzmann` if 8000 is free; else report not-executed. Expected: green with NO spec edits (this task is behaviour-preserving for every existing pair).

- [ ] **Step 6: Commit**

```bash
git add client/State/PaneScope.cs client/State/ViewArrangement.cs client/Views client/Components/CompositionSplit.razor client/Program.cs client/Pages client.Tests/State
git commit -F - <<'EOF'
feat(state): D2 -- pane-scoped atoms and generic view pairing

PaneScopes gives the guest pane its own Locus/TimeWindow/Selection atoms
(the host keeps the DI singletons, so every single-pane behaviour and
URL is unchanged). Follow becomes a one-way StateLink between panes,
typed by capability. The five hardcoded host->guest closures collapse
into EnterSplitAny(host, guest) over the registry -- any pair, itself
included. ?guest= carries the guest locus. Laws in PaneScopeTests and
ViewRegistryConformanceTests.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
EOF
```

---

### Task 8: D2b — the pair picker and same-view splits (map and reader)

**Files:**
- Modify: `client/Views/ViewRegistrySetup.cs` (guest menus: reader → {world, reader}; world → {reader, world}; other hosts keep {reader}), `client/Views/EnterSplitHatch.cs` (renders a menu when `PartnerViews.Count > 1`), `client/Pages/World.razor:42–57` (follow chip present only when the pair is linkable; D1 guard now reads the PANE's follow state), `client/Pages/Reader.razor` (its enter-split affordance offers both guests)
- Create: `tests/ux/split-pairs.spec.ts`; modify `tests/ux/CONTRACT.md`
- Modify: `client.Tests/State/ViewRegistryConformanceTests.cs` (menu law)

**Interfaces:**
- Consumes: Task 7's `EnterSplitAny`, `PaneScopes`, `ViewArrangement.GuestRef`.
- Produces: a guest menu on the enter-split hatch (`data-testid="enter-split-menu"`, items `enter-split-guest-{viewName}`); same-view pairs `reader‖reader` and `world‖world`; follow toggle offered iff `PaneScopes.Link` has a capability to link (reader‖reader: Locus; world‖world: TimeWindow; reader‖world either way: Locus→Window as today), default OFF for same-view pairs and unchanged for existing pairs. The D1 law holds per pane: the guest reader in `reader‖reader` keeps its picker (it is a Reader), the host reader keeps its picker, a following world pane has none.

- [ ] **Step 1: Write the failing menu law** in `ViewRegistryConformanceTests.cs`: reader's and world's `EnterSplitHatch.PartnerViews` equal exactly `{world, reader}` (order: other first, self second); every other host's equals `{reader}`.

- [ ] **Step 2: Run to verify it fails.**

- [ ] **Step 3: Implement** the menus and the follow-availability rule; the world page's follow chip renders only when `PaneScopes` reports a linkable capability for the current pair (so `world‖world` shows a follow chip meaning "share the time window", `reader‖reader` shows one meaning "guest mirrors the host's chapter"). Update the chip's title text per pair meaning.

- [ ] **Step 4: Client suites green.**

- [ ] **Step 5: Playwright `tests/ux/split-pairs.spec.ts`**:

```ts
test('reader‖reader: two chapters side by side, independent until follow', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('enter-split').click();
  await page.getByTestId('enter-split-guest-reader').click();
  const guest = page.locator('.split-pane-guest');
  await expect(guest.getByTestId('picker')).toBeVisible();           // D1: a Reader keeps its picker
  await guest.getByTestId('picker-book').selectOption('EXO');
  await guest.getByTestId('picker-chapter').selectOption('20');
  await guest.getByTestId('picker-apply').click();
  await expect(guest.getByTestId('chapter-head')).toContainText('Exodus 20');
  await expect(page.locator('.split-view > *').first().getByTestId('chapter-head')).toContainText('Genesis 1');
  await expect(page).toHaveURL(/split=reader/); await expect(page).toHaveURL(/guest=EXO\.20/);
  await guest.getByTestId('follow-chip').click();                      // now mirrors the host
  await expect(guest.getByTestId('chapter-head')).toContainText('Genesis 1');
});

test('world‖world: two time windows; follow shares the window', async ({ page }) => {
  await page.goto('/world');
  await page.getByTestId('enter-split').click();
  await page.getByTestId('enter-split-guest-world').click();
  await expect(page.locator('.split-pane-guest [data-testid=timeline]')).toBeVisible();
  // move the guest's window; the host's stays put (use the spec suite's existing timeline helper)
});

test('the pair menu offers exactly map and reader, on both hosts', async ({ page }) => {
  for (const [url, expected] of [['/read/GEN/1', ['world', 'reader']], ['/world', ['reader', 'world']]] as const) {
    await page.goto(url); await page.getByTestId('enter-split').click();
    const items = page.getByTestId('enter-split-menu').locator('[data-testid^=enter-split-guest-]');
    expect(await items.evaluateAll(els => els.map(e => e.getAttribute('data-testid')!.replace('enter-split-guest-', '')))).toEqual([...expected]);
  }
});
```

Use the real testids for the enter-split hatch, the chapter head, and the timeline from CONTRACT.md (the names above are the inventory's current ones where I could see them; verify). Add the new testids to the inventory. Run `npx playwright test split-pairs split-view` if 8000 is free; else report not-executed.

- [ ] **Step 6: Commit**

```bash
git add client/Views client/Pages/World.razor client/Pages/Reader.razor client.Tests/State/ViewRegistryConformanceTests.cs tests/ux/split-pairs.spec.ts tests/ux/CONTRACT.md
git commit -F - <<'EOF'
feat(client): D2 -- corpus x corpus for map and reader: pick the guest, split with anything, itself included

The enter-split hatch offers a guest menu (reader and world, for now);
reader‖reader shows two chapters, world‖world two time windows, and the
follow toggle appears exactly when the pair has a capability to link.
The D1 law holds per pane.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
EOF
```

---

---

### Task 9: D5 — person cards: lifespan, events, family (no more "every verse")

**Owner's words (2026-09-15, binding):** "when we click on a person's name there's no point to just see every verse that name is mentioned … when clicking on a person, then i want to see the years that person is alive (years are explorable, remember), and then events (explorable!) where they're mentioned. the exception is God because he is eternal so keep that in mind. you might also show a family tree for each person where the names therein are explorable."

**Files:**
- Modify: `server/atlas-etl/src/people.rs:84–124` (`PersonFields` gains `father`, `mother`, `children`, `partners`, `siblings`, `minYear`, `maxYear`, `timeline` — all `#[serde(default)]`, Theographic record ids resolved to `personLookup` ids the same way `verses` resolve to refs), `server/atlas-core/src/data.rs` (the `Person` struct these land on), `server/atlas-etl/src/theographic.rs` (events gain `participants: Vec<String>` — Theographic `events.json` carries `participants` and `people (from verses)`; ingest `participants` only)
- Create: `data/curated/people-eternal.toml` (see Step 3)
- Modify: `graph-types/src/edge.rs` (`relations!` manifest: `ParentOf` directed `parent-of`/`child-of`; `Partners` symmetric `partner-of`; `Participates` directed `participates-in`/`participants`), `graph-types/src/graph.rs` (three row `Vec`s + lowering), `graph-types/src/node.rs` (`NodePayload::Person` gains `active_from: Option<i32>`, `active_to: Option<i32>`, `eternal: bool` — extend-only), `graph-types/src/frontier.rs` (Person row offers `ParentOf` both directions, `Partners`, `Participates` forward; Event row offers `Participates` inverse)
- Modify: `server/atlas-graph/src/person_adapter.rs` (rows + payload fields), `server/atlas-graph/src/law_check.rs` (one law, Step 4), `server/atlas-graph/tests/peoples_real_data.rs` or a new `kinship_real_data.rs`
- Regenerate: `data/compiled/graph.bin`, `report.txt`
- Modify: `client/Explore/PersonNode.cs:73–74`, `client/Explore/PopoverSectionProviders.cs:3027` (`PersonCardAndMentionsSection`), `client/Explore/YearNode.cs` (a sibling `LifeYearNode` built from a year + label, since today's `YearNode` takes a place `DateClaimOut`), the TRACKED frontier-matrix mirror, `client/Dtos.cs` (Person payload fields on the node-card wire if the card reads them), the Playwright spec that covers person cards (grep `Person` in `tests/ux`), `tests/ux/CONTRACT.md`
- Modify: `LICENSES.md` only if a new Theographic field class needs its own row (kinship is the same CC BY-SA bundle already credited — probably no change; say so in the report).

**Interfaces:**
- Consumes: Theographic `people.json` fields (verified 2026-09-15: `father`/`mother`/`children`/`partners`/`siblings` as record-id lists — 1,599 of 3,067 people have a parent; `minYear`/`maxYear` ints; `timeline` = event record ids on 268 people); `events.json` `participants`; existing `YearNode`/`EventNode`/`PersonNode`; `/api/node/{id}/edges?kind=…`.
- Produces: three relations with rows (imported, provenance `theographic`, no justification); Person payload `active_from/active_to/eternal`; a person card whose sections are, in this order and each only when inhabited: **Life** → **Events** → **Family** → **Mentioned in** (the old verse list, collapsed by default, last).

- [ ] **Step 1: Failing ETL tests** in `people.rs`'s test module (follow its existing JSON-fixture style, e.g. the `aaron_1` birth/death test near `:300`): (a) a person with `father: ["recF"]`, `mother: ["recM"]`, `children: ["recC1","recC2"]`, `partners: ["recP"]` resolves each to the referenced record's `personLookup` id, in source order, dangling ids dropped and counted in `PeopleStats`; (b) `minYear`/`maxYear` parse as `active_from/active_to`; (c) a record with no parent fields yields empty vectors, never a panic. In `theographic.rs`'s tests: an event with `participants: ["recA","recB"]` resolves to person ids. Run `cargo test -p atlas-etl people theographic` → FAIL (fields do not exist).

- [ ] **Step 2: Implement the parsing** (extend-only structs; `#[serde(default)]` everywhere; resolution through the same record-id → `personLookup` map the file already builds for verses/`memberOf`). GREEN.

- [ ] **Step 3: The eternity exception, as curated truth with grounds** — create `data/curated/people-eternal.toml`:

```toml
# D5 (owner, 2026-09-15): "the exception is God because he is eternal."
# A person listed here has NO lifespan: no birth, no death, no active years.
# Theographic carries God and the Holy Spirit as person records with
# minYear -4004 / maxYear 96 (the corpus span) -- that is the dataset's
# bookkeeping, not a lifespan, and the card must never render it as one.
# Jesus (jesus_905, born 4 BC, died AD 30) is NOT listed: His earthly life
# has dates, and the card labels them "Earthly life" (Step 7).
[[eternal]]
id = "god_1324"
grounds = ["PSA.90.2", "REV.1.8"]

[[eternal]]
id = "holy_spirit_7400"
grounds = ["HEB.9.14"]
```

Read it in `curated.rs` (same pattern as the other small curated tables), validate that every id resolves to a real person (fail loud otherwise), and set `eternal = true` on those payloads in `person_adapter.rs`; for eternal persons, `active_from/active_to/birth_year/death_year` are forced to `None` in the payload (the card can then rely on the flag alone). Test: a real-data test asserts `Person:god_1324` has `eternal == true` and no years, and `Person:jesus_905` has `eternal == false`, `birth_year == Some(-4)`, `death_year == Some(30)`.

- [ ] **Step 4: Relations and the law.** Add the three relations to the `relations!` manifest and the three row structs (`ParentOf { parent: PersonId, child: PersonId, provenance }`, `Partners { a, b, provenance }` with `a < b` canonical like `Analogue`, `Participates { person: PersonId, event: EventId, provenance }`). Adapter: one `ParentOf` row per `(father|mother) → person` and per `person → child` — deduplicated by `(parent, child)` since Theographic states the edge from both ends; one `Partners` row per unordered pair; one `Participates` row per `(event.participants[i], event)`, skipping participants that are not persons (Theographic `participants` may include groups; count the skips in stats). Law in `law_check.rs`: **`kinship_is_acyclic`** — following `parent-of` edges never returns to the start (nobody is their own ancestor); fail loud with the cycle. Real-data test: Abraham has ≥ 8 `parent-of` rows; Isaac's `child-of` frontier contains Abraham; the law passes on the real graph; `Partners` count > 0; `Participates` count > 0. Frontier matrix (Rust, then the tracked C# mirror) offers the new kinds on Person and Event.

- [ ] **Step 5: Regenerate** (`cargo run -p atlas-etl`, then `atlas-graph-compile` as in Task 2 Step 5); `scene_byte_identity` unchanged (scenes do not read kinship); the version root MOVES this time because Person payloads changed — record old and new roots in the report (this is a node-payload change; map-generator's stale-check will fire once, as designed — flag it for the owner).

- [ ] **Step 6: Wire: nothing new needed** — `/api/node/Person:{id}` already returns the payload and `edge_summary`; `/api/node/{id}/edges?kind=parent-of|child-of|partner-of|participates-in` pages the rows. Add the new payload fields to the client's node-card DTO if it is typed (grep `birth_year` in `client/`).

- [ ] **Step 7: The card.** `PersonNode.ExploreAsync` fetches the four edge kinds (limit 200 each) and the node card, and returns explorations in this order, each only when inhabited:
  1. **Life** — for `eternal`: one line "Eternal — Psalm 90:2", the verse a `VerseNode` chip, no years. Otherwise: "Born c. 1997 BC · Died c. 1821 BC" when both known, each year a `LifeYearNode` (title "Born c. 1997 BC" / "Died c. 1821 BC"; exploring it sets the world time window to that year exactly as `YearNode` does — reuse `YearNode`'s exploration body, parameterised by a year and a label rather than a `DateClaimOut`); when birth/death are unknown but `active_from/active_to` exist: "Active c. 2056–64 BC" with the two ends as `LifeYearNode`s and the word "Active" (never "Born"/"Died"). For `jesus_905` the heading reads "Earthly life". Formatting through `YearText` (BC/AD conventions already pinned by CONTRACT.md).
  2. **Events (n)** — `participates-in` targets as `EventNode` chips in the events' canonical order (sort client-side by the event's date if the wire carries it; otherwise the edge order).
  3. **Family** — a compact tree: Parents (`child-of` targets) above, Partners beside, Children (`parent-of` targets) below, Siblings = other children of the same parents (derived client-side from the parents' `parent-of` pages; capped at 12 with "+n more"). Every name is a `PersonNode` chip. Render as labelled rows with `role="group"`, not a canvas.
  4. **Mentioned in (n)** — today's verse list, collapsed behind a disclosure, last.
  Retire the verse list as the card's default body (`PersonCardAndMentionsSection` becomes the fourth section only). POPOVER-LAW-1: no section renders empty.

- [ ] **Step 8: Tests.** xunit: `PopoverSectionRegistryTests` asserts the four providers and their order for kind `Person`; a pure-C# test for the sibling derivation and the +n cap. Playwright (add to the person spec or create `tests/ux/person-card.spec.ts`): (a) Abraham's card shows a Life line with two year chips, an Events section with ≥ 1 chip, a Family section naming Isaac under Children and Terah under Parents, and the verse list collapsed; (b) God's card shows "Eternal" and NO year chip and NO Family section; (c) clicking a Family name opens that person's card (frontier round trip); (d) clicking a year chip changes the world time window (reuse the existing YearNode assertion). Register testids `person-life`, `person-life-year-{born|died|from|to}`, `person-events`, `person-family`, `person-family-{parents|partners|children|siblings}`, `person-mentions`. Run if port 8000 is free; else report not-executed.

- [ ] **Step 9: Commit** (two commits are fine: server+data, then client). Message for the combined change:

```
feat(graph+client): D5 -- person cards show life, events and family, not every verse

Theographic kinship (father/mother/children/partners) and event
participants enter the graph as parent-of, partner-of and participates-in
rows (imported, provenance-tagged); Person payloads gain active years and
an `eternal` flag driven by data/curated/people-eternal.toml (God, the Holy
Spirit; grounds Psalm 90:2, Revelation 1:8, Hebrews 9:14). A new law:
kinship is acyclic. The person card now reads Life -> Events -> Family ->
Mentioned in (collapsed), every year and every name explorable.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W
```

**Judgment calls (owner can reverse):** eternity is curated data with Scripture grounds, not a hardcoded name check; Jesus keeps His earthly dates under an "Earthly life" heading; siblings are derived from shared parents rather than stored (Theographic's `siblings` list is redundant with parents and would be a second path to the same fact); Theographic `people (from verses)` is NOT ingested (derived data — the graph already has Mentions); the verse list survives, collapsed and last, because it is still a real edge set.

## Self-review (done while writing)

**Directive coverage.** D5 → Task 9 (lifespan with explorable years, eternity as curated truth with grounds, events via participates-in, family tree with explorable names, verse list demoted). D1 → Task 1 (guard on both non-Reader mounts + source-scan law + specs). D3 → Tasks 2 (containers, "analogous to how we did the Bible"), 3 (the wire fact that makes "pointlessly clickable" fixable), 4 (the symmetric traversal verse ↔ item ↔ Concord in both directions, dead clicks removed, deep link into the corpus). D4 → Tasks 5–6 (tree from the containment forest, toggle semantics, stop at chapter/article, no pages). D2 → Tasks 7–8 (cartesian product over {reader, world} incl. same-view, follow where linkable).

**Judgment calls made (owner can reverse any):** (1) task order D1 → D3 → D4 → D2 by dependency and risk; (2) clickability decided server-side via an additive `edge_summary` on text-window units rather than N+1 node-card calls or a hardcoded part list; (3) the contents endpoint is graph-derived (containers) rather than a hand-maintained list, so a new corpus gets a ToC for free; (4) `PaneScopes` keeps the host on the existing singletons so no single-pane URL or behaviour changes; (5) follow stays one toggle per split (one guest at a time), typed by capability, default off for same-view pairs; (6) the CatechismNode's new "Scripture" edge section defers to the existing bespoke `CatechismScripturesSection` when they would show the same verses (POPOVER-LAW-1); (7) Playwright may be unrunnable on this machine while port 8000 is held — tasks report not-executed rather than editing the harness.

**Placeholders.** Code blocks name real files and line anchors from the survey; where a helper's exact name is unknown to the plan (`Build()`, `get_json`, `ClientRoot`, `EdgesAsync`), the step says which existing test/file to copy it from. No TBDs.

**Name consistency.** Section-provider ids (`catechism-scripture-edges`, `catechism-in-concord`, `concord-small-catechism`) match between Task 4 Steps 3 and 5. Testids (`contents-open`, `contents-panel`, `contents-node-*`, `contents-toggle-*`, `contents-current`, `enter-split-menu`, `enter-split-guest-*`) match between component specs and Playwright. `PaneId.Host/Guest`, `PaneScopes.For/Link/ResetGuest`, `EnterSplitAny`, `GuestRef`, `GuestParam` match between Tasks 7 and 8. `/api/contents/{corpus}` shape matches between Task 5's tests and Task 6's DTOs.
