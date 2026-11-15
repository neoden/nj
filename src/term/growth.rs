//! Which shape of growth a cost has over the values of `%for`. `sandbox`
//! has one cost for each value, a number of bytes or of nanoseconds, and
//! asks here.
//!
//! Each shape is a formula with two unknown numbers, a part that does not
//! depend on the value and a factor for the part that does:
//!
//!   constant    cost = a
//!   log         cost = a + b · ln(value)
//!   linear      cost = a + b · value
//!   quadratic   cost = a + b · value²
//!
//! For each formula the least-squares fit finds the `a` and `b` that put
//! it nearest the points, and how far from them it still is. The shape of
//! the cost is the slowest one that fits. The part `a` is in every
//! formula, so a cost that is mostly constant still shows how the rest of
//! it grows.
//!
//! Test machinery, so debug builds only.

use super::scenario::Growth;

/// A cost that differs from the formula by less than this part of itself
/// is on the formula. Costs are not exact: what a step allocates changes
/// by some bytes with the number of digits in a line number.
const NEAR: f64 = 0.01;

/// How many times the noise of a point it may be off the formula and
/// still be on it.
const NOISE: f64 = 3.0;

/// A formula no more than this many times as far from the points as the
/// nearest of all is taken to fit as well. For costs that follow no
/// formula closely, a buffer that doubles say: the slowest shape that is
/// about as near as any is the one.
const AS_NEAR: f64 = 2.0;

/// A shape of growth, from the slowest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Shape {
    Constant,
    Log,
    Linear,
    /// Not a growth a scenario can ask for: here so that a cost growing
    /// faster than `linear` has something to fit better.
    Quadratic,
}

const SHAPES: [Shape; 4] = [Shape::Constant, Shape::Log, Shape::Linear, Shape::Quadratic];

impl Shape {
    pub fn name(self) -> &'static str {
        match self {
            Shape::Constant => "constant",
            Shape::Log => "log",
            Shape::Linear => "linear",
            Shape::Quadratic => "quadratic",
        }
    }

    /// Whether a cost of this shape grows no faster than `growth` allows.
    pub fn within(self, growth: Growth) -> bool {
        let allowed = match growth {
            Growth::Constant => Shape::Constant,
            Growth::Log => Shape::Log,
            Growth::Linear => Shape::Linear,
        };
        self <= allowed
    }

    /// The part of the formula that depends on the value.
    fn of(self, value: f64) -> f64 {
        match self {
            Shape::Constant => 0.0,
            Shape::Log => value.ln(),
            Shape::Linear => value,
            Shape::Quadratic => value * value,
        }
    }
}

/// The cost measured at one value.
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub value: f64,
    pub cost: f64,
    /// How much the cost varied between the passes at this value.
    pub noise: f64,
}

/// What the points say.
#[derive(Debug, Clone, PartialEq)]
pub struct Fit {
    /// The slowest shape that fits.
    pub shape: Shape,
    /// For each shape, the furthest a point is from its formula, as a part
    /// of the point's cost.
    pub off: [(Shape, f64); 4],
}

/// The shape of the growth of `points`, three or more at different values.
pub fn fit(points: &[Point]) -> Fit {
    let fits = SHAPES.map(|shape| (shape, fit_of(points, shape)));
    let nearest = fits
        .iter()
        .map(|(_, fit)| fit.off)
        .fold(f64::INFINITY, f64::min);
    let (shape, _) = fits
        .iter()
        .find(|(_, fit)| fit.on || fit.off <= nearest * AS_NEAR)
        .expect("the nearest is as near as itself");
    Fit {
        shape: *shape,
        off: fits.map(|(shape, fit)| (shape, fit.off)),
    }
}

/// How one formula fits.
struct FitOf {
    /// The furthest a point is from the formula, as a part of its cost.
    off: f64,
    /// Whether every point is on the formula, by `NEAR` and `NOISE`.
    on: bool,
}

/// The least-squares fit of `shape` to `points`. The distance that is
/// made least is the one relative to the cost, so that a point at a small
/// value counts as much as one at a large value, where the cost may be a
/// thousand times more.
fn fit_of(points: &[Point], shape: Shape) -> FitOf {
    // Divided by the largest so that the sums below stay in range.
    let largest = points
        .iter()
        .map(|point| shape.of(point.value).abs())
        .fold(f64::MIN_POSITIVE, f64::max);
    // A cost of nothing, no allocation at all, has no part to be relative
    // to. One byte or one nanosecond stands for it.
    let rows: Vec<(f64, f64, f64)> = points
        .iter()
        .map(|point| {
            let cost = point.cost.max(1.0);
            (shape.of(point.value) / largest, cost, 1.0 / (cost * cost))
        })
        .collect();
    let sum = |term: fn(f64, f64) -> f64| -> f64 {
        rows.iter().map(|&(x, y, weight)| weight * term(x, y)).sum()
    };
    let (one, x, xx, y, xy) = (
        sum(|_, _| 1.0),
        sum(|x, _| x),
        sum(|x, _| x * x),
        sum(|_, y| y),
        sum(|x, y| x * y),
    );

    // The two unknowns of the formula. Neither is below zero: a cost does
    // not fall as the value grows, and no part of it is less than nothing.
    // Where the free fit would have one below, the fit without that part
    // is the nearest allowed.
    let flat = (y / one, 0.0);
    let (a, b) = if shape == Shape::Constant {
        flat
    } else {
        let b = (one * xy - x * y) / (one * xx - x * x);
        let a = (y - b * x) / one;
        if b < 0.0 {
            flat
        } else if a < 0.0 {
            (0.0, xy / xx)
        } else {
            (a, b)
        }
    };

    let mut fit = FitOf { off: 0.0, on: true };
    for (point, &(x, cost, _)) in points.iter().zip(&rows) {
        let apart = (a + b * x - cost).abs();
        fit.off = fit.off.max(apart / cost);
        fit.on &= apart <= cost * NEAR + point.noise * NOISE;
    }
    fit
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of `cost` at five values a factor of ten apart, measured
    /// without noise.
    fn shape(cost: impl Fn(f64) -> f64) -> Shape {
        let points: Vec<Point> = [1e3, 1e4, 1e5, 1e6, 1e7]
            .into_iter()
            .map(|value| Point {
                value,
                cost: cost(value),
                noise: 0.0,
            })
            .collect();
        fit(&points).shape
    }

    #[test]
    fn a_cost_has_the_shape_of_its_formula() {
        assert_eq!(shape(|_| 5000.0), Shape::Constant);
        assert_eq!(shape(|value| 300.0 * value.ln()), Shape::Log);
        assert_eq!(shape(|value| 3.0 * value), Shape::Linear);
        assert_eq!(shape(|value| value * value), Shape::Quadratic);
    }

    /// The part that does not grow is most of the cost at every value but
    /// the last, and the growth is still seen for what it is.
    #[test]
    fn a_large_constant_part_does_not_hide_the_growth() {
        assert_eq!(shape(|value| 600_000.0 + 14_000.0 * value.ln()), Shape::Log);
        assert_eq!(shape(|value| 600_000.0 + 0.03 * value), Shape::Linear);
        assert_eq!(shape(|value| 1e7 + value * value / 1e6), Shape::Quadratic);
    }

    #[test]
    fn a_shape_is_within_its_growth_and_every_faster_one() {
        assert!(Shape::Constant.within(Growth::Constant));
        assert!(Shape::Constant.within(Growth::Linear));
        assert!(Shape::Log.within(Growth::Log));
        assert!(!Shape::Log.within(Growth::Constant));
        assert!(!Shape::Linear.within(Growth::Log));
        assert!(Shape::Linear.within(Growth::Linear));
        assert!(!Shape::Quadratic.within(Growth::Linear));
    }

    /// A cost that changes by less than a hundredth of itself does not
    /// grow, whichever way the change happens to lean.
    #[test]
    fn a_change_of_some_bytes_is_no_growth() {
        let points = [(2e5, 89249.0), (2e6, 89345.0), (2e7, 89377.0)].map(|(value, cost)| Point {
            value,
            cost,
            noise: 0.0,
        });
        assert_eq!(fit(&points).shape, Shape::Constant);
    }

    /// The same costs are a logarithm when measured exactly and constant
    /// when the passes at each value differed by as much as the values do.
    #[test]
    fn a_growth_within_the_noise_is_no_growth() {
        let points = |noise| {
            [(2e5, 620.0), (2e6, 654.0), (2e7, 687.0)].map(|(value, cost)| Point {
                value,
                cost,
                noise,
            })
        };
        assert_eq!(fit(&points(1.0)).shape, Shape::Log);
        assert_eq!(fit(&points(30.0)).shape, Shape::Constant);
    }

    /// A buffer that doubles follows no formula closely: what it allocates
    /// is between one and two times the value. It is linear all the same.
    #[test]
    fn a_cost_that_jumps_about_a_line_is_linear() {
        assert_eq!(
            shape(|value| value.log2().ceil().exp2() * 2.0),
            Shape::Linear
        );
    }

    #[test]
    fn a_cost_of_nothing_is_constant() {
        assert_eq!(shape(|_| 0.0), Shape::Constant);
    }
}
