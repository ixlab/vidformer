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
    filters.insert("cv2.ellipse".to_string(), Box::new(Ellipse {}));
    filters.insert("cv2.setTo".to_string(), Box::new(SetTo {}));
    filters.insert("cv2.addWeighted".to_string(), Box::new(AddWeighted {}));
    filters.insert("cv2.polylines".to_string(), Box::new(Polylines {}));
    filters.insert("cv2.fillPoly".to_string(), Box::new(FillPoly {}));
    filters.insert(
        "cv2.fillConvexPoly".to_string(),
        Box::new(FillConvexPoly {}),
    );
    filters.insert("cv2.drawContours".to_string(), Box::new(DrawContours {}));
    filters.insert("cv2.drawMarker".to_string(), Box::new(DrawMarker {}));
    filters.insert("cv2.flip".to_string(), Box::new(Flip {}));
    filters.insert("cv2.rotate".to_string(), Box::new(Rotate {}));
    filters.insert(
        "cv2.copyMakeBorder".to_string(),
        Box::new(CopyMakeBorder {}),
    );
    filters.insert("cv2.hconcat".to_string(), Box::new(Hconcat {}));
    filters.insert("cv2.vconcat".to_string(), Box::new(Vconcat {}));
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
                Parameter::Positional { name: "color" },
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
                Parameter::Positional { name: "text" },
                Parameter::Positional { name: "org" },
                Parameter::Positional { name: "fontFace" },
                Parameter::Positional { name: "fontScale" },
                Parameter::Positional { name: "color" },
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
                Parameter::Positional { name: "color" },
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
                Parameter::Positional { name: "color" },
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
                Parameter::Positional { name: "center" },
                Parameter::Positional { name: "radius" },
                Parameter::Positional { name: "color" },
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

/*
void cv::ellipse 	( 	InputOutputArray 	img,
        Point 	center,
        Size 	axes,
        double 	angle,
        double 	startAngle,
        double 	endAngle,
        const Scalar & 	color,
        int 	thickness = 1,
        int 	lineType = LINE_8,
        int 	shift = 0 )
Python:
    cv.ellipse(	img, center, axes, angle, startAngle, endAngle, color[, thickness[, lineType[, shift]]]	) -> 	img
*/
pub struct Ellipse {}

struct EllipseArgs {
    img: filter_utils::FrameArg,
    center: (i32, i32),
    axes: (i32, i32),
    angle: f64,
    start_angle: f64,
    end_angle: f64,
    color: [f64; 4],
    thickness: i32,
    linetype: i32,
    shift: i32,
}

impl Ellipse {
    fn args(
        args: &[Val],
        kwargs: &BTreeMap<std::string::String, Val>,
    ) -> Result<EllipseArgs, String> {
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img" },
                Parameter::Positional { name: "center" },
                Parameter::Positional { name: "axes" },
                Parameter::Positional { name: "angle" },
                Parameter::Positional { name: "startAngle" },
                Parameter::Positional { name: "endAngle" },
                Parameter::Positional { name: "color" },
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

        // axes is a list of two integers
        let axes = filter_utils::get_point(&parsed_args, "axes")?;

        // angle is a float
        let angle = match parsed_args.get("angle") {
            Some(Val::Float(value)) => *value,
            _ => return Err("Expected 'angle' to be a float".into()),
        };

        // startAngle is a float
        let start_angle = match parsed_args.get("startAngle") {
            Some(Val::Float(value)) => *value,
            _ => return Err("Expected 'startAngle' to be a float".into()),
        };

        // endAngle is a float
        let end_angle = match parsed_args.get("endAngle") {
            Some(Val::Float(value)) => *value,
            _ => return Err("Expected 'endAngle' to be a float".into()),
        };

        // color is a list of four floats
        let color = filter_utils::get_color(&parsed_args)?;

        let thickness = match parsed_args.get("thickness") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'thickness' to be an integer".into()),
        };

        let linetype = match parsed_args.get("lineType") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'lineType' to be an integer".into()),
        };

        let shift = match parsed_args.get("shift") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'shift' to be an integer".into()),
        };

        Ok(EllipseArgs {
            img,
            center,
            axes,
            angle,
            start_angle,
            end_angle,
            color,
            thickness,
            linetype,
            shift,
        })
    }
}

impl Filter for Ellipse {
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<std::string::String, Val>,
    ) -> std::result::Result<Frame, crate::dve::Error> {
        let opts: EllipseArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(crate::dve::Error::AVError(err)),
        };

        let img = opts.img.unwrap_frame();
        let (width, height) = (img.width, img.height);
        debug_assert_eq!(img.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let mut mat = filter_utils::frame_to_mat_rgb24(&img, width, height);

        let center = opencv::core::Point::new(opts.center.0, opts.center.1);
        let axes = opencv::core::Size::new(opts.axes.0, opts.axes.1);
        let color =
            opencv::core::Scalar::new(opts.color[0], opts.color[1], opts.color[2], opts.color[3]);

        imgproc::ellipse(
            &mut mat,
            center,
            axes,
            opts.angle,
            opts.start_angle,
            opts.end_angle,
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

        Ok(Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<std::string::String, Val>,
    ) -> std::result::Result<FrameType, crate::dve::Error> {
        let opts: EllipseArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(crate::dve::Error::AVError(err)),
        };

        if opts.img.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(crate::dve::Error::FilterInternalError(
                "Expected RGB24 frame".into(),
            ));
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
                Parameter::Positional { name: "color" },
                Parameter::Positional { name: "mask" },
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
                Parameter::Positional { name: "src1" },
                Parameter::Positional { name: "alpha" },
                Parameter::Positional { name: "src2" },
                Parameter::Positional { name: "beta" },
                Parameter::Positional { name: "gamma" },
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
            &[height, width, 3],
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

        assert_eq!(out_mat.rows(), { height });
        assert_eq!(out_mat.cols(), { width });
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

pub struct Polylines {}

struct PolylinesArgs {
    img: filter_utils::FrameArg,
    pts: Vec<Vec<(i32, i32)>>,
    is_closed: bool,
    color: [f64; 4],
    thickness: i32,
    linetype: i32,
    shift: i32,
}

impl Polylines {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<PolylinesArgs, String> {
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img" },
                Parameter::Positional { name: "pts" },
                Parameter::Positional { name: "isClosed" },
                Parameter::Positional { name: "color" },
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

        let pts = parse_polygon_list(&parsed_args)?;

        // isClosed is a boolean
        let is_closed = match parsed_args.get("isClosed") {
            Some(Val::Bool(value)) => *value,
            _ => return Err("Expected 'isClosed' to be a boolean".into()),
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

        Ok(PolylinesArgs {
            img,
            pts,
            is_closed,
            color,
            thickness,
            linetype,
            shift,
        })
    }
}

impl filter::Filter for Polylines {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: PolylinesArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let img = opts.img.unwrap_frame();
        let (width, height) = (img.width, img.height);
        debug_assert_eq!(img.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let mut mat = filter_utils::frame_to_mat_rgb24(&img, width, height);
        let pts_vec = polygon_list_to_opencv(&opts.pts);
        let color =
            opencv::core::Scalar::new(opts.color[0], opts.color[1], opts.color[2], opts.color[3]);

        imgproc::polylines(
            &mut mat,
            &pts_vec,
            opts.is_closed,
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
        let opts: PolylinesArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        if opts.img.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(dve::Error::AVError("Expected RGB24 frame".into()));
        }

        Ok(opts.img.unwrap_frame_type())
    }
}

// Helper functions for parsing and converting polygon data (shared by polylines, fillPoly, fillConvexPoly)

fn parse_polygon_list(parsed_args: &BTreeMap<&str, Val>) -> Result<Vec<Vec<(i32, i32)>>, String> {
    match parsed_args.get("pts") {
        Some(Val::List(polygons)) => {
            let mut result = Vec::new();
            for polygon in polygons {
                match polygon {
                    Val::List(points) => {
                        result.push(parse_points_list(points)?);
                    }
                    _ => return Err("Expected polygon to be a list of points".into()),
                }
            }
            Ok(result)
        }
        _ => Err("Expected 'pts' to be a list of polygons".into()),
    }
}

fn parse_single_polygon(
    parsed_args: &BTreeMap<&str, Val>,
    name: &str,
) -> Result<Vec<(i32, i32)>, String> {
    match parsed_args.get(name) {
        Some(Val::List(points)) => parse_points_list(points),
        _ => Err(format!("Expected '{}' to be a list of points", name)),
    }
}

fn parse_points_list(points: &[Val]) -> Result<Vec<(i32, i32)>, String> {
    let mut poly_points = Vec::new();
    for point in points {
        match point {
            Val::List(coords) => {
                if coords.len() != 2 {
                    return Err("Each point must have exactly 2 coordinates".into());
                }
                let x = match &coords[0] {
                    Val::Int(v) => *v as i32,
                    _ => return Err("Point coordinates must be integers".into()),
                };
                let y = match &coords[1] {
                    Val::Int(v) => *v as i32,
                    _ => return Err("Point coordinates must be integers".into()),
                };
                poly_points.push((x, y));
            }
            _ => return Err("Expected point to be a list".into()),
        }
    }
    Ok(poly_points)
}

fn polygon_list_to_opencv(
    pts: &[Vec<(i32, i32)>],
) -> opencv::core::Vector<opencv::core::Vector<opencv::core::Point>> {
    let mut pts_vec: opencv::core::Vector<opencv::core::Vector<opencv::core::Point>> =
        opencv::core::Vector::new();
    for polygon in pts {
        pts_vec.push(single_polygon_to_opencv(polygon));
    }
    pts_vec
}

fn single_polygon_to_opencv(points: &[(i32, i32)]) -> opencv::core::Vector<opencv::core::Point> {
    let mut points_vec: opencv::core::Vector<opencv::core::Point> = opencv::core::Vector::new();
    for (x, y) in points {
        points_vec.push(opencv::core::Point::new(*x, *y));
    }
    points_vec
}

pub struct FillPoly {}

struct FillPolyArgs {
    img: filter_utils::FrameArg,
    pts: Vec<Vec<(i32, i32)>>,
    color: [f64; 4],
    linetype: i32,
    shift: i32,
    offset: (i32, i32),
}

impl FillPoly {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<FillPolyArgs, String> {
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img" },
                Parameter::Positional { name: "pts" },
                Parameter::Positional { name: "color" },
                Parameter::PositionalOptional {
                    name: "lineType",
                    default_value: Val::Int(8),
                },
                Parameter::PositionalOptional {
                    name: "shift",
                    default_value: Val::Int(0),
                },
                Parameter::PositionalOptional {
                    name: "offset",
                    default_value: Val::List(vec![Val::Int(0), Val::Int(0)]),
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

        let pts = parse_polygon_list(&parsed_args)?;
        let color = filter_utils::get_color(&parsed_args)?;

        let linetype = match parsed_args.get("lineType") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'lineType' to be an integer".into()),
        };

        let shift = match parsed_args.get("shift") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'shift' to be an integer".into()),
        };

        let offset = match parsed_args.get("offset") {
            Some(Val::List(coords)) => {
                if coords.len() != 2 {
                    return Err("Offset must have exactly 2 coordinates".into());
                }
                let x = match &coords[0] {
                    Val::Int(v) => *v as i32,
                    _ => return Err("Offset coordinates must be integers".into()),
                };
                let y = match &coords[1] {
                    Val::Int(v) => *v as i32,
                    _ => return Err("Offset coordinates must be integers".into()),
                };
                (x, y)
            }
            _ => return Err("Expected 'offset' to be a list".into()),
        };

        Ok(FillPolyArgs {
            img,
            pts,
            color,
            linetype,
            shift,
            offset,
        })
    }
}

impl filter::Filter for FillPoly {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: FillPolyArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let img = opts.img.unwrap_frame();
        let (width, height) = (img.width, img.height);
        debug_assert_eq!(img.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let mut mat = filter_utils::frame_to_mat_rgb24(&img, width, height);
        let pts_vec = polygon_list_to_opencv(&opts.pts);
        let color =
            opencv::core::Scalar::new(opts.color[0], opts.color[1], opts.color[2], opts.color[3]);

        imgproc::fill_poly(
            &mut mat,
            &pts_vec,
            color,
            opts.linetype,
            opts.shift,
            opencv::core::Point::new(opts.offset.0, opts.offset.1),
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
        let opts: FillPolyArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        if opts.img.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(dve::Error::AVError("Expected RGB24 frame".into()));
        }

        Ok(opts.img.unwrap_frame_type())
    }
}

pub struct FillConvexPoly {}

struct FillConvexPolyArgs {
    img: filter_utils::FrameArg,
    points: Vec<(i32, i32)>,
    color: [f64; 4],
    linetype: i32,
    shift: i32,
}

impl FillConvexPoly {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<FillConvexPolyArgs, String> {
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img" },
                Parameter::Positional { name: "points" },
                Parameter::Positional { name: "color" },
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

        let points = parse_single_polygon(&parsed_args, "points")?;
        let color = filter_utils::get_color(&parsed_args)?;

        let linetype = match parsed_args.get("lineType") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'lineType' to be an integer".into()),
        };

        let shift = match parsed_args.get("shift") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'shift' to be an integer".into()),
        };

        Ok(FillConvexPolyArgs {
            img,
            points,
            color,
            linetype,
            shift,
        })
    }
}

impl filter::Filter for FillConvexPoly {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: FillConvexPolyArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let img = opts.img.unwrap_frame();
        let (width, height) = (img.width, img.height);
        debug_assert_eq!(img.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let mut mat = filter_utils::frame_to_mat_rgb24(&img, width, height);
        let points_vec = single_polygon_to_opencv(&opts.points);
        let color =
            opencv::core::Scalar::new(opts.color[0], opts.color[1], opts.color[2], opts.color[3]);

        imgproc::fill_convex_poly(&mut mat, &points_vec, color, opts.linetype, opts.shift).unwrap();

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
        let opts: FillConvexPolyArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        if opts.img.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(dve::Error::AVError("Expected RGB24 frame".into()));
        }

        Ok(opts.img.unwrap_frame_type())
    }
}

pub struct DrawContours {}

struct DrawContoursArgs {
    img: filter_utils::FrameArg,
    contours: Vec<Vec<(i32, i32)>>,
    contour_idx: i32,
    color: [f64; 4],
    thickness: i32,
    linetype: i32,
    hierarchy: Option<Vec<Vec<i64>>>,
    max_level: i32,
    offset: (i32, i32),
}

impl DrawContours {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<DrawContoursArgs, String> {
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img" },
                Parameter::Positional { name: "contours" },
                Parameter::Positional { name: "contourIdx" },
                Parameter::Positional { name: "color" },
                Parameter::PositionalOptional {
                    name: "thickness",
                    default_value: Val::Int(1),
                },
                Parameter::PositionalOptional {
                    name: "lineType",
                    default_value: Val::Int(8),
                },
                Parameter::PositionalOptional {
                    name: "hierarchy",
                    default_value: Val::List(vec![]),
                },
                Parameter::PositionalOptional {
                    name: "maxLevel",
                    default_value: Val::Int(i32::MAX as i64),
                },
                Parameter::PositionalOptional {
                    name: "offset",
                    default_value: Val::List(vec![Val::Int(0), Val::Int(0)]),
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

        // contours is a list of contours, each contour is a list of points
        let contours = parse_polygon_list_named(&parsed_args, "contours")?;

        let contour_idx = match parsed_args.get("contourIdx") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'contourIdx' to be an integer".into()),
        };

        let color = filter_utils::get_color(&parsed_args)?;

        let thickness = match parsed_args.get("thickness") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'thickness' to be an integer".into()),
        };

        let linetype = match parsed_args.get("lineType") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'lineType' to be an integer".into()),
        };

        // hierarchy is optional - list of [next, prev, first_child, parent] for each contour
        let hierarchy = match parsed_args.get("hierarchy") {
            Some(Val::List(h)) if h.is_empty() => None,
            Some(Val::List(h)) => {
                let mut result = Vec::new();
                for item in h {
                    match item {
                        Val::List(vec) => {
                            let mut row = Vec::new();
                            for v in vec {
                                match v {
                                    Val::Int(i) => row.push(*i),
                                    _ => return Err("Hierarchy values must be integers".into()),
                                }
                            }
                            result.push(row);
                        }
                        _ => return Err("Hierarchy must be a list of lists".into()),
                    }
                }
                Some(result)
            }
            _ => None,
        };

        let max_level = match parsed_args.get("maxLevel") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'maxLevel' to be an integer".into()),
        };

        let offset = match parsed_args.get("offset") {
            Some(Val::List(coords)) => {
                if coords.len() != 2 {
                    return Err("Offset must have exactly 2 coordinates".into());
                }
                let x = match &coords[0] {
                    Val::Int(v) => *v as i32,
                    _ => return Err("Offset coordinates must be integers".into()),
                };
                let y = match &coords[1] {
                    Val::Int(v) => *v as i32,
                    _ => return Err("Offset coordinates must be integers".into()),
                };
                (x, y)
            }
            _ => return Err("Expected 'offset' to be a list".into()),
        };

        Ok(DrawContoursArgs {
            img,
            contours,
            contour_idx,
            color,
            thickness,
            linetype,
            hierarchy,
            max_level,
            offset,
        })
    }
}

impl filter::Filter for DrawContours {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: DrawContoursArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let img = opts.img.unwrap_frame();
        let (width, height) = (img.width, img.height);
        debug_assert_eq!(img.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let mut mat = filter_utils::frame_to_mat_rgb24(&img, width, height);
        let contours_vec = polygon_list_to_opencv(&opts.contours);
        let color =
            opencv::core::Scalar::new(opts.color[0], opts.color[1], opts.color[2], opts.color[3]);

        // Build hierarchy vector if provided
        let hierarchy_vec: opencv::core::Vector<opencv::core::Vec4i> =
            if let Some(ref h) = opts.hierarchy {
                let mut vec = opencv::core::Vector::new();
                for row in h {
                    let v = opencv::core::Vec4i::from([
                        row.first().copied().unwrap_or(-1) as i32,
                        row.get(1).copied().unwrap_or(-1) as i32,
                        row.get(2).copied().unwrap_or(-1) as i32,
                        row.get(3).copied().unwrap_or(-1) as i32,
                    ]);
                    vec.push(v);
                }
                vec
            } else {
                opencv::core::Vector::new()
            };

        imgproc::draw_contours(
            &mut mat,
            &contours_vec,
            opts.contour_idx,
            color,
            opts.thickness,
            opts.linetype,
            &hierarchy_vec,
            opts.max_level,
            opencv::core::Point::new(opts.offset.0, opts.offset.1),
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
        let opts: DrawContoursArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        if opts.img.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(dve::Error::AVError("Expected RGB24 frame".into()));
        }

        Ok(opts.img.unwrap_frame_type())
    }
}

pub struct DrawMarker {}

struct DrawMarkerArgs {
    img: filter_utils::FrameArg,
    position: (i32, i32),
    color: [f64; 4],
    marker_type: i32,
    marker_size: i32,
    thickness: i32,
    line_type: i32,
}

impl DrawMarker {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<DrawMarkerArgs, String> {
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "img" },
                Parameter::Positional { name: "position" },
                Parameter::Positional { name: "color" },
                Parameter::PositionalOptional {
                    name: "markerType",
                    default_value: Val::Int(0), // MARKER_CROSS
                },
                Parameter::PositionalOptional {
                    name: "markerSize",
                    default_value: Val::Int(20),
                },
                Parameter::PositionalOptional {
                    name: "thickness",
                    default_value: Val::Int(1),
                },
                Parameter::PositionalOptional {
                    name: "line_type",
                    default_value: Val::Int(8),
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

        let position = match parsed_args.get("position") {
            Some(Val::List(coords)) => {
                if coords.len() != 2 {
                    return Err("Position must have exactly 2 coordinates".into());
                }
                let x = match &coords[0] {
                    Val::Int(v) => *v as i32,
                    _ => return Err("Position coordinates must be integers".into()),
                };
                let y = match &coords[1] {
                    Val::Int(v) => *v as i32,
                    _ => return Err("Position coordinates must be integers".into()),
                };
                (x, y)
            }
            _ => return Err("Expected 'position' to be a list".into()),
        };

        let color = filter_utils::get_color(&parsed_args)?;

        let marker_type = match parsed_args.get("markerType") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'markerType' to be an integer".into()),
        };

        let marker_size = match parsed_args.get("markerSize") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'markerSize' to be an integer".into()),
        };

        let thickness = match parsed_args.get("thickness") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'thickness' to be an integer".into()),
        };

        let line_type = match parsed_args.get("line_type") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'line_type' to be an integer".into()),
        };

        Ok(DrawMarkerArgs {
            img,
            position,
            color,
            marker_type,
            marker_size,
            thickness,
            line_type,
        })
    }
}

impl filter::Filter for DrawMarker {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: DrawMarkerArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let img = opts.img.unwrap_frame();
        let (width, height) = (img.width, img.height);
        debug_assert_eq!(img.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let mut mat = filter_utils::frame_to_mat_rgb24(&img, width, height);
        let color =
            opencv::core::Scalar::new(opts.color[0], opts.color[1], opts.color[2], opts.color[3]);

        imgproc::draw_marker(
            &mut mat,
            opencv::core::Point::new(opts.position.0, opts.position.1),
            color,
            opts.marker_type,
            opts.marker_size,
            opts.thickness,
            opts.line_type,
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
        let opts: DrawMarkerArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        if opts.img.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(dve::Error::AVError("Expected RGB24 frame".into()));
        }

        Ok(opts.img.unwrap_frame_type())
    }
}

// Helper to parse polygon list with a custom field name
fn parse_polygon_list_named(
    parsed_args: &BTreeMap<&str, Val>,
    name: &str,
) -> Result<Vec<Vec<(i32, i32)>>, String> {
    match parsed_args.get(name) {
        Some(Val::List(polygons)) => {
            let mut result = Vec::new();
            for polygon in polygons {
                match polygon {
                    Val::List(points) => {
                        result.push(parse_points_list(points)?);
                    }
                    _ => return Err("Expected polygon to be a list of points".into()),
                }
            }
            Ok(result)
        }
        _ => Err(format!("Expected '{}' to be a list of polygons", name)),
    }
}

pub struct Flip {}

struct FlipArgs {
    src: filter_utils::FrameArg,
    flip_code: i32,
}

impl Flip {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<FlipArgs, String> {
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "src" },
                Parameter::Positional { name: "flipCode" },
            ],
        };

        let kwargs = kwargs.clone();
        let args = args.to_vec();
        let parsed_args = filter_utils::parse_arguments(&signature, args, kwargs)?;

        let src = match parsed_args.get("src") {
            Some(Val::Frame(frame)) => filter_utils::FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => {
                filter_utils::FrameArg::FrameType(frame_type.clone())
            }
            _ => return Err("Expected 'src' to be a Frame".into()),
        };

        let flip_code = match parsed_args.get("flipCode") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'flipCode' to be an integer".into()),
        };

        Ok(FlipArgs { src, flip_code })
    }
}

impl filter::Filter for Flip {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: FlipArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let src = opts.src.unwrap_frame();
        let (width, height) = (src.width, src.height);
        debug_assert_eq!(src.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let src_mat = filter_utils::frame_to_mat_rgb24(&src, width, height);
        let mut dst_mat = opencv::core::Mat::default();

        opencv::core::flip(&src_mat, &mut dst_mat, opts.flip_code).unwrap();

        let f = match filter_utils::mat_to_frame_rgb24(dst_mat, width, height) {
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
        let opts: FlipArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        if opts.src.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(dve::Error::AVError("Expected RGB24 frame".into()));
        }

        Ok(opts.src.unwrap_frame_type())
    }
}

pub struct Rotate {}

struct RotateArgs {
    src: filter_utils::FrameArg,
    rotate_code: i32,
}

impl Rotate {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<RotateArgs, String> {
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "src" },
                Parameter::Positional { name: "rotateCode" },
            ],
        };

        let kwargs = kwargs.clone();
        let args = args.to_vec();
        let parsed_args = filter_utils::parse_arguments(&signature, args, kwargs)?;

        let src = match parsed_args.get("src") {
            Some(Val::Frame(frame)) => filter_utils::FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => {
                filter_utils::FrameArg::FrameType(frame_type.clone())
            }
            _ => return Err("Expected 'src' to be a Frame".into()),
        };

        let rotate_code = match parsed_args.get("rotateCode") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'rotateCode' to be an integer".into()),
        };

        Ok(RotateArgs { src, rotate_code })
    }
}

impl filter::Filter for Rotate {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: RotateArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let src = opts.src.unwrap_frame();
        let (width, height) = (src.width, src.height);
        debug_assert_eq!(src.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let src_mat = filter_utils::frame_to_mat_rgb24(&src, width, height);
        let mut dst_mat = opencv::core::Mat::default();

        opencv::core::rotate(&src_mat, &mut dst_mat, opts.rotate_code).unwrap();

        // Calculate new dimensions based on rotation
        let (new_width, new_height) = match opts.rotate_code {
            0 | 2 => (height, width), // 90 CW or 90 CCW: swap dimensions
            1 => (width, height),     // 180: same dimensions
            _ => (width, height),
        };

        let f = match filter_utils::mat_to_frame_rgb24(dst_mat, new_width, new_height) {
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
        let opts: RotateArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        if opts.src.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(dve::Error::AVError("Expected RGB24 frame".into()));
        }

        let src_type = opts.src.unwrap_frame_type();

        // Calculate new dimensions based on rotation
        let (new_width, new_height) = match opts.rotate_code {
            0 | 2 => (src_type.height, src_type.width), // 90 CW or 90 CCW: swap dimensions
            1 => (src_type.width, src_type.height),     // 180: same dimensions
            _ => (src_type.width, src_type.height),
        };

        Ok(filter::FrameType {
            width: new_width,
            height: new_height,
            format: src_type.format,
        })
    }
}

pub struct CopyMakeBorder {}

struct CopyMakeBorderArgs {
    src: filter_utils::FrameArg,
    top: i32,
    bottom: i32,
    left: i32,
    right: i32,
    border_type: i32,
    value: [f64; 4],
}

impl CopyMakeBorder {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<CopyMakeBorderArgs, String> {
        let signature = filter_utils::FunctionSignature {
            parameters: vec![
                Parameter::Positional { name: "src" },
                Parameter::Positional { name: "top" },
                Parameter::Positional { name: "bottom" },
                Parameter::Positional { name: "left" },
                Parameter::Positional { name: "right" },
                Parameter::Positional { name: "borderType" },
                Parameter::PositionalOptional {
                    name: "value",
                    default_value: Val::List(vec![
                        Val::Float(0.0),
                        Val::Float(0.0),
                        Val::Float(0.0),
                        Val::Float(255.0),
                    ]),
                },
            ],
        };

        let kwargs = kwargs.clone();
        let args = args.to_vec();
        let parsed_args = filter_utils::parse_arguments(&signature, args, kwargs)?;

        let src = match parsed_args.get("src") {
            Some(Val::Frame(frame)) => filter_utils::FrameArg::Frame(frame.clone()),
            Some(Val::FrameType(frame_type)) => {
                filter_utils::FrameArg::FrameType(frame_type.clone())
            }
            _ => return Err("Expected 'src' to be a Frame".into()),
        };

        let top = match parsed_args.get("top") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'top' to be an integer".into()),
        };

        let bottom = match parsed_args.get("bottom") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'bottom' to be an integer".into()),
        };

        let left = match parsed_args.get("left") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'left' to be an integer".into()),
        };

        let right = match parsed_args.get("right") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'right' to be an integer".into()),
        };

        let border_type = match parsed_args.get("borderType") {
            Some(Val::Int(value)) => *value as i32,
            _ => return Err("Expected 'borderType' to be an integer".into()),
        };

        let value = filter_utils::get_color_with_key(&parsed_args, "value")?;

        Ok(CopyMakeBorderArgs {
            src,
            top,
            bottom,
            left,
            right,
            border_type,
            value,
        })
    }
}

impl filter::Filter for CopyMakeBorder {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: CopyMakeBorderArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let src = opts.src.unwrap_frame();
        let (width, height) = (src.width, src.height);
        debug_assert_eq!(src.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

        let src_mat = filter_utils::frame_to_mat_rgb24(&src, width, height);
        let mut dst_mat = opencv::core::Mat::default();

        let border_value =
            opencv::core::Scalar::new(opts.value[0], opts.value[1], opts.value[2], opts.value[3]);

        opencv::core::copy_make_border(
            &src_mat,
            &mut dst_mat,
            opts.top,
            opts.bottom,
            opts.left,
            opts.right,
            opts.border_type,
            border_value,
        )
        .unwrap();

        let new_width = width + opts.left + opts.right;
        let new_height = height + opts.top + opts.bottom;

        let f = match filter_utils::mat_to_frame_rgb24(dst_mat, new_width, new_height) {
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
        let opts: CopyMakeBorderArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        if opts.src.unwrap_frame_type().format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(dve::Error::AVError("Expected RGB24 frame".into()));
        }

        let src_type = opts.src.unwrap_frame_type();
        let new_width = src_type.width + opts.left as usize + opts.right as usize;
        let new_height = src_type.height + opts.top as usize + opts.bottom as usize;

        Ok(filter::FrameType {
            width: new_width,
            height: new_height,
            format: src_type.format,
        })
    }
}

pub struct Hconcat {}

struct HconcatArgs {
    sources: Vec<filter_utils::FrameArg>,
}

impl Hconcat {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<HconcatArgs, String> {
        let signature = filter_utils::FunctionSignature {
            parameters: vec![Parameter::Positional { name: "tup" }],
        };

        let kwargs = kwargs.clone();
        let args = args.to_vec();
        let parsed_args = filter_utils::parse_arguments(&signature, args, kwargs)?;

        let sources = match parsed_args.get("tup") {
            Some(Val::List(list)) => {
                let mut sources = Vec::new();
                for item in list {
                    match item {
                        Val::Frame(frame) => {
                            sources.push(filter_utils::FrameArg::Frame(frame.clone()));
                        }
                        Val::FrameType(frame_type) => {
                            sources.push(filter_utils::FrameArg::FrameType(frame_type.clone()));
                        }
                        _ => return Err("Expected 'tup' to contain Frames".into()),
                    }
                }
                sources
            }
            _ => return Err("Expected 'tup' to be a list".into()),
        };

        if sources.is_empty() {
            return Err("Expected 'tup' to contain at least one Frame".into());
        }

        Ok(HconcatArgs { sources })
    }
}

impl filter::Filter for Hconcat {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: HconcatArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        // Convert all frames to Mats
        let mut mats: opencv::core::Vector<opencv::core::Mat> = opencv::core::Vector::new();
        let mut total_width: i32 = 0;
        let mut height: Option<i32> = None;

        for source in &opts.sources {
            let src = source.unwrap_frame();
            debug_assert_eq!(src.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

            if let Some(h) = height {
                if src.height != h {
                    return Err(dve::Error::AVError(
                        "All frames must have the same height for hconcat".into(),
                    ));
                }
            } else {
                height = Some(src.height);
            }

            total_width += src.width;
            let mat = filter_utils::frame_to_mat_rgb24(&src, src.width, src.height);
            mats.push(mat);
        }

        let mut dst_mat = opencv::core::Mat::default();
        opencv::core::hconcat(&mats, &mut dst_mat).unwrap();

        let f = match filter_utils::mat_to_frame_rgb24(dst_mat, total_width, height.unwrap()) {
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
        let opts: HconcatArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let mut total_width: usize = 0;
        let mut height: Option<usize> = None;
        let mut format = None;

        for source in &opts.sources {
            let src_type = source.unwrap_frame_type();

            if src_type.format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
                return Err(dve::Error::AVError("Expected RGB24 frame".into()));
            }

            if let Some(h) = height {
                if src_type.height != h {
                    return Err(dve::Error::AVError(
                        "All frames must have the same height for hconcat".into(),
                    ));
                }
            } else {
                height = Some(src_type.height);
                format = Some(src_type.format);
            }

            total_width += src_type.width;
        }

        Ok(filter::FrameType {
            width: total_width,
            height: height.unwrap(),
            format: format.unwrap(),
        })
    }
}

pub struct Vconcat {}

struct VconcatArgs {
    sources: Vec<filter_utils::FrameArg>,
}

impl Vconcat {
    fn args(
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> Result<VconcatArgs, String> {
        let signature = filter_utils::FunctionSignature {
            parameters: vec![Parameter::Positional { name: "tup" }],
        };

        let kwargs = kwargs.clone();
        let args = args.to_vec();
        let parsed_args = filter_utils::parse_arguments(&signature, args, kwargs)?;

        let sources = match parsed_args.get("tup") {
            Some(Val::List(list)) => {
                let mut sources = Vec::new();
                for item in list {
                    match item {
                        Val::Frame(frame) => {
                            sources.push(filter_utils::FrameArg::Frame(frame.clone()));
                        }
                        Val::FrameType(frame_type) => {
                            sources.push(filter_utils::FrameArg::FrameType(frame_type.clone()));
                        }
                        _ => return Err("Expected 'tup' to contain Frames".into()),
                    }
                }
                sources
            }
            _ => return Err("Expected 'tup' to be a list".into()),
        };

        if sources.is_empty() {
            return Err("Expected 'tup' to contain at least one Frame".into());
        }

        Ok(VconcatArgs { sources })
    }
}

impl filter::Filter for Vconcat {
    fn filter(
        &self,
        args: &[filter::Val],
        kwargs: &BTreeMap<std::string::String, filter::Val>,
    ) -> std::result::Result<filter::Frame, dve::Error> {
        let opts: VconcatArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        // Convert all frames to Mats
        let mut mats: opencv::core::Vector<opencv::core::Mat> = opencv::core::Vector::new();
        let mut total_height: i32 = 0;
        let mut width: Option<i32> = None;

        for source in &opts.sources {
            let src = source.unwrap_frame();
            debug_assert_eq!(src.format, ffi::AVPixelFormat_AV_PIX_FMT_RGB24);

            if let Some(w) = width {
                if src.width != w {
                    return Err(dve::Error::AVError(
                        "All frames must have the same width for vconcat".into(),
                    ));
                }
            } else {
                width = Some(src.width);
            }

            total_height += src.height;
            let mat = filter_utils::frame_to_mat_rgb24(&src, src.width, src.height);
            mats.push(mat);
        }

        let mut dst_mat = opencv::core::Mat::default();
        opencv::core::vconcat(&mats, &mut dst_mat).unwrap();

        let f = match filter_utils::mat_to_frame_rgb24(dst_mat, width.unwrap(), total_height) {
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
        let opts: VconcatArgs = match Self::args(args, kwargs) {
            Ok(args) => args,
            Err(err) => return Err(dve::Error::AVError(err)),
        };

        let mut total_height: usize = 0;
        let mut width: Option<usize> = None;
        let mut format = None;

        for source in &opts.sources {
            let src_type = source.unwrap_frame_type();

            if src_type.format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
                return Err(dve::Error::AVError("Expected RGB24 frame".into()));
            }

            if let Some(w) = width {
                if src_type.width != w {
                    return Err(dve::Error::AVError(
                        "All frames must have the same width for vconcat".into(),
                    ));
                }
            } else {
                width = Some(src_type.width);
                format = Some(src_type.format);
            }

            total_height += src_type.height;
        }

        Ok(filter::FrameType {
            width: width.unwrap(),
            height: total_height,
            format: format.unwrap(),
        })
    }
}
