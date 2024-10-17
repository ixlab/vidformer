//! vidformer built-in filters

use crate::dve;
use crate::dve::AVFrame;
use crate::filter;
use crate::filter::Val;
use opencv::imgproc;
use opencv::prelude::MatTraitConst;
use rusty_ffmpeg::ffi;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
enum Parameter {
    Positional { name: String },
    PositionalOptional { name: String, default_value: Val },
    VarArgs { name: String },
    KeywordOnly { name: String },
    KeywordOnlyOptional { name: String, default_value: Val },
    KwArgs { name: String },
}

struct FunctionSignature {
    parameters: Vec<Parameter>,
}

fn parse_arguments(
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

fn get_color(parsed_args: &BTreeMap<String, Val>) -> Result<[f64; 4], String> {
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

enum FrameArg {
    Frame(filter::Frame),
    FrameType(filter::FrameType),
}

impl FrameArg {
    fn unwrap_frame_type(&self) -> filter::FrameType {
        match self {
            FrameArg::Frame(_frame) => panic!(),
            FrameArg::FrameType(frame_type) => frame_type.clone(),
        }
    }

    fn unwrap_frame(&self) -> filter::Frame {
        match self {
            FrameArg::Frame(frame) => frame.clone(),
            FrameArg::FrameType(_frame_type) => panic!(),
        }
    }
}

fn mat_to_frame(
    mat: opencv::prelude::Mat,
    width: i32,
    height: i32,
) -> Result<*mut ffi::AVFrame, Result<filter::Frame, dve::Error>> {
    let f = unsafe { ffi::av_frame_alloc() };
    if f.is_null() {
        return Err(Err(dve::Error::AVError("Failed to allocate frame".into())));
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

fn frame_to_mat(img: filter::Frame, width: i32, height: i32) -> opencv::prelude::Mat {
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

pub struct Rectangle {}

struct RectangleArgs {
    img: FrameArg,
    pt1: (i32, i32),
    pt2: (i32, i32),
    color: [f64; 4],
    thickness: i32,
    linetype: i32,
    shift: i32,
}

impl Rectangle {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<RectangleArgs, String> {
        let signature = FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img".into() },
                Parameter::Positional { name: "pt1".into() },
                Parameter::Positional { name: "pt2".into() },
                Parameter::Positional {
                    name: "color".into(),
                },
                Parameter::PositionalOptional {
                    name: "thickness".into(),
                    default_value: Val::Int(1),
                },
                Parameter::PositionalOptional {
                    name: "lineType".into(),
                    default_value: Val::Int(8),
                },
                Parameter::PositionalOptional {
                    name: "shift".into(),
                    default_value: Val::Int(0),
                },
            ],
        };

        let kwargs = kwargs.clone();
        let args = args.to_vec();
        let parsed_args = parse_arguments(&signature, args, kwargs)?;

        let img = match parsed_args.get("img") {
            Some(Val::Frame(frame)) => FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => FrameArg::FrameType(frame_type.clone()),
            x => {
                dbg! {x};
                return Err("Expected 'img' to be a Frame".into());
            }
        };

        // pt1 is a list of two integers
        let pt1 = get_point(&parsed_args, "pt1")?;

        // pt2 is a list of two integers
        let pt2 = get_point(&parsed_args, "pt2")?;

        // color is a list of four floats
        let color = get_color(&parsed_args)?;

        // thickness is an integer
        let thickness = match parsed_args.get("thickness") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'thickness' to be an integer".into()),
        };

        // lineType is an integer
        let linetype = match parsed_args.get("lineType") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'lineType' to be an integer".into()),
        };

        // shift is an integer
        let shift = match parsed_args.get("shift") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'shift' to be an integer".into()),
        };

        Ok(RectangleArgs {
            img,
            pt1,
            pt2,
            color,
            thickness,
            linetype,
            shift,
        })
    }
}

impl filter::Filter for Rectangle {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: RectangleArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let img = opts.img.unwrap_frame();
        let (width, height) = (img.width, img.height);
        debug_assert_eq!(img.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let mut mat = frame_to_mat(img, width, height);

        let pt1 = opencv::core::Point::new(opts.pt1.0, opts.pt1.1);
        let pt2 = opencv::core::Point::new(opts.pt2.0, opts.pt2.1);

        let color =
            opencv::core::Scalar::new(opts.color[0], opts.color[1], opts.color[2], opts.color[3]);

        imgproc::rectangle_points(
            &mut mat,
            pt1,
            pt2,
            color,
            opts.thickness,
            opts.linetype,
            opts.shift,
        )
        .unwrap();

        let f = match mat_to_frame(mat, width, height) {
            Ok(value) => value,
            Err(value) => return value,
        };

        Ok(filter::Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::FrameType, dve::Error> {
        let opts: RectangleArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        if opts.img.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(dve::Error::AVError("Expected RGB24 frame".into()));
        }

        Ok(opts.img.unwrap_frame_type())
    }
}

pub struct PutText {}

struct PutTextArgs {
    img: FrameArg,
    text: String,
    org: (i32, i32),
    font_face: i32,
    font_scale: f64,
    color: [f64; 4],
    thickness: i32,
    linetype: i32,
    bottom_left_origin: bool,
}

impl PutText {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<PutTextArgs, String> {
        let signature = FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img".into() },
                Parameter::Positional {
                    name: "text".into(),
                },
                Parameter::Positional { name: "org".into() },
                Parameter::Positional {
                    name: "fontFace".into(),
                },
                Parameter::Positional {
                    name: "fontScale".into(),
                },
                Parameter::Positional {
                    name: "color".into(),
                },
                Parameter::PositionalOptional {
                    name: "thickness".into(),
                    default_value: Val::Int(1),
                },
                Parameter::PositionalOptional {
                    name: "lineType".into(),
                    default_value: Val::Int(8),
                },
                Parameter::PositionalOptional {
                    name: "bottomLeftOrigin".into(),
                    default_value: Val::Bool(false),
                },
            ],
        };

        let kwargs = kwargs.clone();
        let args: Vec<Val> = args.to_vec();
        let parsed_args = parse_arguments(&signature, args, kwargs)?;

        let img = match parsed_args.get("img") {
            Some(Val::Frame(frame)) => FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => FrameArg::FrameType(frame_type.clone()),
            x => {
                dbg! {x};
                return Err("Expected 'img' to be a Frame".into());
            }
        };

        // text is a string
        let text = match parsed_args.get("text") {
            Some(Val::String(value)) => value.clone(),
            _ => return Err("Expected 'text' to be a string".into()),
        };

        // org is a list of two integers
        let org = get_point(&parsed_args, "org")?;

        // fontFace is an integer
        let font_face = match parsed_args.get("fontFace") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'fontFace' to be an integer".into()),
        };

        // fontScale is a float
        let font_scale = match parsed_args.get("fontScale") {
            Some(Val::Float(value)) => *value,
            _ => return Err("Expected 'fontScale' to be a float".into()),
        };

        // color is a list of four floats
        let color = get_color(&parsed_args)?;

        // thickness is an integer
        let thickness = match parsed_args.get("thickness") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'thickness' to be an integer".into()),
        };

        // lineType is an integer
        let linetype = match parsed_args.get("lineType") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'lineType' to be an integer".into()),
        };

        // bottomLeftOrigin is a boolean
        let bottom_left_origin = match parsed_args.get("bottomLeftOrigin") {
            Some(Val::Bool(value)) => *value,
            _ => return Err("Expected 'bottomLeftOrigin' to be a boolean".into()),
        };

        Ok(PutTextArgs {
            img,
            text,
            org,
            font_face,
            font_scale,
            color,
            thickness,
            linetype,
            bottom_left_origin,
        })
    }
}

impl filter::Filter for PutText {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: PutTextArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let img = opts.img.unwrap_frame();
        let (width, height) = (img.width, img.height);
        debug_assert_eq!(img.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let mut mat = frame_to_mat(img, width, height);

        let org = opencv::core::Point::new(opts.org.0, opts.org.1);
        let color =
            opencv::core::Scalar::new(opts.color[0], opts.color[1], opts.color[2], opts.color[3]);

        imgproc::put_text(
            &mut mat,
            &opts.text,
            org,
            opts.font_face,
            opts.font_scale,
            color,
            opts.thickness,
            opts.linetype,
            opts.bottom_left_origin,
        )
        .unwrap();

        let f = match mat_to_frame(mat, width, height) {
            Ok(value) => value,
            Err(value) => return value,
        };

        Ok(filter::Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::FrameType, dve::Error> {
        let opts: PutTextArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        if opts.img.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(dve::Error::AVError("Expected RGB24 frame".into()));
        }

        Ok(opts.img.unwrap_frame_type())
    }
}

pub struct ArrowedLine {}

struct ArrowedLineArgs {
    img: FrameArg,
    pt1: (i32, i32),
    pt2: (i32, i32),
    color: [f64; 4],
    thickness: i32,
    linetype: i32,
    shift: i32,
    tip_length: f64,
}

impl ArrowedLine {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<ArrowedLineArgs, String> {
        let signature = FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img".into() },
                Parameter::Positional { name: "pt1".into() },
                Parameter::Positional { name: "pt2".into() },
                Parameter::Positional {
                    name: "color".into(),
                },
                Parameter::PositionalOptional {
                    name: "thickness".into(),
                    default_value: Val::Int(1),
                },
                Parameter::PositionalOptional {
                    name: "lineType".into(),
                    default_value: Val::Int(8),
                },
                Parameter::PositionalOptional {
                    name: "shift".into(),
                    default_value: Val::Int(0),
                },
                Parameter::PositionalOptional {
                    name: "tipLength".into(),
                    default_value: Val::Float(0.1),
                },
            ],
        };

        let kwargs = kwargs.clone();
        let args = args.to_vec();
        let parsed_args = parse_arguments(&signature, args, kwargs)?;

        let img = match parsed_args.get("img") {
            Some(Val::Frame(frame)) => FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => FrameArg::FrameType(frame_type.clone()),
            x => {
                dbg! {x};
                return Err("Expected 'img' to be a Frame".into());
            }
        };

        // pt1 is a list of two integers
        let pt1 = get_point(&parsed_args, "pt1")?;

        // pt2 is a list of two integers
        let pt2 = get_point(&parsed_args, "pt2")?;

        // color is a list of four floats
        let color = get_color(&parsed_args)?;

        // thickness is an integer
        let thickness = match parsed_args.get("thickness") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'thickness' to be an integer".into()),
        };

        // lineType is an integer
        let linetype = match parsed_args.get("lineType") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'lineType' to be an integer".into()),
        };

        // shift is an integer
        let shift = match parsed_args.get("shift") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'shift' to be an integer".into()),
        };

        // tipLength is a float
        let tip_length = match parsed_args.get("tipLength") {
            Some(Val::Float(value)) => *value,
            _ => return Err("Expected 'tipLength' to be a float".into()),
        };

        Ok(ArrowedLineArgs {
            img,
            pt1,
            pt2,
            color,
            thickness,
            linetype,
            shift,
            tip_length,
        })
    }
}

impl filter::Filter for ArrowedLine {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: ArrowedLineArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let img = opts.img.unwrap_frame();
        let (width, height) = (img.width, img.height);
        debug_assert_eq!(img.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let mut mat = frame_to_mat(img, width, height);

        let pt1 = opencv::core::Point::new(opts.pt1.0, opts.pt1.1);
        let pt2 = opencv::core::Point::new(opts.pt2.0, opts.pt2.1);
        let color =
            opencv::core::Scalar::new(opts.color[0], opts.color[1], opts.color[2], opts.color[3]);

        imgproc::arrowed_line(
            &mut mat,
            pt1,
            pt2,
            color,
            opts.thickness,
            opts.linetype,
            opts.shift,
            opts.tip_length,
        )
        .unwrap();

        let f = match mat_to_frame(mat, width, height) {
            Ok(value) => value,
            Err(value) => return value,
        };

        Ok(filter::Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::FrameType, dve::Error> {
        let opts: ArrowedLineArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        if opts.img.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(dve::Error::AVError("Expected RGB24 frame".into()));
        }

        Ok(opts.img.unwrap_frame_type())
    }
}

pub struct Line {}

struct LineArgs {
    img: FrameArg,
    pt1: (i32, i32),
    pt2: (i32, i32),
    color: [f64; 4],
    thickness: i32,
    linetype: i32,
    shift: i32,
}

impl Line {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<LineArgs, String> {
        let signature = FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img".into() },
                Parameter::Positional { name: "pt1".into() },
                Parameter::Positional { name: "pt2".into() },
                Parameter::Positional {
                    name: "color".into(),
                },
                Parameter::PositionalOptional {
                    name: "thickness".into(),
                    default_value: Val::Int(1),
                },
                Parameter::PositionalOptional {
                    name: "lineType".into(),
                    default_value: Val::Int(8),
                },
                Parameter::PositionalOptional {
                    name: "shift".into(),
                    default_value: Val::Int(0),
                },
            ],
        };

        let kwargs = kwargs.clone();
        let args = args.to_vec();
        let parsed_args = parse_arguments(&signature, args, kwargs)?;

        let img = match parsed_args.get("img") {
            Some(Val::Frame(frame)) => FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => FrameArg::FrameType(frame_type.clone()),
            x => {
                dbg! {x};
                return Err("Expected 'img' to be a Frame".into());
            }
        };

        // pt1 is a list of two integers
        let pt1 = get_point(&parsed_args, "pt1")?;

        // pt2 is a list of two integers
        let pt2 = get_point(&parsed_args, "pt2")?;

        // color is a list of four floats
        let color = get_color(&parsed_args)?;

        // thickness is an integer
        let thickness = match parsed_args.get("thickness") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'thickness' to be an integer".into()),
        };

        // lineType is an integer
        let linetype = match parsed_args.get("lineType") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'lineType' to be an integer".into()),
        };

        // shift is an integer
        let shift = match parsed_args.get("shift") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'shift' to be an integer".into()),
        };

        Ok(LineArgs {
            img,
            pt1,
            pt2,
            color,
            thickness,
            linetype,
            shift,
        })
    }
}

impl filter::Filter for Line {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: LineArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let img = opts.img.unwrap_frame();
        let (width, height) = (img.width, img.height);
        debug_assert_eq!(img.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let mut mat = frame_to_mat(img, width, height);

        let pt1 = opencv::core::Point::new(opts.pt1.0, opts.pt1.1);
        let pt2 = opencv::core::Point::new(opts.pt2.0, opts.pt2.1);
        let color =
            opencv::core::Scalar::new(opts.color[0], opts.color[1], opts.color[2], opts.color[3]);

        imgproc::line(
            &mut mat,
            pt1,
            pt2,
            color,
            opts.thickness,
            opts.linetype,
            opts.shift,
        )
        .unwrap();

        let f = match mat_to_frame(mat, width, height) {
            Ok(value) => value,
            Err(value) => return value,
        };

        Ok(filter::Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::FrameType, dve::Error> {
        let opts: LineArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        if opts.img.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(dve::Error::AVError("Expected RGB24 frame".into()));
        }

        Ok(opts.img.unwrap_frame_type())
    }
}

pub struct Circle {}

struct CircleArgs {
    img: FrameArg,
    center: (i32, i32),
    radius: i32,
    color: [f64; 4],
    thickness: i32,
    linetype: i32,
    shift: i32,
}

impl Circle {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<CircleArgs, String> {
        let signature = FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img".into() },
                Parameter::Positional {
                    name: "center".into(),
                },
                Parameter::Positional {
                    name: "radius".into(),
                },
                Parameter::Positional {
                    name: "color".into(),
                },
                Parameter::PositionalOptional {
                    name: "thickness".into(),
                    default_value: Val::Int(1),
                },
                Parameter::PositionalOptional {
                    name: "lineType".into(),
                    default_value: Val::Int(8),
                },
                Parameter::PositionalOptional {
                    name: "shift".into(),
                    default_value: Val::Int(0),
                },
            ],
        };

        let kwargs = kwargs.clone();
        let args = args.to_vec();
        let parsed_args = parse_arguments(&signature, args, kwargs)?;

        let img = match parsed_args.get("img") {
            Some(Val::Frame(frame)) => FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => FrameArg::FrameType(frame_type.clone()),
            x => {
                dbg! {x};
                return Err("Expected 'img' to be a Frame".into());
            }
        };

        // center is a list of two integers
        let center = get_point(&parsed_args, "center")?;

        // radius is an integer
        let radius = match parsed_args.get("radius") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'radius' to be an integer".into()),
        };

        // color is a list of four floats
        let color = get_color(&parsed_args)?;

        // thickness is an integer
        let thickness = match parsed_args.get("thickness") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'thickness' to be an integer".into()),
        };

        // lineType is an integer
        let linetype = match parsed_args.get("lineType") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'lineType' to be an integer".into()),
        };

        // shift is an integer
        let shift = match parsed_args.get("shift") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'shift' to be an integer".into()),
        };

        Ok(CircleArgs {
            img,
            center,
            radius,
            color,
            thickness,
            linetype,
            shift,
        })
    }
}

impl filter::Filter for Circle {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: CircleArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let img = opts.img.unwrap_frame();
        let (width, height) = (img.width, img.height);
        debug_assert_eq!(img.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let mut mat = frame_to_mat(img, width, height);

        let center = opencv::core::Point::new(opts.center.0, opts.center.1);
        let color =
            opencv::core::Scalar::new(opts.color[0], opts.color[1], opts.color[2], opts.color[3]);

        imgproc::circle(
            &mut mat,
            center,
            opts.radius,
            color,
            opts.thickness,
            opts.linetype,
            opts.shift,
        )
        .unwrap();

        let f = match mat_to_frame(mat, width, height) {
            Ok(value) => value,
            Err(value) => return value,
        };

        Ok(filter::Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::FrameType, dve::Error> {
        let opts: CircleArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        if opts.img.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(dve::Error::AVError("Expected RGB24 frame".into()));
        }

        Ok(opts.img.unwrap_frame_type())
    }
}
