-- | THE CONSUMED PROJECTION -- and, under CDC-1's addendum ruling ("the
-- primary juncture is over our graph"), the place where that ruling is
-- actually cashed out in code.
--
-- The DSL ('Proj', 'project', 'field1', 'fieldAlt', 'flattenField',
-- 'firstDiff') is map-generator's, vendored from
-- @contracts/runner/src/Steps.hs@ with its comments -- it is the idiom the
-- owner ordered stolen, and it is a good one: a projection names ONLY the
-- fields a consumer actually reads, so the provider may add fields freely
-- and may not change a consumed one without someone going red.
--
-- THE REGISTRY BELOW is the atlas's, and it is organised by the addendum's
-- rule rather than by endpoint: a projection is named for the thing it
-- projects, never for the route that happens to carry it. Transports then
-- inherit -- the day a second carrier serves the same thing it reuses the
-- same projection name and the same fixture rather than growing a second
-- vocabulary for one promise. That is the addendum's "a consumer that adds
-- a transport must not need a new contract vocabulary", and it is checked
-- rather than claimed: @transport\/cli.feature@ asks whether bibex and the
-- HTTP wire project to the SAME value through @node-card@ and
-- @edge-page@, with no fixture on either side.
--
-- HOW HONESTLY THAT HOLDS, entry by entry (fix round 1, review M-6, which
-- caught this paragraph describing a @place@ projection that does not
-- exist and an assertion in @transport\/http.feature@ that was never
-- written -- in the file the header calls the proof the ruling was
-- honoured):
--
--   * GRAPH THINGS, in the full sense: @vocabulary@, @node-card@,
--     @edge-page@, @gazetteer@, @version-root@. These name nodes, edge
--     families, the declared manifests and the artifact root.
--   * PAYLOAD PROJECTIONS, named after an answer rather than a graph
--     thing: @contract@, @sources@, @xref-list@, @catechism-list@,
--     @export-format@. They are still transport-agnostic -- a second
--     carrier for cross-references would reuse @xref-list@ -- so the
--     addendum's test still passes, but the banner below claims more
--     tidiness than these five deliver, and saying so is cheaper than
--     pretending otherwise.
--
-- There is NO @place@ projection. @GET \/api\/place\/{id}@ is juncture
-- T-14 in the batch report's inventory, listed as uncovered.
--
-- The six CARTOGRAPHIC entries at the bottom (@eras@, @event@,
-- @land-mask@, @landmarks@, @narratives@, @polities@) are NOT ours to
-- author: they are map-generator's own consumed projection of us, ported
-- field-for-field from its @Steps.hs@ so that its @contracts/atlas-edge@
-- suite -- ITS expectations of US -- runs unmodified inside our gate. See
-- contracts/atlas-edge/RECEIVED.md.
module Proj
  ( Proj (..)
  , Extractor
  , project
  , shapeName
  , field1
  , fieldAlt
  , flattenField
  , projections
  , ProjName (..)
  , firstDiff
  , bounded
  , collectKey
  ) where

import Data.Aeson (Value (..))
import qualified Data.Aeson.Key as K
import qualified Data.Aeson.KeyMap as KM
import Data.Foldable (asum)
import Data.List (sort)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Vector as V
import Capture

-- `Right Nothing` is "this field is absent" (which includes present as
-- JSON `null`), and `Left` reports a SHAPE mismatch found while
-- projecting that field's own value -- distinct from the field being
-- merely absent.
type Extractor = Value -> Either Text (Maybe Value)

data Proj = Keep | Fields [(Text, Extractor)] | Each Proj

-- A shape mismatch (the tree expects an object and the provider sent an
-- array, or vice versa) used to fall through to `v` UNCHANGED upstream,
-- so it surfaced downstream only as an opaque "consumed projection X
-- differs from fixture Y" -- a strictly worse message than naming the
-- actual problem, on exactly the provider-drift case this CDC suite
-- exists to catch. `project` reports its own shape mismatches as `Left`,
-- distinct from an ordinary value mismatch against the fixture.
project :: Proj -> Value -> Either Text Value
project Keep v = Right v
project (Each p) (Array a) = Array <$> traverse (project p) a
project (Each _) v = Left ("expected an array, got " <> shapeName v)
project (Fields fs) v@(Object _) = do
  pairs <- traverse (\(k, ext) -> fmap (fmap (\pv -> (K.fromText k, pv))) (ext v)) fs
  Right (Object (KM.fromList [ p | Just p <- pairs ]))
project (Fields _) v = Left ("expected an object, got " <> shapeName v)

shapeName :: Value -> Text
shapeName (Object _) = "an object"
shapeName (Array _)  = "an array"
shapeName (String _) = "a string"
shapeName (Number _) = "a number"
shapeName (Bool _)   = "a boolean"
shapeName Null       = "null"

-- The ordinary case: keep the field named `k`, projecting its value
-- through `p`. `Just Null` is treated the same as absent.
field1 :: Text -> Proj -> (Text, Extractor)
field1 k = fieldAlt k [k]

-- The general case field1 specializes: the OUTPUT key `outKey` is sourced
-- from the FIRST of `srcKeys` that is present (and non-null) on the
-- enclosing object -- map-generator's `label` (from `title`, falling back
-- to `label`) is `fieldAlt "label" ["title", "label"] Keep`.
fieldAlt :: Text -> [Text] -> Proj -> (Text, Extractor)
fieldAlt outKey srcKeys p = (outKey, \v -> case v of
  Object o -> case asum [ nonNull (KM.lookup (K.fromText k) o) | k <- srcKeys ] of
    Nothing -> Right Nothing
    Just fv -> fmap Just (project p fv)
  _        -> Right Nothing)
  where
    nonNull (Just Null) = Nothing
    nonNull other       = other

-- A field gathered by walking a path of array-valued keys and
-- concatenating every leaf array found at the end of it -- the atlas's
-- `verses` (not a top-level field; flattened out of
-- witnesses[].verse_groups[].verses) is
-- `flattenField "verses" ["witnesses", "verse_groups", "verses"]`.
flattenField :: Text -> [Text] -> (Text, Extractor)
flattenField outKey path = (outKey, \v -> Right (Just (Array (V.fromList (flattenGather path v)))))

flattenGather :: [Text] -> Value -> [Value]
flattenGather [] v = [v]
flattenGather (k : ks) (Object o) = case KM.lookup (K.fromText k) o of
  Just (Array arr)
    | null ks   -> V.toList arr
    | otherwise -> concatMap (flattenGather ks) (V.toList arr)
  _ -> []
flattenGather _ _ = []

-- | Every value found at ANY key named `k`, anywhere in the tree, in
-- document order. The atlas's own addition to the vendored DSL, and it is
-- what makes the vocabulary laws transport-agnostic: "every \"kind\" in
-- this response names a declared node kind" has to be answerable without
-- the step knowing the response's shape, because the whole point is that
-- the SAME law holds over a node card, an edge page, a scene and a CLI
-- dump, which have four different shapes and one vocabulary.
collectKey :: Text -> Value -> [Value]
collectKey k = go
  where
    go (Object o) =
      [ v | Just v <- [KM.lookup (K.fromText k) o] ]
        ++ concatMap go (KM.elems o)
    go (Array a) = concatMap go (V.toList a)
    go _ = []

-- ===================================================================
-- THE REGISTRY
-- ===================================================================
--
-- Its universe IS the vocabulary the Vocabulary blocks publish, so a typo
-- in a feature file gets a did-you-mean naming the real projections, and
-- `contract-runner vocab` fails when this list and a feature file's
-- Vocabulary table disagree.
projections :: Map.Map Text Proj
projections = Map.fromList
  -- ---------------- GRAPH-PRIMARY (addendum ruling 1) ----------------
  -- These project the GRAPH, not a route. Every one of them is a promise
  -- about a node, an edge family, or the graph's declared vocabulary, and
  -- the transport suites assert that each transport carries them
  -- faithfully rather than restating them.
  [ ( "vocabulary"
      -- The graph's DECLARED vocabulary -- graph-types' own `kind_tags!`
      -- and `relations!` manifests, emitted into the recorded pact
      -- straight from those macros by `tests/contract_pact.rs` (no
      -- hand-copied list, so the manifest cannot drift from the types
      -- without the recorder noticing). This is the root of the primary
      -- juncture: every other projection's `kind` field, on every
      -- transport, is drawn from this set, and `every "kind" ... names a
      -- declared node kind` is checked against exactly this.
    , Fields
        [ -- The artifact format wall (fix round 1, review M-5).
          -- `artifact::load` refuses any graph.bin whose format_version is
          -- not exactly this, so a bump refuses every holder of an older
          -- artifact -- the sharpest break available here, and previously
          -- invisible to all five gate legs.
          field1 "manifest_schema" Keep
        , field1 "section_schema_version" Keep
        , field1 "node_kinds" Keep
        , field1 "relations" (Each (Fields
            [ field1 "name" Keep, field1 "forward" Keep, field1 "inverse" Keep ]))
        , field1 "symmetric" (Each (Fields
            [ field1 "name" Keep, field1 "label" Keep ]))
        ]
    )
  , ( "node-card"
      -- A node's IDENTITY surface: who it is, what kind of thing it is,
      -- what it is called, where the claim came from, which edge families
      -- it participates in and how many of each, and the atlas version
      -- root the answer was computed at. `provenance` is in the projection
      -- deliberately -- PROV-1 put it on the wire, and a projection is how
      -- a promise stops being retractable by accident.
    , Fields
        [ field1 "id" Keep
        , field1 "kind" Keep
        , field1 "label" Keep
        , field1 "provenance" Keep
        , field1 "edge_summary" (Each (Fields [ field1 "kind" Keep, field1 "count" Keep ]))
        ]
    )
  , ( "edge-page"
      -- One page of ONE edge family, and the bijection witness with it:
      -- `edge` is the id the target's own inverse-kind page carries back
      -- for the same connection, so a consumer can join both directions
      -- without guessing.
    , Fields
        [ field1 "kind" Keep
        , field1 "entries" (Each (Fields
            [ field1 "edge" Keep
            , field1 "node" (Fields [ field1 "id" Keep, field1 "kind" Keep, field1 "label" Keep ])
            ]))
        , field1 "next" Keep
        ]
    )
  , ( "gazetteer"
      -- THE COORDINATE AUTHORITY, whole. Contract C3 states it -- "atlas
      -- Place nodes are the coordinate authority; a place moving in the
      -- atlas moves every border built through it (one fact, one home)" --
      -- and map-generator vendors this exact artifact and builds geometry
      -- through it.
      --
      -- Pinned ROW BY ROW, all 1,358 of them, all coordinates, for the
      -- same reason map-generator pins our whole polity book: a silently
      -- moved place fails here before it can move a pixel of anyone's. It
      -- is a large fixture and that is the correct size for the promise --
      -- the alternative, a count or a spot check, is satisfiable by its
      -- own failure mode.
      --
      -- This is also the projection that makes an id DELETION visible.
      -- PLACE-1a absorbed 15 ids; against a consumer pinned to the older
      -- book that is 15 dangling references, and this fixture is where a
      -- future one shows up as a diff rather than as someone else's
      -- broken build.
    , Fields
        [ field1 "places" (Each (Fields
            [ field1 "id" Keep
            , fieldAlt "name" ["name", "canonical"] Keep
            , field1 "lat" Keep
            , field1 "lon" Keep
            , field1 "provenance" Keep
            ]))
        ]
    )
  , ( "version-root"
      -- C6, as a projection: every artifact and every answer that carries
      -- the atlas version root carries THE SAME one. Deliberately a
      -- one-field projection over several differently-shaped carriers --
      -- `atlas_version_root` on an export, `version` on a wire answer --
      -- so the "same root everywhere" law is one comparison, not five.
      -- `graphPin` is in the list because map-generator's own
      -- /api/contract publishes the atlas root it compiled against under
      -- that name (contracts/map-api/meta/contract.feature masks it as
      -- sixteen hex characters). Including it is what makes the C6
      -- stale-pin an EXECUTABLE cross-repo law rather than prose: their
      -- contract answer and our gazetteer can be asked, in one step,
      -- whether they were built from the same graph.
    , Fields [ fieldAlt "root" ["atlas_version_root", "version", "graphPin"] Keep ]
    )
  , ( "export-format"
      -- The half of an export's header that is a STABLE promise: which
      -- format it is in. A consumer pins this and refuses an artifact it
      -- does not understand (map-generator does exactly that).
      --
      -- `atlas_version_root` is deliberately NOT here. It changes on every
      -- recompile, so pinning it in a fixture would mean re-blessing this
      -- as a matter of routine -- and a check that is re-blessed as
      -- routine stops being read. The root gets the law it actually
      -- deserves instead: the `version-root` projection plus the
      -- "declare the same atlas version root" step, which compares two
      -- artifacts to each other and therefore fires only when they
      -- genuinely disagree.
    , Fields [ field1 "format_version" Keep ]
    )
  , ( "contract"
      -- The AQC version range this server advertises.
    , Fields [ field1 "min_version" Keep, field1 "max_version" Keep ]
    )
  , ( "sources"
      -- The source registry: every source a provenance string can resolve
      -- to, with the licence it ships under. A provenance affordance that
      -- names a source this list does not carry is a dangling reference,
      -- and this is the list that makes that statement checkable from
      -- outside the server. `license` is consumed by LICENSES.md drift
      -- checking and by anything that decides what we may redistribute, so
      -- it is a consumed field, not decoration.
      --
      -- The document's `provenances` table is deliberately NOT projected
      -- here yet. It is PROV-1's own surface and PROV-1's fix round is in
      -- flight as this lands; pinning a table another batch is actively
      -- editing would produce a red that says nothing about either batch.
      -- Recorded in CDC-1's juncture inventory as covered-next rather than
      -- left unsaid.
    , Fields
        [ field1 "sources" (Each (Fields
            [ field1 "id" Keep
            , field1 "category" Keep
            , field1 "title" Keep
            , field1 "license" Keep
            ]))
        , field1 "categories" (Each (Fields [ field1 "id" Keep, field1 "label" Keep ]))
        ]
    )
  , ( "xref-list"
      -- Cross-references out of one scripture span, as the client's
      -- Cross References section consumes them: the target reference, the
      -- vote weight it is ranked by, and the preview line it renders.
      -- An element-level projection, because the carrier is an array of
      -- elements -- which is exactly why an element may grow fields (it
      -- has its own object to grow them on) without this projection
      -- moving.
    , Each (Fields
        [ field1 "target" Keep, field1 "votes" Keep, field1 "preview" Keep ])
    )
  , ( "catechism-list"
      -- Catechism items citing one scripture span, as the Small Catechism
      -- section consumes them. `question` is optional by construction
      -- (`skip_serializing_if`), and the projection treats absent and null
      -- alike, so an item with no question title is not a drift.
    , Each (Fields
        [ field1 "id" Keep, field1 "name" Keep, field1 "question" Keep ])
    )

  -- ------- CARTOGRAPHIC: map-generator's OWN consumed projection -------
  -- Ported field-for-field from map-generator/contracts/runner/src/
  -- Steps.hs so that its `contracts/atlas-edge` suite runs unmodified
  -- here. These six are THEIRS. Changing one of them is not a refactor --
  -- it is editing another repo's expectations of us, and the right move
  -- is to break, report, and coordinate.
  , ( "polities"
    , Fields
        [ field1 "polities" (Each (Fields
            [ field1 "id" Keep
            , field1 "name" Keep
            , field1 "from" Keep
            , field1 "to" Keep
            , field1 "rings" Keep
            , field1 "color_key" Keep
            , field1 "transition" (Fields [field1 "verses" Keep])
            , field1 "fall" (Fields [field1 "verses" Keep])
            ]))
        ]
    )
  , ( "narratives"
    , Each (Fields
        [ field1 "id" Keep, field1 "name" Keep, field1 "color" Keep, field1 "legs" Keep ])
    )
  , ( "event"
    , Fields
        [ field1 "id" Keep
        , fieldAlt "label" ["title", "label"] Keep
        , field1 "when" (Fields [field1 "from_year" Keep, field1 "to_year" Keep])
        , field1 "places" (Each (Fields [field1 "id" Keep]))
        , flattenField "verses" ["witnesses", "verse_groups", "verses"]
        ]
    )
  , ( "eras"
    , Each (Fields
        [ field1 "id" Keep, field1 "name" Keep, field1 "from_year" Keep, field1 "to_year" Keep ])
    )
  , ( "landmarks"
    , Each (Fields
        [ field1 "name" Keep, field1 "kind" Keep, field1 "lat" Keep, field1 "lon" Keep ])
    )
  , ("land-mask", Fields [ field1 "rings" Keep ])
  ]

-- Its universe IS the registry -- the Vocabulary block lists every
-- projection automatically, and a typo gets a did-you-mean naming the
-- real ones.
newtype ProjName = ProjName Text deriving (Eq, Show)
instance FromCapture ProjName where
  capName _ = "projection"
  universe _ = Enumerated (Map.keys projections)
  renderCap (ProjName p) = p
  parseCap t = let s = T.strip t in
    if s `Map.member` projections then Right (ProjName s)
    else Left ("'" <> s <> "' is not a projection."
              <> didYouMean (Map.keys projections) s
              <> "\n  Projections are: " <> T.intercalate ", " (Map.keys projections))

-- The first leaf on which two JSON values disagree, as a path -- so a
-- failure says `$.places[3].lat` rather than dumping two documents.
firstDiff :: Value -> Value -> Maybe Text
firstDiff = go "$"
  where
    go path expected actual
      | expected == actual = Nothing
      | otherwise = case (expected, actual) of
          (Object oe, Object oa) ->
            let ke = sort (map K.toText (KM.keys oe))
                ka = sort (map K.toText (KM.keys oa))
            in if ke /= ka
                 then Just (path <> ": object keys differ -- fixture has ["
                            <> T.intercalate ", " ke <> "], response has ["
                            <> T.intercalate ", " ka <> "]")
                 else firstJust
                        [ go (path <> "." <> k) ve va
                        | k <- ke
                        , Just ve <- [KM.lookup (K.fromText k) oe]
                        , Just va <- [KM.lookup (K.fromText k) oa]
                        ]
          (Array ae, Array aa)
            | V.length ae /= V.length aa ->
                Just (path <> ": array length differs -- fixture has "
                      <> tshow (V.length ae) <> " element(s), response has "
                      <> tshow (V.length aa))
            | otherwise ->
                firstJust
                  [ go (path <> "[" <> tshow i <> "]") (ae V.! i) (aa V.! i)
                  | i <- [0 .. V.length ae - 1] ]
          _ -> Just (path <> ": fixture has " <> bounded expected
                     <> ", response has " <> bounded actual)
    -- Short-circuiting "any": stops at the first Just without forcing the
    -- rest of the list.
    firstJust :: [Maybe Text] -> Maybe Text
    firstJust = foldr (\x acc -> case x of Just _ -> x; Nothing -> acc) Nothing

tshow :: Show a => a -> Text
tshow = T.pack . show

-- A JSON value in an error message, truncated so a large response cannot
-- turn one failure line into the whole body.
bounded :: Value -> Text
bounded v = let s = tshow v
            in if T.length s > 120 then T.take 120 s <> "..." else s
