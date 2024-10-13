//! Arrays provide access to data.
//!
//! Arrays store conventional data (e.g. numbers, strings) and can be referenced by specs.
//! As with the rest of the vidformer data model, arrays are duel-indexed by a timestamp or a position.
//! Arrays can be backed by a variety of sources, such as a JSON file or a database.

use num::Rational64;

/// A trait for an array
pub trait Array: Sync + Send {
    /// Returns the domain of the array.
    ///
    /// The domain is the set of times at which the array is defined.
    /// Each time corresponds to a single output value at that timestamp.
    ///
    /// The output must:
    /// - Be sorted in ascending order
    /// - Contain no duplicate values
    /// - Begin with 0
    fn domain(&self) -> Vec<Rational64>;

    /// Returns the value at a given positional index.
    fn index(&self, idx: usize) -> crate::sir::DataExpr;

    /// Returns the value at a given time index.
    fn index_t(&self, idx: Rational64) -> crate::sir::DataExpr;
}

/// An array backed by a JSON file
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct JsonArary {
    array: Vec<(Rational64, crate::sir::DataExpr)>,
}

impl Array for JsonArary {
    fn domain(&self) -> Vec<Rational64> {
        self.array.iter().map(|(t, _)| *t).collect()
    }

    fn index(&self, idx: usize) -> crate::sir::DataExpr {
        self.array[idx].1.clone()
    }

    fn index_t(&self, idx: Rational64) -> crate::sir::DataExpr {
        let idx = self
            .array
            .binary_search_by(|(t, _)| t.partial_cmp(&idx).unwrap())
            .unwrap();
        self.index(idx)
    }
}
