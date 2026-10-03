use atlas_graph_types::canon::{obj, serialize, str_value, RowFamily, Value};

pub fn row_line_body(family: RowFamily, ord: i64, row: Value) -> Vec<u8> {
    serialize(&obj(vec![("family", str_value(family.name())), ("ord", Value::Int(ord)), ("row", row)]))
}
