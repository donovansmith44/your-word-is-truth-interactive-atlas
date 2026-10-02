use atlas_contract::wire::{DateClaim, TextRef, TextSpan, TimeRange, Year};
use atlas_core::refs::BookId;
use atlas_core::{time, CoreError};

const EN_DASH: &str = " – ";
const FIRST_KINGS: u8 = 10;
const YEAR_ZERO: i32 = 0;
const THE_LABEL_A_YEAR_ZERO_WOULD_CARRY: &str = "AD 0";

#[test]
fn a_year_is_labelled_bc_before_christ_and_ad_after() {
    // Arrange
    let years = [-4004, -1447, -1, 1, 30, 33, 100, 2000];
    // Act
    let labels: Vec<String> = years.iter().map(|y| Year::of(*y).expect("none of these years is zero").label).collect();
    // Assert
    assert_eq!(labels, ["4004 BC", "1447 BC", "1 BC", "AD 1", "AD 30", "AD 33", "AD 100", "AD 2000"]);
}

#[test]
fn there_is_no_year_zero() {
    // Arrange
    let value = YEAR_ZERO;
    // Act
    let year = Year::of(value);
    // Assert
    assert_eq!(year, Err(CoreError::ZeroYear));
}

#[test]
fn a_year_read_off_the_wire_is_refused_at_year_zero_as_one_made_here_is() {
    // Arrange
    let wire = serde_json::json!({ "value": YEAR_ZERO, "label": THE_LABEL_A_YEAR_ZERO_WOULD_CARRY });
    // Act
    let year = serde_json::from_value::<Year>(wire).map_err(|e| e.to_string());
    // Assert
    assert_eq!(year, Err(CoreError::ZeroYear.to_string()));
}

#[test]
fn a_range_names_the_era_once_when_both_ends_share_it_and_twice_otherwise() {
    // Arrange
    let ranges = [(-1450, -1400), (-1447, -1400), (-1447, -1447), (-5, 30), (33, 33), (1, 100), (30, 70)];
    // Act
    let labels: Vec<String> = ranges.iter().map(|(from, to)| TimeRange::of(core_range(*from, *to)).label).collect();
    // Assert
    assert_eq!(
        labels,
        [
            format!("1450{EN_DASH}1400 BC"),
            format!("1447{EN_DASH}1400 BC"),
            "1447 BC".to_string(),
            format!("5 BC{EN_DASH}AD 30"),
            "AD 33".to_string(),
            format!("AD 1{EN_DASH}100"),
            format!("AD 30{EN_DASH}70"),
        ]
    );
}

#[test]
fn a_range_read_off_the_wire_is_refused_when_it_ends_before_it_starts_as_one_made_here_is() {
    // Arrange
    let wire = serde_json::json!({
        "from": { "value": -1400, "label": "1400 BC" },
        "to": { "value": -1450, "label": "1450 BC" },
        "label": format!("1400{EN_DASH}1450 BC"),
    });
    // Act
    let range = serde_json::from_value::<TimeRange>(wire).map_err(|e| e.to_string());
    // Assert
    assert_eq!(range, Err(CoreError::InvertedRange.to_string()));
}

#[test]
fn a_range_read_off_the_wire_carries_the_label_written_here_not_the_one_it_arrived_with() {
    // Arrange
    let wire = serde_json::json!({
        "from": { "value": -1450, "label": "1450 BC" },
        "to": { "value": -1400, "label": "1400 BC" },
        "label": "fifteenth century BC",
    });
    // Act
    let range = serde_json::from_value::<TimeRange>(wire).map_err(|e| e.to_string());
    // Assert
    assert_eq!(range, Ok(TimeRange::of(core_range(-1450, -1400))));
}

#[test]
fn a_claim_with_a_note_is_labelled_circa() {
    // Arrange
    let claims = [(-1003, -1003, Some("traditional")), (-586, -586, None), (-1447, -1400, Some("traditional"))];
    // Act
    let labels: Vec<String> =
        claims.iter().map(|(from, to, note)| DateClaim::of(TimeRange::of(core_range(*from, *to)), vec![], note.map(str::to_string), None).label).collect();
    // Assert
    assert_eq!(labels, ["c. 1003 BC".to_string(), "586 BC".to_string(), format!("c. 1447{EN_DASH}1400 BC")]);
}

#[test]
fn a_claim_travels_with_its_range_its_verses_and_its_note() {
    // Arrange
    let temple_begun = TextSpan::whole(TextRef::Bible { book: BookId(FIRST_KINGS), chapter: 6, verse: 1 });
    // Act
    let claim = DateClaim::of(TimeRange::of(core_range(-966, -966)), vec![temple_begun], Some("traditional".to_string()), None);
    // Assert
    assert_eq!(
        serde_json::to_value(&claim).unwrap(),
        serde_json::json!({
            "when": { "from": { "value": -966, "label": "966 BC" }, "to": { "value": -966, "label": "966 BC" }, "label": "966 BC" },
            "label": "c. 966 BC",
            "verses": [ { "from": { "unit": { "corpus": "bible", "book": "1KI", "chapter": 6, "verse": 1 } }, "to": { "unit": { "corpus": "bible", "book": "1KI", "chapter": 6, "verse": 1 } } } ],
            "note": "traditional",
        })
    );
}

#[test]
fn the_vectors_document_is_the_same_rules_written_out() {
    // Arrange
    let expected = serde_json::json!({
        "years": [
            { "value": -4004, "label": "4004 BC" },
            { "value": -1447, "label": "1447 BC" },
            { "value": -1, "label": "1 BC" },
            { "value": 1, "label": "AD 1" },
            { "value": 30, "label": "AD 30" },
            { "value": 33, "label": "AD 33" },
            { "value": 100, "label": "AD 100" },
            { "value": 2000, "label": "AD 2000" },
        ],
        "ranges": [
            { "from": -1450, "to": -1400, "label": format!("1450{EN_DASH}1400 BC") },
            { "from": -1447, "to": -1400, "label": format!("1447{EN_DASH}1400 BC") },
            { "from": -1447, "to": -1447, "label": "1447 BC" },
            { "from": -5, "to": 30, "label": format!("5 BC{EN_DASH}AD 30") },
            { "from": 33, "to": 33, "label": "AD 33" },
            { "from": 1, "to": 100, "label": format!("AD 1{EN_DASH}100") },
            { "from": 30, "to": 70, "label": format!("AD 30{EN_DASH}70") },
        ],
        "claims": [
            { "from": -1003, "to": -1003, "note": "traditional", "label": "c. 1003 BC" },
            { "from": -586, "to": -586, "note": null, "label": "586 BC" },
            { "from": -1447, "to": -1400, "note": "traditional", "label": format!("c. 1447{EN_DASH}1400 BC") },
        ],
    });
    // Act
    let actual: serde_json::Value = serde_json::from_str(&atlas_contract::document::year_labels_json()).unwrap();
    // Assert
    assert_eq!(actual, expected);
}

fn core_range(from: i32, to: i32) -> time::TimeRange {
    time::TimeRange::new(from, to).expect("every range here is in order and names no year zero")
}

#[test]
fn the_document_publishes_the_labelled_span_as_time_range_and_the_computed_one_as_year_span() {
    // Arrange
    let document = serde_json::to_value(atlas_contract::document::openapi()).unwrap();
    let year = serde_json::json!({ "$ref": "#/components/schemas/Year" });
    let integer = serde_json::json!({ "type": "integer", "format": "int32" });
    // Act
    let published = (&document["components"]["schemas"]["TimeRange"], &document["components"]["schemas"]["YearSpan"]);
    // Assert
    assert_eq!(
        published,
        (
            &serde_json::json!({
                "type": "object",
                "description": format!("A span of years, both ends included, with the label a reader sees for it: an era both ends share is named once, as in `1450{EN_DASH}1400 BC`, `5 BC{EN_DASH}AD 30` or, for a single year, `AD 33`."),
                "required": ["from", "to", "label"],
                "properties": {
                    "from": year,
                    "to": year,
                    "label": { "type": "string" },
                },
                "additionalProperties": false,
            }),
            &serde_json::json!({
                "type": "object",
                "description": "A span of years on this atlas's scale: negative for BC, positive for AD, with no year zero. A single year is a span whose ends are equal.",
                "required": ["from_year", "to_year"],
                "properties": { "from_year": integer, "to_year": integer },
                "additionalProperties": false,
            }),
        )
    );
}
