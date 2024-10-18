use opencv::core::MatTrait;

use super::*;
use std::collections::BTreeMap;

use super::Val;

#[derive(Clone, Debug)]
pub(crate) enum Parameter {
    Positional { name: &'static str },
    PositionalOptional { name: &'static str, default_value: Val },
    VarArgs { name: &'static str },
    KeywordOnly { name: &'static str },
    KeywordOnlyOptional { name: &'static str, default_value: Val },
    KwArgs { name: &'static str },
}

pub(crate) struct FunctionSignature {
    pub(crate) parameters: Vec<Parameter>,
}

pub(crate) fn parse_arguments(
    signature: &FunctionSignature,
    args: Vec<Val>,
    mut kwargs: std::collections::BTreeMap<String, Val>,
) -> Result<std::collections::BTreeMap<&'static str, Val>, String> {
    let mut parsed_args: BTreeMap<&'static str, Val> = std::collections::BTreeMap::new();
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
                    parsed_args.insert(name, val);
                } else if let Some(val) = kwargs.remove(*name) {
                    parsed_args.insert(name, val);
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
                    parsed_args.insert(name, val);
                } else if let Some(val) = kwargs.remove(*name) {
                    parsed_args.insert(name, val);
                } else {
                    parsed_args.insert(name, default_value.clone());
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
                parsed_args.insert(name, Val::List(varargs.clone()));
                keyword_only = true; // Everything after *args is keyword-only
            }
            Parameter::KeywordOnly { name } => {
                if let Some(val) = kwargs.remove(*name) {
                    parsed_args.insert(name, val);
                } else {
                    return Err(format!("Missing required keyword-only argument '{}'", name));
                }
            }
            Parameter::KeywordOnlyOptional {
                name,
                default_value,
            } => {
                if let Some(val) = kwargs.remove(*name) {
                    parsed_args.insert(name, val);
                } else {
                    parsed_args.insert(name, default_value.clone());
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

pub(crate) fn get_color(parsed_args: &BTreeMap<&'static str, Val>) -> Result<[f64; 4], String> {
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

pub(crate) fn get_point(
    parsed_args: &BTreeMap<&'static str, Val>,
    key: &str,
) -> Result<(i32, i32), String> {
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
    pub(crate) fn unwrap_frame_type(&self) -> FrameType {
        match self {
            FrameArg::Frame(_frame) => panic!(),
            FrameArg::FrameType(frame_type) => frame_type.clone(),
        }
    }

    pub(crate) fn unwrap_frame(&self) -> Frame {
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

    let mut mat = unsafe {
        opencv::core::Mat::new_rows_cols(height as i32, width as i32, opencv::core::CV_8UC3)
    }
    .unwrap();

    // copy img data into mat
    unsafe {
        let mut src = (*img).data[0];
        let mut dst = mat.data_mut();
        for _ in 0..height {
            std::ptr::copy_nonoverlapping(src, dst, width as usize * 3);
            src = src.add((*img).linesize[0] as usize);
            dst = dst.add(width as usize * 3);
        }
    }

    mat
}

pub(crate) fn frame_to_mat_gray8(img: &Frame, width: i32, height: i32) -> opencv::prelude::Mat {
    debug_assert!(img.format == ffi::AVPixelFormat_AV_PIX_FMT_GRAY8);
    let img: *mut ffi::AVFrame = img.inner.inner;

    let mut mat = unsafe {
        opencv::core::Mat::new_rows_cols(height as i32, width as i32, opencv::core::CV_8UC1)
    }
    .unwrap();

    // copy img data into mat
    unsafe {
        let mut src = (*img).data[0];
        let mut dst = mat.data_mut();
        for _ in 0..height {
            std::ptr::copy_nonoverlapping(src, dst, width as usize);
            src = src.add((*img).linesize[0] as usize);
            dst = dst.add(width as usize);
        }
    }

    mat
}
