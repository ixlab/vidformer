//! Spec Intermediate Representation
//!
//! The SIR is a high-level representation of constructing a video frame from source frames and data.
//! This is the primary interface between specs and the rest of the system.

use crate::dve::Range;
use num_rational::Rational64;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Ord, PartialOrd, PartialEq, Eq, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub enum IndexConst {
    ILoc(usize),
    T(Rational64),
}

#[derive(Ord, PartialOrd, PartialEq, Eq, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct FrameSource {
    pub(crate) video: String,
    pub(crate) index: IndexConst,
}

impl FrameSource {
    pub fn new(video: String, index: IndexConst) -> Self {
        FrameSource { video, index }
    }

    pub fn video(&self) -> &str {
        &self.video
    }

    pub fn index(&self) -> &IndexConst {
        &self.index
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub struct FilterExpr {
    pub name: String,
    pub args: Vec<Expr>,
    pub kwargs: BTreeMap<String, Expr>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub enum Expr {
    Frame(FrameExpr),
    Data(DataExpr),
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Expr::Frame(frame) => write!(f, "{}", frame),
            Expr::Data(data) => write!(f, "{}", data),
        }
    }
}

impl Expr {
    pub(crate) fn add_source_deps<'a>(&'a self, deps: &mut BTreeSet<&'a FrameSource>) {
        match self {
            Expr::Frame(frame) => {
                frame.add_source_deps(deps);
            }
            Expr::Data(_) => {}
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq)]

pub enum DataExpr {
    Bool(bool),
    Int(i64),
    String(String),
    Bytes(Vec<u8>),
    Float(f64),
    List(Vec<DataExpr>),
}

impl Display for DataExpr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            DataExpr::Bool(b) => write!(f, "{}", b),
            DataExpr::Int(i) => write!(f, "{}", i),
            DataExpr::String(s) => write!(f, "\"{}\"", s),
            DataExpr::Bytes(b) => write!(f, "<{} bytes>", b.len()),
            DataExpr::Float(n) => write!(f, "{}", n),
            DataExpr::List(list) => {
                write!(f, "[")?;
                for (idx, item) in list.iter().enumerate() {
                    write!(f, "{}", item)?;
                    if idx < list.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "]")
            }
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub enum FrameExpr {
    Source(FrameSource),
    Filter(FilterExpr),
}

impl Display for FrameExpr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            FrameExpr::Source(src) => {
                write!(f, "{}{}", src.video, src.index)
            }
            FrameExpr::Filter(filter) => {
                write!(f, "{}(", filter.name)?;
                for arg in &filter.args {
                    write!(f, "{}, ", arg)?;
                }
                for (k, v) in &filter.kwargs {
                    write!(f, "{}={}, ", k, v)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl Display for IndexConst {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            IndexConst::ILoc(i) => write!(f, ".iloc[{}]", i),
            IndexConst::T(t) => write!(f, "[{}]", t),
        }
    }
}

impl FrameExpr {
    /// Add all referenced frame sources to a set.
    pub fn add_source_deps<'a>(&'a self, deps: &mut BTreeSet<&'a FrameSource>) {
        match self {
            FrameExpr::Source(src) => {
                deps.insert(src);
            }
            FrameExpr::Filter(filter) => {
                for arg in &filter.args {
                    arg.add_source_deps(deps);
                }
                for arg in filter.kwargs.values() {
                    arg.add_source_deps(deps);
                }
            }
        }
    }
}

pub(crate) struct ProcessSpan {
    pub(crate) ts: Vec<Rational64>,
    pub(crate) frames: Vec<FrameExpr>,
    pub(crate) output_ts_offset: Option<Rational64>,
}

pub(crate) fn spec_domain(
    spec: &dyn crate::spec::Spec,
    context: &crate::dve::Context,
) -> Vec<Rational64> {
    spec.domain(&context.spec_ctx())
}

impl ProcessSpan {
    pub(crate) fn create(
        spec: &dyn crate::spec::Spec,
        context: &crate::dve::Context,
        range: &Option<Range>,
    ) -> Self {
        let spec_ctx = context.spec_ctx();

        let mut ts = spec.domain(&spec_ctx);
        ts.sort();
        let ts = ts;

        let mut range_start_ts = None;
        let ts = match range {
            Some(range_config) => {
                assert!(ts.binary_search(&range_config.start).is_ok());
                assert!(ts.binary_search(&range_config.end).is_ok());
                if matches!(
                    range_config.ts_format,
                    crate::dve::RangeTsFormat::SegmentLocal
                ) {
                    range_start_ts = Some(range_config.start);
                }
                ts.into_iter()
                    .filter(|t: &num_rational::Ratio<i64>| {
                        t >= &range_config.start && t <= &range_config.end
                    })
                    .collect()
            }
            None => ts,
        };

        // TODO: parallelize?
        let mut frames = Vec::with_capacity(ts.len());
        for t in &ts {
            frames.push(spec.render(&spec_ctx, t));
        }

        // TODO: Data-dependent optimizations somewhere here?

        ProcessSpan {
            ts,
            frames,
            output_ts_offset: range_start_ts,
        }
    }
}
