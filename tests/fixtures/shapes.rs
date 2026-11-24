//! Geometry primitives.

use std::fmt::Debug;
use std::ops::Add;

pub mod utils;

extern crate num_traits;

/// A drawable shape.
///
/// Every shape exposes its area and a human-readable name.
pub trait Shape: Debug {
    /// @brief Compute the shape's area.
    /// @return Area in square units.
    fn area(&self) -> f64;

    /// @brief Describe the shape.
    /// @return A short label.
    fn describe(&self) -> String {
        let label = self.name();
        format!("{label}: {}", self.area())
    }

    /// @brief The shape's display name.
    fn name(&self) -> String;
}

/// A rectangle with sides parallel to the axes.
#[derive(Debug, Clone)]
pub struct Rectangle {
    pub width: f64,
    height: f64,

    /// Cached area, when computed.
    cached: Option<f64>,
}

impl Rectangle {
    /// @brief Create a rectangle.
    /// @param[in] width The horizontal extent.
    /// @param[in] height The vertical extent.
    /// @return A new rectangle.
    pub fn new(width: f64, height: f64) -> Rectangle {
        Rectangle {
            width,
            height,
            cached: None,
        }
    }

    /// @brief Scale both dimensions.
    /// @param[in] factor The scale factor.
    pub fn scale(&mut self, factor: f64) -> &mut Self {
        self.width *= factor;
        self.height *= factor;
        self.cached = None;
        self
    }

    /// @brief Helper that must not appear in the contract.
    fn recalculate(&mut self) {
        self.cached = Some(self.area());
    }
}

impl Shape for Rectangle {
    fn area(&self) -> f64 {
        self.width * self.height
    }

    fn name(&self) -> String {
        String::from("rectangle")
    }
}

impl<T> Add for Rectangle
where
    T: Into<f64>,
{
    type Output = Rectangle;

    fn add(self, other: Rectangle) -> Rectangle {
        Rectangle::new(self.width + other.width, self.height + other.height)
    }
}

/// Canonical shape kinds.
pub enum Kind {
    /// A circle.
    Circle,
    /// An axis-aligned rectangle.
    Rectangle(Rectangle),
    /// An arbitrary polygon.
    Polygon {
        /// Number of vertices.
        sides: usize,
    },
}
