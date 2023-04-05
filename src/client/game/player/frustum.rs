use nalgebra::{Point3, Vector3};

pub struct Frustum {
    forward: Vector3<f32>,
    right: Vector3<f32>,
    up: Vector3<f32>,
    origin: Point3<f32>,
    height: f32,
    aspect: f32,
    znear: f32,
    zfar: f32,
    sphere_factor_x: f32,
    sphere_factor_y: f32,
}

impl Frustum {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        forward: Vector3<f32>,
        right: Vector3<f32>,
        up: Vector3<f32>,
        origin: Point3<f32>,
        fovy: f32,
        aspect: f32,
        znear: f32,
        zfar: f32,
    ) -> Self {
        let height = (fovy * 0.5).tan();
        let width = height * aspect;
        let sphere_factor_x = 1.0 / width.atan().cos();
        let sphere_factor_y = 1.0 / height.atan().cos();
        Self {
            forward,
            right,
            up,
            origin,
            height,
            aspect,
            znear,
            zfar,
            sphere_factor_x,
            sphere_factor_y,
        }
    }
}

pub trait FrustumCheck {
    fn is_visible(&self, frustum: &Frustum) -> bool;
}

pub struct BoundingSphere {
    pub center: Point3<f32>,
    pub radius: f32,
}

impl BoundingSphere {
    pub fn new(center: Point3<f32>, radius: f32) -> Self {
        Self { center, radius }
    }
}

impl FrustumCheck for BoundingSphere {
    fn is_visible(&self, frustum: &Frustum) -> bool {
        let v = self.center - frustum.origin;

        let az = v.dot(&frustum.forward);
        if !(frustum.znear - self.radius..=frustum.zfar + self.radius).contains(&az) {
            return false;
        }

        let ay = v.dot(&frustum.up);
        let az = az * frustum.height;
        let d = frustum.sphere_factor_y * self.radius;
        if !(-az - d..=az + d).contains(&ay) {
            return false;
        }

        let ax = v.dot(&frustum.right);
        let az = az * frustum.aspect;
        let d = frustum.sphere_factor_x * self.radius;
        if !(-az - d..=az + d).contains(&ax) {
            return false;
        }

        true
    }
}
