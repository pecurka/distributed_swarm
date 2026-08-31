//! A simple 2D vector.
//!
//! Written by hand instead of using a library like `glam`, for two reasons.
//! First, `Agent` has to be sent between nodes over MPI, and Rust only lets you
//! add that ability to types your own crate defines. Second, the sequential and
//! distributed runs have to produce exactly the same numbers, which is easier
//! to be sure of when nothing is reordering the maths behind our back.
//!
//! It has only the operations the boids rules actually need.

/// A 2D vector. Used for both positions and velocities.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector2D {
    pub x: f64,
    pub y: f64,
}

impl Vector2D {
    pub const ZERO: Vector2D = Vector2D { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Length of the vector.
    pub fn len(self) -> f64 {
        self.len_sq().sqrt()
    }

    /// Length squared. Use this for distance checks — it skips the square
    /// root, which is the slow part.
    pub fn len_sq(self) -> f64 {
        self.x * self.x + self.y * self.y
    }
}

impl std::ops::Add for Vector2D {
    type Output = Vector2D;
    fn add(self, other: Vector2D) -> Vector2D {
        Vector2D::new(self.x + other.x, self.y + other.y)
    }
}

impl std::ops::Sub for Vector2D {
    type Output = Vector2D;
    fn sub(self, other: Vector2D) -> Vector2D {
        Vector2D::new(self.x - other.x, self.y - other.y)
    }
}

impl std::ops::Mul<f64> for Vector2D {
    type Output = Vector2D;
    fn mul(self, scalar: f64) -> Vector2D {
        Vector2D::new(self.x * scalar, self.y * scalar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_sums_each_axis() {
        let left = Vector2D::new(1.0, 2.0);
        let right = Vector2D::new(10.0, 20.0);
        assert_eq!(left + right, Vector2D::new(11.0, 22.0));
    }

    #[test]
    fn sub_subtracts_each_axis() {
        let left = Vector2D::new(10.0, 20.0);
        let right = Vector2D::new(1.0, 2.0);
        // Deliberately asymmetric numbers: if x and y were swapped somewhere,
        // equal values would hide it.
        assert_eq!(left - right, Vector2D::new(9.0, 18.0));
    }

    #[test]
    fn sub_flips_sign_when_arguments_swap() {
        let left = Vector2D::new(3.0, 7.0);
        let right = Vector2D::new(11.0, 2.0);
        let forward = left - right;
        let backward = right - left;
        assert_eq!(forward.x, -backward.x);
        assert_eq!(forward.y, -backward.y);
    }

    #[test]
    fn mul_scales_both_axes() {
        assert_eq!(Vector2D::new(3.0, -4.0) * 2.0, Vector2D::new(6.0, -8.0));
    }

    #[test]
    fn mul_by_zero_gives_the_zero_vector() {
        assert_eq!(Vector2D::new(3.0, -4.0) * 0.0, Vector2D::ZERO);
    }

    #[test]
    fn len_measures_the_hypotenuse() {
        // 3-4-5 triangle, so the answer is exact in floating point.
        assert_eq!(Vector2D::new(3.0, 4.0).len(), 5.0);
    }

    #[test]
    fn len_sq_is_len_without_the_square_root() {
        assert_eq!(Vector2D::new(3.0, 4.0).len_sq(), 25.0);
    }

    #[test]
    fn len_ignores_direction() {
        // Distance checks must not care which way the vector points.
        assert_eq!(Vector2D::new(-3.0, -4.0).len(), 5.0);
    }

    #[test]
    fn zero_has_no_length() {
        assert_eq!(Vector2D::ZERO.len(), 0.0);
        assert_eq!(Vector2D::ZERO, Vector2D::new(0.0, 0.0));
    }
}
