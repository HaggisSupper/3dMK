use anyhow::{Context, Result};
use std::path::Path;
use truck_base::bounding_box::BoundingBox;
use truck_geometry::prelude::Point3;
use truck_stepio::in::Table;

/// Import a STEP file and return parsed geometry table.
/// `Table` contains all entities (points, curves, surfaces, shapes) indexed by STEP IDs.
pub fn import_step(path: &Path) -> Result<Table> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read STEP file: {}", path.display()))?;
    let section = step_parser::parser::parse_str(&text)
        .map_err(|e| anyhow::anyhow!("STEP parse error: {:?}", e))?;
    Table::from_data_section(&section)
        .map_err(|e| anyhow::anyhow!("STEP table build error: {:?}", e))
}

/// Extract bounding box from all vertices found in the parsed STEP table.
pub fn bounding_box_from_table(table: &Table) -> BoundingBox<Point3> {
    let mut bb = BoundingBox::new();
    for (_, item) in table.geometry.iter().enumerate() {
        // push each geometric entity's representative point(s) into the bbox
        if let Some(pt) = point_from_geometry(item) {
            bb.push(pt);
        }
    }
    bb
}

/// Import a STEP file and return its bounding box in one call.
pub fn import_step_bbox(path: &Path) -> Result<(Table, BoundingBox<Point3>)> {
    let table = import_step(path)?;
    let bb = bounding_box_from_table(&table);
    Ok((table, bb))
}

/// Export a truck topology shape (solid/shell) to STEP.
/// Returns the STEP string. Caller writes it to disk.
pub fn export_step_string(shape: &impl std::fmt::Display) -> String {
    shape.to_string()
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// Best-effort extraction of a Point3 from a geometry table entry.
/// Not all entries are points; returns None for non-point geometry.
fn point_from_geometry(item: &truck_stepio::in::GeometryItem) -> Option<Point3> {
    use truck_stepio::in::GeometryItem;
    match item {
        GeometryItem::CartesianPoint(pt) => {
            let c = pt.coordinates;
            Some(Point3::new(c[0], c[1], c[2]))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_nonexistent_file_errors() {
        let result = import_step(Path::new("nonexistent_file_abc123.step"));
        assert!(result.is_err());
    }

    #[test]
    fn bbox_empty_table_is_default() {
        let table = Table::new();
        let bb = bounding_box_from_table(&table);
        // empty bbox: min == max == origin
        let center = bb.center();
        assert!(center.x.is_nan() || (center.x == 0.0 && center.y == 0.0 && center.z == 0.0));
    }
}
