-- | The atlas's contract-runner entry point.
--
-- Vendored in shape from map-generator's @app/Main.hs@ -- same three
-- verbs, same UTF-8 handling, same exit discipline -- with ONE addition
-- that the whole pre-push gate rests on: @--replay@.
--
--   contract-runner check DIR                     -- totality, no I/O
--   contract-runner vocab DIR [--write]           -- vocabulary drift, no I/O
--   contract-runner run --replay PACT DIR         -- against a recorded pact
--   contract-runner run --base-url URL DIR        -- against a live server
--   contract-runner tags DIR [--forbid TAG]       -- the tag oracle
--   contract-runner grade OLD.json NEW.json       -- the fixture grader
--
-- Fix round 3 adds @run --results FILE@ and @grade@, and they exist for one
-- reason: THE GATE'S PROTECTION IS NO LONGER A NEGATIVE GUARD.
--
-- Every defence in the previous two rounds was conditioned on something an
-- author could remove: a marker file, a registry word, a file extension.
-- Delete the condition and the guard silently switched off. The gate now
-- derives the COMPLETE SET OF EXPECTATIONS from committed git content and
-- requires evidence that each one executed and passed — so @--results@ is
-- that evidence, emitted by the executor itself, one machine-readable row
-- per scenario, and reconciled against the derived inventory. A suite that
-- stops running does not shorten a list; it produces MISSING COVERAGE.
--
-- @run@ takes exactly one source. @--replay@ is not a weaker @--base-url@:
-- the pact it reads is regenerated from the real committed graph, through
-- the real Router, by a Rust test that fails on a one-byte difference, and
-- that test is part of the same gate. What @--replay@ removes is the
-- SERVER, not the evidence -- which is what lets contract disagreement
-- block a push in seconds instead of requiring two processes on two ports
-- that this machine has already spoken for.
module Main where

import Data.Aeson (Value (..), eitherDecodeStrict, (.:), (.:?), withObject)
import qualified Data.Aeson.Types as AT
import Data.Aeson.Types (parseEither)
import qualified Data.ByteString as BS
import qualified Data.Map.Strict as Map
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import qualified Data.Text.Encoding as TE
import Network.HTTP.Client (newManager, defaultManagerSettings)
import Options.Applicative
import System.Directory (listDirectory, doesDirectoryExist, doesFileExist)
import System.Exit (exitFailure, exitSuccess)
import System.FilePath ((</>), takeExtension)
import System.IO (stdout, stderr, hSetEncoding, utf8)
import Gherkin.Ast (Tag (..))
import Run
import Steps (allSteps)
import World
import qualified Check
import qualified Vocab
import qualified Tags
import qualified Grade

data Source = Live String | Replay FilePath

data Cmd
  = CmdRun { cSource :: Source, cDir :: FilePath, cBless :: Bool, cExports :: Maybe FilePath
           -- ^ where to write the per-scenario execution record the
           -- coverage reconciliation consumes (fix round 3).
           , cResults :: Maybe FilePath }
  | CmdCheck FilePath
  | CmdVocab { vDir :: FilePath, vWrite :: Bool }
  -- Fix round 3 (review H-R2-2): the fixture grader, moved out of a
  -- `python` found on PATH and into the binary the gate builds itself.
  | CmdGrade FilePath FilePath
  -- Fix round 2 (review C-NEW-1): the tag oracle. The shell guards used to
  -- grep for `@target` anchored at the start of a line, while the parser
  -- reads every whitespace-separated word on a tag line -- so `  @wip
  -- @target` disarmed a received suite and both guards saw nothing. A guard
  -- that has to parse a format must use that format's parser.
  | CmdTags { tDir :: FilePath, tForbid :: [String] }

sourceP :: Parser Source
sourceP =
      (Live <$> strOption (long "base-url" <> metavar "URL"
                           <> help "run against a live server (the fuller pass)"))
  <|> (Replay <$> strOption (long "replay" <> metavar "PACT"
                              <> help "run against a recorded pact (the pre-push gate)"))

cmd :: Parser Cmd
cmd = hsubparser
  (  command "run"   (info (CmdRun <$> sourceP
                                   <*> argument str (metavar "DIR")
                                   <*> switch (long "bless")
                                   <*> optional (strOption (long "exports" <> metavar "DIR"
                                         <> help "directory of published exports (data/exports)"))
                                   <*> optional (strOption (long "results" <> metavar "FILE"
                                         <> help "write one machine-readable row per scenario executed")))
                       (progDesc "execute a contract directory against a server or a recorded pact"))
  <> command "check" (info (CmdCheck <$> argument str (metavar "DIR"))
                       (progDesc "totality: every step matches exactly one definition"))
  <> command "vocab" (info (CmdVocab <$> argument str (metavar "DIR")
                                     <*> switch (long "write"))
                       (progDesc "verify (or --write) Vocabulary blocks against the types"))
  <> command "tags"  (info (CmdTags <$> argument str (metavar "DIR")
                                    <*> many (strOption (long "forbid" <> metavar "TAG"
                                          <> help "exit non-zero if this tag appears anywhere")))
                       (progDesc "report every scenario's tags, as the PARSER reads them"))
  <> command "grade" (info (CmdGrade <$> argument str (metavar "OLD.json")
                                     <*> argument str (metavar "NEW.json"))
                       (progDesc "grade a fixture change: same | wider | changed"))
  )

featureFiles :: FilePath -> IO [FilePath]
featureFiles dir = do
  entries <- listDirectory dir
  fmap concat . mapM walk $ [ dir </> e | e <- entries ]
  where
    walk p = do
      isDir <- doesDirectoryExist p
      if isDir then featureFiles p
      else pure [ p | takeExtension p == ".feature" ]

-- The pact's on-disk shape, parsed strictly: an entry with neither a body
-- nor a text is refused HERE, at load time, naming itself -- rather than
-- becoming a confusing per-step failure later. A malformed pact is a
-- broken gate, and a broken gate must say so before it reports on
-- anything.
-- The pact is a DIRECTORY of fragments, one per crate that owns a
-- transport (`http.json` from atlas-server, `cli.json` from atlas-cli),
-- merged here. One recorder per transport-owning crate is a smaller rule
-- than one recorder reaching across crate boundaries, and merging is the
-- cheap half of it.
loadPact :: FilePath -> IO (Map.Map T.Text PactEntry)
loadPact dir = do
  isDir <- doesDirectoryExist dir
  fragments <-
    if isDir
      then do
        names <- listDirectory dir
        pure [ dir </> n | n <- names, takeExtension n == ".json" ]
      else do
        isFile <- doesFileExist dir
        pure [ dir | isFile ]
  case fragments of
    [] -> die' ("no recorded pact at " <> T.pack dir
                <> "\n  Generate it with:\n    ATLAS_BLESS_PACT=1 cargo test -p atlas-server --test contract_pact\n    ATLAS_BLESS_PACT=1 cargo test -p atlas-cli --test contract_pact_cli")
    fs -> do
      maps <- mapM loadFragment fs
      let merged = Map.unions maps
      if Map.null merged
        then die' ("the recorded pact at " <> T.pack dir <> " carries no entries")
        else pure merged
  where
    loadFragment path = do
      raw <- BS.readFile path
      case eitherDecodeStrict raw of
        Left e -> die' ("the recorded pact fragment " <> T.pack path <> " is not JSON: " <> T.pack e)
        Right v -> case parseEither parser v of
          Left e -> die' ("the recorded pact fragment " <> T.pack path <> " is malformed: " <> T.pack e)
          Right m -> pure m
    die' msg = TIO.putStrLn msg >> exitFailure
    parser :: Value -> AT.Parser (Map.Map T.Text PactEntry)
    parser = withObject "pact" $ \o -> do
      entries <- o .: "entries"
      traverse entry entries
    entry = withObject "pact entry" $ \o -> do
      st <- o .: "status"
      body <- o .:? "body"
      text <- o .:? "text"
      pure (PactEntry st body text)

main :: IO ()
main = do
  -- This is a Windows-first project, and Windows consoles default to a
  -- legacy code page that cannot encode characters like an em dash --
  -- upstream's own note records the crash this prevents. Force UTF-8 on
  -- both output handles at the entry point so ALL of this runner's output
  -- is safe regardless of the console's code page.
  hSetEncoding stdout utf8
  hSetEncoding stderr utf8
  c <- execParser (info (cmd <**> helper) fullDesc)
  case c of
    CmdRun src dir bless exports resultsPath -> do
      w <- worldFor src dir bless exports
      files <- featureFiles dir
      -- One file at a time, so each ScenarioResult keeps the PATH it came
      -- from. `Run.runFeatureFiles` returns a flat list keyed by the
      -- feature TITLE, and two files may share a title -- which is fine for
      -- a human report and useless for a coverage reconciliation, whose
      -- whole job is to match rows against a git-derived inventory keyed by
      -- path. Pairing here rather than adding a field to `ScenarioResult`
      -- keeps `src/Run.hs` byte-identical to upstream (see VENDOR.md).
      paired <- concat <$> mapM (\p -> map ((,) p) <$> runFeatureFiles allSteps w [p]) files
      let results = map snd paired
      TIO.putStrLn (reportTable results)
      -- The execution record is written BEFORE the verdict is decided, and
      -- for a failing run as well as a passing one. A reconciliation that
      -- only ever saw the results of successful runs could not tell "this
      -- scenario failed" from "this scenario never ran", and those are the
      -- two facts it exists to distinguish.
      case resultsPath of
        Nothing -> pure ()
        -- Written as explicit UTF-8 bytes, not through the locale: this
        -- is a Windows-first project whose console code page cannot encode
        -- the em dashes and box glyphs this corpus contains, and a
        -- coverage record that dies on encoding would fail the gate for a
        -- reason that has nothing to do with contracts.
        Just rp -> BS.writeFile rp (TE.encodeUtf8 (T.unlines (map resultRow paired)))
      let reds = hardReds results
      if null reds then exitSuccess
      else TIO.putStrLn (T.pack (show (length reds)) <> " non-target failures")
           >> exitFailure
    CmdCheck dir -> Check.checkDir allSteps dir
    CmdVocab dir wr -> Vocab.vocabDir allSteps dir wr
    CmdTags dir forbid -> Tags.tagsCmd dir (map T.pack forbid)
    CmdGrade o n -> Grade.gradeCmd o n
  where
    -- `path \t scenario \t verdict \t tags`, every field percent-escaped by
    -- `Tags.escField` -- the SAME row shape the tag oracle emits, so the
    -- coverage reconciliation joins two files that agree on what a row is.
    -- Escaping is what makes `-F'\t'` a total parse (review H-R2-4).
    resultRow (p, r) =
      Tags.escField (T.pack p) <> "\t"
        <> Tags.escField (srScenario r) <> "\t"
        <> verdictWord r <> "\t"
        <> T.intercalate "," [ Tags.escField t | Tag t <- srTags r ]
    verdictWord r = case (srVerdict r, isTarget r) of
      (Passed,    False) -> "passed"
      (Passed,    True)  -> "target-met"
      (Failed _,  True)  -> "target-red"
      (Failed _,  False) -> "failed"
      (Skipped _, _)     -> "skipped"

    worldFor (Live base) dir bless ex = do
      mgr <- newManager defaultManagerSettings
      let b = T.pack base
      pure (World b (exportsFirst ex (httpTransport mgr b)) (dir </> "fixtures") Map.empty bless
                   (exportsFirstRaw ex (httpTransportRaw mgr b)) (httpTransportProbe mgr b))
    worldFor (Replay pactPath) dir bless ex = do
      pact <- loadPact pactPath
      pure (World (T.pack ("replay:" <> pactPath)) (exportsFirst ex (replayTransport pact))
                  (dir </> "fixtures") Map.empty bless
                  (exportsFirstRaw ex (replayTransportRaw pact)) (replayTransportProbe pact))
