use nalgebra::{Matrix3, Point3, SymmetricEigen, Unit, UnitVector3, Vector3};

pub fn p3(a: [f32; 3]) -> Point3<f64> {
    Point3::new(a[0] as f64, a[1] as f64, a[2] as f64)
}

pub fn v3(a: [f64; 3]) -> Vector3<f64> {
    Vector3::new(a[0], a[1], a[2])
}

pub fn arr3(v: Vector3<f64>) -> [f64; 3] {
    [v.x, v.y, v.z]
}

pub fn centroid(points: &[Point3<f64>]) -> Point3<f64> {
    let mut acc = Vector3::zeros();
    for p in points {
        acc += p.coords;
    }
    Point3::from(acc / points.len() as f64)
}

pub fn covariance(points: &[Point3<f64>], center: Point3<f64>) -> Matrix3<f64> {
    let mut c = Matrix3::zeros();
    for p in points {
        let d = p - center;
        c += d * d.transpose();
    }
    c / points.len() as f64
}

pub fn plane_from_points(points: &[Point3<f64>]) -> Option<(UnitVector3<f64>, f64, f64)> {
    if points.len() < 3 {
        return None;
    }
    let center = centroid(points);
    let cov = covariance(points, center);
    let eig = SymmetricEigen::new(cov);

    let mut min_idx = 0usize;
    let mut min_val = eig.eigenvalues[0];
    for i in 1..3 {
        if eig.eigenvalues[i] < min_val {
            min_val = eig.eigenvalues[i];
            min_idx = i;
        }
    }

    let n = eig.eigenvectors.column(min_idx).into_owned();
    let unit = Unit::try_new(n, 1e-12)?;
    let d = -unit.dot(&center.coords);

    let mut rmse_sum = 0.0;
    for p in points {
        let dist = unit.dot(&p.coords) + d;
        rmse_sum += dist * dist;
    }
    let rmse = (rmse_sum / points.len() as f64).sqrt();

    Some((unit, d, rmse))
}

pub fn orthonormal_plane_basis(normal: UnitVector3<f64>) -> (Vector3<f64>, Vector3<f64>) {
    let n = normal.into_inner();
    let helper = if n.z.abs() < 0.9 {
        Vector3::z_axis().into_inner()
    } else {
        Vector3::x_axis().into_inner()
    };
    let u = n.cross(&helper).normalize();
    let v = n.cross(&u).normalize();
    (u, v)
}

pub fn project_to_plane_2d(
    origin: Point3<f64>,
    u: Vector3<f64>,
    v: Vector3<f64>,
    p: Point3<f64>,
) -> [f64; 2] {
    let d = p - origin;
    [d.dot(&u), d.dot(&v)]
}

pub fn angle_between_degrees(a: UnitVector3<f64>, b: UnitVector3<f64>) -> f64 {
    let cos = a.dot(&b).clamp(-1.0, 1.0);
    cos.acos().to_degrees()
}
