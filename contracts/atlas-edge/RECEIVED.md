# These are not our expectations. They are map-generator's, of us.

**Received from:** `donovansmith44/map-generator`, `contracts/atlas-edge/`,
at commit `6608db440111d96e9e2ebf63ba7a3bf5c67f2448`.

Under the consumer-driven contract law the CONSUMER writes the
expectations and the PROVIDER runs them and breaks when it violates them.
map-generator is the consumer here: its `crates/map-compile/src/vendor.rs`
parses six of our endpoints, and these six feature files pin exactly the
fields those parsers read — the *consumed projection*, not our whole
response. We may add fields freely; we may not change a consumed one
without someone going red.

Their own words, in `polities.feature`:

> The consumed projection of the whole book is pinned — all rows, all
> coordinates. A silently moved border fails here before it can move a
> pixel of ours.

**Why a copy lives in our repo.** Until now this suite ran only from
map-generator's `make ci`, against a live atlas on 8080 — which nobody runs
during an atlas batch. So an atlas change that broke a consumed field was
discovered in the *other* repo, late, by the owner. Running it here means
we discover it in our own build, at the moment we cause it. That is the
whole point of CDC-1, and it is MAPS-1's open question 4 answered by
building it rather than by voting on it.

## What we changed, and what we did not

**Not changed:** every `Feature:` title, every line of preamble prose,
every `Scenario:`, every `When`/`Then`, and all six fixtures — byte for
byte. Verified by diff at the time of vendoring, and re-verifiable at any
time against the upstream checkout.

**Changed, mechanically:** the `Vocabulary:` table in each file. That table
is not an expectation — it is a *generated description of the runner's own
parameter space*, produced by `contract-runner vocab --write` and checked
by `contract-runner vocab`. Our runner's projection registry is wider than
theirs (it carries the atlas's graph-primary projections as well as these
six), so the table legitimately reads differently here. The rows that
matter — the projection NAMES these scenarios use — resolve to
definitions ported field-for-field from their own `Steps.hs`.

To confirm the substantive content is untouched:

```
diff <(grep -v '| projection |' contracts/atlas-edge/eras.feature) \
     <(grep -v '| projection |' ../map-generator/contracts/atlas-edge/eras.feature)
```

## The rule for changing anything here

Don't. A red in this suite is not a test to fix — it is a message that we
have changed something another system reads. The response is to break,
report, and coordinate a version bump on both sides, exactly as the
addendum's cross-repo negotiation describes. Editing one of these files to
make our build green would be forging the other side's signature on our own
change.

If map-generator's expectations genuinely move, re-vendor from upstream and
re-run `contract-runner vocab --write` — never hand-edit.

## Status as of batch CDC-1

All six scenarios **green** against our current graph
(`atlas_version_root dfcf6ee4c2a39965`), run via
`contract-runner run --replay contracts/pacts --exports data/exports contracts/atlas-edge`.

That is a finding worth stating plainly, because MAPS-1 predicted the
opposite might be true: map-generator's vendored gazetteer is pinned at
`a1b93a3b049a1fe0` and is two roots stale across PLACE-1a, which deleted 15
place ids. **The stale pin is real; the edge is intact.** None of the six
endpoints this suite consumes is the gazetteer, so the drift does not reach
them. The exposure is in their *vendored C2/C3 exports*, not in this
projection. See the CDC-1 report for the measured id list.
