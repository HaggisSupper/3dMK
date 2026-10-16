use vwm_implicit::{
    extract_structured_ray_surface, Ray3, RayIntersectionConfig, SphereField, StructuredRayGrid,
    StructuredSurfaceConfig,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let field = SphereField::new([0.0, 0.0, 0.0], 1.0, 0.5)?;
    let width = 65;
    let height = 65;
    let mut rays = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let px = -1.2 + 2.4 * x as f64 / (width - 1) as f64;
            let py = -1.2 + 2.4 * y as f64 / (height - 1) as f64;
            rays.push(Ray3::new([px, py, -2.0], [0.0, 0.0, 1.0])?);
        }
    }
    let ray_grid = StructuredRayGrid::new(width, height, rays)?;
    let (point_cloud, mesh) = extract_structured_ray_surface(
        &field,
        &ray_grid,
        StructuredSurfaceConfig {
            intersection: RayIntersectionConfig {
                t_max: 4.0,
                step: 0.025,
                ..RayIntersectionConfig::default()
            },
            hit_index: 0,
            max_edge_length: 0.1,
        },
    )?;
    println!(
        "generated {} points and {} triangles",
        point_cloud.points.len(),
        mesh.triangles.len()
    );
    Ok(())
}
