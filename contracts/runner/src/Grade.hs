-- | THE FIXTURE GRADER — the three-way judgement that turns a fixture diff
-- into a semver class, moved OUT of the shell and INTO the compiled binary
-- the gate already builds from vendored source.
--
-- # Why this module exists (fix round 3, review H-R2-2)
--
-- The judgement used to be delegated to whatever @python@ was first on
-- @PATH@, with the two JSON documents passed in @argv@:
--
-- > grade_pair() { python -c "...json.loads(sys.argv[1])..." "$1" "$2"; }
--
-- Round 1's review shadowed @python@ with two lines (@echo same@) and every
-- re-bless graded PATCH. Round 2 answered that by CALIBRATING the grader —
-- asking it three questions with known answers before trusting it. Round
-- 2's review then wrote nine lines that answer the quiz:
--
-- > case "$old|$new" in
-- >   '{"a":1}|{"a":1}')       echo same ;;
-- >   '{"a":1}|{"a":1,"b":2}') echo wider ;;
-- >   '{"a":1}|{"a":2}')       echo changed ;;
-- >   *)                       echo same ;;
-- > esac
--
-- ...and graded a total fixture re-bless PATCH against the real repo. The
-- pairs were HARDCODED CONSTANTS in the shipped source, in every clone.
--
-- The lesson is not "calibrate harder" — a constant quiz guards three
-- spellings of discrimination, not discrimination. It is that a fifteen-line
-- tree comparison never needed a general-purpose interpreter found by a
-- @PATH@ lookup. Doing it here removes the entire shellable grader from the
-- gate's trusted base: there is no longer a program the gate consults that
-- an attacker can put earlier on @PATH@.
--
-- The three grades are stated from the point of view of a CONSUMER OF THE
-- PROMISES, which is what the suite's semver means:
--
--   [@same@]    the parsed values are equal. A pure reformat pins exactly
--               what it pinned -> PATCH.
--   [@wider@]   every key/element the old fixture pinned is still pinned to
--               the SAME value, and the new one pins more -> MINOR. Nothing
--               a consumer relied on moved; the fixture promises more.
--   [@changed@] anything else: a pinned value was re-blessed, or a pinned
--               key disappeared -> MAJOR.
--
-- Documents are read from FILES, not from @argv@. The shell version passed
-- whole JSON bodies as command-line arguments, which on a large fixture is
-- an argument-length limit waiting to happen and, worse, a quoting surface.
module Grade
  ( Grade (..)
  , gradeValues
  , widened
  , gradeCmd
  ) where

import qualified Data.Aeson as A
import qualified Data.Aeson.KeyMap as KM
import qualified Data.ByteString as BS
import qualified Data.Text.IO as TIO
import qualified Data.Vector as V
import System.Exit (exitFailure, exitSuccess)

data Grade = Same | Wider | Changed deriving (Eq, Show)

-- | Is @b@ a WIDENING of @a@ — everything @a@ pinned still pinned
-- identically, and possibly more besides?
--
-- Note the list rule: same length, element-wise. A list is a pinned
-- SEQUENCE in this corpus (the era table, the landmark list), so a
-- different length is a different promise, not a wider one. Appending an
-- era is a change a consumer iterating the table can see.
widened :: A.Value -> A.Value -> Bool
widened (A.Object ao) (A.Object bo) =
  all (\(k, v) -> maybe False (widened v) (KM.lookup k bo)) (KM.toList ao)
widened (A.Array av) (A.Array bv) =
  V.length av == V.length bv && and (V.zipWith widened av bv)
widened a b = a == b

gradeValues :: A.Value -> A.Value -> Grade
gradeValues a b
  | a == b        = Same
  | widened a b   = Wider
  | otherwise     = Changed

-- | @contract-runner grade OLD.json NEW.json@ — prints @same@, @wider@ or
-- @changed@ and exits 0; prints @unreadable@ and exits non-zero when either
-- document will not parse.
--
-- Unparseable is its OWN answer and its own exit code, deliberately. The
-- shell version collapsed a parse failure into @changed@, which is the safe
-- direction but tells the caller a falsehood about what happened; the caller
-- here refuses to classify rather than inventing a class.
gradeCmd :: FilePath -> FilePath -> IO ()
gradeCmd oldPath newPath = do
  oldRaw <- BS.readFile oldPath
  newRaw <- BS.readFile newPath
  case (A.eitherDecodeStrict oldRaw, A.eitherDecodeStrict newRaw) of
    (Right a, Right b) -> TIO.putStrLn (word (gradeValues a b)) >> exitSuccess
    _                  -> TIO.putStrLn "unreadable" >> exitFailure
  where
    word Same    = "same"
    word Wider   = "wider"
    word Changed = "changed"
