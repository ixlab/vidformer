//! vidformer cv2 filters

use crate::dve;
use crate::dve::AVFrame;
use crate::filter;
use crate::filter::Val;
use crate::filter::*;
use filter::filter_utils::*;
use opencv::imgproc;
use opencv::prelude::MatTrait;
use rusty_ffmpeg::ffi;
use std::collections::BTreeMap;

/// vidformer cv2 filters
pub fn filters() -> BTreeMap<String, Box<dyn filter::Filter>> {
    let mut filters: BTreeMap<String, Box<dyn filter::Filter>> = BTreeMap::new();
    filters.insert("cv2.rectangle".to_string(), Box::new(Rectangle {}));
    filters.insert("cv2.putText".to_string(), Box::new(PutText {}));
    filters.insert("cv2.arrowedLine".to_string(), Box::new(ArrowedLine {}));
    filters.insert("cv2.line".to_string(), Box::new(Line {}));
    filters.insert("cv2.circle".to_string(), Box::new(Circle {}));
    filters.insert("cv2.setTo".to_string(), Box::new(SetTo {}));
    filters.insert("cv2.addWeighted".to_string(), Box::new(AddWeighted {}));
    filters
}

pub struct Rectangle {}

struct RectangleArgs {
    img: filter_utils::FrameArg,
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
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img" },
                Parameter::Positional { name: "pt1" },
                Parameter::Positional { name: "pt2" },
                Parameter::Positional {
                    name: "color",
                },
                Parameter::PositionalOptional {
                    name: "thickness",
                    default_value: Val::Int(1),
                },
                Parameter::PositionalOptional {
                    name: "lineType",
                    default_value: Val::Int(8),
                },
                Parameter::PositionalOptional {
                    name: "shift",
                    default_value: Val::Int(0),
                },
            ],
        };

        let kwargs = kwargs.clone();
        let args = args.to_vec();
        let parsed_args = filter_utils::parse_arguments(&signature, args, kwargs)?;

        let img = match parsed_args.get("img") {
            Some(Val::Frame(frame)) => filter_utils::FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => {
                filter_utils::FrameArg::FrameType(frame_type.clone())
            }
            x => {
                dbg! {x};
                return Err("Expected 'img' to be a Frame".into());
            }
        };

        // pt1 is a list of two integers
        let pt1 = filter_utils::get_point(&parsed_args, "pt1")?;

        // pt2 is a list of two integers
        let pt2 = filter_utils::get_point(&parsed_args, "pt2")?;

        // color is a list of four floats
        let color = filter_utils::get_color(&parsed_args)?;

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

        let mut mat = filter_utils::frame_to_mat_rgb24(&img, width, height);

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

        let f = match filter_utils::mat_to_frame_rgb24(mat, width, height) {
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
    img: filter_utils::FrameArg,
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
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img" },
                Parameter::Positional {
                    name: "text",
                },
                Parameter::Positional { name: "org" },
                Parameter::Positional {
                    name: "fontFace",
                },
                Parameter::Positional {
                    name: "fontScale",
                },
                Parameter::Positional {
                    name: "color",
                },
                Parameter::PositionalOptional {
                    name: "thickness",
                    default_value: Val::Int(1),
                },
                Parameter::PositionalOptional {
                    name: "lineType",
                    default_value: Val::Int(8),
                },
                Parameter::PositionalOptional {
                    name: "bottomLeftOrigin",
                    default_value: Val::Bool(false),
                },
            ],
        };

        let kwargs = kwargs.clone();
        let args: Vec<Val> = args.to_vec();
        let parsed_args = filter_utils::parse_arguments(&signature, args, kwargs)?;

        let img = match parsed_args.get("img") {
            Some(Val::Frame(frame)) => filter_utils::FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => {
                filter_utils::FrameArg::FrameType(frame_type.clone())
            }
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
        let org = filter_utils::get_point(&parsed_args, "org")?;

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
        let color = filter_utils::get_color(&parsed_args)?;

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

        let mut mat = filter_utils::frame_to_mat_rgb24(&img, width, height);

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

        let f = match filter_utils::mat_to_frame_rgb24(mat, width, height) {
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
                Parameter::Positional { name: "img" },
                Parameter::Positional { name: "pt1" },
                Parameter::Positional { name: "pt2" },
                Parameter::Positional {
                    name: "color",
                },
                Parameter::PositionalOptional {
                    name: "thickness",
                    default_value: Val::Int(1),
                },
                Parameter::PositionalOptional {
                    name: "lineType",
                    default_value: Val::Int(8),
                },
                Parameter::PositionalOptional {
                    name: "shift",
                    default_value: Val::Int(0),
                },
                Parameter::PositionalOptional {
                    name: "tipLength",
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

        let mut mat = frame_to_mat_rgb24(&img, width, height);

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

        let f = match mat_to_frame_rgb24(mat, width, height) {
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
                Parameter::Positional { name: "img" },
                Parameter::Positional { name: "pt1" },
                Parameter::Positional { name: "pt2" },
                Parameter::Positional {
                    name: "color",
                },
                Parameter::PositionalOptional {
                    name: "thickness",
                    default_value: Val::Int(1),
                },
                Parameter::PositionalOptional {
                    name: "lineType",
                    default_value: Val::Int(8),
                },
                Parameter::PositionalOptional {
                    name: "shift",
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

        let mut mat = filter_utils::frame_to_mat_rgb24(&img, width, height);

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

        let f = match filter_utils::mat_to_frame_rgb24(mat, width, height) {
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
    img: filter_utils::FrameArg,
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
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img" },
                Parameter::Positional {
                    name: "center",
                },
                Parameter::Positional {
                    name: "radius",
                },
                Parameter::Positional {
                    name: "color",
                },
                Parameter::PositionalOptional {
                    name: "thickness",
                    default_value: Val::Int(1),
                },
                Parameter::PositionalOptional {
                    name: "lineType",
                    default_value: Val::Int(8),
                },
                Parameter::PositionalOptional {
                    name: "shift",
                    default_value: Val::Int(0),
                },
            ],
        };

        let kwargs = kwargs.clone();
        let args = args.to_vec();
        let parsed_args = filter_utils::parse_arguments(&signature, args, kwargs)?;

        let img = match parsed_args.get("img") {
            Some(Val::Frame(frame)) => filter_utils::FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => {
                filter_utils::FrameArg::FrameType(frame_type.clone())
            }
            x => {
                dbg! {x};
                return Err("Expected 'img' to be a Frame".into());
            }
        };

        // center is a list of two integers
        let center = filter_utils::get_point(&parsed_args, "center")?;

        // radius is an integer
        let radius = match parsed_args.get("radius") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'radius' to be an integer".into()),
        };

        // color is a list of four floats
        let color = filter_utils::get_color(&parsed_args)?;

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

        let mut mat = filter_utils::frame_to_mat_rgb24(&img, width, height);

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

        let f = match filter_utils::mat_to_frame_rgb24(mat, width, height) {
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

pub struct SetTo {}

struct SetToArgs {
    img: filter_utils::FrameArg,
    color: [f64; 4],
    mask: filter_utils::FrameArg,
}

impl SetTo {
    fn args(
        args: &[Val],
        kwargs: &BTreeMap<std::string::String, Val>,
    ) -> Result<SetToArgs, String> {
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img" },
                Parameter::Positional {
                    name: "color",
                },
                Parameter::Positional {
                    name: "mask",
                },
            ],
        };

        let kwargs = kwargs.clone();
        let args = args.to_vec();
        let parsed_args = filter_utils::parse_arguments(&signature, args, kwargs)?;

        let img = match parsed_args.get("img") {
            Some(Val::Frame(frame)) => filter_utils::FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => {
                filter_utils::FrameArg::FrameType(frame_type.clone())
            }
            x => {
                dbg! {x};
                return Err("Expected 'img' to be a Frame".into());
            }
        };

        let color = filter_utils::get_color(&parsed_args)?;

        let mask = match parsed_args.get("mask") {
            Some(Val::Frame(frame)) => filter_utils::FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => {
                filter_utils::FrameArg::FrameType(frame_type.clone())
            }
            x => {
                dbg! {x};
                return Err("Expected 'mask' to be a Frame".into());
            }
        };

        Ok(SetToArgs { img, mask, color })
    }
}

impl Filter for SetTo {
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<std::string::String, Val>,
    ) -> std::result::Result<Frame, crate::dve::Error> {
        let opts: SetToArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(crate::dve::Error::AVError(err)),
        };

        let img = opts.img.unwrap_frame();
        let mask = opts.mask.unwrap_frame();

        let mut img_mat = filter_utils::frame_to_mat_rgb24(&img, img.width, img.height);
        let mask_mat = filter_utils::frame_to_mat_gray8(&mask, mask.width, mask.height);

        let color =
            opencv::core::Scalar::new(opts.color[0], opts.color[1], opts.color[2], opts.color[3]);

        // set all pixels in img_mat to color where mask_mat is not zero
        img_mat.set_to(&color, &mask_mat).unwrap();

        let f = match filter_utils::mat_to_frame_rgb24(img_mat, img.width, img.height) {
            Ok(value) => value,
            Err(value) => return value,
        };

        Ok(Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<std::string::String, Val>,
    ) -> std::result::Result<FrameType, crate::dve::Error> {
        let opts: SetToArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(crate::dve::Error::AVError(err)),
        };

        // check img is RGB24
        if opts.img.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(crate::dve::Error::FilterInternalError(
                "Expected img to be an RGB24 frame".into(),
            ));
        }

        // check mask is grayscale
        if opts.mask.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_GRAY8 {
            return Err(crate::dve::Error::FilterInternalError(
                "Expected mask to be a grayscale frame".into(),
            ));
        }

        // check mask is same size as img
        if opts.img.unwrap_frame_type().width != opts.mask.unwrap_frame_type().width
            || opts.img.unwrap_frame_type().height != opts.mask.unwrap_frame_type().height
        {
            return Err(crate::dve::Error::FilterInternalError(
                "Expected mask to be the same size as img".into(),
            ));
        }

        Ok(opts.img.unwrap_frame_type())
    }
}

pub struct AddWeighted {}

struct AddWeightedArgs {
    src1: filter_utils::FrameArg,
    alpha: f64,
    src2: filter_utils::FrameArg,
    beta: f64,
    gamma: f64,
}

impl AddWeighted {
    fn args(
        args: &[Val],
        kwargs: &BTreeMap<std::string::String, Val>,
    ) -> Result<AddWeightedArgs, String> {
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional {
                    name: "src1",
                },
                Parameter::Positional {
                    name: "alpha",
                },
                Parameter::Positional {
                    name: "src2",
                },
                Parameter::Positional {
                    name: "beta",
                },
                Parameter::Positional {
                    name: "gamma",
                },
            ],
        };

        let kwargs = kwargs.clone();
        let args = args.to_vec();
        let parsed_args = filter_utils::parse_arguments(&signature, args, kwargs)?;

        let src1 = match parsed_args.get("src1") {
            Some(Val::Frame(frame)) => filter_utils::FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => {
                filter_utils::FrameArg::FrameType(frame_type.clone())
            }
            x => {
                dbg! {x};
                return Err("Expected 'src1' to be a Frame".into());
            }
        };

        let alpha = match parsed_args.get("alpha") {
            Some(Val::Float(value)) => *value,
            _ => return Err("Expected 'alpha' to be a float".into()),
        };

        let src2 = match parsed_args.get("src2") {
            Some(Val::Frame(frame)) => filter_utils::FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => {
                filter_utils::FrameArg::FrameType(frame_type.clone())
            }
            x => {
                dbg! {x};
                return Err("Expected 'src2' to be a Frame".into());
            }
        };

        let beta = match parsed_args.get("beta") {
            Some(Val::Float(value)) => *value,
            _ => return Err("Expected 'beta' to be a float".into()),
        };

        let gamma = match parsed_args.get("gamma") {
            Some(Val::Float(value)) => *value,
            _ => return Err("Expected 'gamma' to be a float".into()),
        };

        Ok(AddWeightedArgs {
            src1,
            alpha,
            src2,
            beta,
            gamma,
        })
    }
}

impl Filter for AddWeighted {
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<std::string::String, Val>,
    ) -> std::result::Result<Frame, crate::dve::Error> {
        let opts: AddWeightedArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(crate::dve::Error::AVError(err)),
        };

        let src1 = opts.src1.unwrap_frame();
        let src2 = opts.src2.unwrap_frame();

        let (width, height) = (src1.width, src1.height);
        debug_assert_eq!(src1.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);
        debug_assert_eq!(src2.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let src1_mat = filter_utils::frame_to_mat_rgb24(&src1, width, height);
        let src2_mat = filter_utils::frame_to_mat_rgb24(&src2, width, height);

        let mut out_mat = opencv::core::Mat::new_nd_with_default(
            &[height as i32, width as i32, 3],
            opencv::core::CV_8UC3,
            opencv::core::Scalar::all(0.0),
        )
        .unwrap();

        opencv::core::add_weighted(
            &src1_mat,
            opts.alpha,
            &src2_mat,
            opts.beta,
            opts.gamma,
            &mut out_mat,
            -1,
        )
        .unwrap();

        assert_eq!(out_mat.rows(), height as i32);
        assert_eq!(out_mat.cols(), width as i32);
        assert_eq!(out_mat.channels(), 3);

        let f = match filter_utils::mat_to_frame_rgb24(out_mat, width, height) {
            Ok(value) => value,
            Err(value) => return value,
        };

        Ok(Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<std::string::String, Val>,
    ) -> std::result::Result<FrameType, crate::dve::Error> {
        let opts: AddWeightedArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(crate::dve::Error::AVError(err)),
        };

        // check src1 is RGB24
        if opts.src1.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(crate::dve::Error::FilterInternalError(
                "Expected src1 to be an RGB24 frame".into(),
            ));
        }

        // check src2 is RGB24
        if opts.src2.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(crate::dve::Error::FilterInternalError(
                "Expected src2 to be an RGB24 frame".into(),
            ));
        }

        // check src1 and src2 are the same size
        if opts.src1.unwrap_frame_type().width != opts.src2.unwrap_frame_type().width
            || opts.src1.unwrap_frame_type().height != opts.src2.unwrap_frame_type().height
        {
            return Err(crate::dve::Error::FilterInternalError(
                "Expected src1 and src2 to be the same size".into(),
            ));
        }

        Ok(opts.src1.unwrap_frame_type())
    }
}
