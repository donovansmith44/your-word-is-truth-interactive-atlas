-- VENDORED from map-generator/contracts/runner/src/Capture.hs, PRUNED to
-- its domain-neutral core. Everything below this header is upstream's,
-- verbatim, comments included.
--
-- WHAT WAS DROPPED, and why (VENDOR.md carries the same list): upstream's
-- @Piece@, @PieceSet@, @StyleName@, @Year@, @Center@, @Zoom@,
-- @DetailTier@ and @ScaleQual@ are map-generator's OWN query vocabulary,
-- with its server's constants transcribed into them (style names, zoom
-- clamps, LOD tiers, the camera's latitude clamp). Carrying them into the
-- atlas would put a second system's tuned constants in this repo where
-- nothing can falsify them, which is worse than a fork: it is a fork that
-- looks authoritative. The atlas's own capture types live in "Proj" and
-- "Steps", declared against the atlas's own vocabulary.
--
-- WHAT WAS KEPT is the part that is about CAPTURING, not about maps: the
-- 'Universe'/'FromCapture' pair that makes a step's parameter space
-- self-describing (which is what @vocab@ checks), the one sentence a
-- Vocabulary table reads, the did-you-mean, and 'FixtureRef'.
--
-- An explicit export list, so the round-trip laws can quantify over the
-- WHOLE type (via Arbitrary) rather than only over values that `parseCap`
-- happened to produce -- which is the set `parseCap` accepts, making the
-- law circular. Constructors are exported deliberately for that reason.
module Capture
  ( Universe (..)
  , FromCapture (..)
  , describeUniverse
  , didYouMean
  , editDistance
  , FixtureRef (..)
  ) where

import Data.List (sortOn)
import Data.Text (Text)
import qualified Data.Text as T

data Universe = Enumerated [Text] | Ranged Text Text Text | Described Text
  deriving (Eq, Show)

class FromCapture a where
  capName   :: proxy a -> Text
  parseCap  :: Text -> Either Text a
  renderCap :: a -> Text
  universe  :: proxy a -> Universe

-- The one sentence a dummy reads in the Vocabulary table.
describeUniverse :: Universe -> Text
describeUniverse (Enumerated vs)   = "any of: " <> T.intercalate ", " vs
describeUniverse (Ranged lo hi m)  = "whole number from " <> lo <> " to " <> hi <> " (" <> m <> ")"
describeUniverse (Described d)     = d

didYouMean :: [Text] -> Text -> Text
didYouMean vocab w =
  case sortOn (editDistance w) vocab of
    (best : _) | editDistance w best <= 3 -> "  Did you mean: " <> best <> "?"
    _ -> ""

-- Bounded-time edit distance (Final review cleanup, Fix 1): the previous
-- definition was the naive recursive Levenshtein, exponential in input
-- length -- fine for the short garbage a real bad step produces, but a
-- pathological long input (a garbage feature-file value) could hang
-- `check`/`run` outright, and QuickCheck's own size ramp already had to
-- be capped (`resize 8` in Spec.hs) just to keep ONE property test from
-- hitting it. Fixed the algorithm itself, not the caller: this is the
-- textbook Wagner-Fischer dynamic-programming table, built one row per
-- character of `b` via `scanl` (each row reuses the previous row's
-- values, so no cell is ever recomputed) -- O(length a * length b) time,
-- same edit-distance definition (unit insert/delete/substitute cost) as
-- the naive version it replaces, so genuine near-misses score exactly as
-- before.
editDistance :: Text -> Text -> Int
editDistance ta tb = last (foldl transform [0 .. length a] b)
  where
    a = T.unpack ta
    b = T.unpack tb
    -- Every row this produces has length `length a + 1` (never []), but
    -- that's a length invariant GHC can't see -- `head`/`tail` would
    -- compile clean structurally but trip -Wx-partial (this project
    -- builds with -Wall), and a bare `(x:xs')` function-clause pattern
    -- with no `[]` equation trips -Wincomplete-patterns. An exhaustive
    -- case with an unreachable-in-practice `[]` branch satisfies both
    -- warnings honestly, without pretending the empty case is possible.
    transform row c = case row of
      [] -> []
      (x : xs') -> scanl step (x + 1) (zip3 a row xs')
      where
        -- diag (x') carries the substitution cost; above (y, one column
        -- back in the row just finished) carries a plain +1. Getting
        -- these two swapped type-checks fine and still terminates, just
        -- computes the wrong number -- checked against known distances
        -- (e.g. "kitten" -> "sitting" = 3) in the test added alongside
        -- this fix, not just eyeballed.
        step z (ca, diag, above) = minimum [above + 1, z + 1, diag + fromEnum (ca /= c)]

-- ---------- FixtureRef ----------
newtype FixtureRef = FixtureRef Text deriving (Eq, Show)

instance FromCapture FixtureRef where
  capName _ = "fixture"
  universe _ = Described "a named answer in fixtures/, e.g. \"polities-consumed\""
  renderCap (FixtureRef f) = "\"" <> f <> "\""
  parseCap t =
    let s = T.strip t
    in if "\"" `T.isPrefixOf` s && "\"" `T.isSuffixOf` s && T.length s >= 2
         then Right (FixtureRef (T.dropEnd 1 (T.drop 1 s)))
         else Left "a fixture reference is quoted, e.g. \"polities-consumed\""
