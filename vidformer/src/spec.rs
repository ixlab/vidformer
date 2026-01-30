//! Specs declaratively define an edited video.
//!
//! A spec defines an output video (as a logical time-indexed array).
//! It creates this array by defining its timestamps (the set of times at which the spec is defined) and a function that maps each time to a frame.
//! The frame is represented as an [`FrameExpr`] expression.
//!
//! Specs are stateless and immutable. They are intended to be smaller than just storing the entire output video as an array.
//! Additionally, this generic spec interface allows the general case of a video-editing DSL, as done in V2V, while allowing for whatever language the use case needs.

use crate::sir::FrameExpr;
use num_rational::Rational64;

/// A trait for providing information to a spec during runtime
pub trait SpecContext {}

/// A trait for a spec
///
/// A spec defines a video from a sequence of transformations.
/// A [`SpecContext`] is provided for information about the available source videos and arrays.
pub trait Spec: Sync + Send {
    /// Returns the timestamps of the output video.
    ///
    /// The timestamps are the set of times at which the spec is defined.
    /// Each time corresponds to a single output frame at that timestamp.
    ///
    /// The output must:
    /// - Be sorted in ascending order
    /// - Contain no duplicate values
    /// - Begin with 0
    fn timestamps(&self, context: &dyn SpecContext) -> Vec<Rational64>;

    /// Returns the "logically" rendered frame at a given time.
    ///
    /// This function should assume that the time is in the domain of the spec.
    fn render(&self, context: &dyn SpecContext, t: &Rational64) -> FrameExpr;
}

pub(crate) fn get_framerate(spec: &dyn Spec) -> usize {
    // TODO: Not all framerates are widely supported in HLS. We should probably add a check for that.

    let context = crate::dve::EmptySpecCtx;
    let frame_times = spec.timestamps(&context);

    let mut frame_deltas = frame_times.windows(2).map(|w| w[1] - w[0]);

    // Default to 30fps if there's only one frame. Not ideal, but framerate doesn't matter in that case.
    // We don't want a user to try one frame and it break on them.
    // TODO: Error on this case?
    let first_delta = frame_deltas.next().unwrap_or(Rational64::new(1, 30));

    assert!(frame_deltas.all(|d| d == first_delta));
    assert_eq!(*first_delta.numer(), 1, "Frame rate must be an integer");

    *first_delta.denom() as usize
}

/// A spec backed by a JSON file
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct JsonSpec {
    pub frames: Vec<(Rational64, FrameExpr)>,
}

impl Spec for JsonSpec {
    fn timestamps(&self, _context: &dyn SpecContext) -> Vec<Rational64> {
        self.frames.iter().map(|(t, _)| *t).collect()
    }

    fn render(&self, _context: &dyn SpecContext, t: &Rational64) -> FrameExpr {
        let index = self
            .frames
            .binary_search_by(|(t2, _)| t2.partial_cmp(t).unwrap())
            .unwrap();

        self.frames[index].1.clone()
    }
}
