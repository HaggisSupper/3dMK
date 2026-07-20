use vwm_implicit::{
    sample_field_to_grid, GridSamplingConfig, ImplicitField, SphereField, SurfaceNetsConfig,
    SurfaceNetsExtractor,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let field = SphereField::new([0.0, 0.0, 0.0], 1.0, 0.25)?;
    let grid = sample_field_to_grid(
        &field,
        GridSamplingConfig {
            bounds: field.bounds(),
            dimensions: [65, 65, 65],
        },
    )?;
    let mesh = SurfaceNetsExtractor.extract(&grid, SurfaceNetsConfig::default())?;
    println!(
        "generated {} vertices and {} triangles",
        mesh.positions.len(),
        mesh.triangles.len()
    );
    Ok(())
}
