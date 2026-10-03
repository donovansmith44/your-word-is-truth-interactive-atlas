use std::collections::{BTreeMap, BTreeSet};

use atlas_graph_types::passage::{Passage, PassageError, PassageMark, ReadingOrder};
use atlas_graph_types::text::{BibleTag, ConcordRef, ConcordTag, Corpus, VerseRef};
use proptest::prelude::*;

const MOST_BOOKS: u8 = 3;
const MOST_CHAPTERS: u16 = 3;
const MOST_UNITS_PER_CHAPTER: u16 = 4;

proptest! {
    #[test]
    fn containment_is_a_partial_order_and_agrees_with_unit_inclusion(drawn in any_corpora_and_spans(3)) {
        // Arrange
        let (bible, concord) = drawn;

        // Act
        let bible_findings = containment_findings(&bible);
        let concord_findings = containment_findings(&concord);

        // Assert
        prop_assert_eq!((bible_findings, concord_findings), (ContainmentFindings::LAWFUL, ContainmentFindings::LAWFUL));
    }

    #[test]
    fn join_is_commutative_associative_idempotent_and_the_least_upper_bound(drawn in any_corpora_and_spans(3)) {
        // Arrange
        let (bible, concord) = drawn;

        // Act
        let bible_findings = join_findings(&bible);
        let concord_findings = join_findings(&concord);

        // Assert
        prop_assert_eq!((bible_findings, concord_findings), (JoinFindings::LAWFUL, JoinFindings::LAWFUL));
    }

    #[test]
    fn join_is_undefined_exactly_when_a_unit_separates_the_spans(drawn in any_corpora_and_spans(2)) {
        // Arrange
        let (bible, concord) = drawn;

        // Act
        let bible_join = joined_units(&bible);
        let concord_join = joined_units(&concord);

        // Assert
        prop_assert_eq!((bible_join, concord_join), (union_when_touching(&bible), union_when_touching(&concord)));
    }

    #[test]
    fn joining_one_span_twice_makes_one_passage_whose_mark_is_the_greater(drawn in any_corpora_and_spans(1), marks in (any_mark(), any_mark())) {
        // Arrange
        let (bible, concord) = drawn;

        // Act
        let bible_joined = rejoined(&bible, marks);
        let concord_joined = rejoined(&concord, marks);

        // Assert
        prop_assert_eq!((bible_joined, concord_joined), (remarked(&bible, marks.0.max(marks.1)), remarked(&concord, marks.0.max(marks.1))));
    }

    #[test]
    fn a_one_unit_span_is_not_a_passage(drawn in any_corpora_and_units()) {
        // Arrange
        let (bible, concord) = drawn;

        // Act
        let bible_refused = Passage::new(bible.unit(), bible.unit(), PassageMark::Plain, &bible.spine);
        let concord_refused = Passage::new(concord.unit(), concord.unit(), PassageMark::Plain, &concord.spine);

        // Assert
        prop_assert_eq!((bible_refused, concord_refused), (Err(PassageError::OneUnit), Err(PassageError::OneUnit)));
    }

    #[test]
    fn a_span_whose_first_unit_follows_its_last_is_backwards(drawn in any_corpora_and_spans(1)) {
        // Arrange
        let (bible, concord) = drawn;

        // Act
        let bible_refused = reversed(&bible);
        let concord_refused = reversed(&concord);

        // Assert
        prop_assert_eq!((bible_refused, concord_refused), (Err(PassageError::Backwards), Err(PassageError::Backwards)));
    }

    #[test]
    fn a_span_with_an_end_off_the_spine_is_refused(drawn in any_corpora_and_units()) {
        // Arrange
        let (bible, _) = drawn;
        let off_the_spine = VerseRef { book: MOST_BOOKS + 1, chapter: 1, verse: 1 };

        // Act
        let refused = [
            Passage::new(bible.unit(), off_the_spine.clone(), PassageMark::Plain, &bible.spine),
            Passage::new(off_the_spine, bible.unit(), PassageMark::Plain, &bible.spine),
        ];

        // Assert
        prop_assert_eq!(refused, [Err(PassageError::OffTheSpine), Err(PassageError::OffTheSpine)]);
    }
}

#[derive(Clone, Debug)]
struct Spine<C: Corpus> {
    units: Vec<C::Ref>,
}

impl<C: Corpus> ReadingOrder<C> for Spine<C> {
    fn position(&self, unit: &C::Ref) -> Option<usize> {
        self.units.iter().position(|placed| placed == unit)
    }
}

#[derive(Clone, Debug)]
struct Drawn<C: Corpus> {
    spine: Spine<C>,
    spans: Vec<(usize, usize, PassageMark)>,
}

impl<C: Corpus> Drawn<C> {
    fn passage(&self, index: usize) -> Passage<C> {
        let (first, last, mark) = self.spans[index];
        Passage::new(self.spine.units[first].clone(), self.spine.units[last].clone(), mark, &self.spine).expect("a drawn span is a passage")
    }

    fn units_of(&self, index: usize) -> BTreeSet<usize> {
        let (first, last, _) = self.spans[index];
        (first..=last).collect()
    }

    fn unit(&self) -> C::Ref {
        self.spine.units[self.spans[0].0].clone()
    }
}

fn units_of_passage<C: Corpus>(drawn: &Drawn<C>, passage: &Passage<C>) -> BTreeSet<usize> {
    let first = drawn.spine.position(passage.first()).expect("on the spine");
    let last = drawn.spine.position(passage.last()).expect("on the spine");
    (first..=last).collect()
}

#[derive(Debug, PartialEq)]
struct ContainmentFindings {
    reflexive: bool,
    mutual_exactly_when_one_span: bool,
    transitive: bool,
    contained_exactly_when_its_units_are: bool,
}

impl ContainmentFindings {
    const LAWFUL: ContainmentFindings = ContainmentFindings { reflexive: true, mutual_exactly_when_one_span: true, transitive: true, contained_exactly_when_its_units_are: true };
}

fn containment_findings<C: Corpus>(drawn: &Drawn<C>) -> ContainmentFindings {
    let [p, q, r] = [drawn.passage(0), drawn.passage(1), drawn.passage(2)];
    let (p_units, q_units) = (drawn.units_of(0), drawn.units_of(1));
    ContainmentFindings {
        reflexive: p.contains(&p),
        mutual_exactly_when_one_span: (p.contains(&q) && q.contains(&p)) == (p_units == q_units),
        transitive: !(r.contains(&q) && q.contains(&p)) || r.contains(&p),
        contained_exactly_when_its_units_are: q.contains(&p) == p_units.is_subset(&q_units),
    }
}

#[derive(Debug, PartialEq)]
struct JoinFindings {
    commutative: bool,
    associative: bool,
    idempotent: bool,
    upper_bound: bool,
    least: bool,
}

impl JoinFindings {
    const LAWFUL: JoinFindings = JoinFindings { commutative: true, associative: true, idempotent: true, upper_bound: true, least: true };
}

fn join_findings<C: Corpus>(drawn: &Drawn<C>) -> JoinFindings {
    let [p, q, r] = [drawn.passage(0), drawn.passage(1), drawn.passage(2)];
    let pq = p.join(&q);
    let left = pq.as_ref().and_then(|pq| pq.join(&r));
    let right = q.join(&r).and_then(|qr| p.join(&qr));
    JoinFindings {
        commutative: pq == q.join(&p),
        associative: left.is_none() || right.is_none() || left == right,
        idempotent: p.join(&p).as_ref() == Some(&p),
        upper_bound: pq.as_ref().is_none_or(|pq| pq.contains(&p) && pq.contains(&q)),
        least: pq.as_ref().is_none_or(|pq| [&p, &q, &r].iter().all(|bound| !(bound.contains(&p) && bound.contains(&q)) || bound.contains(pq))),
    }
}

fn joined_units<C: Corpus>(drawn: &Drawn<C>) -> Option<BTreeSet<usize>> {
    drawn.passage(0).join(&drawn.passage(1)).map(|joined| units_of_passage(drawn, &joined))
}

fn union_when_touching<C: Corpus>(drawn: &Drawn<C>) -> Option<BTreeSet<usize>> {
    let (p, q) = (drawn.units_of(0), drawn.units_of(1));
    let union: BTreeSet<usize> = p.union(&q).copied().collect();
    let first = *union.first().expect("a span holds units");
    let last = *union.last().expect("a span holds units");
    (union.len() == last - first + 1).then_some(union)
}

fn rejoined<C: Corpus>(drawn: &Drawn<C>, marks: (PassageMark, PassageMark)) -> Option<Passage<C>> {
    let once = remarked(drawn, marks.0).expect("a drawn span is a passage");
    let twice = remarked(drawn, marks.1).expect("a drawn span is a passage");
    once.join(&twice)
}

fn remarked<C: Corpus>(drawn: &Drawn<C>, mark: PassageMark) -> Option<Passage<C>> {
    let (first, last, _) = drawn.spans[0];
    Passage::new(drawn.spine.units[first].clone(), drawn.spine.units[last].clone(), mark, &drawn.spine).ok()
}

fn reversed<C: Corpus>(drawn: &Drawn<C>) -> Result<Passage<C>, PassageError> {
    let (first, last, _) = drawn.spans[0];
    Passage::new(drawn.spine.units[last].clone(), drawn.spine.units[first].clone(), PassageMark::Plain, &drawn.spine)
}

fn any_mark() -> impl Strategy<Value = PassageMark> {
    prop::sample::select(PassageMark::ALL.to_vec())
}

fn any_corpora_and_spans(spans: usize) -> impl Strategy<Value = (Drawn<BibleTag>, Drawn<ConcordTag>)> {
    (any_shape(), any_shape()).prop_flat_map(move |(bible_shape, concord_shape)| {
        let bible = bible_spine(&bible_shape);
        let concord = concord_spine(&concord_shape);
        (drawn_spans(bible, spans), drawn_spans(concord, spans))
    })
}

fn any_corpora_and_units() -> impl Strategy<Value = (Drawn<BibleTag>, Drawn<ConcordTag>)> {
    (any_shape(), any_shape()).prop_flat_map(|(bible_shape, concord_shape)| {
        let bible = bible_spine(&bible_shape);
        let concord = concord_spine(&concord_shape);
        (drawn_units(bible), drawn_units(concord))
    })
}

fn drawn_spans<C: Corpus>(spine: Spine<C>, spans: usize) -> impl Strategy<Value = Drawn<C>>
where
    C::Ref: 'static,
    C: 'static + Clone + std::fmt::Debug,
{
    let length = spine.units.len();
    prop::collection::vec((0..length, 0..length, any_mark()), spans)
        .prop_filter("a passage spans two units or more", |drawn| drawn.iter().all(|(first, last, _)| first != last))
        .prop_map(move |drawn| Drawn {
            spine: spine.clone(),
            spans: drawn.into_iter().map(|(a, b, mark)| (a.min(b), a.max(b), mark)).collect(),
        })
}

fn drawn_units<C: Corpus>(spine: Spine<C>) -> impl Strategy<Value = Drawn<C>>
where
    C::Ref: 'static,
    C: 'static + Clone + std::fmt::Debug,
{
    let length = spine.units.len();
    (0..length).prop_map(move |unit| Drawn { spine: spine.clone(), spans: vec![(unit, unit, PassageMark::Plain)] })
}

fn any_shape() -> impl Strategy<Value = Vec<Vec<u16>>> {
    prop::collection::vec(prop::collection::vec(1..=MOST_UNITS_PER_CHAPTER, 1..=MOST_CHAPTERS as usize), 1..=MOST_BOOKS as usize)
        .prop_filter("a spine holds two units or more", |books| books.iter().flatten().map(|units| usize::from(*units)).sum::<usize>() >= 2)
}

fn bible_spine(shape: &[Vec<u16>]) -> Spine<BibleTag> {
    Spine { units: placed(shape).map(|(book, chapter, verse)| VerseRef { book, chapter, verse }).collect() }
}

fn concord_spine(shape: &[Vec<u16>]) -> Spine<ConcordTag> {
    Spine { units: placed(shape).map(|(part, article, paragraph)| ConcordRef { part, article, paragraph }).collect() }
}

fn placed(shape: &[Vec<u16>]) -> impl Iterator<Item = (u8, u16, u16)> + '_ {
    let numbered: BTreeMap<u8, &Vec<u16>> = (1..).zip(shape).collect();
    numbered.into_iter().flat_map(|(book, chapters)| {
        (1..).zip(chapters.iter()).flat_map(move |(chapter, units)| (1..=*units).map(move |unit| (book, chapter, unit)))
    })
}
