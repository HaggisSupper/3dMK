use vwm_implicit_core::{find_ray_intersections, Ray3, RayIntersectionConfig, SphereField};

#[test]
fn ray_finds_entry_and_exit_on_sphere() {
    let sphere = SphereField::new([0.0, 0.0, 0.0], 1.0, 1.0).unwrap();
    let ray = Ray3::new([-2.0, 0.0, 0.0], [1.0, 0.0, 0.0]).unwrap();
    let hits = find_ray_intersections(
        &sphere,
        ray,
        RayIntersectionConfig {
            t_max: 4.0,
            step: 0.2,
            ..RayIntersectionConfig::default()
        },
    )
    .unwrap();
    assert_eq!(hits.len(), 2);
    assert!((hits[0].position[0] + 1.0).abs() < 1.0e-4);
    assert!((hits[1].position[0] - 1.0).abs() < 1.0e-4);
}

#[test]
fn ray_reports_no_hit_when_it_misses() {
    let sphere = SphereField::new([0.0, 0.0, 0.0], 1.0, 1.0).unwrap();
    let ray = Ray3::new([-2.0, 2.0, 0.0], [1.0, 0.0, 0.0]).unwrap();
    let hits = find_ray_intersections(
        &sphere,
        ray,
        RayIntersectionConfig {
            t_max: 4.0,
            step: 0.1,
            ..RayIntersectionConfig::default()
        },
    )
    .unwrap();
    assert!(hits.is_empty());
}

#[test]
fn cubic_bezier_curve_finds_sphere_intersections() {
    use vwm_implicit_core::{find_curve_intersections, CubicBezierCurve};

    let sphere = SphereField::new([0.0, 0.0, 0.0], 1.0, 1.0).unwrap();
    let curve = CubicBezierCurve::new([
        [-2.0, 0.0, 0.0],
        [-0.5, 0.0, 0.0],
        [0.5, 0.0, 0.0],
        [2.0, 0.0, 0.0],
    ])
    .unwrap();
    let hits = find_curve_intersections(
        &sphere,
        &curve,
        RayIntersectionConfig {
            t_min: 0.0,
            t_max: 1.0,
            step: 0.02,
            ..RayIntersectionConfig::default()
        },
    )
    .unwrap();
    assert_eq!(hits.len(), 2);
}

#[test]
fn structured_rays_create_point_cloud_and_mesh() {
    use vwm_implicit_core::{
        extract_structured_ray_surface, StructuredRayGrid, StructuredSurfaceConfig,
    };

    let sphere = SphereField::new([0.0, 0.0, 0.0], 1.0, 1.0).unwrap();
    let width = 11;
    let height = 11;
    let mut rays = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let px = -1.2 + 2.4 * x as f64 / (width - 1) as f64;
            let py = -1.2 + 2.4 * y as f64 / (height - 1) as f64;
            rays.push(Ray3::new([px, py, -2.0], [0.0, 0.0, 1.0]).unwrap());
        }
    }
    let grid = StructuredRayGrid::new(width, height, rays).unwrap();
    let (points, mesh) = extract_structured_ray_surface(
        &sphere,
        &grid,
        StructuredSurfaceConfig {
            intersection: RayIntersectionConfig {
                t_max: 4.0,
                step: 0.05,
                ..RayIntersectionConfig::default()
            },
            hit_index: 0,
            max_edge_length: 0.5,
        },
    )
    .unwrap();
    assert!(!points.points.is_empty());
    assert!(!mesh.triangles.is_empty());
}
