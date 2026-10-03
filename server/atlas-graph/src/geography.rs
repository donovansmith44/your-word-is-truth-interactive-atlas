use std::collections::BTreeMap;

use atlas_core::data::AtlasData;
use atlas_core::history::resolve_display_name_and_canonical;
use atlas_core::time::TimeRange;
use atlas_graph_types::canon::ids::{any_node_id_str, parse_any_node_id};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::AnyNodeId;
use atlas_graph_types::node::NodePayload;
use rusqlite::Connection;

use crate::sqlite::extras::{read_table, Col, ExtraTable, PLACE_DEFAULT, POLITY_REIGN};
use crate::sqlite::SqliteError;

#[derive(Debug, Clone, PartialEq)]
pub struct PlaceDefault {
    pub display_name: String,
    pub canonical_name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Geography {
    reigns: BTreeMap<AnyNodeId, TimeRange>,
    places: BTreeMap<AnyNodeId, PlaceDefault>,
}

impl Geography {
    pub fn compile(graph: &Graph, atlas: &AtlasData) -> Geography {
        let mut geography = Geography::default();
        for node in graph.nodes.values() {
            match &node.payload {
                NodePayload::Polity { eras, .. } => {
                    if let (Some(from_year), Some(to_year)) = (eras.iter().map(|era| era.from_year).min(), eras.iter().map(|era| era.to_year).max()) {
                        geography.reigns.insert(node.id.clone(), TimeRange { from_year, to_year });
                    }
                }
                NodePayload::Place { canonical, .. } => {
                    let history = atlas.place_history_for(&node.id.raw);
                    let (display_name, canonical_name) = resolve_display_name_and_canonical(canonical, history, None, atlas.place_name_alias_for(&node.id.raw));
                    geography.places.insert(node.id.clone(), PlaceDefault { display_name, canonical_name });
                }
                _ => {}
            }
        }
        geography
    }

    pub fn reign_of(&self, polity: &AnyNodeId) -> Option<TimeRange> {
        self.reigns.get(polity).copied()
    }

    pub fn place_default(&self, place: &AnyNodeId) -> Option<&PlaceDefault> {
        self.places.get(place)
    }

    pub fn tables(&self) -> Vec<ExtraTable> {
        let reigns = self
            .reigns
            .iter()
            .map(|(polity, reign)| vec![Col::Text(any_node_id_str(polity)), Col::Int(reign.from_year.into()), Col::Int(reign.to_year.into())])
            .collect();
        let places = self
            .places
            .iter()
            .map(|(place, default)| vec![Col::Text(any_node_id_str(place)), Col::Text(default.display_name.clone()), text_or_null(&default.canonical_name)])
            .collect();
        vec![ExtraTable { spec: &POLITY_REIGN, rows: reigns }, ExtraTable { spec: &PLACE_DEFAULT, rows: places }]
    }

    pub fn load(conn: &Connection) -> Result<Geography, SqliteError> {
        let mut geography = Geography::default();
        for row in read_table(conn, &POLITY_REIGN)? {
            match row.as_slice() {
                [Col::Text(polity), Col::Int(from_year), Col::Int(to_year)] => {
                    geography.reigns.insert(node_id(polity, POLITY_REIGN.name)?, TimeRange { from_year: year(*from_year)?, to_year: year(*to_year)? });
                }
                other => return Err(unreadable(POLITY_REIGN.name, other)),
            }
        }
        for row in read_table(conn, &PLACE_DEFAULT)? {
            match row.as_slice() {
                [Col::Text(place), Col::Text(display_name), canonical_name] => {
                    let default = PlaceDefault { display_name: display_name.clone(), canonical_name: optional_text(canonical_name, PLACE_DEFAULT.name)? };
                    geography.places.insert(node_id(place, PLACE_DEFAULT.name)?, default);
                }
                other => return Err(unreadable(PLACE_DEFAULT.name, other)),
            }
        }
        Ok(geography)
    }
}

fn text_or_null(text: &Option<String>) -> Col {
    text.as_ref().map(|text| Col::Text(text.clone())).unwrap_or(Col::Null)
}

fn optional_text(col: &Col, table: &str) -> Result<Option<String>, SqliteError> {
    match col {
        Col::Null => Ok(None),
        Col::Text(text) => Ok(Some(text.clone())),
        other => Err(unreadable(table, std::slice::from_ref(other))),
    }
}

fn node_id(raw: &str, table: &str) -> Result<AnyNodeId, SqliteError> {
    parse_any_node_id(raw, table).map_err(|refused| SqliteError(format!("{table}: {refused}")))
}

fn year(value: i64) -> Result<i32, SqliteError> {
    i32::try_from(value).map_err(|_| SqliteError(format!("{value} is not a year")))
}

fn unreadable(table: &str, row: &[Col]) -> SqliteError {
    SqliteError(format!("{table}: {row:?} is not a row of its columns"))
}
