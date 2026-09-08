-- | THE ATLAS'S STEP DEFINITIONS -- the whole of what an atlas contract
-- feature file is allowed to say.
--
-- This module is the ONE part of the runner that is not vendored. Owner
-- order CDC-1.3 draws the line exactly here: "thou shalt steal the haskell
-- gherkin parser ... and write OUR OWN set of contract tests". The
-- executor is map-generator's; the vocabulary is ours, because a
-- vocabulary is an expectation and we are the author of our own.
--
-- Two rules shape the list, both from the addendum:
--
--   * THE GRAPH IS PRIMARY. There is no step that names an endpoint's
--     shape. There is a step that fetches over a transport, and there are
--     steps that state a law about the GRAPH's consumed projection -- so a
--     law written once holds over every transport that carries it, and
--     adding a transport adds a `When`, never a `Then`.
--
--   * A LAW THAT CANNOT FAIL IS NOT A LAW. Every step below either
--     compares against a committed fixture, or checks a value against the
--     graph's own declared vocabulary. None of them assert non-emptiness
--     and stop, which is the shape that passes against a server that has
--     nothing to say.
module Steps (allSteps) where

import Data.Aeson (Value (..), eitherDecodeStrict)
import Data.Aeson.Encode.Pretty (encodePretty)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Lazy as BL
import Data.List (nub, sort)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import System.Directory (doesFileExist)
import System.FilePath ((</>))
import Capture
import Gherkin.Ast (Keyword (..))
import Pattern
import Proj
import World

-- ===================== CAPTURE TYPES =====================
--
-- Each one publishes its own universe, which is what `contract-runner
-- vocab` verifies a feature file's Vocabulary block against. A capture
-- with a `Described` universe is a free-form value; a capture with an
-- `Enumerated` universe cannot be typo'd without a did-you-mean.

newtype UrlPath = UrlPath Text deriving (Eq, Show)
instance FromCapture UrlPath where
  capName _ = "path"
  universe _ = Described "an absolute API path, e.g. /api/eras or /api/node/Place:ur_1189"
  renderCap (UrlPath p) = p
  parseCap t =
    let s = T.strip t
    in if "/" `T.isPrefixOf` s then Right (UrlPath s)
       else Left ("'" <> s <> "' is not an API path -- a path starts with '/', e.g. /api/eras")

newtype BindName = BindName Text deriving (Eq, Show)
instance FromCapture BindName where
  capName _ = "name"
  universe _ = Described "a name this scenario binds an answer under, e.g. first"
  renderCap (BindName n) = n
  parseCap t =
    let s = T.strip t
    in if not (T.null s) && T.all (\c -> c == '-' || c == '_' || c `elem` ['a'..'z'] || c `elem` ['A'..'Z'] || c `elem` ['0'..'9']) s
         then Right (BindName s)
         else Left ("'" <> s <> "' is not a binding name -- letters, digits, '-' and '_' only")

newtype FieldName = FieldName Text deriving (Eq, Show)
instance FromCapture FieldName where
  capName _ = "field"
  universe _ = Described "a JSON field name, e.g. kind"
  renderCap (FieldName f) = f
  parseCap t =
    let s = T.strip t
    in if not (T.null s) && not (T.any (== '"') s) then Right (FieldName s)
       else Left "a field name is a bare JSON key, without quotes"

newtype ArgLine = ArgLine Text deriving (Eq, Show)
instance FromCapture ArgLine where
  capName _ = "arguments"
  universe _ = Described "the arguments bibex is invoked with, e.g. kinds"
  renderCap (ArgLine a) = a
  parseCap t = let s = T.strip t in
    if T.null s then Left "bibex needs at least one argument" else Right (ArgLine s)

-- The three C2/C3 export artifacts, by name. Enumerated on purpose: these
-- are published files with published consumers (map-generator vendors two
-- of them), so a feature file naming a fourth is a typo, not a new
-- artifact.
newtype ExportName = ExportName Text deriving (Eq, Show)

exportNames :: [Text]
exportNames = ["gazetteer", "chronology", "kretzmann-chronology"]

instance FromCapture ExportName where
  capName _ = "export"
  universe _ = Enumerated exportNames
  renderCap (ExportName e) = e
  parseCap t = let s = T.strip t in
    if s `elem` exportNames then Right (ExportName s)
    else Left ("'" <> s <> "' is not a published export."
              <> didYouMean exportNames s
              <> "\n  Exports are: " <> T.intercalate ", " exportNames)

newtype StatusCode = StatusCode Int deriving (Eq, Show)
instance FromCapture StatusCode where
  capName _ = "status"
  universe _ = Ranged "400" "599" "the refusal a malformed request must get"
  renderCap (StatusCode c) = T.pack (show c)
  parseCap t = case reads (T.unpack (T.strip t)) of
    [(c, "")] | c >= 400 && c < 600 -> Right (StatusCode c)
    [(c, "")] -> Left ("HTTP " <> T.pack (show (c :: Int)) <> " is not a refusal -- this step states a law about a REFUSAL, and a 2xx would satisfy it vacuously")
    _ -> Left ("'" <> T.strip t <> "' is not an HTTP status code")

-- ===================== HELPERS =====================

-- Every fetch goes through here, so "what did the last step fetch" has one
-- answer and one failure message. `_last` is the implicit binding every
-- `the response ...` law reads.
fetchInto :: Text -> Text -> World -> IO (Either Text World)
fetchInto name key w = do
  r <- transport w key
  pure $ case r of
    Left e   -> Left e
    Right rv -> Right w { bound = Map.insert name rv (Map.insert "_last" rv (bound w)) }

boundValue :: Text -> World -> Either Text Value
boundValue n w = case Map.lookup n (bound w) of
  Nothing -> Left (if n == "_last"
                     then "no response yet -- this law reads the last answer, and this scenario has not fetched one"
                     else "nothing is bound under '" <> n <> "' in this scenario")
  Just (_, v) -> Right v

fixturePath :: World -> Text -> String -> FilePath
fixturePath w f ext = fixtureDir w </> T.unpack f <> ext

loadJsonFixture :: World -> Text -> IO (Either Text Value)
loadJsonFixture w f = do
  let path = fixturePath w f ".json"
  ok <- doesFileExist path
  if not ok then pure (Left ("no fixture " <> f <> " at " <> T.pack path))
  else do
    raw <- BS.readFile path
    pure $ case eitherDecodeStrict raw of
      Right v -> Right v
      Left e  -> Left ("fixture " <> f <> " is not JSON: " <> T.pack e)

-- Compare a projected value against its fixture, or (in --bless mode)
-- write it. Bless writes the PROJECTED value, not the raw response -- the
-- fixture pins what a consumer consumes, not everything the provider
-- happens to send.
settleAgainstFixture :: Text -> Text -> Value -> World -> IO (Either Text World)
settleAgainstFixture label f got w
  | blessMode w = do
      BS.writeFile (fixturePath w f ".json") (BL.toStrict (encodePretty got))
      pure (Right w)
  | otherwise = do
      fx <- loadJsonFixture w f
      pure $ case fx of
        Left e -> Left e
        Right expected
          | got == expected -> Right w
          | otherwise -> Left (label <> " differs from fixture " <> f <> ": "
                               <> maybe "(no leaf difference found)" id (firstDiff expected got))

projectBound :: Text -> Text -> World -> Either Text Value
projectBound bindName pn w = do
  actual <- boundValue bindName w
  case Map.lookup pn projections of
    Nothing -> Left ("unknown projection " <> pn)
    Just p -> case project p actual of
      Left e -> Left ("consumed projection " <> pn <> ": " <> e)
      Right got -> Right got

-- THE GRAPH'S DECLARED VOCABULARY, fetched through the same transport as
-- everything else. Emitted into the recorded pact by
-- `server/atlas-server/tests/contract_pact.rs` directly from graph-types'
-- `kind_tags!` and `relations!` macros, so this is the types' own answer,
-- not a list anyone maintains by hand.
vocabKey :: Text
vocabKey = "graph vocabulary"

data GraphVocab = GraphVocab { gvKinds :: [Text], gvFamilies :: [Text] }

readVocab :: World -> IO (Either Text GraphVocab)
readVocab w = do
  r <- transport w vocabKey
  pure $ case r of
    Left e -> Left ("cannot read the graph's declared vocabulary: " <> e)
    Right (_, v) -> case (strings (dig "node_kinds" v), families v) of
      ([], _) -> Left "the graph vocabulary declares no node kinds -- refusing to check anything against an empty vocabulary"
      (_, []) -> Left "the graph vocabulary declares no edge families -- refusing to check anything against an empty vocabulary"
      (ks, fs) -> Right (GraphVocab ks fs)
  where
    dig k (Object o) = maybe Null id (lookupKey k (Object o))
    dig _ _ = Null
    families v = sort (nub (concat
      ([ strings (dig "forward" e) ++ strings (dig "inverse" e) | e <- arr (dig "relations" v) ]
       ++ [ strings (dig "label" e) | e <- arr (dig "symmetric" v) ])))
    arr (Array a) = foldr (:) [] a
    arr _ = []
    strings (Array a) = [ s | String s <- foldr (:) [] a ]
    strings (String s) = [s]
    strings _ = []

-- The first value found under key `k`, at any depth. Used only for the
-- one-field `version-root` projection and the vocabulary's own top-level
-- lists, both of which carry the key exactly once.
lookupKey :: Text -> Value -> Maybe Value
lookupKey k v = case collectKey k v of
  (x : _) -> Just x
  []      -> Nothing

-- The shared body of the two vocabulary laws. Quantified over EVERY
-- occurrence of the field anywhere in the answer, at any depth, because
-- the same promise has to hold for a kind on a card, a kind on an edge
-- page entry, and a kind nested in a summary row -- three shapes, one
-- vocabulary.
checkVocabField :: Text -> (GraphVocab -> [Text]) -> Text -> World -> IO (Either Text World)
checkVocabField what pick fieldName w = do
  gv <- readVocab w
  pure $ do
    vocab <- gv
    actual <- boundValue "_last" w
    let found = [ s | String s <- collectKey fieldName actual ]
        declared = pick vocab
        strays = nub [ s | s <- found, s `notElem` declared ]
    if null found
      then Left ("no \"" <> fieldName <> "\" field anywhere in this answer -- this law would pass vacuously, so it fails instead")
      else if null strays then Right w
      else Left (T.pack (show (length strays)) <> " value(s) of \"" <> fieldName
                 <> "\" are not a declared " <> what <> ": "
                 <> T.intercalate ", " (map (\s -> "'" <> s <> "'") strays)
                 <> "\n  declared " <> what <> "s are: " <> T.intercalate ", " declared)

-- ===================== THE STEPS =====================

allSteps :: [StepDef]
allSteps =
  -- ---------------- TRANSPORT: how an answer is obtained ----------------
  -- Three transports, three `When`s, and NOTHING else in this module knows
  -- which one was used. That is the addendum's ruling made operational:
  -- the laws below are stated over the graph's projection and are blind to
  -- the carrier.
  [ mkStep When (lit "I GET " *> ((,) <$> capUntil @UrlPath " as " <*> capRest @BindName)) $
      \(UrlPath path, BindName n) w -> fetchInto n ("GET " <> path) w

  , mkStep When (lit "I GET " *> capRest @UrlPath) $
      \(UrlPath path) w -> fetchInto "_last" ("GET " <> path) w

  , mkStep When (lit "I read the " *> ((,) <$> capUntil @ExportName " export as " <*> capRest @BindName)) $
      \(ExportName e, BindName n) w -> fetchInto n ("export " <> e) w

  , mkStep When (lit "I read the " *> capUntil @ExportName " export") $
      \(ExportName e) w -> fetchInto "_last" ("export " <> e) w

  , mkStep When (lit "I read the graph's declared vocabulary") $
      \() w -> fetchInto "_last" vocabKey w

    -- bibex's stdout is a contract too (the transcript idiom already
    -- treats it as one). It is not JSON, so it rides the raw transport and
    -- binds under its own name -- a `Then` about the graph's projection
    -- cannot be pointed at it by accident.
  , mkStep When (lit "I run bibex " *> capRest @ArgLine) $
      \(ArgLine args) w -> do
        r <- transportRaw w ("bibex " <> args)
        pure $ case r of
          Left e -> Left e
          Right bs -> Right w { bound = Map.insert "_out" (bs, String (TE.decodeUtf8 bs)) (bound w) }

  -- ---------------- THE PRIMARY LAW: the consumed projection ------------
  -- Deliberately the SAME phrasing map-generator's atlas-edge suite uses,
  -- so that suite runs here unmodified. One sentence, one meaning, both
  -- repos.
  , mkStep Then (lit "the consumed projection " *> ((,) <$> capUntil @ProjName " equals fixture "
                                                       <*> capRest @FixtureRef)) $
      \(ProjName pn, FixtureRef f) w -> case projectBound "_last" pn w of
        Left e -> pure (Left e)
        Right got -> settleAgainstFixture ("consumed projection " <> pn) f got w

  , mkStep Then (lit "" *> ((,,) <$> capUntil @BindName "'s consumed projection "
                                 <*> capUntil @ProjName " equals fixture "
                                 <*> capRest @FixtureRef)) $
      \(BindName n, ProjName pn, FixtureRef f) w -> case projectBound n pn w of
        Left e -> pure (Left e)
        Right got -> settleAgainstFixture (n <> "'s consumed projection " <> pn) f got w

    -- The same law over TWO answers instead of an answer and a fixture:
    -- two carriers of one graph fact must project to the same value. This
    -- is the step that makes "transports inherit from the graph suite"
    -- enforceable rather than aspirational -- it needs no fixture at all,
    -- so it cannot be satisfied by re-blessing.
  , mkStep Then (lit "" *> ((,,) <$> capUntil @BindName " and "
                                 <*> capUntil @BindName " agree on the consumed projection "
                                 <*> capRest @ProjName)) $
      \(BindName a, BindName b, ProjName pn) w -> pure $ do
        va <- projectBound a pn w
        vb <- projectBound b pn w
        if va == vb then Right w
        else Left ("two transports disagree about the graph's " <> pn <> " projection: "
                   <> maybe "(no leaf difference found)" id (firstDiff va vb))

  -- ---------------- VOCABULARY: the graph's declared families ----------
  , mkStep Then (lit "every \"" *> capUntil @FieldName "\" in the answer names a declared node kind") $
      \(FieldName f) w -> checkVocabField "node kind" gvKinds f w

  , mkStep Then (lit "every \"" *> capUntil @FieldName "\" in the answer names a declared edge family") $
      \(FieldName f) w -> checkVocabField "edge family" gvFamilies f w

  -- ---------------- WHOLE-ANSWER AND REFUSAL LAWS ----------------------
  , mkStep Then (lit "the response equals fixture " *> capRest @FixtureRef) $
      \(FixtureRef f) w -> case boundValue "_last" w of
        Left e -> pure (Left e)
        Right v -> settleAgainstFixture "the response" f v w

  , mkStep Then (lit "the response is the empty list") $
      \() w -> pure $ do
        v <- boundValue "_last" w
        case v of
          Array a | null a -> Right w
          _ -> Left ("expected the empty list, got " <> bounded v)

    -- A refusal is a promise too, and it is the one promise the ordinary
    -- transports structurally cannot check (they turn a non-2xx into a
    -- transport failure). This one goes through the probe.
  , mkStep Then (lit "GET " *> ((,) <$> capUntil @UrlPath " is refused with " <*> capRest @StatusCode)) $
      \(UrlPath path, StatusCode code) w -> do
        r <- transportProbe w ("GET " <> path)
        pure $ case r of
          Left e -> Left e
          Right (got, _)
            | got == code -> Right w
            | otherwise -> Left ("expected HTTP " <> T.pack (show code) <> " from " <> path
                                 <> ", got HTTP " <> T.pack (show got))

  -- ---------------- C6: one root, everywhere ---------------------------
  -- Every artifact and every answer that carries the atlas version root
  -- must carry the SAME one. Stated over two bound answers rather than
  -- against a literal, so it keeps holding across every re-compile
  -- without anyone re-blessing a hash.
  , mkStep Then (lit "" *> ((,) <$> capUntil @BindName " and "
                                <*> capUntil @BindName " declare the same atlas version root")) $
      \(BindName a, BindName b) w -> pure $ do
        va <- projectBound a "version-root" w
        vb <- projectBound b "version-root" w
        ra <- rootOf a va
        rb <- rootOf b vb
        if ra == rb then Right w
        else Left ("version-root drift: " <> a <> " declares " <> ra
                   <> " but " <> b <> " declares " <> rb
                   <> " -- one of these was compiled against a different graph")

  -- ---------------- bibex's stdout -------------------------------------
  , mkStep Then (lit "the output equals fixture " *> capRest @FixtureRef) $
      \(FixtureRef f) w -> do
        let path = fixturePath w f ".txt"
        case Map.lookup "_out" (bound w) of
          Nothing -> pure (Left "no bibex output yet -- this law reads the last CLI run, and this scenario has not made one")
          Just (bs, _)
            | blessMode w -> BS.writeFile path bs >> pure (Right w)
            | otherwise -> do
                ok <- doesFileExist path
                if not ok then pure (Left ("no fixture " <> f <> " at " <> T.pack path))
                else do
                  expected <- BS.readFile path
                  pure $ if normalize expected == normalize bs then Right w
                         else Left ("bibex output differs from fixture " <> f
                                    <> "\n  --- expected ---\n" <> clip (TE.decodeUtf8 expected)
                                    <> "\n  --- actual ---\n" <> clip (TE.decodeUtf8 bs))
  ]
  where
    -- Line endings are a checkout property on Windows, not a contract
    -- property: a fixture that fails because git handed it CRLF would be a
    -- gate that cries wolf, and a gate that cries wolf gets disabled.
    -- Trailing whitespace per line, likewise. Nothing else is normalized.
    normalize = BS.filter (/= 13)
    clip t = let s = T.strip t in if T.length s > 400 then T.take 400 s <> "..." else s
    rootOf who v = case lookupKey "root" v of
      Just (String s) -> Right s
      _ -> Left (who <> " declares no atlas version root (no `atlas_version_root` and no `version` field)")
