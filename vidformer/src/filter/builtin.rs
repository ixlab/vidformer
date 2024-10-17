//! vidformer built-in filters

use crate::dve::AVFrame;
use crate::dve::Error;
use crate::filter::*;
use opencv::core::*;
use opencv::imgproc;
use rusty_ffmpeg::ffi;
use std::collections::BTreeMap;
use std::ffi::CString;
use std::ptr;
use std::sync::Arc;

mod drawutils;
mod ipc;
pub use ipc::IPC;

/// vidformer built-in filters
pub fn filters() -> BTreeMap<String, std::boxed::Box<dyn Filter>> {
    let mut filters: BTreeMap<String, std::boxed::Box<dyn Filter>> = BTreeMap::new();
    filters.insert(
        "PlaceholderFrame".to_string(),
        std::boxed::Box::new(PlaceholderFrame {}),
    );
    filters.insert("Annotate".to_string(), std::boxed::Box::new(Annotate {}));
    filters.insert("Box".to_string(), std::boxed::Box::new(Box {}));
    filters.insert(
        "BoundingBox".to_string(),
        std::boxed::Box::new(BoundingBox {}),
    );
    filters.insert("DrawBox".to_string(), std::boxed::Box::new(DrawBox {}));
    filters.insert("Scale".to_string(), std::boxed::Box::new(Scale {}));
    filters.insert("Pad".to_string(), std::boxed::Box::new(Pad {}));
    filters.insert("HStack".to_string(), std::boxed::Box::new(HStack {}));
    filters.insert("VStack".to_string(), std::boxed::Box::new(VStack {}));
    filters.insert("DrawText".to_string(), std::boxed::Box::new(DrawText {}));

    filters
}

fn avfilter_backed_uniframe(
    frame: &Arc<AVFrame>,
    filter: &str,
) -> Result<Frame, crate::dve::Error> {
    let buffer_str = CString::new("buffer").unwrap();
    let buffersink_str = CString::new("buffersink").unwrap();

    let buffersrc = unsafe { rusty_ffmpeg::ffi::avfilter_get_by_name(buffer_str.as_ptr()) };
    let buffersink = unsafe { rusty_ffmpeg::ffi::avfilter_get_by_name(buffersink_str.as_ptr()) };
    let mut outputs = unsafe { rusty_ffmpeg::ffi::avfilter_inout_alloc() };
    if outputs.is_null() {
        panic!("ERROR could not allocate output");
    }
    let mut inputs = unsafe { rusty_ffmpeg::ffi::avfilter_inout_alloc() };
    if inputs.is_null() {
        panic!("ERROR could not allocate input");
    }

    let args: CString = CString::new(format!(
        "video_size={}x{}:pix_fmt={}:time_base={}/{}:pixel_aspect={}/{}",
        unsafe { (*frame.inner).width },
        unsafe { (*frame.inner).height },
        unsafe { (*frame.inner).format },
        1, // time base doesn't matter since we're doing single-frame filters only
        12288,
        unsafe { (*frame.inner).sample_aspect_ratio.num },
        unsafe { (*frame.inner).sample_aspect_ratio.den }
    ))
    .unwrap();

    let mut filter_graph = unsafe { rusty_ffmpeg::ffi::avfilter_graph_alloc() };
    if filter_graph.is_null() {
        panic!("ERROR could not allocate filter graph");
    }
    unsafe {
        (*filter_graph).nb_threads = 1;
    }

    let mut buffersrc_ctx: *mut ffi::AVFilterContext = ptr::null_mut();

    let in_str = CString::new("in").unwrap();
    if unsafe {
        ffi::avfilter_graph_create_filter(
            &mut buffersrc_ctx,
            buffersrc,
            in_str.as_ptr(),
            args.as_ptr(),
            std::ptr::null_mut(),
            filter_graph,
        )
    } < 0
    {
        panic!("ERROR could not create buffer source");
    }

    let mut buffersink_ctx: *mut ffi::AVFilterContext = ptr::null_mut();
    let out_str = CString::new("out").unwrap();
    if unsafe {
        ffi::avfilter_graph_create_filter(
            &mut buffersink_ctx,
            buffersink,
            out_str.as_ptr(),
            std::ptr::null(),
            std::ptr::null_mut(),
            filter_graph,
        )
    } < 0
    {
        panic!("ERROR could not create buffer sink");
    }

    unsafe {
        (*outputs).name = CString::new("in").unwrap().into_raw();
        (*outputs).filter_ctx = buffersrc_ctx;
        (*outputs).pad_idx = 0;
        (*outputs).next = std::ptr::null_mut();

        (*inputs).name = CString::new("out").unwrap().into_raw();
        (*inputs).filter_ctx = buffersink_ctx;
        (*inputs).pad_idx = 0;
        (*inputs).next = std::ptr::null_mut();
    }

    let filter_cstr = CString::new(filter).unwrap();
    if unsafe {
        ffi::avfilter_graph_parse_ptr(
            filter_graph,
            filter_cstr.as_ptr(),
            &mut inputs,
            &mut outputs,
            std::ptr::null_mut(),
        )
    } < 0
    {
        panic!("ERROR could not parse filter graph");
    }

    if unsafe { ffi::avfilter_graph_config(filter_graph, std::ptr::null_mut()) } < 0 {
        panic!("ERROR could not configure filter graph");
    }

    // clone frame just in case something mutates it
    let mut f = unsafe { ffi::av_frame_alloc() };
    if f.is_null() {
        panic!("ERROR could not allocate frame");
    }

    unsafe {
        (*f).width = (*frame.inner).width;
        (*f).height = (*frame.inner).height;
        (*f).format = ffi::AVPixelFormat_AV_PIX_FMT_YUV420P;

        if ffi::av_frame_get_buffer(f, 0) < 0 {
            panic!(
                "ERROR could not allocate frame data buffer ({}x{}, {})",
                (*f).width,
                (*f).height,
                (*f).format
            );
        }
    }
    if unsafe { ffi::av_frame_copy(f, frame.inner) } < 0 {
        panic!("ERROR could not copy frame data");
    }

    if unsafe { ffi::av_buffersrc_add_frame_flags(buffersrc_ctx, f, 0) } < 0 {
        panic!("ERROR could not add frame to buffer source");
    }

    let filtered_frame = unsafe { ffi::av_frame_alloc() };
    if filtered_frame.is_null() {
        panic!("ERROR could not allocate filtered frame");
    }
    if unsafe { ffi::av_buffersink_get_frame(buffersink_ctx, filtered_frame) } < 0 {
        panic!("ERROR could not get frame from buffer sink");
    }

    let filtered_frame = AVFrame {
        inner: filtered_frame,
    };

    unsafe {
        ffi::av_frame_free(&mut f);
        ffi::avfilter_inout_free(&mut inputs);
        ffi::avfilter_inout_free(&mut outputs);
        ffi::avfilter_graph_free(&mut filter_graph);
    }

    Ok(Frame::new(filtered_frame))
}

fn opencv_backed_uniframe<F>(input_frame: &Frame, filter: F) -> Result<Frame, crate::dve::Error>
where
    F: FnOnce(&mut Mat),
{
    let width = input_frame.width;
    let height = input_frame.height;

    let data_y = unsafe {
        std::slice::from_raw_parts(
            (*input_frame.inner.inner).data[0],
            (*input_frame.inner.inner).linesize[0] as usize * height as usize,
        )
    };

    let data_u = unsafe {
        std::slice::from_raw_parts(
            (*input_frame.inner.inner).data[1],
            (*input_frame.inner.inner).linesize[1] as usize * height as usize / 2,
        )
    };

    let data_v = unsafe {
        std::slice::from_raw_parts(
            (*input_frame.inner.inner).data[2],
            (*input_frame.inner.inner).linesize[2] as usize * height as usize / 2,
        )
    };

    let mut yuv_data = vec![
        0;
        height as usize * width as usize
            + 2 * (height as usize / 2) * (width as usize / 2)
    ];
    yuv_data[..(height as usize * width as usize)].copy_from_slice(data_y);
    yuv_data[(height as usize * width as usize)
        ..(height as usize * width as usize + (height as usize / 2) * (width as usize / 2))]
        .copy_from_slice(data_u);
    yuv_data[(height as usize * width as usize + (height as usize / 2) * (width as usize / 2))..]
        .copy_from_slice(data_v);

    let mut yuv_mat =
        Mat::new_rows_cols_with_data_mut(height + height / 2, width, &mut yuv_data).unwrap();
    let mut bgr_mat = unsafe { Mat::new_size(Size::new(width, height), CV_8UC3).unwrap() };
    imgproc::cvt_color(&yuv_mat, &mut bgr_mat, imgproc::COLOR_YUV2BGR_I420, 0).unwrap();

    // draw bounding boxes

    filter(&mut bgr_mat);

    // convert back to yuv
    imgproc::cvt_color(&bgr_mat, &mut yuv_mat, imgproc::COLOR_BGR2YUV_I420, 0).unwrap();

    let f = unsafe { ffi::av_frame_alloc() };
    unsafe {
        (*f).width = width;
        (*f).height = height;
        (*f).format = ffi::AVPixelFormat_AV_PIX_FMT_YUV420P;

        if ffi::av_frame_get_buffer(f, 0) < 0 {
            panic!("ERROR could not allocate frame data");
        }
    }

    // copy data
    let data_y = unsafe {
        std::slice::from_raw_parts_mut((*f).data[0], (*f).linesize[0] as usize * height as usize)
    };
    let data_u = unsafe {
        std::slice::from_raw_parts_mut(
            (*f).data[1],
            (*f).linesize[1] as usize * height as usize / 2,
        )
    };
    let data_v = unsafe {
        std::slice::from_raw_parts_mut(
            (*f).data[2],
            (*f).linesize[2] as usize * height as usize / 2,
        )
    };

    data_y.copy_from_slice(&yuv_data[..(height as usize * width as usize)]);
    data_u.copy_from_slice(
        &yuv_data[(height as usize * width as usize)
            ..(height as usize * width as usize + (height as usize / 2) * (width as usize / 2))],
    );
    data_v.copy_from_slice(
        &yuv_data
            [(height as usize * width as usize + (height as usize / 2) * (width as usize / 2))..],
    );

    Ok(Frame::new(AVFrame { inner: f }))
}

pub struct PlaceholderFrame {}
impl super::Filter for PlaceholderFrame {
    fn filter(
        &self,
        _args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        let width = kwargs.get("width").unwrap().as_int().unwrap();
        let height: i64 = kwargs.get("height").unwrap().as_int().unwrap();

        let f = unsafe { ffi::av_frame_alloc() };
        unsafe {
            (*f).width = width as i32;
            (*f).height = height as i32;
            (*f).format = ffi::AVPixelFormat_AV_PIX_FMT_YUV420P;

            if ffi::av_frame_get_buffer(f, 0) < 0 {
                panic!("ERROR could not allocate frame data");
            }
        }

        for y in 0..unsafe { (*f).height } as usize {
            for x in 0..unsafe { (*f).width } as usize {
                unsafe {
                    *(*f).data[0].add(y * (*f).linesize[0] as usize + x) = ((x + y) % 256) as u8;
                }
            }
        }

        for y in 0..unsafe { (*f).height } as usize / 2 {
            for x in 0..unsafe { (*f).width } as usize / 2 {
                unsafe {
                    *(*f).data[1].add(y * (*f).linesize[1] as usize + x) = ((x + y) % 256) as u8;
                    *(*f).data[2].add(y * (*f).linesize[2] as usize + x) = ((x + y) % 256) as u8;
                }
            }
        }

        Ok(Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        _args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, Error> {
        let width = kwargs.get("width").unwrap().as_int().unwrap();
        let height: i64 = kwargs.get("height").unwrap().as_int().unwrap();

        Ok(FrameType::new(
            width as usize,
            height as usize,
            ffi::AVPixelFormat_AV_PIX_FMT_YUV420P,
        ))
    }
}

pub struct Annotate {}
impl super::Filter for Annotate {
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        assert!(args.len() == 1);
        assert!(kwargs.is_empty());

        let input_frame = match &args[0] {
            Val::Frame(f) => f,
            _ => panic!("Expected frame"),
        };

        assert_eq!(input_frame.format, ffi::AVPixelFormat_AV_PIX_FMT_YUV420P);

        let f = unsafe { ffi::av_frame_alloc() };
        unsafe {
            (*f).width = input_frame.width;
            (*f).height = input_frame.height;
            (*f).format = ffi::AVPixelFormat_AV_PIX_FMT_YUV420P;

            if ffi::av_frame_get_buffer(f, 0) < 0 {
                panic!("ERROR could not allocate frame data");
            }
        }

        if unsafe { ffi::av_frame_copy(f, input_frame.inner()) } < 0 {
            panic!("ERROR could not copy frame data");
        }

        for y in 100..200 {
            for x in 100..200 {
                unsafe {
                    *(*f).data[0].add(y * (*f).linesize[0] as usize + x) = ((x + y) % 256) as u8;
                }
            }
        }

        for y in 100..200 {
            for x in 100..200 {
                unsafe {
                    let x = x / 2;
                    let y = y / 2;

                    *(*f).data[1].add(y * (*f).linesize[1] as usize + x) = ((x + y) % 256) as u8;
                    *(*f).data[2].add(y * (*f).linesize[2] as usize + x) = ((x + y) % 256) as u8;
                }
            }
        }

        Ok(Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        _args: &[Val],
        _kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, Error> {
        todo!()
    }
}

pub struct Box {}
impl super::Filter for Box {
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        assert!(args.len() == 1);
        assert!(kwargs.is_empty());

        let input_frame = match &args[0] {
            Val::Frame(f) => f,
            _ => panic!("Expected frame"),
        };

        avfilter_backed_uniframe(
            &input_frame.inner,
            "drawbox=x=100:y=100:w=100:h=100:color=Cyan",
        )
    }

    fn filter_type(
        &self,
        args: &[Val],
        _kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, Error> {
        let input_frame = &args[0];
        match input_frame {
            Val::FrameType(frame_type) => Ok(frame_type.clone()),
            _ => Err(Error::MissingFilterArg),
        }
    }
}

pub struct BoundingBox {}

/*
   For example: [{"class": "person", "confidence": 0.9243842363357544, "x1": 2049.759765625, "y1": 244.0765838623047, "x2": 3025.21875, "y2": 1714.0}, {"class": "person", "confidence": 0.9086535573005676, "x1": 1062.2205810546875, "y1": 111.629638671875, "x2": 1809.4146728515625, "y2": 1708.45458984375}]
*/
#[allow(dead_code)] // used in parsing
#[derive(Debug, serde::Deserialize)]
pub struct BoundingBoxBound {
    class: String,
    confidence: f64,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl super::Filter for BoundingBox {
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        assert!(args.len() == 1);
        let bounds_str = match kwargs.get("bounds").unwrap() {
            Val::String(s) => s,
            _ => panic!("Expected string"),
        };

        let bounds: Vec<BoundingBoxBound> = serde_json::from_str(bounds_str).unwrap();

        let input_frame = match &args[0] {
            Val::Frame(f) => f,
            _ => panic!("Expected frame"),
        };

        if bounds.is_empty() {
            let frame_copy = unsafe { ffi::av_frame_clone(input_frame.inner.inner) };
            return Ok(Frame::new(AVFrame { inner: frame_copy }));
        }

        let filter = |frame: &mut Mat| {
            for bound in bounds {
                let x1 = bound.x1 as i32;
                let y1 = bound.y1 as i32;
                let x2 = bound.x2 as i32;
                let y2 = bound.y2 as i32;

                let color = Scalar::new(255.0, 0.0, 0.0, 0.0);
                let thickness = 2;
                let line_type = opencv::imgproc::LINE_8;
                let shift = 0;

                let pt1 = Point::new(x1, y1);
                let pt2 = Point::new(x2, y2);

                opencv::imgproc::rectangle(
                    frame,
                    Rect2i::from_points(pt1, pt2),
                    color,
                    thickness,
                    line_type,
                    shift,
                )
                .unwrap();
            }
        };

        opencv_backed_uniframe(input_frame, filter)
    }

    fn filter_type(
        &self,
        args: &[Val],
        _kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, Error> {
        let frame = args[0].as_frame_type().unwrap();
        Ok(frame.clone())
    }
}

pub struct DrawBox {}

impl super::Filter for DrawBox {
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        assert!(args.len() == 1);

        let input_frame = match &args[0] {
            Val::Frame(f) => f,
            _ => panic!("Expected frame"),
        };

        let x1 = match kwargs.get("x1").unwrap() {
            Val::Int(val) => *val as i32,
            _ => panic!("Expected a int for x1"),
        };

        let y1 = match kwargs.get("y1").unwrap() {
            Val::Int(val) => *val as i32,
            _ => panic!("Expected a int for y1"),
        };

        let x2 = match kwargs.get("x2").unwrap() {
            Val::Int(val) => *val as i32,
            _ => panic!("Expected a int for x2"),
        };

        let y2 = match kwargs.get("y2").unwrap() {
            Val::Int(val) => *val as i32,
            _ => panic!("Expected a int for y2"),
        };

        let filter = |frame: &mut Mat| {
            let color = Scalar::new(255.0, 0.0, 0.0, 0.0);
            let thickness = 2;
            let line_type = opencv::imgproc::LINE_8;
            let shift = 0;

            let pt1 = Point::new(x1, y1);
            let pt2 = Point::new(x2, y2);

            imgproc::rectangle(
                frame,
                opencv::core::Rect2i::from_points(pt1, pt2),
                color,
                thickness,
                line_type,
                shift,
            )
            .unwrap();
        };

        opencv_backed_uniframe(input_frame, filter)
    }

    fn filter_type(
        &self,
        args: &[Val],
        _kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, Error> {
        let frame = args[0].as_frame_type().unwrap();
        Ok(frame.clone())
    }
}

/// Scale & convert pixel format of a frame
///
/// # Arguments
///
/// * `width` - Width of the output frame (optional, but required if `height` is provided)
/// * `height` - Height of the output frame (optional, but required if `width` is provided)
/// * `format` - Pixel format of the output frame (optional)
pub struct Scale {}
impl super::Filter for Scale {
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        let (width, height) = {
            if kwargs.contains_key("width") || kwargs.contains_key("height") {
                if !(kwargs.contains_key("width") && kwargs.contains_key("height")) {
                    return Err(Error::MissingFilterArg);
                }
                (
                    match kwargs.get("width").unwrap() {
                        Val::Int(i) => Some(*i as usize),
                        _ => return Err(Error::MissingFilterArg),
                    },
                    match kwargs.get("height").unwrap() {
                        Val::Int(i) => Some(*i as usize),
                        _ => return Err(Error::MissingFilterArg),
                    },
                )
            } else {
                (None, None)
            }
        };

        let format = {
            if kwargs.contains_key("pix_fmt") {
                match kwargs.get("pix_fmt").unwrap() {
                    Val::String(s) => {
                        let format_cstr = CString::new(s.as_str()).unwrap();
                        let format = unsafe { ffi::av_get_pix_fmt(format_cstr.as_ptr()) };
                        if format == ffi::AVPixelFormat_AV_PIX_FMT_NONE {
                            return Err(Error::InvalidFilterArgValue(
                                s.clone(),
                                "Invalid pixel format".to_string(),
                            ));
                        }
                        Some(format)
                    }
                    _ => return Err(Error::MissingFilterArg),
                }
            } else {
                None
            }
        };

        assert!(args.len() == 1);

        let input_frame = match &args[0] {
            Val::Frame(f) => f,
            _ => return Err(Error::MissingFilterArg),
        };

        let output_type = FrameType {
            width: width.unwrap_or(input_frame.width as usize),
            height: height.unwrap_or(input_frame.height as usize),
            format: format.unwrap_or(input_frame.format),
        };

        let swscale_ctx = unsafe {
            ffi::sws_getContext(
                input_frame.width,
                input_frame.height,
                input_frame.format,
                output_type.width as i32,
                output_type.height as i32,
                output_type.format,
                ffi::SWS_BICUBIC as i32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };

        let f = unsafe { ffi::av_frame_alloc() };
        unsafe {
            (*f).width = output_type.width as i32;
            (*f).height = output_type.height as i32;
            (*f).format = output_type.format;

            if ffi::av_frame_get_buffer(f, 0) < 0 {
                panic!("ERROR could not allocate frame data");
            }
        }

        unsafe {
            ffi::sws_scale(
                swscale_ctx,
                (*input_frame.inner.inner).data.as_ptr() as *const *const u8,
                (*input_frame.inner.inner).linesize.as_ptr(),
                0,
                input_frame.height,
                (*f).data.as_mut_ptr(),
                (*f).linesize.as_mut_ptr(),
            );
        }

        unsafe {
            ffi::sws_freeContext(swscale_ctx);
        }

        Ok(Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, Error> {
        let (width, height) = {
            if kwargs.contains_key("width") || kwargs.contains_key("height") {
                if !(kwargs.contains_key("width") && kwargs.contains_key("height")) {
                    return Err(Error::MissingFilterArg);
                }
                (
                    match kwargs.get("width").unwrap() {
                        Val::Int(i) => Some(*i),
                        _ => return Err(Error::MissingFilterArg),
                    },
                    match kwargs.get("height").unwrap() {
                        Val::Int(i) => Some(*i),
                        _ => return Err(Error::MissingFilterArg),
                    },
                )
            } else {
                (None, None)
            }
        };

        let format = {
            if kwargs.contains_key("pix_fmt") {
                match kwargs.get("pix_fmt").unwrap() {
                    Val::String(s) => {
                        let format_cstr = CString::new(s.as_str()).unwrap();
                        let format = unsafe { ffi::av_get_pix_fmt(format_cstr.as_ptr()) };
                        if format == ffi::AVPixelFormat_AV_PIX_FMT_NONE {
                            return Err(Error::InvalidFilterArgValue(
                                s.clone(),
                                "Invalid pixel format".to_string(),
                            ));
                        }
                        Some(format)
                    }
                    _ => return Err(Error::MissingFilterArg),
                }
            } else {
                None
            }
        };

        assert!(args.len() == 1);

        let frame = &args[0];
        match frame {
            Val::FrameType(frame_type) => {
                let mut new_frame_type = frame_type.clone();
                if let Some(width) = width {
                    assert!(width > 0 && width % 2 == 0);
                    new_frame_type.width = width as usize;
                }
                if let Some(height) = height {
                    assert!(height > 0 && height % 2 == 0);
                    new_frame_type.height = height as usize;
                }
                if let Some(format) = format {
                    new_frame_type.format = format;
                }
                Ok(new_frame_type)
            }
            _ => Err(Error::MissingFilterArg),
        }
    }
}

pub struct Pad {}
impl super::Filter for Pad {
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        let width = match kwargs.get("width").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let height = match kwargs.get("height").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let x = match kwargs.get("x").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let y = match kwargs.get("y").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let input_frame = match &args[0] {
            Val::Frame(f) => f,
            _ => panic!("Expected frame"),
        };

        let f = unsafe { ffi::av_frame_alloc() };
        if f.is_null() {
            panic!("ERROR could not allocate frame");
        }

        unsafe {
            (*f).width = width as i32;
            (*f).height = height as i32;
            (*f).format = ffi::AVPixelFormat_AV_PIX_FMT_YUV420P;

            if ffi::av_frame_get_buffer(f, 0) < 0 {
                panic!("ERROR could not allocate frame data");
            }
        }

        let draw_context: *mut drawutils::FFDrawContext = unsafe {
            ffi::av_calloc(1, std::mem::size_of::<drawutils::FFDrawContext>())
                as *mut drawutils::FFDrawContext
        };
        if draw_context.is_null() {
            panic!("ERROR could not allocate draw context");
        }
        let ret = unsafe { drawutils::ff_draw_init(draw_context, input_frame.format, 0) };
        if ret < 0 {
            panic!("ERROR could not initialize draw context");
        }

        let color: *mut drawutils::FFDrawColor = unsafe {
            ffi::av_calloc(1, std::mem::size_of::<drawutils::FFDrawColor>())
                as *mut drawutils::FFDrawColor
        };
        if color.is_null() {
            panic!("ERROR could not allocate color");
        }

        let mut color_rgba = [0u8; 4];
        match kwargs.get("color") {
            Some(Val::String(s)) => {
                let color_str = CString::new(s.to_string()).unwrap();
                let ret = unsafe {
                    ffi::av_parse_color(
                        color_rgba.as_ptr() as *mut u8,
                        color_str.as_ptr(),
                        -1,
                        std::ptr::null_mut(),
                    )
                };
                if ret < 0 {
                    return Err(Error::InvalidFilterArgValue(
                        s.to_string(),
                        "Invalid color".to_string(),
                    ));
                }
            }
            _ => {
                color_rgba[0] = 0;
                color_rgba[1] = 0;
                color_rgba[2] = 0;
                color_rgba[3] = 255;
            }
        }

        unsafe { drawutils::ff_draw_color(draw_context, color, color_rgba.as_ptr()) };

        let in_w = input_frame.width as i64;
        let in_h = input_frame.height as i64;

        // top bar
        if y != 0 {
            unsafe {
                drawutils::ff_fill_rectangle(
                    draw_context,
                    color,
                    &mut (*f).data as *mut *mut u8,
                    &mut (*f).linesize as *mut i32,
                    0,
                    0,
                    width as i32,
                    height as i32,
                )
            };
        }

        // bottom bar
        if height > y + in_h {
            unsafe {
                drawutils::ff_fill_rectangle(
                    draw_context,
                    color,
                    &mut (*f).data as *mut *mut u8,
                    &mut (*f).linesize as *mut i32,
                    0,
                    (y + in_h) as i32,
                    width as i32,
                    (height - y - in_h) as i32,
                )
            };
        }

        // left border
        if x != 0 {
            unsafe {
                drawutils::ff_fill_rectangle(
                    draw_context,
                    color,
                    &mut (*f).data as *mut *mut u8,
                    &mut (*f).linesize as *mut i32,
                    0,
                    y as i32,
                    x as i32,
                    in_h as i32,
                )
            };
        }

        // copy input frame
        unsafe {
            drawutils::ff_copy_rectangle2(
                draw_context,
                &mut (*f).data as *mut *mut u8,
                &mut (*f).linesize as *mut i32,
                &mut (*input_frame.inner.inner).data as *mut *mut u8,
                &mut (*input_frame.inner.inner).linesize as *mut i32,
                x as i32,
                y as i32,
                0,
                0,
                in_w as i32,
                in_h as i32,
            )
        }

        // right border
        if width > x + in_w {
            unsafe {
                drawutils::ff_fill_rectangle(
                    draw_context,
                    color,
                    &mut (*f).data as *mut *mut u8,
                    &mut (*f).linesize as *mut i32,
                    (x + in_w) as i32,
                    y as i32,
                    (width - x - in_w) as i32,
                    in_h as i32,
                )
            };
        }

        unsafe {
            ffi::av_free(color as *mut std::ffi::c_void);
            ffi::av_free(draw_context as *mut std::ffi::c_void);
        }

        Ok(Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, crate::dve::Error> {
        let _x = match kwargs.get("x").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let _y = match kwargs.get("y").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let width = match kwargs.get("width").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let height = match kwargs.get("height").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let input_frame = match &args[0] {
            Val::FrameType(f) => f,
            _ => panic!("Expected frame"),
        };

        let mut color = [0u8; 4];
        match kwargs.get("color") {
            Some(Val::String(s)) => {
                let color_str = CString::new(s.to_string()).unwrap();
                let ret = unsafe {
                    ffi::av_parse_color(
                        color.as_ptr() as *mut u8,
                        color_str.as_ptr(),
                        -1,
                        std::ptr::null_mut(),
                    )
                };
                if ret < 0 {
                    return Err(Error::InvalidFilterArgValue(
                        s.to_string(),
                        "Invalid color".to_string(),
                    ));
                }
            }
            _ => {
                color[0] = 0;
                color[1] = 0;
                color[2] = 0;
                color[3] = 255;
            }
        }

        let out_frame_type = FrameType {
            width: width as usize,
            height: height as usize,
            format: input_frame.format,
        };

        Ok(out_frame_type)
    }
}

pub struct HStack {}
impl super::Filter for HStack {
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        let width = match kwargs.get("width").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let height = match kwargs.get("height").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let format = match kwargs.get("format").unwrap() {
            Val::String(s) => {
                let format_cstr = CString::new(s.as_str()).unwrap();
                let format = unsafe { ffi::av_get_pix_fmt(format_cstr.as_ptr()) };
                if format == ffi::AVPixelFormat_AV_PIX_FMT_NONE {
                    return Err(Error::InvalidFilterArgValue(
                        s.clone(),
                        "Invalid pixel format".to_string(),
                    ));
                }
                format
            }
            _ => panic!("Expected string"),
        };

        // create output frame
        let f = unsafe { ffi::av_frame_alloc() };
        if f.is_null() {
            panic!("ERROR could not allocate frame");
        }

        unsafe {
            (*f).width = width as i32;
            (*f).height = height as i32;
            (*f).format = format;

            if ffi::av_frame_get_buffer(f, 0) < 0 {
                panic!("ERROR could not allocate frame data");
            }

            let draw: *mut drawutils::FFDrawContext =
                ffi::av_calloc(1, std::mem::size_of::<drawutils::FFDrawContext>())
                    as *mut drawutils::FFDrawContext;

            if draw.is_null() {
                panic!("ERROR could not allocate draw context");
            }

            let ret = drawutils::ff_draw_init(draw, format, 0);
            if ret < 0 {
                panic!("ERROR could not initialize draw context");
            }

            let color_rgba = [0u8, 0u8, 0u8, 255u8];
            let color: *mut drawutils::FFDrawColor =
                ffi::av_calloc(1, std::mem::size_of::<drawutils::FFDrawColor>())
                    as *mut drawutils::FFDrawColor;
            if color.is_null() {
                panic!("ERROR could not allocate color");
            }

            drawutils::ff_draw_color(draw, color, color_rgba.as_ptr());

            // fill with black
            drawutils::ff_fill_rectangle(
                draw,
                color,
                &mut (*f).data as *mut *mut u8,
                &mut (*f).linesize as *mut i32,
                0,
                0,
                width as i32,
                height as i32,
            );

            ffi::av_free(color as *mut std::ffi::c_void);
            ffi::av_free(draw as *mut std::ffi::c_void);
        }

        let input_frames: Vec<&Frame> = args
            .iter()
            .map(|arg| match arg {
                Val::Frame(f) => f,
                _ => panic!("Expected frame"),
            })
            .collect();

        let each_frame_width = width / input_frames.len() as i64;

        for (i, frame) in input_frames.iter().enumerate() {
            let new_height =
                (frame.height as f64 / frame.width as f64 * each_frame_width as f64).round() as i64;

            let mut temp_frame = unsafe { ffi::av_frame_alloc() };
            if temp_frame.is_null() {
                panic!("ERROR could not allocate frame");
            }

            unsafe {
                (*temp_frame).width = each_frame_width as i32;
                (*temp_frame).height = new_height as i32;
                (*temp_frame).format = format;

                if ffi::av_frame_get_buffer(temp_frame, 0) < 0 {
                    panic!("ERROR could not allocate frame data");
                }
            }

            let sws_ctx = unsafe {
                ffi::sws_getContext(
                    frame.width,
                    frame.height,
                    frame.format,
                    each_frame_width as i32,
                    new_height as i32,
                    format,
                    ffi::SWS_BICUBIC as i32,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };

            unsafe {
                ffi::sws_scale(
                    sws_ctx,
                    (*frame.inner.inner).data.as_ptr() as *const *const u8,
                    (*frame.inner.inner).linesize.as_ptr(),
                    0,
                    frame.height,
                    (*temp_frame).data.as_mut_ptr(),
                    (*temp_frame).linesize.as_mut_ptr(),
                );
            }

            // Try to keep the image vertically centered
            let dst_y = (height - new_height) / 2;

            let draw: *mut drawutils::FFDrawContext = unsafe {
                ffi::av_calloc(1, std::mem::size_of::<drawutils::FFDrawContext>())
                    as *mut drawutils::FFDrawContext
            };
            if draw.is_null() {
                panic!("ERROR could not allocate draw context");
            }
            let ret = unsafe { drawutils::ff_draw_init(draw, format, 0) };
            if ret < 0 {
                panic!("ERROR could not initialize draw context");
            }

            // use ff_copy_rectangle2 to copy temp_frame to f
            unsafe {
                drawutils::ff_copy_rectangle2(
                    draw,
                    &mut (*f).data as *mut *mut u8,
                    &mut (*f).linesize as *mut i32,
                    &mut (*temp_frame).data as *mut *mut u8,
                    &mut (*temp_frame).linesize as *mut i32,
                    i as i32 * each_frame_width as i32, // i as i32 * each_frame_width as i32
                    dst_y as i32,                       // dst_y as i32,
                    0,
                    0,
                    each_frame_width as i32,
                    new_height as i32,
                )
            };

            unsafe {
                ffi::av_frame_free(&mut temp_frame);
                ffi::sws_freeContext(sws_ctx);
                ffi::av_free(draw as *mut std::ffi::c_void);
            }
        }

        Ok(Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, Error> {
        let width = match kwargs.get("width").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let height = match kwargs.get("height").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let format = match kwargs.get("format").unwrap() {
            Val::String(s) => {
                let format_cstr = CString::new(s.as_str()).unwrap();
                let format = unsafe { ffi::av_get_pix_fmt(format_cstr.as_ptr()) };
                if format == ffi::AVPixelFormat_AV_PIX_FMT_NONE {
                    return Err(Error::InvalidFilterArgValue(
                        s.clone(),
                        "Invalid pixel format".to_string(),
                    ));
                }
                format
            }
            _ => panic!("Expected string"),
        };

        for input in args {
            match input {
                Val::FrameType(_) => {}
                _ => return Err(Error::MissingFilterArg),
            }
        }

        Ok(FrameType {
            width: width as usize,
            height: height as usize,
            format,
        })
    }
}

pub struct VStack {}
impl super::Filter for VStack {
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        let width = match kwargs.get("width").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let height = match kwargs.get("height").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let format = match kwargs.get("format").unwrap() {
            Val::String(s) => {
                let format_cstr = CString::new(s.as_str()).unwrap();
                let format = unsafe { ffi::av_get_pix_fmt(format_cstr.as_ptr()) };
                if format == ffi::AVPixelFormat_AV_PIX_FMT_NONE {
                    return Err(Error::InvalidFilterArgValue(
                        s.clone(),
                        "Invalid pixel format".to_string(),
                    ));
                }
                format
            }
            _ => panic!("Expected string"),
        };

        // create output frame
        let f = unsafe { ffi::av_frame_alloc() };
        if f.is_null() {
            panic!("ERROR could not allocate frame");
        }

        unsafe {
            (*f).width = width as i32;
            (*f).height = height as i32;
            (*f).format = format;

            if ffi::av_frame_get_buffer(f, 0) < 0 {
                panic!("ERROR could not allocate frame data");
            }

            let draw: *mut drawutils::FFDrawContext =
                ffi::av_calloc(1, std::mem::size_of::<drawutils::FFDrawContext>())
                    as *mut drawutils::FFDrawContext;

            if draw.is_null() {
                panic!("ERROR could not allocate draw context");
            }

            let ret = drawutils::ff_draw_init(draw, format, 0);
            if ret < 0 {
                panic!("ERROR could not initialize draw context");
            }

            let color_rgba = [0u8, 0u8, 0u8, 255u8];
            let color: *mut drawutils::FFDrawColor =
                ffi::av_calloc(1, std::mem::size_of::<drawutils::FFDrawColor>())
                    as *mut drawutils::FFDrawColor;
            if color.is_null() {
                panic!("ERROR could not allocate color");
            }

            drawutils::ff_draw_color(draw, color, color_rgba.as_ptr());

            // fill with black
            drawutils::ff_fill_rectangle(
                draw,
                color,
                &mut (*f).data as *mut *mut u8,
                &mut (*f).linesize as *mut i32,
                0,
                0,
                width as i32,
                height as i32,
            );

            ffi::av_free(color as *mut std::ffi::c_void);
            ffi::av_free(draw as *mut std::ffi::c_void);
        }

        let input_frames: Vec<&Frame> = args
            .iter()
            .map(|arg| match arg {
                Val::Frame(f) => f,
                _ => panic!("Expected frame"),
            })
            .collect();

        let each_frame_height = height / input_frames.len() as i64;

        for (i, frame) in input_frames.iter().enumerate() {
            let new_width = (frame.width as f64 / frame.height as f64 * each_frame_height as f64)
                .round() as i64;

            let mut temp_frame = unsafe { ffi::av_frame_alloc() };
            if temp_frame.is_null() {
                panic!("ERROR could not allocate frame");
            }

            unsafe {
                (*temp_frame).width = new_width as i32;
                (*temp_frame).height = each_frame_height as i32;
                (*temp_frame).format = format;

                if ffi::av_frame_get_buffer(temp_frame, 0) < 0 {
                    panic!("ERROR could not allocate frame data");
                }
            }

            let sws_ctx = unsafe {
                ffi::sws_getContext(
                    frame.width,
                    frame.height,
                    frame.format,
                    new_width as i32,
                    each_frame_height as i32,
                    format,
                    ffi::SWS_BICUBIC as i32,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };

            unsafe {
                ffi::sws_scale(
                    sws_ctx,
                    (*frame.inner.inner).data.as_ptr() as *const *const u8,
                    (*frame.inner.inner).linesize.as_ptr(),
                    0,
                    frame.height,
                    (*temp_frame).data.as_mut_ptr(),
                    (*temp_frame).linesize.as_mut_ptr(),
                );
            }

            // Try to keep the image horizontally centered
            let dst_x = (width - new_width) / 2;

            let draw: *mut drawutils::FFDrawContext = unsafe {
                ffi::av_calloc(1, std::mem::size_of::<drawutils::FFDrawContext>())
                    as *mut drawutils::FFDrawContext
            };
            if draw.is_null() {
                panic!("ERROR could not allocate draw context");
            }
            let ret = unsafe { drawutils::ff_draw_init(draw, format, 0) };
            if ret < 0 {
                panic!("ERROR could not initialize draw context");
            }

            // use ff_copy_rectangle2 to copy temp_frame to f
            unsafe {
                drawutils::ff_copy_rectangle2(
                    draw,
                    &mut (*f).data as *mut *mut u8,
                    &mut (*f).linesize as *mut i32,
                    &mut (*temp_frame).data as *mut *mut u8,
                    &mut (*temp_frame).linesize as *mut i32,
                    dst_x as i32,
                    i as i32 * each_frame_height as i32,
                    0,
                    0,
                    new_width as i32,
                    each_frame_height as i32,
                )
            };

            unsafe {
                ffi::av_frame_free(&mut temp_frame);
                ffi::sws_freeContext(sws_ctx);
                ffi::av_free(draw as *mut std::ffi::c_void);
            }
        }

        Ok(Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, Error> {
        let width = match kwargs.get("width").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let height = match kwargs.get("height").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let format = match kwargs.get("format").unwrap() {
            Val::String(s) => {
                let format_cstr = CString::new(s.as_str()).unwrap();
                let format = unsafe { ffi::av_get_pix_fmt(format_cstr.as_ptr()) };
                if format == ffi::AVPixelFormat_AV_PIX_FMT_NONE {
                    return Err(Error::InvalidFilterArgValue(
                        s.clone(),
                        "Invalid pixel format".to_string(),
                    ));
                }
                format
            }
            _ => panic!("Expected string"),
        };

        for input in args {
            match input {
                Val::FrameType(_) => {}
                _ => return Err(Error::MissingFilterArg),
            }
        }

        Ok(FrameType {
            width: width as usize,
            height: height as usize,
            format,
        })
    }
}
pub struct DrawText {}
impl super::Filter for DrawText {
    fn filter(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        assert!(args.len() == 1);

        let input_frame = match &args[0] {
            Val::Frame(f) => f,
            _ => panic!("Expected frame"),
        };

        let text = match kwargs.get("text").unwrap() {
            Val::String(s) => s,
            _ => panic!("Expected string"),
        };

        let x = match kwargs.get("x").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };
        let y = match kwargs.get("y").unwrap() {
            Val::Int(i) => *i,
            _ => panic!("Expected int"),
        };

        let size = match kwargs.get("size") {
            Some(Val::Int(i)) => *i,
            _ => 20,
        };
        let scale = size as f64 / 20.0;

        let filter = |mat: &mut Mat| {
            let color = opencv::core::Scalar::new(255.0, 255.0, 255.0, 0.0);
            let font_face = opencv::imgproc::FONT_HERSHEY_SIMPLEX;
            let font_scale = scale;
            let thickness = scale as i32 * 2;
            let line_type = opencv::imgproc::LINE_8;
            let bottom_left_origin = false;

            opencv::imgproc::put_text(
                mat,
                text,
                Point::new(x as i32, y as i32),
                font_face,
                font_scale,
                color,
                thickness,
                line_type,
                bottom_left_origin,
            )
            .unwrap();
        };

        opencv_backed_uniframe(input_frame, filter)
    }

    fn filter_type(
        &self,
        args: &[Val],
        _kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, Error> {
        let input_frame = &args[0];
        match input_frame {
            Val::FrameType(frame_type) => Ok(frame_type.clone()),
            _ => Err(Error::MissingFilterArg),
        }
    }
}
