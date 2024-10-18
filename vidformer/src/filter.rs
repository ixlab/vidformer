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

#[derive(Clone, Debug)]
pub(crate) enum Parameter {
    Positional { name: String },
    PositionalOptional { name: String, default_value: Val },
    VarArgs { name: String },
    KeywordOnly { name: String },
    KeywordOnlyOptional { name: String, default_value: Val },
    KwArgs { name: String },
}

pub(crate) struct FunctionSignature {
    parameters: Vec<Parameter>,
}

pub(crate) fn parse_arguments(
    signature: &FunctionSignature,
    args: Vec<Val>,
    mut kwargs: std::collections::BTreeMap<String, Val>,
) -> Result<std::collections::BTreeMap<String, Val>, String> {
    let mut parsed_args = std::collections::BTreeMap::new();
    let mut arg_iter = args.into_iter();
    let mut varargs = Vec::new();
    let mut keyword_only: bool = false;

    for param in &signature.parameters {
        match param {
            Parameter::Positional { name } => {
                assert!(
                    !keyword_only,
                    "Positional argument after keyword-only argument"
                );
                if let Some(val) = arg_iter.next() {
                    parsed_args.insert(name.clone(), val);
                } else if let Some(val) = kwargs.remove(name) {
                    parsed_args.insert(name.clone(), val);
                } else {
                    return Err(format!("Missing required positional argument '{}'", name));
                }
            }
            Parameter::PositionalOptional {
                name,
                default_value,
            } => {
                assert!(
                    !keyword_only,
                    "PositionalOptional argument after keyword-only argument"
                );
                if let Some(val) = arg_iter.next() {
                    parsed_args.insert(name.clone(), val);
                } else if let Some(val) = kwargs.remove(name) {
                    parsed_args.insert(name.clone(), val);
                } else {
                    parsed_args.insert(name.clone(), default_value.clone());
                }
            }
            Parameter::VarArgs { name } => {
                assert!(
                    !keyword_only,
                    "VarArgs argument after keyword-only argument"
                );
                while let Some(val) = arg_iter.next() {
                    varargs.push(val);
                }
                parsed_args.insert(name.clone(), Val::List(varargs.clone()));
                keyword_only = true; // Everything after *args is keyword-only
            }
            Parameter::KeywordOnly { name } => {
                if let Some(val) = kwargs.remove(name) {
                    parsed_args.insert(name.clone(), val);
                } else {
                    return Err(format!("Missing required keyword-only argument '{}'", name));
                }
            }
            Parameter::KeywordOnlyOptional {
                name,
                default_value,
            } => {
                if let Some(val) = kwargs.remove(name) {
                    parsed_args.insert(name.clone(), val);
                } else {
                    parsed_args.insert(name.clone(), default_value.clone());
                }
            }
            Parameter::KwArgs { name: _ } => {
                todo!()
            }
        }
    }

    // Check for any remaining positional arguments
    if arg_iter.next().is_some() {
        return Err("Too many positional arguments".into());
    }

    // Check for any unexpected keyword arguments
    if !kwargs.is_empty() {
        return Err(format!(
            "Got unexpected keyword arguments: {:?}",
            kwargs.keys()
        ));
    }

    Ok(parsed_args)
}

pub(crate) fn get_color(parsed_args: &BTreeMap<String, Val>) -> Result<[f64; 4], String> {
    let color = match parsed_args.get("color") {
        Some(Val::List(list)) => {
            if list.len() != 4 {
                return Err("Expected 'color' to be a list of four floats".into());
            }
            match (
                list[0].clone(),
                list[1].clone(),
                list[2].clone(),
                list[3].clone(),
            ) {
                (Val::Float(r), Val::Float(g), Val::Float(b), Val::Float(a)) => [r, g, b, a],
                _ => return Err("Expected 'color' to be a list of four floats".into()),
            }
        }
        _ => return Err("Expected 'color' to be a list of four floats".into()),
    };
    Ok(color)
}

fn get_point(parsed_args: &BTreeMap<String, Val>, key: &str) -> Result<(i32, i32), String> {
    let pt = match parsed_args.get(key) {
        Some(Val::List(list)) => {
            if list.len() != 2 {
                return Err(format!("Expected '{key}' to be a list of two integers"));
            }
            match (list[0].clone(), list[1].clone()) {
                (Val::Int(x), Val::Int(y)) => (x as i32, y as i32),
                _ => return Err(format!("Expected '{key}' to be a list of two integers")),
            }
        }
        _ => return Err(format!("Expected '{key}' to be a list of two integers")),
    };
    Ok(pt)
}

pub(crate) enum FrameArg {
    Frame(Frame),
    FrameType(FrameType),
}

impl FrameArg {
    fn unwrap_frame_type(&self) -> FrameType {
        match self {
            FrameArg::Frame(_frame) => panic!(),
            FrameArg::FrameType(frame_type) => frame_type.clone(),
        }
    }

    fn unwrap_frame(&self) -> Frame {
        match self {
            FrameArg::Frame(frame) => frame.clone(),
            FrameArg::FrameType(_frame_type) => panic!(),
        }
    }
}

pub(crate) fn mat_to_frame_rgb24(
    mat: opencv::prelude::Mat,
    width: i32,
    height: i32,
) -> Result<*mut ffi::AVFrame, Result<Frame, crate::dve::Error>> {
    let f = unsafe { ffi::av_frame_alloc() };
    if f.is_null() {
        return Err(Err(crate::dve::Error::AVError(
            "Failed to allocate frame".into(),
        )));
    }
    unsafe {
        (*f).width = width;
        (*f).height = height;
        (*f).format = ffi::AVPixelFormat_AV_PIX_FMT_RGB24;

        if ffi::av_frame_get_buffer(f, 0) < 0 {
            panic!("ERROR could not allocate frame data");
        }
    }
    unsafe {
        let mut src = mat.data();
        let mut dst = (*f).data[0];
        for _ in 0..height {
            std::ptr::copy_nonoverlapping(src, dst, width as usize * 3);
            src = src.add((*f).linesize[0] as usize);
            dst = dst.add(width as usize * 3);
        }
    }
    Ok(f)
}

pub(crate) fn frame_to_mat_rgb24(img: &Frame, width: i32, height: i32) -> opencv::prelude::Mat {
    debug_assert!(img.format == ffi::AVPixelFormat_AV_PIX_FMT_RGB24);
    let img: *mut ffi::AVFrame = img.inner.inner;
    let mut data_copy = vec![0u8; (width * height * 3) as usize];

    // copy img data into data_copy
    unsafe {
        let mut src = (*img).data[0];
        let mut dst = data_copy.as_mut_ptr();
        for _ in 0..height {
            std::ptr::copy_nonoverlapping(src, dst, width as usize * 3);
            src = src.add((*img).linesize[0] as usize);
            dst = dst.add(width as usize * 3);
        }
    }

    let mat = unsafe {
        opencv::core::Mat::new_rows_cols_with_data_unsafe(
            height as i32,
            width as i32,
            opencv::core::CV_8UC3,
            data_copy.as_mut_ptr() as *mut std::ffi::c_void,
            width as usize * 3,
        )
    }
    .unwrap();

    mat
}

pub(crate) fn frame_to_mat_gray8(img: &Frame, width: i32, height: i32) -> opencv::prelude::Mat {
    debug_assert!(img.format == ffi::AVPixelFormat_AV_PIX_FMT_GRAY8);
    let img: *mut ffi::AVFrame = img.inner.inner;
    let mut data_copy = vec![0u8; (width * height) as usize];

    // copy img data into data_copy
    unsafe {
        let mut src = (*img).data[0];
        let mut dst = data_copy.as_mut_ptr();
        for _ in 0..height {
            std::ptr::copy_nonoverlapping(src, dst, width as usize);
            src = src.add((*img).linesize[0] as usize);
            dst = dst.add(width as usize);
        }
    }

    let mat = unsafe {
        opencv::core::Mat::new_rows_cols_with_data_unsafe(
            height as i32,
            width as i32,
            opencv::core::CV_8UC1,
            data_copy.as_mut_ptr() as *mut std::ffi::c_void,
            width as usize,
        )
    }
    .unwrap();

    mat
}
