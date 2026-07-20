use anyhow::{bail, Result};
use std::path::Path;
use truck_modeling::*;
use truck_stepio::out::*;

/// Build a B-Rep solid by extruding a 2D floorplan polygon.
/// Coordinates are (x, y) pairs (meters). Solid is extruded along Y axis
/// by `height` meters. Result is written to `output_path` as STEP.
pub fn extrude_floorplan(coords: &[(f64, f64)], height: f64, output_path: &Path) -> Result<()> {
    if coords.len() < 3 {
        bail!(
            "Need at least 3 coordinates to form a polygon, got {}",
            coords.len()
        );
    }

    println!(
        "    [>] Building B-Rep solid ({} vertices, height {} m)...",
        coords.len(),
        height
    );

    // Create vertices on the XZ plane (Y = 0), then edges between consecutive pairs
    let vertices: Vec<_> = coords
        .iter()
        .map(|(x, z)| builder::vertex(Point3::new(*x, 0.0, *z)))
        .collect();

    let mut edges = Vec::new();
    for i in 0..vertices.len() {
        let next = (i + 1) % vertices.len();
        edges.push(builder::line(&vertices[i], &vertices[next]));
    }

    // Attach face from loop of edges
    let wire: Wire = edges.into();
    let face = builder::try_attach_plane(&[wire])
        .map_err(|e| anyhow::anyhow!("Failed to attach plane to polygon: {:?}", e))?;

    // Sweep (extrude) along Y axis
    let solid = builder::tsweep(&face, Vector3::new(0.0, height, 0.0));

    // Compute bounding box from vertices (Solid has no bounding_box method)
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for v in solid.vertex_iter() {
        let p = v.point();
        let coords = [p.x, p.y, p.z];
        for i in 0..3 {
            min[i] = min[i].min(coords[i]);
            max[i] = max[i].max(coords[i]);
        }
    }
    println!(
        "    [>] Solid generated. Bounding box: min={:?}, max={:?}",
        min, max
    );

    // Write STEP file
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let compressed = solid.compress();
    let step_model = StepModel::from(&compressed);
    let complete = CompleteStepDisplay::new(step_model, StepHeaderDescriptor::default());
    std::fs::write(output_path, complete.to_string())?;
    println!("    [>] STEP file written to {}", output_path.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extrude_rectangle_succeeds() {
        let coords = [(0.0, 0.0), (10.0, 0.0), (10.0, 8.0), (0.0, 8.0)];
        let out = PathBuf::from("test_output_rectangle.step");
        let result = extrude_floorplan(&coords, 3.0, &out);
        assert!(
            result.is_ok(),
            "extrude_floorplan failed: {:?}",
            result.err()
        );
        assert!(out.exists(), "STEP file should have been written");
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn extrude_triangle_succeeds() {
        let coords = [(0.0, 0.0), (10.0, 0.0), (5.0, 8.0)];
        let out = PathBuf::from("test_output_triangle.step");
        let result = extrude_floorplan(&coords, 3.0, &out);
        assert!(
            result.is_ok(),
            "extrude_triangle failed: {:?}",
            result.err()
        );
        assert!(out.exists());
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn extrude_hexagon_succeeds() {
        let coords = [
            (0.0, 5.0),
            (4.33, 2.5),
            (4.33, -2.5),
            (0.0, -5.0),
            (-4.33, -2.5),
            (-4.33, 2.5),
        ];
        let out = PathBuf::from("test_output_hexagon.step");
        let result = extrude_floorplan(&coords, 3.0, &out);
        assert!(result.is_ok(), "extrude_hexagon failed: {:?}", result.err());
        assert!(out.exists());
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn too_few_coords_errors() {
        let coords = [(0.0, 0.0), (1.0, 0.0)];
        let out = PathBuf::from("test_output_bad.step");
        let result = extrude_floorplan(&coords, 3.0, &out);
        assert!(result.is_err(), "Should error on < 3 coords");
    }

    #[test]
    fn empty_coords_errors() {
        let coords: [(f64, f64); 0] = [];
        let out = PathBuf::from("test_output_empty.step");
        let result = extrude_floorplan(&coords, 3.0, &out);
        assert!(result.is_err());
    }
}
