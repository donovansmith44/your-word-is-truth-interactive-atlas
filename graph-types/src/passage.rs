use std::cmp::Ordering;

use crate::text::Corpus;

crate::vocabulary! {
    #[derive(PartialOrd, Ord)]
    PassageMark {
        Plain => "plain",
        RecordsHistory => "recordsHistory",
    }
}

pub trait ReadingOrder<C: Corpus> {
    fn position(&self, unit: &C::Ref) -> Option<usize>;
}

#[derive(Debug, PartialEq)]
pub enum PassageError {
    OffTheSpine,
    Backwards,
    OneUnit,
}

pub struct Passage<C: Corpus> {
    first: Placed<C>,
    last: Placed<C>,
    mark: PassageMark,
}

struct Placed<C: Corpus> {
    position: usize,
    unit: C::Ref,
}

impl<C: Corpus> Passage<C> {
    pub fn new(first: C::Ref, last: C::Ref, mark: PassageMark, order: &impl ReadingOrder<C>) -> Result<Passage<C>, PassageError> {
        let first = Placed::on(first, order)?;
        let last = Placed::on(last, order)?;
        match first.position.cmp(&last.position) {
            Ordering::Less => Ok(Passage { first, last, mark }),
            Ordering::Equal => Err(PassageError::OneUnit),
            Ordering::Greater => Err(PassageError::Backwards),
        }
    }

    pub fn first(&self) -> &C::Ref {
        &self.first.unit
    }

    pub fn last(&self) -> &C::Ref {
        &self.last.unit
    }

    pub fn contains(&self, other: &Passage<C>) -> bool {
        self.first.position <= other.first.position && other.last.position <= self.last.position
    }

    pub fn join(&self, other: &Passage<C>) -> Option<Passage<C>> {
        let (earlier, later) = if self.first.position <= other.first.position { (self, other) } else { (other, self) };
        (later.first.position <= earlier.last.position + 1).then(|| Passage {
            first: earlier.first.clone(),
            last: if later.last.position > earlier.last.position { later.last.clone() } else { earlier.last.clone() },
            mark: self.mark.max(other.mark),
        })
    }
}

impl<C: Corpus> Placed<C> {
    fn on(unit: C::Ref, order: &impl ReadingOrder<C>) -> Result<Placed<C>, PassageError> {
        let position = order.position(&unit).ok_or(PassageError::OffTheSpine)?;
        Ok(Placed { position, unit })
    }
}

impl<C: Corpus> PartialEq for Passage<C> {
    fn eq(&self, other: &Self) -> bool {
        self.first == other.first && self.last == other.last && self.mark == other.mark
    }
}

impl<C: Corpus> std::fmt::Debug for Passage<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Passage").field("first", &self.first.unit).field("last", &self.last.unit).field("mark", &self.mark).finish()
    }
}

impl<C: Corpus> Clone for Placed<C> {
    fn clone(&self) -> Self {
        Placed { position: self.position, unit: self.unit.clone() }
    }
}

impl<C: Corpus> PartialEq for Placed<C> {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position && self.unit == other.unit
    }
}
