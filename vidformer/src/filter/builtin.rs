//! vidformer built-in filters

use crate::dve::AVFrame;
use crate::dve::Error;
use crate::filter::*;
use std::collections::BTreeMap;
use std::ffi::CString;

/// vidformer built-in filters
pub fn filters() -> BTreeMap<String, std::boxed::Box<dyn Filter>> {
    let mut filters: BTreeMap<String, std::boxed::Box<dyn Filter>> = BTreeMap::new();
    filters.insert(
        "PlaceholderFrame".to_string(),
        std::boxed::Box::new(PlaceholderFrame {}),
    );
    filters.insert("Scale".to_string(), std::boxed::Box::new(Scale {}));
    filters.insert(
        "_inline_mat".to_string(),
        std::boxed::Box::new(InlineMat {}),
    );
    filters.insert("_slice_mat".to_string(), std::boxed::Box::new(SliceMat {}));
    filters.insert(
        "_slice_write_mat".to_string(),
        std::boxed::Box::new(SliceWriteMat {}),
    );
    filters.insert("_black".to_string(), std::boxed::Box::new(Black {}));
    filters
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

pub struct InlineMat;
impl super::Filter for InlineMat {
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

        let compression = match kwargs.get("compression") {
            Some(Val::String(s)) => Option::Some(s),
            _ => Option::None,
        };

        let mut data = match &args[0] {
            Val::Bytes(b) => b,
            _ => panic!("Expected bytes"),
        };

        let mut decompressed: Vec<u8> = Vec::new();
        if let Some(compression) = compression {
            assert_eq!(compression, "zlib");

            let mut decoder = flate2::read::ZlibDecoder::new(&data[..]);
            use std::io::Read;
            decoder.read_to_end(&mut decompressed).unwrap();
            data = &decompressed;
            if data.len() != width as usize * height as usize * 3 {
                return Err(Error::InvalidFilterArgValue(
                    format!("{:?}", data.len()),
                    "Invalid data length".to_string(),
                ));
            }
        }

        let f = unsafe { ffi::av_frame_alloc() };
        if f.is_null() {
            panic!("ERROR could not allocate frame");
        }

        unsafe {
            (*f).width = width as i32;
            (*f).height = height as i32;
            (*f).format = ffi::AVPixelFormat_AV_PIX_FMT_RGB24;

            if ffi::av_frame_get_buffer(f, 0) < 0 {
                panic!("ERROR could not allocate frame data");
            }
        }

        // check if ffmpeg wants padding
        let linesize = unsafe { (*f).linesize[0] as usize };
        let data_len = data.len();
        let data_linesize = width as usize * 3;

        if linesize != data_linesize {
            for i in 0..height as usize {
                let src = &data[i * data_linesize..(i + 1) * data_linesize];
                let dst = unsafe { (*f).data[0].add(i * linesize) };
                unsafe {
                    std::ptr::copy_nonoverlapping(src.as_ptr(), dst, data_linesize);
                }
            }
        } else {
            let src = data.as_ptr();
            let dst = unsafe { (*f).data[0] };
            unsafe {
                std::ptr::copy_nonoverlapping(src, dst, data_len);
            }
        }

        Ok(Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, Error> {
        let width = match kwargs.get("width") {
            Some(Val::Int(i)) => *i,
            _ => return Err(Error::MissingFilterArg),
        };

        let height = match kwargs.get("height") {
            Some(Val::Int(i)) => *i,
            _ => return Err(Error::MissingFilterArg),
        };

        let pix_fmt = match kwargs.get("pix_fmt") {
            Some(Val::String(s)) => s,
            _ => return Err(Error::MissingFilterArg),
        };

        let compression = match kwargs.get("compression") {
            Some(Val::String(s)) => Option::Some(s),
            _ => Option::None,
        };

        if let Some(compression) = compression {
            if compression != "zlib" {
                return Err(Error::InvalidFilterArgValue(
                    compression.clone(),
                    "Invalid compression".to_string(),
                ));
            }
        }

        if pix_fmt != "rgb24" {
            return Err(Error::InvalidFilterArgValue(
                pix_fmt.clone(),
                "Invalid pixel format".to_string(),
            ));
        }

        if args.is_empty() {
            return Err(Error::MissingFilterArg);
        }

        if args.len() > 1 {
            return Err(Error::InvalidFilterArgValue(
                format!("{:?}", args.len()),
                "Invalid number of arguments".to_string(),
            ));
        }

        let data: &Vec<u8> = match args[0] {
            Val::Bytes(ref b) => b,
            _ => return Err(Error::MissingFilterArg),
        };

        // check if data length matches width, height, and pix_fmt
        if compression.is_none() && data.len() != width as usize * height as usize * 3 {
            return Err(Error::InvalidFilterArgValue(
                format!("{:?}", data.len()),
                "Invalid data length".to_string(),
            ));
        }

        Ok(FrameType {
            width: width as usize,
            height: height as usize,
            format: ffi::AVPixelFormat_AV_PIX_FMT_RGB24,
        })
    }
}

pub struct Black;
impl super::Filter for Black {
    fn filter(
        &self,
        _args: &[Val],
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

        let pix_fmt = match kwargs.get("pix_fmt").unwrap() {
            Val::String(s) => s,
            _ => panic!("Expected string"),
        };

        let pix_fmt_ffmpeg = match crate::util::pixel_fmt_str_to_av_pix_fmt(pix_fmt.as_str()) {
            Ok(pix_fmt) => pix_fmt,
            Err(_e) => {
                return Err(Error::InvalidFilterArgValue(
                    "pix_fmt".to_string(),
                    pix_fmt.clone(),
                ))
            }
        };

        let f = unsafe { ffi::av_frame_alloc() };
        if f.is_null() {
            panic!("ERROR could not allocate frame");
        }

        unsafe {
            (*f).width = width as i32;
            (*f).height = height as i32;
            (*f).format = pix_fmt_ffmpeg;

            if ffi::av_frame_get_buffer(f, 0) < 0 {
                panic!("ERROR could not allocate frame data");
            }
        }

        // Fill the frame with zeros
        let data_size = unsafe { (*f).linesize[0] as usize * height as usize };
        unsafe {
            std::ptr::write_bytes((*f).data[0], 0, data_size);
        }

        Ok(Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, Error> {
        let width = match kwargs.get("width") {
            Some(Val::Int(i)) => *i,
            _ => return Err(Error::MissingFilterArg),
        };

        let height = match kwargs.get("height") {
            Some(Val::Int(i)) => *i,
            _ => return Err(Error::MissingFilterArg),
        };

        let pix_fmt = match kwargs.get("pix_fmt") {
            Some(Val::String(s)) => s,
            _ => return Err(Error::MissingFilterArg),
        };

        let ff_pix_fmt = match crate::util::pixel_fmt_str_to_av_pix_fmt(pix_fmt.as_str()) {
            Ok(fmt) => fmt,
            Err(_e) => {
                return Err(Error::InvalidFilterArgValue(
                    "pix_fmt".to_string(),
                    pix_fmt.clone(),
                ))
            }
        };

        if !args.is_empty() {
            return Err(Error::InvalidFilterArgValue(
                format!("{:?}", args.len()),
                "Invalid number of arguments".to_string(),
            ));
        }

        Ok(FrameType {
            width: width as usize,
            height: height as usize,
            format: ff_pix_fmt,
        })
    }
}

// _slice_mat(frame, miny, maxy, minx, maxx)
pub struct SliceMat {}
impl super::Filter for SliceMat {
    fn filter(
        &self,
        args: &[Val],
        _kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        let frame = match &args[0] {
            Val::Frame(f) => f,
            _ => panic!("Expected frame"),
        };

        let miny = args[1].as_int().unwrap();
        let maxy = args[2].as_int().unwrap();
        let minx = args[3].as_int().unwrap();
        let maxx = args[4].as_int().unwrap();

        // create output frame
        let f = unsafe { ffi::av_frame_alloc() };
        if f.is_null() {
            panic!("ERROR could not allocate frame");
        }

        unsafe {
            (*f).width = (maxx - minx) as i32;
            (*f).height = (maxy - miny) as i32;
            (*f).format = ffi::AVPixelFormat_AV_PIX_FMT_RGB24;

            if ffi::av_frame_get_buffer(f, 0) < 0 {
                panic!("ERROR could not allocate frame data");
            }
        }

        for y in miny..maxy {
            let src = unsafe {
                (*frame.inner.inner).data[0]
                    .add(y as usize * (*frame.inner.inner).linesize[0] as usize + minx as usize * 3)
            };
            let dst = unsafe { (*f).data[0].add((y - miny) as usize * (*f).linesize[0] as usize) };
            unsafe {
                std::ptr::copy_nonoverlapping(src, dst, ((maxx - minx) * 3) as usize);
            }
        }

        Ok(Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, Error> {
        if args.len() < 5 {
            return Err(Error::MissingFilterArg);
        } else if args.len() > 5 {
            return Err(Error::InvalidFilterArgValue(
                format!("{:?}", args.len()),
                "Invalid number of arguments".to_string(),
            ));
        }

        if !kwargs.is_empty() {
            return Err(Error::InvalidFilterArgValue(
                format!("{:?}", kwargs.len()),
                "Invalid number of keyword arguments".to_string(),
            ));
        }

        let frame = match args[0] {
            Val::FrameType(ref f) => f,
            _ => return Err(Error::MissingFilterArg),
        };

        if frame.format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(Error::InvalidFilterArgValue(
                format!("{:?}", frame.format),
                "Invalid pixel format".to_string(),
            ));
        }

        let miny = match args[1] {
            Val::Int(i) => i,
            _ => return Err(Error::MissingFilterArg),
        };

        let maxy = match args[2] {
            Val::Int(i) => i,
            _ => return Err(Error::MissingFilterArg),
        };

        let minx = match args[3] {
            Val::Int(i) => i,
            _ => return Err(Error::MissingFilterArg),
        };

        let maxx = match args[4] {
            Val::Int(i) => i,
            _ => return Err(Error::MissingFilterArg),
        };

        if miny < 0
            || miny > frame.height as i64
            || maxy < 0
            || maxy > frame.height as i64
            || minx < 0
            || minx > frame.width as i64
            || maxx < 0
            || maxx > frame.width as i64
            || miny >= maxy
            || minx >= maxx
        {
            return Err(Error::InvalidFilterArgValue(
                format!("{:?}", args),
                "Invalid slice bounds".to_string(),
            ));
        }

        Ok(FrameType {
            width: (maxx - minx) as usize,
            height: (maxy - miny) as usize,
            format: frame.format,
        })
    }
}

// _slice_write_mat(f1, f2, miny, maxy, minx, maxx)
// equiv to python f1[miny:maxy, minx:maxx] = f2
pub struct SliceWriteMat {}
impl super::Filter for SliceWriteMat {
    fn filter(
        &self,
        args: &[Val],
        _kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        let f1 = match &args[0] {
            Val::Frame(f) => f,
            _ => panic!("Expected frame"),
        };

        let f2 = match &args[1] {
            Val::Frame(f) => f,
            _ => panic!("Expected frame"),
        };

        let miny = args[2].as_int().unwrap() as usize;
        let maxy = args[3].as_int().unwrap() as usize;
        let minx = args[4].as_int().unwrap() as usize;
        let maxx = args[5].as_int().unwrap() as usize;

        let f = unsafe { ffi::av_frame_alloc() };
        if f.is_null() {
            panic!("ERROR could not allocate frame");
        }

        unsafe {
            (*f).width = f1.width;
            (*f).height = f1.height;
            (*f).format = ffi::AVPixelFormat_AV_PIX_FMT_RGB24;

            if ffi::av_frame_get_buffer(f, 0) < 0 {
                panic!("ERROR could not allocate frame data");
            }
        }

        for y in 0..f1.height as usize {
            let src = unsafe {
                (*f1.inner.inner).data[0].add(y * (*f1.inner.inner).linesize[0] as usize)
            };
            let dst = unsafe { (*f).data[0].add(y * (*f).linesize[0] as usize) };
            unsafe {
                std::ptr::copy_nonoverlapping(src, dst, f1.width as usize * 3);
            }

            if y >= miny && y < maxy {
                let src = unsafe {
                    (*f2.inner.inner).data[0]
                        .add((y - miny) * (*f2.inner.inner).linesize[0] as usize)
                };
                let dst = unsafe { (*f).data[0].add(y * (*f).linesize[0] as usize) };
                unsafe {
                    std::ptr::copy_nonoverlapping(src, dst.add(minx * 3), (maxx - minx) * 3);
                }
            }
        }

        Ok(Frame::new(AVFrame { inner: f }))
    }

    fn filter_type(
        &self,
        args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<FrameType, Error> {
        if args.len() < 6 {
            return Err(Error::MissingFilterArg);
        } else if args.len() > 6 {
            return Err(Error::InvalidFilterArgValue(
                format!("{:?}", args.len()),
                "Invalid number of arguments".to_string(),
            ));
        }

        if !kwargs.is_empty() {
            return Err(Error::InvalidFilterArgValue(
                format!("{:?}", kwargs.len()),
                "Invalid number of keyword arguments".to_string(),
            ));
        }

        let f1 = match args[0] {
            Val::FrameType(ref f) => f,
            _ => return Err(Error::MissingFilterArg),
        };

        let f2 = match args[1] {
            Val::FrameType(ref f) => f,
            _ => return Err(Error::MissingFilterArg),
        };

        if f1.format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(Error::InvalidFilterArgValue(
                format!("{:?}", f1.format),
                "Invalid pixel format".to_string(),
            ));
        }

        if f2.format != ffi::AVPixelFormat_AV_PIX_FMT_RGB24 {
            return Err(Error::InvalidFilterArgValue(
                format!("{:?}", f2.format),
                "Invalid pixel format".to_string(),
            ));
        }

        let miny = match args[2] {
            Val::Int(i) => i,
            _ => return Err(Error::MissingFilterArg),
        };

        let maxy = match args[3] {
            Val::Int(i) => i,
            _ => return Err(Error::MissingFilterArg),
        };

        let minx = match args[4] {
            Val::Int(i) => i,
            _ => return Err(Error::MissingFilterArg),
        };

        let maxx = match args[5] {
            Val::Int(i) => i,
            _ => return Err(Error::MissingFilterArg),
        };

        if miny < 0
            || miny > f1.height as i64
            || maxy < 0
            || maxy > f1.height as i64
            || minx < 0
            || minx > f1.width as i64
            || maxx < 0
            || maxx > f1.width as i64
            || miny >= maxy
            || minx >= maxx
        {
            return Err(Error::InvalidFilterArgValue(
                format!("{:?}", args),
                "Invalid slice bounds".to_string(),
            ));
        }

        // check if f2 dimensions match slice dimensions
        if f2.width != (maxx - minx) as usize || f2.height != (maxy - miny) as usize {
            return Err(Error::InvalidFilterArgValue(
                format!("{:?}", args),
                "Frame f2 does not match slice size".to_string(),
            ));
        }

        Ok(FrameType {
            width: f1.width,
            height: f1.height,
            format: f1.format,
        })
    }
}
