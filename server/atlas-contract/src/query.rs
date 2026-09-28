//! How a route reads the query it was asked with: its own parameters, its own
//! types, and its own refusal when one of them cannot be read.

use std::str::FromStr;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

use atlas_core::time::{TimeRange, Year};

use crate::error::ApiError;

/// The three parameter names this API needs as VALUES: each is matched on to pick a
/// route's refusal and written into that refusal's own sentence, so the two would
/// otherwise spell it twice. Every other parameter is named once, by the field that
/// carries it -- including `ref`, whose field is `r#ref` so that even its name is not
/// repeated in a `serde(rename)` attribute no constant could reach.
pub const DIR: &str = "dir";
pub const SCOPE: &str = "scope";
pub const CORPUS: &str = "corpus";

/// The span two years name, refused rather than adjusted: a zero year, or a span
/// that ends before it starts, is no window at all. Every route that reads a pair of
/// years reads it through here, so two of them cannot come to disagree about which
/// pairs this atlas serves.
pub fn span(from: Year, to: Year) -> Result<TimeRange, ApiError> {
    TimeRange::new(from, to).map_err(|_| ApiError::bad_window())
}

/// One route's query parameters, read so that a caller who spells one wrongly is
/// answered in this API's own error vocabulary rather than the web framework's.
pub struct Contract<T>(pub T);

/// Which refusal a route answers when one of its own parameters cannot be read.
/// `asked_with` is the caller's own word for that parameter, absent when the
/// parameter was left out altogether.
pub trait ContractParams: DeserializeOwned {
    fn unreadable(parameter: &str, asked_with: Option<&str>) -> ApiError;
}

impl<S: Send + Sync, T: ContractParams> FromRequestParts<S> for Contract<T> {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, ApiError> {
        let asked = last_of_each(parts.uri.query().unwrap_or_default());
        let reader = serde_urlencoded::Deserializer::new(form_urlencoded::parse(asked.as_bytes()));
        match serde_path_to_error::deserialize::<_, T>(reader) {
            Ok(parameters) => Ok(Contract(parameters)),
            Err(refusal) => {
                let parameter = unreadable_parameter(&refusal);
                Err(T::unreadable(&parameter, asked_with(&asked, &parameter).as_deref()))
            }
        }
    }
}

/// A parameter given twice reads as the last of the two, which is how every route's
/// query has always read. Without this a repeated parameter would be a refusal of
/// its own, and no route has a code to answer one with.
fn last_of_each(query: &str) -> String {
    let mut kept: Vec<(&str, &str)> = Vec::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (parameter, value) = pair.split_once('=').unwrap_or((pair, ""));
        match kept.iter_mut().find(|(seen, _)| *seen == parameter) {
            Some(slot) => slot.1 = value,
            None => kept.push((parameter, value)),
        }
    }
    kept.iter().map(|(parameter, value)| format!("{parameter}={value}")).collect::<Vec<_>>().join("&")
}

const MISSING: &str = "missing field `";

/// The parameter a refusal is about: the one the reader walked into, or -- for a
/// parameter left out altogether, which is refused at the whole query rather than at
/// any one member of it -- the one the reason names. The mapping's own test drives
/// real refusals through here rather than text written out by hand.
fn unreadable_parameter(refusal: &serde_path_to_error::Error<serde_urlencoded::de::Error>) -> String {
    if refusal.path().iter().next().is_some() {
        return refusal.path().to_string();
    }
    left_out_parameter(&refusal.inner().to_string())
}

fn left_out_parameter(reason: &str) -> String {
    reason.strip_prefix(MISSING).and_then(|named| named.split('`').next()).unwrap_or_default().to_string()
}

/// The caller's own word for one parameter, decoded the way the parameters
/// themselves are, so a refusal quotes back exactly what it refused.
fn asked_with(query: &str, parameter: &str) -> Option<String> {
    let pairs: Vec<(String, String)> = serde_urlencoded::from_str(query).ok()?;
    pairs.into_iter().find(|(name, _)| name == parameter).map(|(_, value)| value)
}

/// A query parameter as the caller gave it: the value when it reads as one, and
/// nothing when it was left out or cannot be read. The counts and cursors this API
/// takes read this way -- a caller who mistypes one is served the route's own
/// default rather than refused, and this API has no refusal code for a count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsGiven<T>(Option<T>);

impl<T> AsGiven<T> {
    pub fn given(self) -> Option<T> {
        self.0
    }
}

impl<T> Default for AsGiven<T> {
    fn default() -> Self {
        AsGiven(None)
    }
}

impl<'de, T: FromStr> Deserialize<'de> for AsGiven<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let asked = String::deserialize(deserializer)?;
        Ok(AsGiven(asked.parse().ok()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{BAD_CORPUS, BAD_DIR, BAD_KIND, BAD_REF, BAD_SCOPE, BAD_WINDOW};
    use crate::graph::{EdgePageQuery, TextWindowQuery};
    use crate::map::{SceneWindow, ScripturePassage};
    use crate::places::PlacePeriod;

    #[test]
    fn a_parameter_given_twice_reads_as_the_last_of_the_two() {
        // Arrange
        let query = "from=-2100&to=-2000&from=-1000";
        // Act
        let read = last_of_each(query);
        // Assert
        assert_eq!(read, "from=-1000&to=-2000");
    }

    #[test]
    fn a_parameter_given_with_no_value_reads_as_the_empty_word() {
        // Arrange
        let query = "scope&corpus=concord";
        // Act
        let read = last_of_each(query);
        // Assert
        assert_eq!(read, "scope=&corpus=concord");
    }

    #[test]
    fn an_empty_query_reads_as_no_parameters_at_all() {
        // Arrange
        let query = "";
        // Act
        let read = last_of_each(query);
        // Assert
        assert_eq!(read, "");
    }

    #[test]
    fn a_left_out_parameter_is_named_by_the_refusal_that_reports_it_missing() {
        // Arrange
        let reasons = ["missing field `ref`", "invalid length 0"];
        // Act
        let named: Vec<String> = reasons.iter().map(|reason| left_out_parameter(reason)).collect();
        // Assert
        assert_eq!(named, vec!["ref".to_string(), String::new()]);
    }

    #[test]
    fn the_word_a_parameter_was_asked_with_is_read_back_percent_decoded() {
        // Arrange
        let query = "ref=BoC%207.2.1&corpus=concord";
        // Act
        let read = (asked_with(query, "ref"), asked_with(query, "corpus"), asked_with(query, "n"));
        // Assert
        assert_eq!(read, (Some("BoC 7.2.1".to_string()), Some("concord".to_string()), None));
    }

    #[test]
    fn a_count_that_is_not_a_count_is_given_as_nothing_at_all() {
        // Arrange
        let asked = ["n=3", "n=", "n=three"];
        // Act
        let read: Vec<Option<usize>> = asked.iter().map(|query| serde_urlencoded::from_str::<Counted>(query).unwrap().n.given()).collect();
        // Assert
        assert_eq!(read, vec![Some(3), None, None]);
    }

    #[derive(Deserialize)]
    struct Counted {
        n: AsGiven<usize>,
    }

    #[tokio::test]
    async fn every_query_a_route_cannot_read_answers_that_routes_own_refusal() {
        // Act
        let refused = vec![
            refusal::<SceneWindow>("to=100").await,
            refusal::<SceneWindow>("from=notayear&to=100").await,
            refusal::<SceneWindow>("from=-2100&to=notayear").await,
            refusal::<ScripturePassage>("").await,
            refusal::<EdgePageQuery>("").await,
            refusal::<EdgePageQuery>("kind=not-a-real-kind").await,
            refusal::<TextWindowQuery>("").await,
            refusal::<TextWindowQuery>("ref=JHN.3.16&scope=paragraph").await,
            refusal::<TextWindowQuery>("ref=JHN.3.16&dir=sideways").await,
            refusal::<TextWindowQuery>("ref=JHN.3.16&corpus=vulgate").await,
            refusal::<PlacePeriod>("from=notayear&to=100").await,
        ];
        // Assert
        assert_eq!(
            refused,
            vec![
                (400, BAD_WINDOW, UNREADABLE_WINDOW.to_string()),
                (400, BAD_WINDOW, UNREADABLE_WINDOW.to_string()),
                (400, BAD_WINDOW, UNREADABLE_WINDOW.to_string()),
                (400, BAD_REF, UNREADABLE_EMPTY_REF.to_string()),
                (400, BAD_KIND, UNREADABLE_EMPTY_KIND.to_string()),
                (400, BAD_KIND, UNREADABLE_KIND.to_string()),
                (400, BAD_REF, UNREADABLE_EMPTY_REF.to_string()),
                (400, BAD_SCOPE, UNREADABLE_SCOPE.to_string()),
                (400, BAD_DIR, UNREADABLE_DIR.to_string()),
                (400, BAD_CORPUS, UNREADABLE_CORPUS.to_string()),
                (400, BAD_WINDOW, UNREADABLE_WINDOW.to_string()),
            ]
        );
    }

    pub const UNREADABLE_WINDOW: &str = "from/to must both be present, non-zero integers with from <= to";
    pub const UNREADABLE_EMPTY_REF: &str = "invalid scripture reference: ''";
    pub const UNREADABLE_EMPTY_KIND: &str = "unknown or missing edge kind: ''";
    pub const UNREADABLE_KIND: &str = "unknown or missing edge kind: 'not-a-real-kind'";
    pub const UNREADABLE_SCOPE: &str = "unknown scope: 'paragraph' (expected 'verse' or 'chapter')";
    pub const UNREADABLE_DIR: &str = "unknown dir: 'sideways' (expected 'onward' or 'backward')";
    pub const UNREADABLE_CORPUS: &str = "unknown corpus: 'vulgate' (expected 'bible' or 'concord')";

    #[tokio::test]
    async fn every_query_a_route_can_read_reaches_the_route_with_no_refusal_at_all() {
        // Act
        let read = vec![
            refusal::<SceneWindow>("from=-2100&to=-2000").await,
            refusal::<ScripturePassage>("ref=JHN.3.16").await,
            refusal::<EdgePageQuery>("kind=cites&cursor=notanumber&limit=notanumber").await,
            refusal::<TextWindowQuery>("ref=JHN.3.16&n=notanumber&dir=backward&scope=verse&corpus=concord").await,
            refusal::<PlacePeriod>("").await,
        ];
        // Assert
        assert_eq!(read, vec![NOTHING_REFUSED; 5]);
    }

    const NOTHING_REFUSED: (u16, &str, String) = (0, "", String::new());

    /// Driven through the real reader rather than text written out by hand, so a
    /// reader that names the parameter it could not read differently fails here
    /// instead of answering a caller with another parameter's code.
    async fn refusal<T: ContractParams>(query: &str) -> (u16, &'static str, String) {
        let (mut parts, _) = axum::http::Request::builder().uri(format!("/?{query}")).body(()).unwrap().into_parts();
        match Contract::<T>::from_request_parts(&mut parts, &()).await {
            Ok(_) => NOTHING_REFUSED,
            Err(refused) => (refused.status.as_u16(), refused.code, refused.message),
        }
    }
}
