{-# LANGUAGE ExistentialQuantification #-}
{-# LANGUAGE FlexibleInstances #-}
{-# LANGUAGE TypeSynonymInstances #-}
module BibleGraph where

import Data.Set (Set)
import Data.Time (UTCTime)
import Numeric.Natural (Natural)

type Name = String

newtype Latitude  = Latitude  Integer deriving (Eq, Ord, Show)
newtype Longitude = Longitude Integer deriving (Eq, Ord, Show)

data Location = Location
  { locationName      :: Name
  , locationLatitude  :: Latitude
  , locationLongitude :: Longitude
  } deriving (Eq, Ord, Show)

type Timestamp = UTCTime

data TimeRange = TimeRange
  { rangeStart      :: Timestamp
  , rangeEnd  :: Timestamp
  } deriving (Eq, Ord, Show)

-- | The spec's @Pair {Location, TimeRange}@.
data TimeAndPlace = TimeAndPlace
  { tapLocation  :: Location
  , tapTimeRange :: TimeRange
  } deriving (Eq, Ord, Show)

newtype Map = Map (Set TimeAndPlace) deriving (Eq, Ord, Show)

getTimeRange :: Timestamp -> Timestamp -> TimeRange
getTimeRange = undefined

class TimeAndPlaceQuery q where
  getTimeAndPlaces :: q -> Set TimeAndPlace

instance TimeAndPlaceQuery Timestamp
instance TimeAndPlaceQuery TimeRange
instance TimeAndPlaceQuery (Latitude, Longitude)
instance TimeAndPlaceQuery (Latitude, Longitude, TimeRange)
instance TimeAndPlaceQuery (Latitude, Longitude, Timestamp)

--------------------------------------------------------------------------------
-- Scripture references
--------------------------------------------------------------------------------

type BookName          = String
type ChapterNum        = Natural
type VerseNum          = Natural
type PassageStartVerse = VerseNum
type PassageEndVerse   = VerseNum

data Verse = Verse
  { verseText       :: String
  , verseBookName   :: BookName
  , verseChapterNum :: ChapterNum
  , verseNum        :: VerseNum
  } deriving (Eq, Ord, Show)

data Passage = Passage
  { passageVerses      :: Set Verse
  , passageBookName    :: BookName
  , passageChapterNum  :: ChapterNum
  , passageStartVerse  :: PassageStartVerse
  , passageEndVerse    :: PassageEndVerse
  } deriving (Eq, Ord, Show)

data Chapter = Chapter
  { chapterPassages :: Set Passage
  , chapterBookName :: BookName
  , chapterNum      :: ChapterNum
  } deriving (Eq, Ord, Show)

-- | The spec's @Pair {{Chapter}, BookName}@.
data Book = Book
  { bookChapters :: Set Chapter
  , bookName     :: BookName
  } deriving (Eq, Ord, Show)

data BibleRef
  = RefBook    Book
  | RefChapter Chapter
  | RefPassage Passage
  | RefVerse   Verse
  deriving (Eq, Ord, Show)

type Author = Name

data BibleRefMetadata = BibleRefMetadata
  { metaAuthor            :: Author
  , metaWriteLocation     :: Location
  , metaWriteTimeRange    :: TimeRange
  , metaNarrativeLocation :: Location
  , metaNarrativeTimeRange :: TimeRange
  } deriving (Eq, Ord, Show)

data RichBibleRef = RichBibleRef
  { richBibleRef      :: BibleRef
  , richBibleRefMeta  :: BibleRefMetadata
  } deriving (Eq, Ord, Show)

getBibleTimeAndPlace :: RichBibleRef -> TimeAndPlace
getBibleTimeAndPlace = undefined

--------------------------------------------------------------------------------
-- Overloaded lookup: GetBibleRefs
--------------------------------------------------------------------------------

class BibleRefQuery q where
  getBibleRefs :: q -> Set RichBibleRef

instance BibleRefQuery TimeAndPlace
instance BibleRefQuery Timestamp
instance BibleRefQuery TimeRange
instance BibleRefQuery (Latitude, Longitude)
-- Author is a type synonym for String; needs FlexibleInstances.
instance BibleRefQuery Author

--------------------------------------------------------------------------------
-- Exploration
--------------------------------------------------------------------------------

-- | @explore@ is monadic bind restricted to a set-like result. Lists are used
--   rather than 'Set' because @Set (e b)@ would demand an @Ord (e b)@ constraint
--   the class head cannot express; see the alternative below.
class Explorable e where
  explore :: e a -> (a -> [e b]) -> [e b]

-- Alternative if genuine sets are required (needs QuantifiedConstraints or a
-- per-method constraint):
--
--   class Explorable e where
--     explore :: Ord (e b) => e a -> (a -> Set (e b)) -> Set (e b)

type CrossRefs = Set RichBibleRef

-- | 'Discovery' is parameterised so it can inhabit 'Explorable', which ranges
--   over type constructors of kind @* -> *@. The payload @a@ is whatever is
--   currently in focus (a 'RichBibleRef', a 'Location', ...).
data Discovery a = Discovery
  { discoveryFocus        :: a
  , discoveryCrossRefs    :: CrossRefs
  , discoveryTimeAndPlace :: TimeAndPlace
  } deriving (Eq, Ord, Show)

instance Explorable Discovery

-- | Existential wrapper: a heterogeneous collection of explorable nodes.
data SomeExplorable = forall e a. Explorable e => SomeExplorable (e a)

newtype BibleGraph = BibleGraph [SomeExplorable]
