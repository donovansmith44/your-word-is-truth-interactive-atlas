-- | The ATLAS's deliberately-minimal stand-in for map-generator's
-- @Prop.hs@ (1,278 lines of QuickCheck generators over ITS query
-- vocabulary: years, zooms, pieces, styles, detail tiers).
--
-- Why a stand-in rather than the real thing, or rather than deleting the
-- module: 'Check' and 'Vocab' are vendored BYTE-IDENTICAL to upstream
-- (that is the whole point of stealing an executor instead of writing
-- one), and both import @Prop@ for the property-hole half of their laws.
-- Keeping the module with the same surface is what lets those two files
-- stay untouched, so a fix on either side of the seam can travel without
-- a merge conflict.
--
-- The divergence is stated in one place -- 'holeRegistry' is EMPTY -- and
-- everything else follows from it honestly rather than by pretending:
--
--   * an atlas feature file containing any @\<hole\>@ token has, by
--     definition, an UNREGISTERED hole, so @Check.classify@ reports it as
--     an ORPHAN. That is the correct answer for this repo today: we
--     publish no generators, so a scenario that asks to be fuzzed would
--     be fuzzed over nothing, and a law that never varies is a law that
--     never ran (upstream @Run.lawTally@'s own rule).
--   * 'holeDefects' therefore has nothing to report, because a scenario
--     that reaches it has already been refused by the orphan rule.
--
-- If the atlas ever wants generative contract scenarios, the honest move
-- is to take upstream's Prop.hs whole (and QuickCheck with it), not to
-- grow generators here -- see VENDOR.md.
module Prop
  ( holeRegistry
  , holesIn
  , deholeFor
  , isProperty
  , holeDefects
  , HoleDefect (..)
  ) where

import Data.Map.Strict (Map)
import qualified Data.Map.Strict as Map
import Data.Text (Text)
import qualified Data.Text as T
import Gherkin.Ast (Scenario, Tag (..))

-- | Upstream's registry maps each @\<hole\>@ token to the generator group
-- that fills it. The atlas publishes no generators, so this is empty --
-- see the module header for what follows from that.
holeRegistry :: Map Text ()
holeRegistry = Map.empty

-- | Every @"\<hole\>"@ token appearing in one raw step body, left to
-- right. Byte-identical to upstream's definition: 'Check' scans a bare
-- step body with it, and the two sides must agree on what a hole IS even
-- when they disagree about what fills one.
holesIn :: Text -> [Text]
holesIn b = case T.breakOn "<" b of
  (_, rest) | T.null rest -> []
  (_, rest) ->
    let (h, rest') = T.breakOn ">" (T.drop 1 rest)
    in h : if T.null rest' then [] else holesIn (T.drop 1 rest')

-- | Upstream's tag test, unchanged -- a scenario declares itself
-- generative by carrying @\@property@.
isProperty :: [Tag] -> Bool
isProperty tags = Tag "property" `elem` tags

-- | Upstream substitutes each hole with a representative example before
-- step matching, so a @\@property@ scenario's step still resolves to
-- exactly one definition at @check@ time. With an empty registry there is
-- nothing to substitute, so this is the identity on both branches -- and
-- the orphan rule above is what refuses the scenario instead.
deholeFor :: [Tag] -> Text -> Text
deholeFor _ = id

-- | Upstream's degenerate-hole diagnoses: a hole that takes one value
-- across a whole run, or two holes bound to the same value every
-- iteration. Neither is reachable here (no generators, no runs).
data HoleDefect = Constant Text | AlwaysEqual Text Text
  deriving (Eq, Show)

holeDefects :: Int -> Scenario -> [HoleDefect]
holeDefects _ _ = []
