# Bible Explorer -- the build graph.
#
# `make` (or `make help`) lists every target. The one you'll use most:
#
#   make dev      -- API (:8000) + Blazor client (:5000), Ctrl+C stops both
#
# Everything downstream of the data is wired as real file rules, so `make dev`
# fetches a missing data/raw/ source and re-runs atlas-etl when a curated TOML
# (or atlas-etl itself) is newer than data/compiled/. `make -n dev` shows what
# would run without running it. Rust and .NET builds stay phony -- cargo and
# dotnet track their own inputs far better than a Makefile could restate them.
#
# On Windows, use the PowerShell commands in README.md instead.

SHELL := /bin/bash
.DEFAULT_GOAL := help
.DELETE_ON_ERROR:

# Same PATH fixup scripts/dev.sh does: rustup and dotnet-install put their
# binaries in user-local directories a fresh non-login shell may not have.
export PATH := $(HOME)/.cargo/bin:$(HOME)/.dotnet:$(PATH)
export DOTNET_ROOT := $(HOME)/.dotnet

API_PORT ?= 8000
CLIENT_PORT ?= 5000
DATA_DIR ?= ../data/compiled

## --- the data graph -------------------------------------------------------

RAW := data/raw
FETCH := ./scripts/fetch-raw.sh

# Pinned in scripts/fetch-raw.sh and data/curated/catechism-mapping.toml too --
# a commit, not a branch, so this path is stable forever.
CATECHISM_SHA := 0be24fee92e6333f817c4c2a08f99cf7c5274295

KJV        := $(RAW)/kjv.json
XREFS      := $(RAW)/xrefs/cross_references.txt
GEO        := $(addprefix $(RAW)/geo/,ancient.jsonl modern.jsonl geometry.jsonl image.jsonl source.jsonl)
THEO       := $(RAW)/theographic/theographic-bible-metadata-master/json/books.json
CATECHISM  := $(RAW)/catechism-mapping/catechism-$(CATECHISM_SHA)/resources/01-What-Is-Christianity.yaml
LEAFLET    := client/wwwroot/vendor/leaflet/leaflet.js client/wwwroot/vendor/leaflet/leaflet.css
RAW_ALL    := $(KJV) $(GEO) $(THEO) $(XREFS) $(CATECHISM) $(LEAFLET)

# M-C retired the startup build: atlas-server now loads the ONE serialized
# graph artifact, <data-dir>/graph.bin, which is committed. So nothing under
# data/raw/ is needed to *run* any more -- raw is a rebuild input only.
# (`atlas-server --build-from-raw` is the disclosed dev fallback that does
# still read data/raw/; `make api-from-raw` drives it.)
GRAPH_BIN := data/compiled/graph.bin
GRAPH_RAW := $(KJV) $(XREFS)

# Every atlas-etl output. report.txt is written last (atlas-etl/src/main.rs),
# so it is the sentinel for the whole multi-output run: one rule builds it,
# the JSON files just depend on it with an empty recipe. That is the make 3.81
# idiom for "one command produces N files" -- 3.81 (what macOS ships) has no
# grouped-target `&:` support.
ETL_REPORT := data/compiled/report.txt
# places/events/narratives/eras/verses-kjv/cross-refs.json were retired at
# M-C2 -- that data lives only on the graph now. These are what atlas-etl
# still writes; keep in sync with atlas-etl/src/main.rs's write_json calls.
COMPILED := $(addprefix data/compiled/,canon.json books-meta.json \
  chronology-anchors.json book-narration-windows.json polities.json \
  landmarks.json place-history.json place-names-kjv.json land-mask.json \
  catechism.json)

# What actually invalidates data/compiled/: the curated inputs, the raw inputs
# atlas-etl reads, and atlas-etl's own code.
#
# Caveat, GNU Make 3.81 (what macOS ships): it compares mtimes at 1-second
# granularity, so an edit saved in the *same second* atlas-etl finished writing
# report.txt is not seen as newer and the next `make` looks current. Only ever
# hit by scripted back-to-back edits, not by a human typing; `brew install
# make` (gmake 4.x, nanosecond timestamps) removes it, and `touch` on the file
# or `make accept-compiled` clears a stuck state either way.
CURATED_FILES := $(shell find data/curated -type f 2>/dev/null)
ETL_SRC := $(shell find server/atlas-etl/src server/atlas-core/src -name '*.rs' 2>/dev/null) \
           server/Cargo.lock server/atlas-etl/Cargo.toml server/atlas-core/Cargo.toml
ETL_RAW := $(KJV) $(GEO) $(THEO) $(XREFS) $(CATECHISM)

$(ETL_REPORT): $(ETL_RAW) $(CURATED_FILES) $(ETL_SRC)
	@echo "==> data/compiled/ is stale -- running atlas-etl"
	cd server && cargo run -p atlas-etl

# Normally a no-op: report.txt is the sentinel, so if it is current every JSON
# beside it is too. The `test -s` still covers the one case a bare sentinel
# would miss -- a single compiled file deleted while report.txt stayed current.
$(COMPILED): $(ETL_REPORT)
	@test -s $@ || (echo "==> $@ missing -- running atlas-etl"; cd server && cargo run -p atlas-etl)

# The graph artifact is its own compile step (atlas-graph-compile), reading the
# same raw+curated inputs through atlas_etl::compile plus the graph adapters.
GRAPH_SRC := $(shell find server/atlas-graph/src graph-types/src -name '*.rs' 2>/dev/null)

# --release matches the command README.md documents: the compile admits the
# artifact against an independently-rebuilt model twice before writing, which
# takes tens of seconds over the full graph and is painful unoptimized.
$(GRAPH_BIN): $(ETL_RAW) $(CURATED_FILES) $(ETL_SRC) $(GRAPH_SRC)
	@echo "==> $(GRAPH_BIN) is stale -- running atlas-graph-compile"
	cd server && cargo run -p atlas-graph --release --bin atlas-graph-compile -- \
	  --data-dir ../data/compiled --out ../data/compiled/graph.bin

# Raw fetches: no prerequisites, so each runs exactly once -- when its own file
# is absent. Multi-file artifacts list every file as a target of one rule (make
# then applies the recipe per missing target), rather than routing them through
# a sentinel: fetch-raw.sh is idempotent, so a redundant call just prints
# "have", and any *one* deleted file is restored. A sentinel would silently
# leave siblings missing.
$(KJV):
	@$(FETCH) kjv
$(GEO):
	@$(FETCH) geo
$(XREFS):
	@$(FETCH) xrefs
$(THEO):
	@$(FETCH) theographic
$(CATECHISM):
	@$(FETCH) catechism
$(LEAFLET):
	@$(FETCH) leaflet

.PHONY: help dev api client raw etl test test-server test-client test-ux \
        test-deep build publish serve-publish smoke doctor ports plan graph \
        api-from-raw \
        accept-compiled clean clean-raw

help: ## Show this help
	@echo "Bible Explorer -- make targets"
	@echo
	@grep -E '^[a-z-]+:.*?## ' $(MAKEFILE_LIST) \
	  | awk 'BEGIN{FS=":.*?## "}{printf "  \033[1m%-15s\033[0m %s\n", $$1, $$2}'
	@echo
	@echo "  Vars: API_PORT=$(API_PORT) CLIENT_PORT=$(CLIENT_PORT) DATA_DIR=$(DATA_DIR)"
	@echo "  Dry run: make -n <target>   (or 'make plan')"

## --- run ------------------------------------------------------------------

dev: $(LEAFLET) $(COMPILED) $(GRAPH_BIN) ## Run API + client together (canonical dev loop; Ctrl+C stops both)
	@./scripts/dev.sh

api: $(COMPILED) $(GRAPH_BIN) ## Run just the Rust API alone (API_PORT, default 8000)
	cd server && cargo run -p atlas-server -- --data-dir $(DATA_DIR) --port $(API_PORT)

client: $(LEAFLET) ## Run just the Blazor dev server alone (CLIENT_PORT, default 5000)
	dotnet run --project client --launch-profile http

## --- data -----------------------------------------------------------------

raw: $(RAW_ALL) ## Fetch any missing data/raw/ source (idempotent)
	@echo "data/raw/ complete"

etl: $(ETL_REPORT) ## Recompile data/compiled/*.json if its inputs changed (no-op if current)
	@echo "data/compiled/*.json is current"

graph: $(GRAPH_BIN) ## Recompile data/compiled/graph.bin if its inputs changed (no-op if current)
	@echo "$(GRAPH_BIN) is current"

api-from-raw: $(COMPILED) $(GRAPH_RAW) ## Run the API with the disclosed --build-from-raw fallback (no graph.bin)
	cd server && cargo run -p atlas-server -- \
	  --data-dir $(DATA_DIR) --port $(API_PORT) --build-from-raw

# git sets mtimes to checkout time, so a fresh clone can look stale even when
# data/compiled/ is exactly what its inputs produce. This asserts "what's
# committed is current" without burning an ETL run.
accept-compiled: ## Mark the committed data/compiled/ as up to date (skip a spurious rebuild)
	@touch $(ETL_REPORT) $(COMPILED) $(GRAPH_BIN)
	@echo "data/compiled/ marked current"

plan: ## Dry-run `make dev` -- show what is stale and what would run
	@$(MAKE) --no-print-directory -n dev

## --- test -----------------------------------------------------------------

test: test-server test-client test-ux ## Run every suite

test-server: ## Rust workspace tests (484 tests, ~40s)
	cd server && cargo test --workspace

# Never build/test the client while its own dev server is serving -- it
# rewrites the WASM output under the running server (a proven flake source),
# which is what the `ports` prerequisite guards against.
test-client: ports ## Blazor xunit tests (~4s) -- stop `make dev` first
	dotnet test client.Tests

test-ux: ports $(LEAFLET) $(COMPILED) $(GRAPH_BIN) ## Playwright UX suite (~1.5m) -- starts and stops its own servers
	cd tests/ux && npx playwright test

test-deep: ports $(LEAFLET) $(COMPILED) $(GRAPH_BIN) ## Two-tier exhaustive property run (API @500 runs, then full @60)
	cd tests/ux && FC_NUM_RUNS=500 npx playwright test api-
	cd tests/ux && FC_NUM_RUNS=60 npx playwright test

## --- build ----------------------------------------------------------------

build: ## Build both sides (cargo build + dotnet build client)
	cd server && cargo build --workspace
	dotnet build client

publish: $(LEAFLET) ## Publish the client into publish/ (gitignored build artifact)
	dotnet publish client -c Release -o publish

serve-publish: publish $(COMPILED) $(GRAPH_BIN) ## Serve API + published client same-origin on API_PORT
	cd server && cargo run -p atlas-server -- \
	  --data-dir $(DATA_DIR) --static-dir ../publish/wwwroot --port $(API_PORT)

## --- utilities ------------------------------------------------------------

smoke: ## Curl a running API/client to confirm both answer
	@curl --noproxy '*' -fsS -o /dev/null -w 'api    :$(API_PORT)/health   -> %{http_code}\n' http://127.0.0.1:$(API_PORT)/health
	@curl --noproxy '*' -fsS -o /dev/null -w 'api    :$(API_PORT)/api/books -> %{http_code}\n' http://127.0.0.1:$(API_PORT)/api/books
	@curl --noproxy '*' -fsS -o /dev/null -w 'client :$(CLIENT_PORT)/         -> %{http_code}\n' http://127.0.0.1:$(CLIENT_PORT)/

doctor: ## Check the toolchain the dev loop needs
	@for c in cargo dotnet curl unzip; do \
	  if command -v $$c >/dev/null 2>&1; then echo "ok      $$c ($$(command -v $$c))"; \
	  else echo "MISSING $$c -- see README.md Prerequisites"; fi; done
	@command -v npx >/dev/null 2>&1 \
	  && echo "ok      npx ($$(command -v npx))" \
	  || echo "MISSING npx -- Node 24 + npm, needed only by test-ux/test-deep"
	@v=$$(dotnet --version 2>/dev/null || echo none); \
	  case "$$v" in 10.*) echo "ok      .NET $$v";; *) echo "MISSING .NET 10 (found $$v)";; esac

# Report, never kill: the README's port hygiene rule is to stop the PID that
# actually owns the port, by hand -- never a process matched by name.
ports: ## Report what owns the two dev ports, if anything
	@busy=0; for p in $(API_PORT) $(CLIENT_PORT); do \
	  if lsof -nP -iTCP:$$p -sTCP:LISTEN 2>/dev/null | tail -n +2 | grep -q .; then \
	    busy=1; echo "port $$p is in use:"; lsof -nP -iTCP:$$p -sTCP:LISTEN; fi; done; \
	  if [ $$busy -eq 1 ]; then \
	    echo "Stop the owning PID above before continuing (never kill by name)." >&2; exit 1; fi; \
	  echo "ports $(API_PORT)/$(CLIENT_PORT) free"

clean: ## Remove build output (cargo target/, dotnet obj+bin, publish/)
	cd server && cargo clean
	@dotnet clean client >/dev/null || true
	@dotnet clean client.Tests >/dev/null || true
	rm -rf publish

clean-raw: ## Delete data/raw/ so the next build re-fetches every source
	@find data/raw -mindepth 1 -maxdepth 1 ! -name README.md -exec rm -rf {} +
	@echo "data/raw/ emptied"
