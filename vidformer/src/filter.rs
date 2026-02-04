//! Filters create and transform video frames
//!
//! A filter is a function that takes some inputs and returns a video frame.
//! For example a filter could blur an input: `MyBlur(frame, amount=5)`.

use crate::dve::AVFrame;
use opencv::prelude::MatTraitConst;
use rusty_ffmpeg::ffi;
use std::collections::BTreeMap;
use std::sync::Arc;

pub mod builtin;
pub mod cv2;
mod filter_utils;

/// A decoded video frame
#[derive(Clone)]
pub struct Frame {
    inner: Arc<AVFrame>,
    pub width: i32,
    pub height: i32,
    pub format: ffi::AVPixelFormat,
}

impl Frame {
    pub(crate) fn new(inner: AVFrame) -> Self {
        let width = unsafe { (*inner.inner).width };
        let height = unsafe { (*inner.inner).height };
        let format = unsafe { (*inner.inner).format };
        Frame {
            inner: Arc::new(inner),
            width,
            height,
            format,
        }
    }

    pub(crate) fn new_arc(inner: Arc<AVFrame>) -> Self {
        let width = unsafe { (*inner.inner).width };
        let height = unsafe { (*inner.inner).height };
        let format = unsafe { (*inner.inner).format };
        Frame {
            inner,
            width,
            height,
            format,
        }
    }

    pub(crate) fn into_avframe(self) -> Arc<AVFrame> {
        self.inner
    }
}

impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frame")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("format", &self.format)
            .finish()
    }
}

/// A value that can be passed to a filter
///
/// This can be a either a video frame or a conventional data value
#[derive(Debug, Clone)]
pub enum Val {
    Frame(Frame),
    FrameType(FrameType),
    Bool(bool),
    Int(i64),
    String(String),
    Bytes(Vec<u8>),
    Float(f64),
    List(Vec<Val>),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct FrameType {
    pub width: usize,
    pub height: usize,
    pub format: ffi::AVPixelFormat,
}

impl FrameType {
    pub fn new(width: usize, height: usize, format: ffi::AVPixelFormat) -> Self {
        FrameType {
            width,
            height,
            format,
        }
    }
}

impl Val {
    pub fn from_data_expr_with_frame_converter<F>(
        expr: &crate::sir::DataExpr,
        _context: &crate::dve::Context,
        frame_converter: F,
    ) -> Self
    where
        F: Fn(&crate::sir::FrameExpr) -> Val + Copy,
    {
        match expr {
            crate::sir::DataExpr::Bool(b) => Val::Bool(*b),
            crate::sir::DataExpr::Int(i) => Val::Int(*i),
            crate::sir::DataExpr::String(s) => Val::String(s.to_string()),
            crate::sir::DataExpr::Bytes(b) => Val::Bytes(b.clone()),
            crate::sir::DataExpr::Float(f) => Val::Float(*f),
            crate::sir::DataExpr::List(list) => {
                let list = list
                    .iter()
                    .map(|item| match item {
                        crate::sir::Expr::Frame(frame) => frame_converter(frame),
                        crate::sir::Expr::Data(data) => Self::from_data_expr_with_frame_converter(
                            data,
                            _context,
                            frame_converter,
                        ),
                    })
                    .collect();
                Val::List(list)
            }
        }
    }

    pub(crate) fn as_int(&self) -> Option<i64> {
        match self {
            Val::Int(i) => Some(*i),
            _ => None,
        }
    }
}

/// A filter that can create a video frame from some inputs
pub trait Filter: Send + Sync {
    /// Creates a video frame from some inputs
    ///
    /// The inputs are arbitrary and user provided. An input can either be
    /// ordered (arg) or named (kwargs). This is similar to Python function calls.
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error>;

    fn filter_type(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, crate::dve::Error>;
}
