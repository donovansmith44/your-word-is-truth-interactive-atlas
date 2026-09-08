-- VENDORED from map-generator/contracts/runner/src/World.hs. The
-- 'Claim' / 'StepOutcome' / 'Precondition' / 'StepDef' / 'mkStep' /
-- 'mkSkippableStep' / 'checkStatus' / 'readFeatureFile' half is
-- upstream's, verbatim, comments included -- 'Run', 'Check' and 'Vocab'
-- are vendored byte-identical and bind to exactly that surface.
--
-- TWO DIVERGENCES, both recorded in VENDOR.md:
--
-- (1) DROPPED upstream's @lastRender :: Maybe (Year, StyleName)@ and
--     @cameras :: Map Text (Center, Zoom)@ fields. They exist to carry
--     map-generator's render vocabulary between two steps of one
--     scenario; the atlas has no render step, and the types they are
--     built from are the ones "Capture" deliberately did not vendor.
--
-- (2) ADDED the recorded-pact transports, and CHANGED what a transport is
--     KEYED BY: upstream hands its transport a fully-built URL
--     (@baseUrl w <> path@); this copy hands it a REQUEST KEY -- the
--     literal text a feature file wrote, e.g. @"GET /api/eras"@ or
--     @"bibex node Place:ur_1189"@ -- and lets the transport decide what
--     that means. That one change is what makes the SAME feature file
--     runnable against a live server AND against a recorded pact, which
--     is how the atlas's pre-push gate runs in seconds without standing
--     up a server.
module World where

import Data.Aeson (Value, eitherDecodeStrict, encode)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Lazy as BL
import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.Encoding as TE
import Gherkin.Ast (Keyword)
import Capture (Universe)
import Pattern
import Network.HTTP.Client
import Network.HTTP.Types.Status (statusCode)

-- Final-review Fix 4: the ONE explicit-UTF-8 reader for a .feature file,
-- shared by every call site that reads one (Run.runFeatureFiles,
-- Check.checkDir, Vocab.vocabDir) -- there used to be two DISAGREEING
-- readers instead of one. Three sites called plain `TIO.readFile`, whose
-- text-handle decoder uses the process's LOCALE encoding, not UTF-8 --
-- verified empirically on this toolchain: the locale encoding here is
-- CP437, and `TIO.readFile` silently mangled a 3-byte UTF-8 em dash
-- (U+2014) into three separate CP437 characters instead of raising an
-- error (no exception, no warning -- a silent corruption). One shared
-- function collapses the disagreement structurally -- a future call site
-- gets the correct decode for free instead of a fresh chance to guess
-- wrong.
readFeatureFile :: FilePath -> IO Text
readFeatureFile p = TE.decodeUtf8 <$> BS.readFile p

data World = World
  { baseUrl      :: Text
  , transport    :: Text -> IO (Either Text (ByteString, Value))
  , fixtureDir   :: FilePath
  , bound        :: Map Text (ByteString, Value)
  , blessMode    :: Bool
  -- Upstream Task 11: some responses are not JSON at all -- `transport`
  -- above decodes JSON and would `Left` on them. A second, parallel
  -- transport that skips the decode entirely, for steps that need raw
  -- bytes rather than a parsed Value. The atlas uses it for the one
  -- juncture whose wire format is not JSON: bibex's stdout.
  , transportRaw :: Text -> IO (Either Text ByteString)
  -- Phase S review, fix 1: a non-2xx status used to come back as `Right`
  -- from both transports above. A check satisfiable by its own failure
  -- mode is no check. This third transport has NO status check at all:
  -- the code and the body, whatever they are -- for the laws that are
  -- ABOUT a refusal (a 400 on a malformed reference) and cannot be given
  -- either by the two transports above.
  , transportProbe :: Text -> IO (Either Text (Int, ByteString))
  }

-- `transport` is a function and has no Show instance, so World cannot
-- derive Show. Show everything except the functions, and the bound map by
-- its keys only (the values are raw response bytes + parsed JSON, not
-- useful in a failure message).
instance Show World where
  show w =
    "World { baseUrl = " <> show (baseUrl w)
      <> ", fixtureDir = " <> show (fixtureDir w)
      <> ", bound = " <> show (Map.keys (bound w))
      <> ", blessMode = " <> show (blessMode w)
      <> " }"

-- Phase S review, fix 1: extracted as a pure predicate, not inlined into
-- either IO transport, so the law itself -- "2xx or a Left naming the
-- code and the request" -- is testable without a live server.
checkStatus :: Text -> Int -> Either Text ()
checkStatus url code
  | code >= 200 && code < 300 = Right ()
  | otherwise = Left ("HTTP " <> T.pack (show code) <> " from " <> url)

-- ==================== REQUEST KEYS ====================
--
-- A request key is the literal text a feature file wrote after `When I`.
-- Two shapes exist today:
--
--   "GET /api/eras"              -- an HTTP transport juncture
--   "bibex node Place:ur_1189"   -- the CLI transport juncture
--
-- Live mode can only answer the first; replay answers both, because the
-- recorder that writes the pact runs bibex the same way it calls the
-- router. A key a transport cannot answer is a `Left`, never a skip.
httpUrlFor :: Text -> Text -> Either Text Text
httpUrlFor base key = case T.stripPrefix "GET " key of
  Just path -> Right (base <> T.strip path)
  Nothing   -> Left ("this runner is in live HTTP mode and cannot answer the non-HTTP request key '"
                     <> key <> "' -- run with --replay against a recorded pact, which carries every juncture")

httpTransport :: Manager -> Text -> Text -> IO (Either Text (ByteString, Value))
httpTransport mgr base key = case httpUrlFor base key of
  Left e -> pure (Left e)
  Right url -> do
    req <- parseRequest (T.unpack url)
    resp <- httpLbs req mgr
    let raw  = BL.toStrict (responseBody resp)
        code = statusCode (responseStatus resp)
    pure $ do
      checkStatus url code
      case eitherDecodeStrict raw of
        Right v -> Right (raw, v)
        Left e  -> Left (T.pack e <> " for " <> url)

httpTransportRaw :: Manager -> Text -> Text -> IO (Either Text ByteString)
httpTransportRaw mgr base key = case httpUrlFor base key of
  Left e -> pure (Left e)
  Right url -> do
    req <- parseRequest (T.unpack url)
    resp <- httpLbs req mgr
    let code = statusCode (responseStatus resp)
    pure $ do
      checkStatus url code
      Right (BL.toStrict (responseBody resp))

httpTransportProbe :: Manager -> Text -> Text -> IO (Either Text (Int, ByteString))
httpTransportProbe mgr base key = case httpUrlFor base key of
  Left e -> pure (Left e)
  Right url -> do
    req <- parseRequest (T.unpack url)
    resp <- httpLbs req mgr
    pure (Right (statusCode (responseStatus resp), BL.toStrict (responseBody resp)))

-- ==================== THE RECORDED PACT ====================
--
-- One entry per juncture the corpus consumes: the status the provider
-- answered with, and the body it answered. Written by the ATLAS side (a
-- Rust test that drives the real committed graph through the real axum
-- Router in-process, and the real bibex binary through its real stdout),
-- read here.
--
-- The reason this is not circular -- a pact checked against a pact proves
-- nothing -- is that the two halves are gated separately and BOTH are
-- required: the Rust recorder REGENERATES this file from the live graph
-- and fails if it differs by one byte from the committed copy (provider
-- drift), and this runner executes every published expectation against it
-- (expectation drift). Neither half can pass alone, and neither is
-- advisory.
data PactEntry = PactEntry { peStatus :: Int, peBody :: Maybe Value, peText :: Maybe Text }

-- A key the pact does not carry is a hard failure that NAMES the missing
-- key -- never a skip, never a pass. That is what makes coverage
-- self-enforcing: writing a `When I GET /api/something-new` line into a
-- feature file fails the gate until the recorder records it, so an
-- expectation can never quietly outrun the evidence behind it.
replayMiss :: Text -> Text
replayMiss key =
  "no pact entry for '" <> key <> "' -- the recorded pact is generated by `cargo test -p atlas-server --test contract_pact`; re-run it so this juncture is recorded, then commit the regenerated pact"

replayTransport :: Map Text PactEntry -> Text -> IO (Either Text (ByteString, Value))
replayTransport pact key = pure $ case Map.lookup key pact of
  Nothing -> Left (replayMiss key)
  Just e -> do
    checkStatus key (peStatus e)
    case peBody e of
      Just v  -> Right (BL.toStrict (encode v), v)
      Nothing -> Left ("pact entry '" <> key <> "' was recorded as text, not JSON -- this step needs JSON")

replayTransportRaw :: Map Text PactEntry -> Text -> IO (Either Text ByteString)
replayTransportRaw pact key = pure $ case Map.lookup key pact of
  Nothing -> Left (replayMiss key)
  Just e -> do
    checkStatus key (peStatus e)
    case (peText e, peBody e) of
      (Just t, _)        -> Right (TE.encodeUtf8 t)
      (Nothing, Just v)  -> Right (BL.toStrict (encode v))
      (Nothing, Nothing) -> Left ("pact entry '" <> key <> "' has no body at all")

replayTransportProbe :: Map Text PactEntry -> Text -> IO (Either Text (Int, ByteString))
replayTransportProbe pact key = pure $ case Map.lookup key pact of
  Nothing -> Left (replayMiss key)
  Just e -> Right (peStatus e, case (peText e, peBody e) of
    (Just t, _)        -> TE.encodeUtf8 t
    (Nothing, Just v)  -> BL.toStrict (encode v)
    (Nothing, Nothing) -> BS.empty)

-- R23 (controller ruling): a step definition's answer to "does this line
-- belong to me" is not a yes/no Maybe -- it's one of THREE outcomes, and
-- conflating two of them into a single `Just` is exactly what made two
-- genuinely different definitions look "ambiguous" over the same line
-- whenever one of them merely recognized the shape without its capture
-- actually parsing.
data Claim
  = NoMatch
    -- ^ a literal didn't match: this step does not apply to this line at
    -- all (R4's "expected literal"-prefixed case).
  | ClaimError Text
    -- ^ the shape matched (every literal was found) but a capture failed
    -- to PARSE -- a bad projection name, a kind that isn't a kind. This
    -- step recognizes the line but the value in it is bad.
  | Matched (World -> IO StepOutcome)
    -- ^ the pattern matched AND every capture parsed: the runnable
    -- action.

-- RUNNING a step has THREE outcomes, not two -- the same shape `Claim`
-- already has for MATCHING one, and for the same reason. The two-outcome
-- `Either Text World` forces an unmeetable precondition into either a
-- PASS (a vacuous green: the law didn't run, and a check satisfiable by
-- its own failure mode is no check) or a FAILURE (a red for a law that
-- isn't broken, just unexercised). Neither is true. `StepSkipped` is the
-- third, honest answer, and `Run` counts them: a law that skipped ALL of
-- its iterations is a Failed, because a law that never ran must never
-- report green.
data StepOutcome
  = StepOk World
    -- ^ the step ran and the law held on this iteration.
  | StepFailed Text
    -- ^ the step ran and the law BROKE, with the reason.
  | StepSkipped Text
    -- ^ the step could not run at all on this draw, with the precondition
    -- that went unmet. NOT a pass, NOT a failure -- counted separately.

-- The typed shape of "this law needs something the draw may not have
-- given it". Three cases, because there really are three: the world is
-- BROKEN (a genuine failure that no amount of redrawing fixes), the
-- precondition is merely UNMET on this draw (skip), or it is MET.
-- Collapsing the first two into one `Left` is exactly how a broken world
-- would come to look like a quiet skip, and how a law would stop being
-- able to fail.
data Precondition a = Broken Text | Unmet Text | Met a
  deriving (Eq, Show)

data StepDef = StepDef
  { defKw     :: Keyword
  , defSketch :: Text
  , defUses   :: [(Text, Universe)]
  , defRun    :: Text -> Claim
  }

-- The ordinary step: two outcomes, and it CANNOT skip -- the lift is
-- total and one-way, so a step written this way can never accidentally
-- report a precondition miss it never reasoned about.
mkStep :: Keyword -> StepP a -> (a -> World -> IO (Either Text World)) -> StepDef
mkStep k p f = mkSkippableStep k p (\a w -> either StepFailed StepOk <$> f a w)

-- The step whose law has a PRECONDITION: it says so in its own type, so
-- "which steps can skip" is answerable by reading their definitions
-- rather than by grepping for a magic string in an error message.
mkSkippableStep :: Keyword -> StepP a -> (a -> World -> IO StepOutcome) -> StepDef
mkSkippableStep k p f = StepDef k (renderP p) (usesOf p) $ \body ->
  case matchP p body of
    Right a -> Matched (f a)
    Left e
      -- a literal mismatch means "not this step" (try the next def);
      -- a CAPTURE failure means "this step, bad value" (report it)
      | "expected literal" `T.isPrefixOf` e -> NoMatch
      | otherwise -> ClaimError e
