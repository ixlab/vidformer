//! Filters create and transform video frames
//!
//! A filter is a function that takes some inputs and returns a video frame.
//! For example a filter could blur an input: `MyBlur(frame, amount=5)`.

use crate::dve::AVFrame;
use opencv::prelude::MatTraitConst;
use rusty_ffmpeg::ffi;
use serde::ser::SerializeMap;
use serde::Deserialize;
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

impl serde::Serialize for Frame {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("width", &self.width)?;
        map.serialize_entry("height", &self.height)?;

        let pix_format_name = crate::util::pixel_fmt_str(self.format);
        map.serialize_entry("format", pix_format_name)?;

        let av_frame: ffi::AVFrame = unsafe { *self.inner.inner };
        let frame_encoded_size_all_planes =
            unsafe { ffi::av_image_get_buffer_size(self.format, self.width, self.height, 1) };

        let frame_buffer = vec![0u8; frame_encoded_size_all_planes as usize];
        let frame_buffer_ptr = frame_buffer.as_ptr() as *mut u8;

        // Copy the frame data to the buffer
        unsafe {
            ffi::av_image_copy_to_buffer(
                frame_buffer_ptr,
                frame_encoded_size_all_planes,
                av_frame.data.as_ptr() as *const *const u8,
                av_frame.linesize.as_ptr(),
                self.format,
                self.width,
                self.height,
                1,
            );
        }

        let frame_buffer = serde_bytes::Bytes::new(&frame_buffer);
        map.serialize_entry("data", &frame_buffer)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for Frame {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FrameVisitor;

        impl<'de> serde::de::Visitor<'de> for FrameVisitor {
            type Value = Frame;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a frame")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut width = None;
                let mut height = None;
                let mut format = None;
                let mut data = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "width" => {
                            if width.is_some() {
                                return Err(serde::de::Error::duplicate_field("width"));
                            }
                            width = Some(map.next_value()?);
                        }
                        "height" => {
                            if height.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height = Some(map.next_value()?);
                        }
                        "format" => {
                            if format.is_some() {
                                return Err(serde::de::Error::duplicate_field("format"));
                            }
                            format = Some(map.next_value()?);
                        }
                        "data" => {
                            if data.is_some() {
                                return Err(serde::de::Error::duplicate_field("data"));
                            }
                            data = Some(map.next_value()?);
                        }
                        _ => {
                            return Err(serde::de::Error::unknown_field(
                                key,
                                &["width", "height", "format", "data"],
                            ));
                        }
                    }
                }

                let width: i32 = width.ok_or_else(|| serde::de::Error::missing_field("width"))?;
                let height: i32 =
                    height.ok_or_else(|| serde::de::Error::missing_field("height"))?;
                let format_str: &str =
                    format.ok_or_else(|| serde::de::Error::missing_field("format"))?;

                let fmt_cstr = std::ffi::CString::new(format_str).unwrap();

                let format = unsafe { ffi::av_get_pix_fmt(fmt_cstr.as_ptr()) };
                if format == ffi::AVPixelFormat_AV_PIX_FMT_NONE {
                    return Err(serde::de::Error::custom(format!(
                        "Invalid pixel format {:?}",
                        format_str
                    )));
                }

                let data: serde_bytes::ByteBuf =
                    data.ok_or_else(|| serde::de::Error::missing_field("data"))?;
                let data = data.into_vec();

                // Check we got the right amount of data
                let frame_encoded_size_all_planes = unsafe {
                    ffi::av_image_get_buffer_size(format as ffi::AVPixelFormat, width, height, 1)
                };
                if data.len() != frame_encoded_size_all_planes as usize {
                    return Err(serde::de::Error::custom(format!(
                        "IPC frame data length ({}) does not match expected size ({})",
                        data.len(),
                        frame_encoded_size_all_planes
                    )));
                }

                if data.len() != frame_encoded_size_all_planes as usize {
                    return Err(serde::de::Error::custom(
                        "Data length does not match frame size",
                    ));
                }

                let av_frame = unsafe { ffi::av_frame_alloc() };
                if av_frame.is_null() {
                    return Err(serde::de::Error::custom("Failed to allocate frame"));
                }

                unsafe {
                    (*av_frame).width = width;
                    (*av_frame).height = height;
                    (*av_frame).format = format;
                };

                // alloc frame buffer
                let ret = unsafe { ffi::av_frame_get_buffer(av_frame, 0) };
                if ret < 0 {
                    return Err(serde::de::Error::custom("Failed to allocate frame buffer"));
                }

                // Copy the data to the frame
                let num_planes =
                    unsafe { ffi::av_pix_fmt_count_planes(format as ffi::AVPixelFormat) };
                if num_planes < 0 {
                    return Err(serde::de::Error::custom("Failed to get number of planes"));
                }

                let mut data_offset = 0;
                for plane in 0..num_planes {
                    let plane_size = unsafe { (*av_frame).linesize[plane as usize] * height };
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            data.as_ptr().add(data_offset),
                            (*av_frame).data[plane as usize],
                            plane_size as usize,
                        );
                    }
                    data_offset += plane_size as usize;
                }

                let frame = AVFrame { inner: av_frame };

                Ok(Frame::new(frame))
            }
        }

        deserializer.deserialize_map(FrameVisitor)
    }
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

    pub(crate) fn inner(&self) -> *mut ffi::AVFrame {
        self.inner.inner
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
#[derive(Debug, Clone, serde::Serialize)]
pub enum Val {
    Frame(Frame),
    FrameType(FrameType),
    Bool(bool),
    Int(i64),
    String(String),
    Float(f64),
    List(Vec<Val>),
}

#[derive(PartialEq, Eq, Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    pub fn from_expr(expr: &crate::sir::DataExpr, context: &crate::dve::Context) -> Self {
        match expr {
            crate::sir::DataExpr::Bool(b) => Val::Bool(*b),
            crate::sir::DataExpr::Int(i) => Val::Int(*i),
            crate::sir::DataExpr::String(s) => Val::String(s.to_string()),
            crate::sir::DataExpr::Float(f) => Val::Float(*f),
            crate::sir::DataExpr::ArrayRef(name, crate::sir::IndexConst::ILoc(idx)) => {
                let array = &context.arrays[name];
                let val = array.index(*idx);
                Val::from_expr(&val, context)
            }
            crate::sir::DataExpr::ArrayRef(name, crate::sir::IndexConst::T(t)) => {
                let array = &context.arrays[name];
                let val = array.index_t(*t);
                Val::from_expr(&val, context)
            }
            crate::sir::DataExpr::List(list) => {
                let list = list
                    .iter()
                    .map(|expr| Val::from_expr(expr, context))
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

    pub(crate) fn as_frame_type(&self) -> Option<&FrameType> {
        match self {
            Val::FrameType(frame_type) => Some(frame_type),
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
