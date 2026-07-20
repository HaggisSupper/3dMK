use serde::{Deserialize, Serialize};

use crate::{
    FieldMesh, FieldPointCloud, ImplicitError, ImplicitField, ImplicitResult, Point3, Scalar,
    Vector3,
};

pub trait ParametricCurve: Send + Sync {
    fn point_at(&self, t: Scalar) -> Point3;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Ray3 {
    pub origin: Point3,
    pub direction: Vector3,
}

impl Ray3 {
    pub fn new(origin: Point3, direction: Vector3) -> ImplicitResult<Self> {
        if !origin
            .iter()
            .chain(direction.iter())
            .all(|value| value.is_finite())
        {
            return Err(ImplicitError::InvalidRayDirection);
        }
        let length = direction
            .iter()
            .map(|value| value * value)
            .sum::<Scalar>()
            .sqrt();
        if length <= f64::EPSILON {
            return Err(ImplicitError::InvalidRayDirection);
        }
        Ok(Self {
            origin,
            direction: [
                direction[0] / length,
                direction[1] / length,
                direction[2] / length,
            ],
        })
    }

    pub fn point_at(&self, t: Scalar) -> Point3 {
        [
            self.origin[0] + self.direction[0] * t,
            self.origin[1] + self.direction[1] * t,
            self.origin[2] + self.direction[2] * t,
        ]
    }
}

impl ParametricCurve for Ray3 {
    fn point_at(&self, t: Scalar) -> Point3 {
        Ray3::point_at(self, t)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CubicBezierCurve {
    pub control_points: [Point3; 4],
}

impl CubicBezierCurve {
    pub fn new(control_points: [Point3; 4]) -> ImplicitResult<Self> {
        if !control_points
            .iter()
            .flatten()
            .all(|value| value.is_finite())
        {
            return Err(ImplicitError::InvalidRayConfiguration(
                "Bezier control points must be finite".to_string(),
            ));
        }
        Ok(Self { control_points })
    }

    pub fn point_at(&self, t: Scalar) -> Point3 {
        let t = t.clamp(0.0, 1.0);
        let u = 1.0 - t;
        let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
        let mut point = [0.0; 3];
        for (control, weight) in self.control_points.iter().zip(weights) {
            for axis in 0..3 {
                point[axis] += control[axis] * weight;
            }
        }
        point
    }
}

impl ParametricCurve for CubicBezierCurve {
    fn point_at(&self, t: Scalar) -> Point3 {
        CubicBezierCurve::point_at(self, t)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct RayIntersectionConfig {
    pub t_min: Scalar,
    pub t_max: Scalar,
    pub step: Scalar,
    pub value_tolerance: Scalar,
    pub position_tolerance: Scalar,
    pub max_root_iterations: u32,
    pub max_hits: usize,
}

impl RayIntersectionConfig {
    pub fn validate(&self) -> ImplicitResult<()> {
        if !self.t_min.is_finite()
            || !self.t_max.is_finite()
            || self.t_max <= self.t_min
            || !self.step.is_finite()
            || self.step <= 0.0
            || !self.value_tolerance.is_finite()
            || self.value_tolerance <= 0.0
            || !self.position_tolerance.is_finite()
            || self.position_tolerance <= 0.0
            || self.max_root_iterations == 0
            || self.max_hits == 0
        {
            return Err(ImplicitError::InvalidRayConfiguration(
                "ranges, tolerances, iteration limits, and hit limits must be positive".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for RayIntersectionConfig {
    fn default() -> Self {
        Self {
            t_min: 0.0,
            t_max: 100.0,
            step: 0.05,
            value_tolerance: 1.0e-6,
            position_tolerance: 1.0e-6,
            max_root_iterations: 64,
            max_hits: 16,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CurveHit {
    pub t: Scalar,
    pub position: Point3,
    pub normal: Vector3,
    pub field_value: Scalar,
}

pub type RayHit = CurveHit;

pub fn find_curve_intersections(
    field: &dyn ImplicitField,
    curve: &dyn ParametricCurve,
    config: RayIntersectionConfig,
) -> ImplicitResult<Vec<CurveHit>> {
    config.validate()?;
    find_intersections_in_interval(field, curve, config)
}

pub fn find_ray_intersections(
    field: &dyn ImplicitField,
    ray: Ray3,
    config: RayIntersectionConfig,
) -> ImplicitResult<Vec<RayHit>> {
    config.validate()?;
    let Some((domain_t_min, domain_t_max)) = ray_aabb_interval(&ray, field.bounds()) else {
        return Ok(Vec::new());
    };
    let effective_t_min = config.t_min.max(domain_t_min);
    let effective_t_max = config.t_max.min(domain_t_max);
    if effective_t_max <= effective_t_min {
        return Ok(Vec::new());
    }
    find_intersections_in_interval(
        field,
        &ray,
        RayIntersectionConfig {
            t_min: effective_t_min,
            t_max: effective_t_max,
            ..config
        },
    )
}

pub fn extract_point_cloud_from_rays(
    field: &dyn ImplicitField,
    rays: &[Ray3],
    config: RayIntersectionConfig,
    hit_index: usize,
) -> ImplicitResult<FieldPointCloud> {
    let mut points = Vec::new();
    let mut normals = Vec::new();
    for ray in rays {
        if let Some(hit) = find_ray_intersections(field, *ray, config)?.get(hit_index) {
            points.push([
                hit.position[0] as f32,
                hit.position[1] as f32,
                hit.position[2] as f32,
            ]);
            normals.push([
                hit.normal[0] as f32,
                hit.normal[1] as f32,
                hit.normal[2] as f32,
            ]);
        }
    }
    let confidence = vec![1.0; points.len()];
    Ok(FieldPointCloud {
        points,
        normals: Some(normals),
        confidence: Some(confidence),
        source_backend: "implicit-ray-zero-crossing".to_string(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuredRayGrid {
    pub width: usize,
    pub height: usize,
    pub rays: Vec<Ray3>,
}

impl StructuredRayGrid {
    pub fn new(width: usize, height: usize, rays: Vec<Ray3>) -> ImplicitResult<Self> {
        let expected = width
            .checked_mul(height)
            .ok_or(ImplicitError::GridSizeOverflow)?;
        if width < 2 || height < 2 || rays.len() != expected {
            return Err(ImplicitError::InvalidRayConfiguration(
                "structured ray grid dimensions must be at least 2x2 and match the ray count"
                    .to_string(),
            ));
        }
        Ok(Self {
            width,
            height,
            rays,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct StructuredSurfaceConfig {
    pub intersection: RayIntersectionConfig,
    pub hit_index: usize,
    pub max_edge_length: Scalar,
}

impl StructuredSurfaceConfig {
    pub fn validate(&self) -> ImplicitResult<()> {
        self.intersection.validate()?;
        if !self.max_edge_length.is_finite() || self.max_edge_length <= 0.0 {
            return Err(ImplicitError::InvalidRayConfiguration(
                "maximum structured-surface edge length must be finite and positive".to_string(),
            ));
        }
        Ok(())
    }
}

pub fn extract_structured_ray_surface(
    field: &dyn ImplicitField,
    grid: &StructuredRayGrid,
    config: StructuredSurfaceConfig,
) -> ImplicitResult<(FieldPointCloud, FieldMesh)> {
    config.validate()?;
    let mut hits = Vec::with_capacity(grid.rays.len());
    for ray in &grid.rays {
        let hit = find_ray_intersections(field, *ray, config.intersection)?
            .get(config.hit_index)
            .copied();
        hits.push(hit);
    }

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut vertex_by_ray = vec![None; hits.len()];
    for (ray_index, hit) in hits.iter().enumerate() {
        if let Some(hit) = hit {
            let vertex_index = positions.len() as u32;
            vertex_by_ray[ray_index] = Some(vertex_index);
            positions.push([
                hit.position[0] as f32,
                hit.position[1] as f32,
                hit.position[2] as f32,
            ]);
            normals.push([
                hit.normal[0] as f32,
                hit.normal[1] as f32,
                hit.normal[2] as f32,
            ]);
        }
    }

    let mut triangles = Vec::new();
    for y in 0..grid.height - 1 {
        for x in 0..grid.width - 1 {
            let i00 = y * grid.width + x;
            let i10 = i00 + 1;
            let i01 = i00 + grid.width;
            let i11 = i01 + 1;

            if let (Some(a), Some(b), Some(c)) =
                (vertex_by_ray[i00], vertex_by_ray[i10], vertex_by_ray[i11])
            {
                let triangle = [a, b, c];
                if triangle_within_edge_limit(&positions, triangle, config.max_edge_length) {
                    triangles.push(triangle);
                }
            }
            if let (Some(a), Some(b), Some(c)) =
                (vertex_by_ray[i00], vertex_by_ray[i11], vertex_by_ray[i01])
            {
                let triangle = [a, b, c];
                if triangle_within_edge_limit(&positions, triangle, config.max_edge_length) {
                    triangles.push(triangle);
                }
            }
        }
    }

    let point_cloud = FieldPointCloud {
        confidence: Some(vec![1.0; positions.len()]),
        points: positions.clone(),
        normals: Some(normals.clone()),
        source_backend: "structured-ray-zero-crossing".to_string(),
    };
    let mesh = FieldMesh {
        positions,
        normals,
        triangles,
        source_backend: "structured-ray-zero-crossing".to_string(),
    };
    mesh.validate()?;
    Ok((point_cloud, mesh))
}

fn find_intersections_in_interval(
    field: &dyn ImplicitField,
    curve: &dyn ParametricCurve,
    config: RayIntersectionConfig,
) -> ImplicitResult<Vec<CurveHit>> {
    let iso = field.iso_value();
    let bounds = field.bounds();
    let max_steps = ((config.t_max - config.t_min) / config.step).ceil() as usize + 1;
    let mut hits = Vec::new();
    let mut previous: Option<(Scalar, Scalar)> = None;

    for step_index in 0..=max_steps {
        if hits.len() >= config.max_hits {
            break;
        }
        let current_t = (config.t_min + step_index as f64 * config.step).min(config.t_max);
        let current_point = curve.point_at(current_t);
        if !bounds.contains(current_point) {
            previous = None;
            if current_t >= config.t_max {
                break;
            }
            continue;
        }
        let current_value = field.value(current_point)? - iso;
        if !current_value.is_finite() {
            return Err(ImplicitError::NonFiniteFieldValue);
        }

        let near_surface = current_value.abs() <= config.value_tolerance;
        let bracket = previous.filter(|(_, value)| *value * current_value < 0.0);
        if near_surface || bracket.is_some() {
            let hit_t = if let Some((previous_t, previous_value)) = bracket {
                bisect_root(field, curve, previous_t, current_t, previous_value, config)?
            } else {
                current_t
            };
            let duplicate = hits
                .last()
                .map(|last: &CurveHit| (last.t - hit_t).abs() <= config.position_tolerance * 4.0)
                .unwrap_or(false);
            if !duplicate {
                let position = curve.point_at(hit_t);
                let normal = normalize_or_default(field.gradient(position)?);
                hits.push(CurveHit {
                    t: hit_t,
                    position,
                    normal,
                    field_value: field.value(position)?,
                });
            }
        }

        previous = Some((current_t, current_value));
        if current_t >= config.t_max {
            break;
        }
    }

    Ok(hits)
}

fn bisect_root(
    field: &dyn ImplicitField,
    curve: &dyn ParametricCurve,
    mut low_t: Scalar,
    mut high_t: Scalar,
    mut low_value: Scalar,
    config: RayIntersectionConfig,
) -> ImplicitResult<Scalar> {
    let iso = field.iso_value();
    for _ in 0..config.max_root_iterations {
        let mid_t = 0.5 * (low_t + high_t);
        let mid_value = field.value(curve.point_at(mid_t))? - iso;
        if mid_value.abs() <= config.value_tolerance
            || (high_t - low_t).abs() <= config.position_tolerance
        {
            return Ok(mid_t);
        }
        if low_value * mid_value < 0.0 {
            high_t = mid_t;
        } else {
            low_t = mid_t;
            low_value = mid_value;
        }
    }
    Ok(0.5 * (low_t + high_t))
}

fn ray_aabb_interval(ray: &Ray3, bounds: crate::Aabb3) -> Option<(Scalar, Scalar)> {
    let mut t_min = f64::NEG_INFINITY;
    let mut t_max = f64::INFINITY;
    for axis in 0..3 {
        let direction = ray.direction[axis];
        if direction.abs() <= f64::EPSILON {
            if ray.origin[axis] < bounds.min[axis] || ray.origin[axis] > bounds.max[axis] {
                return None;
            }
            continue;
        }
        let inverse = 1.0 / direction;
        let mut near = (bounds.min[axis] - ray.origin[axis]) * inverse;
        let mut far = (bounds.max[axis] - ray.origin[axis]) * inverse;
        if near > far {
            std::mem::swap(&mut near, &mut far);
        }
        t_min = t_min.max(near);
        t_max = t_max.min(far);
        if t_max < t_min {
            return None;
        }
    }
    Some((t_min, t_max))
}

fn triangle_within_edge_limit(
    positions: &[[f32; 3]],
    triangle: [u32; 3],
    max_edge_length: Scalar,
) -> bool {
    let a = positions[triangle[0] as usize];
    let b = positions[triangle[1] as usize];
    let c = positions[triangle[2] as usize];
    edge_length(a, b) <= max_edge_length
        && edge_length(b, c) <= max_edge_length
        && edge_length(c, a) <= max_edge_length
}

fn edge_length(a: [f32; 3], b: [f32; 3]) -> Scalar {
    let dx = (a[0] - b[0]) as Scalar;
    let dy = (a[1] - b[1]) as Scalar;
    let dz = (a[2] - b[2]) as Scalar;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn normalize_or_default(vector: Vector3) -> Vector3 {
    let length = vector
        .iter()
        .map(|value| value * value)
        .sum::<Scalar>()
        .sqrt();
    if !length.is_finite() || length <= f64::EPSILON {
        [0.0, 1.0, 0.0]
    } else {
        [vector[0] / length, vector[1] / length, vector[2] / length]
    }
}
