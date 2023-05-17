mod ply;
mod shape;
use bevy::{asset::Error, prelude::Vec3, render::render_resource::Extent3d};

use crate::ray_marching::ShapeImage;

pub struct Model {
    min: Vec3,
    max: Vec3,
    triangles: Vec<Triangle>,
}

impl Model {
    pub fn new() -> Self {
        Self {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
            triangles: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
            triangles: Vec::with_capacity(capacity),
        }
    }

    pub fn from_ply(string: String) -> Result<Self, Error> {
        ply::load(string)
    }

    pub fn to_shape_image(&self, resolution: Extent3d, padding: u32) -> ShapeImage {
        shape::build(self, resolution, padding)
    }

    pub fn min(&self) -> Vec3 {
        self.min
    }

    pub fn max(&self) -> Vec3 {
        self.max
    }

    pub fn push_triangle(&mut self, p1: Vec3, p2: Vec3, p3: Vec3) {
        let min = Vec3::new(
            p1.x.min(p2.x).min(p3.x),
            p1.y.min(p2.y).min(p3.y),
            p1.z.min(p2.z).min(p3.z),
        );
        let max = Vec3::new(
            p1.x.max(p2.x).max(p3.x),
            p1.y.max(p2.y).max(p3.y),
            p1.z.max(p2.z).max(p3.z),
        );

        if self.triangles.is_empty() {
            self.min = min;
            self.max = max;
        } else {
            self.min.x = self.min.x.min(min.x);
            self.min.y = self.min.y.min(min.y);
            self.min.z = self.min.z.min(min.z);
            self.max.x = self.max.x.max(max.x);
            self.max.y = self.max.y.max(max.y);
            self.max.z = self.max.z.max(max.z);
        }
        self.triangles.push(Triangle::new(p1, p2, p3));
    }

    pub fn distance(&self, pnt: Vec3) -> f32 {
        let dir = (pnt - (self.min + self.max) / 2.0).normalize();

        let mut dist = f32::INFINITY;
        let mut intersections = 0;
        for triangle in self.triangles.iter() {
            if triangle.dist_approx(pnt) < dist {
                dist = f32::min(dist, triangle.dist(pnt));
            }
            if triangle.intersects(pnt, dir) {
                intersections += 1;
            }
        }

        if intersections % 2 == 0 {
            dist
        } else {
            -dist
        }
    }
}

struct Triangle {
    p1: Vec3,
    p2: Vec3,
    p3: Vec3,
    e21: Vec3,
    e32: Vec3,
    e13: Vec3,
    norm: Vec3,
    c21: Vec3,
    c32: Vec3,
    c13: Vec3,
    center: Vec3,
    radius: f32,
}

impl Triangle {
    fn new(p1: Vec3, p2: Vec3, p3: Vec3) -> Self {
        let e21 = p2 - p1;
        let e32 = p3 - p2;
        let e13 = p1 - p3;
        let norm = -e21.cross(e13);
        let c21 = e21.cross(norm);
        let c32 = e32.cross(norm);
        let c13 = e13.cross(norm);
        let center = (p1 + p2 + p3) / 3.0;
        let radius = center
            .distance(p1)
            .max(center.distance(p2))
            .max(center.distance(p3));
        Self {
            p1,
            p2,
            p3,
            e21,
            e32,
            e13,
            norm,
            c21,
            c32,
            c13,
            center,
            radius,
        }
    }

    fn dist_approx(&self, pnt: Vec3) -> f32 {
        self.center.distance(pnt) - self.radius
    }

    fn dist(&self, pnt: Vec3) -> f32 {
        let d1 = pnt - self.p1;
        let d2 = pnt - self.p2;
        let d3 = pnt - self.p3;

        return f32::sqrt(
            if self.c21.dot(d1).signum() + self.c32.dot(d2).signum() + self.c13.dot(d3).signum()
               > -2.0
            {
                (self.e21 * (self.e21.dot(d1) / self.e21.length_squared()).clamp(0.0, 1.0) - d1)
                    .length_squared()
                    .min(
                        (self.e32 * (self.e32.dot(d2) / self.e32.length_squared()).clamp(0.0, 1.0)
                         - d2)
                            .length_squared(),
                    )
                    .min(
                        (self.e13 * (self.e13.dot(d3) / self.e13.length_squared()).clamp(0.0, 1.0)
                         - d3)
                            .length_squared(),
                    )
            } else {
                self.norm.dot(d1) * self.norm.dot(d1) / self.norm.length_squared()
            },
        );
    }

    fn intersects(&self, pnt: Vec3, dir: Vec3) -> bool {
        let to_center = self.center - pnt;
        let doc = dir.dot(to_center);
        if to_center.length_squared() - doc * doc > self.radius * self.radius {
            return false;
        }

        let nod = self.norm.dot(dir);
        if nod == 0.0 {
            return false;
        }

        let t = -(self.norm.dot(pnt) - self.norm.dot(self.p1)) / nod;
        if t < 0.0 {
            return false;
        }

        let p = pnt + dir * t;

        if self.norm.dot(self.e21.cross(p - self.p1)) < 0.0 {
            return false;
        }

        if self.norm.dot(self.e32.cross(p - self.p2)) < 0.0 {
            return false;
        }

        if self.norm.dot(self.e13.cross(p - self.p3)) < 0.0 {
            return false;
        }

        true
    }
}
