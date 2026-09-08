-- | THE TAG ORACLE — what tags a scenario actually has, according to the
-- parser that actually runs it.
--
-- # Why this module exists (fix round 2, review C-NEW-1)
--
-- Fix round 1 guarded `@target` with two greps:
--
-- >   grep -rnE '^[[:space:]]*@target([[:space:]]|$)'      (contract-gate.sh)
-- >   grep -qE '^\+[[:space:]]*@target'                    (contract-semver-gate.sh)
--
-- and the parser those guards were guarding reads tags like this
-- (@Gherkin.Parse:27@, vendored byte-identical):
--
-- >   tagsOf l = [ Tag (T.drop 1 w) | w <- T.words (strip l), "@" `T.isPrefixOf` w ]
--
-- Every whitespace-separated word on a tag line is a tag, and
-- @Run.isTarget@ tests membership, not position. So @  \@wip \@target@ is a
-- @\@target@ to the executor and invisible to both greps. The reviewer
-- planted exactly that and got @CONTRACT GATE: PASSED@, exit 0, with
-- map-generator's own fixture violated.
--
-- The lesson the controller drew is the one this module implements: a guard
-- must be as wide as the thing it guards, and where a guard has to parse
-- something, IT MUST USE THE PARSER THE CONSUMER USES. A second
-- implementation of a format's lexing, in grep, is how that finding
-- happened; writing a cleverer regex would only move the goalposts to the
-- next form nobody typed yet (@\@target@ inside a Feature-level tag line, a
-- tab instead of a space, a tag line with trailing comment...).
--
-- So: no regex. This asks 'Gherkin.Parse.parseFeature' — the same function
-- 'Run', 'Check' and 'Vocab' call — and reports what it found. Both shell
-- guards consume this output instead of grepping.
--
-- Feature-level tags are included deliberately. 'Gherkin.Parse' attaches
-- tags written above @Feature:@ to 'ftTags', and while today's 'Run' only
-- consults 'scTags', a tag that the corpus can carry is a tag a future
-- runner change could honour. A guard that ignores it would be narrower
-- than the format again.
--
-- # Why the fields are ESCAPED (fix round 3, review H-R2-4)
--
-- This module was asked correctly and then its answer was RE-LEXED by a
-- second, narrower parser in the shell:
--
-- >   awk -F'\t' '{n=split($3,a,","); ...}'
--
-- A scenario NAME may contain a tab (@Gherkin.Parse@'s @Scenario: @ handler
-- takes the whole stripped remainder of the line), which shifts the tag
-- column from @$3@ to @$4@ and makes @\@target@ invisible to the semver
-- classifier. The reviewer named a scenario @alpha\<TAB\>beta gamma@ and
-- withdrew a guarantee under a MINOR bump.
--
-- Emitting a delimiter inside a delimited field is the producer's bug, not
-- the consumer's. So every field is percent-escaped here: after this, a
-- literal tab CANNOT occur inside a field, one output line is exactly one
-- scenario, and @-F'\\t'@ is a total parse rather than a guess. The shell
-- consumes the structure it was given instead of re-deriving it.
module Tags
  ( ScenarioTags (..)
  , tagsOfDir
  , tagsCmd
  , escField
  , renderRow
  ) where

import Data.List (nub, sort)
import Data.Text (Text)
import qualified Data.Text as T
import qualified Data.Text.IO as TIO
import System.Directory (doesDirectoryExist, listDirectory)
import System.Exit (exitFailure, exitSuccess)
import System.FilePath ((</>), takeExtension)

import Gherkin.Ast
import Gherkin.Parse (parseFeature)
import World (readFeatureFile)

data ScenarioTags = ScenarioTags
  { stFile     :: FilePath
  , stScenario :: Text
  , stTags     :: [Text]
  } deriving (Eq, Show)

-- | Percent-escape the four characters that could otherwise be mistaken for
-- structure by a line/tab reader: @%@ itself (so the escaping is
-- invertible), TAB, LF and CR.
--
-- CR is escaped for a reason this project has already been bitten by: the
-- runner emits a trailing CR on Windows, and the previous round's shell
-- consumer happened to strip it. "Happened to" is not a guarantee, and a CR
-- inside a scenario NAME would not have been stripped at all.
escField :: Text -> Text
escField =
  T.concatMap $ \c -> case c of
    '%'  -> "%25"
    '\t' -> "%09"
    '\n' -> "%0A"
    '\r' -> "%0D"
    _    -> T.singleton c

-- | One scenario, one line, three tab-separated ESCAPED fields:
-- @file\\tscenario\\ttag,tag@. Exported so every consumer of this runner's
-- machine output (leg 0, the semver classifier, the coverage
-- reconciliation) shares one definition of the row shape.
renderRow :: ScenarioTags -> Text
renderRow row =
  escField (T.pack (stFile row)) <> "\t"
    <> escField (stScenario row) <> "\t"
    <> T.intercalate "," (map escField (stTags row))

featureFilesIn :: FilePath -> IO [FilePath]
featureFilesIn dir = do
  isDir <- doesDirectoryExist dir
  if not isDir then pure [] else do
    entries <- listDirectory dir
    fmap (sort . concat) . mapM walk $ [ dir </> e | e <- entries ]
  where
    walk p = do
      d <- doesDirectoryExist p
      if d then featureFilesIn p else pure [ p | takeExtension p == ".feature" ]

-- | Every scenario in a directory with the tags the PARSER gives it --
-- its own plus the feature's, deduplicated and sorted.
--
-- A file that fails to parse makes the whole call fail, naming the file --
-- it is never skipped. A corpus the parser cannot read is not a corpus with
-- no forbidden tags, and returning "nothing found" for an unreadable file
-- is exactly the fail-open shape this round is removing everywhere else.
tagsOfDir :: FilePath -> IO (Either Text [ScenarioTags])
tagsOfDir dir = do
  files <- featureFilesIn dir
  results <- mapM one files
  pure $ case [ e | Left e <- results ] of
    (e : _) -> Left e
    []      -> Right (concat [ r | Right r <- results ])
  where
    one p = do
      src <- readFeatureFile p
      pure $ case parseFeature p src of
        Left e  -> Left ("cannot parse " <> T.pack p <> ": " <> e)
        Right f -> Right
          [ ScenarioTags p (scName sc)
              (dedup (map unTag (scTags sc) ++ map unTag (ftTags f)))
          | sc <- ftScenarios f ]
    unTag (Tag t) = t
    -- `nub . sort`, not a hand-rolled grouping: the first draft used
    -- `map head . groupSorted`, and `head` is partial. This project builds
    -- with -Wall and the vendored `Capture.hs` carries its own note about
    -- refusing partial functions even where a length invariant "obviously"
    -- holds. Tag lists are a handful of elements, so nub's quadratic cost
    -- is irrelevant and its totality is not.
    dedup = nub . sort

-- | @contract-runner tags DIR [--forbid TAG]...@
--
-- Prints one @file\\tscenario\\ttag,tag@ row per scenario (machine-readable
-- and stable), and exits non-zero if any forbidden tag is present anywhere,
-- naming every offending scenario.
--
-- Exits non-zero on an unparseable corpus too -- see 'tagsOfDir'.
tagsCmd :: FilePath -> [Text] -> IO ()
tagsCmd dir forbidden = do
  r <- tagsOfDir dir
  case r of
    Left e -> TIO.putStrLn ("tags: " <> e) >> exitFailure
    Right rows -> do
      mapM_ (TIO.putStrLn . renderRow) rows
      let offending = [ (row, t) | row <- rows, t <- stTags row, t `elem` forbidden ]
      if null offending then exitSuccess
      else do
        mapM_ (\(row, t) -> TIO.putStrLn
                ("FORBIDDEN TAG @" <> t <> " on scenario '" <> escField (stScenario row)
                 <> "' in " <> escField (T.pack (stFile row)))) offending
        exitFailure
