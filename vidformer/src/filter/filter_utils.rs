use opencv::core::MatTrait;

use super::*;
use std::collections::BTreeMap;

use super::Val;

#[derive(Clone, Debug)]
pub(crate) enum Parameter {
    Positional {
        name: &'static str,
    },
    PositionalOptional {
        name: &'static str,
        default_value: Val,
    },
    // VarArgs {
    //     name: &'static str,
    // },
    // KeywordOnly {
    //     name: &'static str,
    // },
    // KeywordOnlyOptional {
    //     name: &'static str,
    //     default_value: Val,
    // },
    // KwArgs {
    //     name: &'static str,
    // },
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
    // let mut varargs = Vec::new();
    // let mut keyword_only: bool = false;

    for param in &signature.parameters {
        match param {
            Parameter::Positional { name } => {
                // assert!(
                //     !keyword_only,
                //     "Positional argument after keyword-only argument"
                // );
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
                // assert!(
                //     !keyword_only,
                //     "PositionalOptional argument after keyword-only argument"
                // );
                if let Some(val) = arg_iter.next() {
                    parsed_args.insert(name, val);
                } else if let Some(val) = kwargs.remove(*name) {
                    parsed_args.insert(name, val);
                } else {
                    parsed_args.insert(name, default_value.clone());
                }
            } // Parameter::VarArgs { name } => {
              //     assert!(
              //         !keyword_only,
              //         "VarArgs argument after keyword-only argument"
              //     );
              //     for val in arg_iter.by_ref() {
              //         varargs.push(val);
              //     }
              //     parsed_args.insert(name, Val::List(varargs.clone()));
              //     keyword_only = true; // Everything after *args is keyword-only
              // }
              // Parameter::KeywordOnly { name } => {
              //     if let Some(val) = kwargs.remove(*name) {
              //         parsed_args.insert(name, val);
              //     } else {
              //         return Err(format!("Missing required keyword-only argument '{}'", name));
              //     }
              // }
              // Parameter::KeywordOnlyOptional {
              //     name,
              //     default_value,
              // } => {
              //     if let Some(val) = kwargs.remove(*name) {
              //         parsed_args.insert(name, val);
              //     } else {
              //         parsed_args.insert(name, default_value.clone());
              //     }
              // }
              // Parameter::KwArgs { name: _ } => {
              //     todo!()
              // }
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

pub(crate) fn get_color_with_key(
    parsed_args: &BTreeMap<&'static str, Val>,
    key: &str,
) -> Result<[f64; 4], String> {
    let color = match parsed_args.get(key) {
        Some(Val::List(list)) => {
            if list.len() != 4 {
                return Err(format!("Expected '{key}' to be a list of four floats"));
            }
            match (
                list[0].clone(),
                list[1].clone(),
                list[2].clone(),
                list[3].clone(),
            ) {
                // Input is BGR (OpenCV convention), convert to RGB for internal use
                // by swapping the first and third channels
                (Val::Float(b), Val::Float(g), Val::Float(r), Val::Float(a)) => [r, g, b, a],
                _ => return Err(format!("Expected '{key}' to be a list of four floats")),
            }
        }
        _ => return Err(format!("Expected '{key}' to be a list of four floats")),
    };
    Ok(color)
}

pub(crate) fn get_color(parsed_args: &BTreeMap<&'static str, Val>) -> Result<[f64; 4], String> {
    get_color_with_key(parsed_args, "color")
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

/// Allocate an output RGB24 `AVFrame`, copy `img` in once, and return it with an
/// OpenCV `Mat` viewing its buffer. The `Mat` must be dropped before the frame.
pub(crate) fn frame_to_owned_mat_rgb24(
    img: &Frame,
    width: i32,
    height: i32,
) -> Result<(crate::dve::AVFrame, opencv::prelude::Mat), Result<Frame, crate::dve::Error>> {
    let f = unsafe { ffi::av_frame_alloc() };
    if f.is_null() {
        return Err(Err(crate::dve::Error::AVError(
            "Failed to allocate frame".into(),
        )));
    }
    // Own `f` now so it is freed if the caller's drawing panics.
    let out = crate::dve::AVFrame { inner: f };
    unsafe {
        (*f).width = width;
        (*f).height = height;
        (*f).format = ffi::AVPixelFormat_AV_PIX_FMT_RGB24;
        if ffi::av_frame_get_buffer(f, 0) < 0 {
            return Err(Err(crate::dve::Error::AVError(
                "Could not allocate frame data".into(),
            )));
        }
    }

    let av = img.inner.inner;
    let src_linesize = unsafe { (*av).linesize[0] };
    let dst_linesize = unsafe { (*f).linesize[0] };
    if src_linesize == width * 3 && dst_linesize == width * 3 {
        unsafe {
            std::ptr::copy_nonoverlapping(
                (*av).data[0],
                (*f).data[0],
                width as usize * height as usize * 3,
            );
        }
    } else {
        unsafe {
            let mut src = (*av).data[0];
            let mut dst = (*f).data[0];
            for _ in 0..height {
                std::ptr::copy_nonoverlapping(src, dst, width as usize * 3);
                src = src.add(src_linesize as usize);
                dst = dst.add(dst_linesize as usize);
            }
        }
    }

    let mat = match unsafe {
        opencv::core::Mat::new_rows_cols_with_data_unsafe(
            height,
            width,
            opencv::core::CV_8UC3,
            (*f).data[0] as *mut std::ffi::c_void,
            dst_linesize as usize,
        )
    } {
        Ok(mat) => mat,
        Err(e) => {
            return Err(Err(crate::dve::Error::AVError(format!(
                "Failed to wrap frame buffer: {}",
                e
            ))))
        }
    };

    Ok((out, mat))
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

    debug_assert_eq!(mat.elem_size().unwrap(), 3);
    debug_assert_eq!(mat.channels(), 3);
    debug_assert_eq!(mat.size().unwrap().height, height);
    debug_assert_eq!(mat.size().unwrap().width, width);
    unsafe {
        (*f).width = width;
        (*f).height = height;
        (*f).format = ffi::AVPixelFormat_AV_PIX_FMT_RGB24;

        if ffi::av_frame_get_buffer(f, 0) < 0 {
            panic!("ERROR could not allocate frame data");
        }
    }

    if unsafe { (*f).linesize[0] } == width * 3 {
        // no padding, just copy the data
        unsafe {
            let src = mat.data();
            let dst = (*f).data[0];
            std::ptr::copy_nonoverlapping(src, dst, width as usize * height as usize * 3);
        }
    } else {
        // there is padding, copy line by line
        debug_assert!(unsafe { (*f).linesize[0] } > width * 3);
        unsafe {
            let mut src = mat.data();
            let mut dst = (*f).data[0];
            for _ in 0..height {
                std::ptr::copy_nonoverlapping(src, dst, width as usize * 3);
                src = src.add(width as usize * 3);
                dst = dst.add((*f).linesize[0] as usize);
            }
        }
    }
    Ok(f)
}

pub(crate) fn frame_to_mat_rgb24(img: &Frame, width: i32, height: i32) -> opencv::prelude::Mat {
    debug_assert!(img.format == ffi::AVPixelFormat_AV_PIX_FMT_RGB24);
    debug_assert_eq!(img.height, height);
    debug_assert_eq!(img.width, width);
    debug_assert_eq!(
        unsafe { (*(img.inner.inner)).format },
        ffi::AVPixelFormat_AV_PIX_FMT_RGB24
    );
    debug_assert_eq!(unsafe { (*(img.inner.inner)).width }, width);
    debug_assert_eq!(unsafe { (*(img.inner.inner)).height }, height);

    let av = img.inner.inner;
    let linesize0 = unsafe { (*av).linesize[0] };
    let src0 = unsafe { (*av).data[0] };

    let mut mat =
        unsafe { opencv::core::Mat::new_rows_cols(height, width, opencv::core::CV_8UC3) }.unwrap();

    debug_assert!(mat.elem_size().unwrap() == 3);
    debug_assert!(mat.channels() == 3);
    debug_assert!(mat.size().unwrap().height == height);
    debug_assert!(mat.size().unwrap().width == width);
    debug_assert!(mat.is_continuous());

    if linesize0 == width * 3 {
        // no padding, just copy the data
        unsafe {
            let dst = mat.data_mut();
            std::ptr::copy_nonoverlapping(src0, dst, width as usize * height as usize * 3);
        }
    } else {
        // there is padding, copy line by line
        debug_assert!(linesize0 > width * 3);
        unsafe {
            let mut src = src0;
            let mut dst = mat.data_mut();
            for _ in 0..height {
                std::ptr::copy_nonoverlapping(src, dst, width as usize * 3);
                src = src.add(linesize0 as usize);
                dst = dst.add(width as usize * 3);
            }
        }
    }

    mat
}

pub(crate) fn frame_to_mat_gray8(img: &Frame, width: i32, height: i32) -> opencv::prelude::Mat {
    debug_assert!(img.format == ffi::AVPixelFormat_AV_PIX_FMT_GRAY8);
    debug_assert_eq!(img.height, height);
    debug_assert_eq!(img.width, width);
    debug_assert_eq!(
        unsafe { (*(img.inner.inner)).format },
        ffi::AVPixelFormat_AV_PIX_FMT_GRAY8
    );
    debug_assert_eq!(unsafe { (*(img.inner.inner)).width }, width);
    debug_assert_eq!(unsafe { (*(img.inner.inner)).height }, height);

    // Read the needed fields through the pointer rather than copying the whole
    // (large) AVFrame struct by value.
    let av = img.inner.inner;
    let linesize0 = unsafe { (*av).linesize[0] };
    let src0 = unsafe { (*av).data[0] };

    let mut mat =
        unsafe { opencv::core::Mat::new_rows_cols(height, width, opencv::core::CV_8UC1) }.unwrap();

    debug_assert!(mat.elem_size().unwrap() == 1);
    debug_assert!(mat.channels() == 1);
    debug_assert!(mat.size().unwrap().height == height);
    debug_assert!(mat.size().unwrap().width == width);
    debug_assert!(mat.is_continuous());

    if linesize0 == width {
        // no padding, just copy the data
        unsafe {
            let dst = mat.data_mut();
            std::ptr::copy_nonoverlapping(src0, dst, width as usize * height as usize);
        }
    } else {
        // there is padding, copy line by line
        debug_assert!(linesize0 > width);
        unsafe {
            let mut src = src0;
            let mut dst = mat.data_mut();
            for _ in 0..height {
                std::ptr::copy_nonoverlapping(src, dst, width as usize);
                src = src.add(linesize0 as usize);
                dst = dst.add(width as usize);
            }
        }
    }

    mat
}

#[cfg(test)]
mod tests {
    use opencv::core::Scalar;
    use opencv::prelude::Mat;
    use opencv::prelude::MatTraitConst;
    use rusty_ffmpeg::ffi;

    #[test]
    fn test_packed_layout_rgb24() {
        // we do some sharing of buffers between libav and opencv so we need to make sure that
        // the layout of the data is the same

        let num_planes =
            unsafe { ffi::av_pix_fmt_count_planes(ffi::AVPixelFormat_AV_PIX_FMT_RGB24) };
        assert_eq!(num_planes, 1);

        let width = 1920;
        let height = 1080;
        let size = (width as usize) * (height as usize) * 3;

        let frame_encoded_size_all_planes = unsafe {
            ffi::av_image_get_buffer_size(ffi::AVPixelFormat_AV_PIX_FMT_RGB24, width, height, 1)
        };

        assert_eq!(size, frame_encoded_size_all_planes as usize);

        // make sure mat data buffer is the same size as the frame buffer
        let color = Scalar::new(255.0, 0.0, 0.0, 0.0);
        let mat =
            Mat::new_rows_cols_with_default(height, width, opencv::core::CV_8UC3, color).unwrap();

        assert_eq!(size, mat.total() * mat.elem_size().unwrap());
        assert!(mat.is_continuous());
    }

    #[test]
    fn test_packed_layout_gray8() {
        let num_planes =
            unsafe { ffi::av_pix_fmt_count_planes(ffi::AVPixelFormat_AV_PIX_FMT_GRAY8) };
        assert_eq!(num_planes, 1);

        let width = 1920;
        let height = 1080;
        let size = (width as usize) * (height as usize);

        let frame_encoded_size_all_planes = unsafe {
            ffi::av_image_get_buffer_size(ffi::AVPixelFormat_AV_PIX_FMT_GRAY8, width, height, 1)
        };

        assert_eq!(size, frame_encoded_size_all_planes as usize);

        // make sure mat data buffer is the same size as the frame buffer
        let color = Scalar::new(255.0, 0.0, 0.0, 0.0);
        let mat =
            Mat::new_rows_cols_with_default(height, width, opencv::core::CV_8UC1, color).unwrap();

        assert_eq!(size, mat.total() * mat.elem_size().unwrap());
        assert!(mat.is_continuous());
    }
}

#[cfg(test)]
mod stride_tests {
    use super::*;
    use opencv::prelude::MatTraitConst;

    fn make_frame(width: i32, height: i32) -> (Frame, i32) {
        make_frame_with(width, height, width, 0)
    }

    /// Allocate for `alloc_width` at `align`, then narrow to `width`.
    fn make_frame_with(width: i32, height: i32, alloc_width: i32, align: i32) -> (Frame, i32) {
        assert!(alloc_width >= width);
        let f = unsafe { ffi::av_frame_alloc() };
        assert!(!f.is_null());
        unsafe {
            (*f).width = alloc_width;
            (*f).height = height;
            (*f).format = ffi::AVPixelFormat_AV_PIX_FMT_RGB24;
            assert!(ffi::av_frame_get_buffer(f, align) >= 0);
            let ls = (*f).linesize[0];
            for y in 0..height {
                let row = (*f).data[0].add((y * ls) as usize);
                for x in 0..width {
                    let p = row.add((x * 3) as usize);
                    *p = (x % 251) as u8;
                    *p.add(1) = (y % 241) as u8;
                    *p.add(2) = ((x + y) % 239) as u8;
                }
            }
            (*f).width = width;
            (Frame::new(crate::dve::AVFrame { inner: f }), ls)
        }
    }

    fn pixels(f: *mut ffi::AVFrame, width: i32, height: i32) -> Vec<u8> {
        let mut out = Vec::with_capacity((width * height * 3) as usize);
        unsafe {
            let ls = (*f).linesize[0];
            for y in 0..height {
                let row = (*f).data[0].add((y * ls) as usize);
                out.extend_from_slice(std::slice::from_raw_parts(row, (width * 3) as usize));
            }
        }
        out
    }

    fn draw(mat: &mut opencv::prelude::Mat, w: i32, h: i32) {
        opencv::imgproc::rectangle(
            mat,
            opencv::core::Rect::new(3, 2, w / 2, h / 2),
            opencv::core::Scalar::new(7.0, 200.0, 90.0, 0.0),
            2,
            opencv::imgproc::LINE_8,
            0,
        )
        .unwrap();
        // Deliberately off every edge. OpenCV clips to the Mat's declared size,
        // not the frame's, so an over-long Mat writes out of bounds here.
        opencv::imgproc::rectangle(
            mat,
            opencv::core::Rect::new(-5, h - 3, w + 50, 60),
            opencv::core::Scalar::new(220.0, 10.0, 30.0, 0.0),
            opencv::imgproc::FILLED,
            opencv::imgproc::LINE_8,
            0,
        )
        .unwrap();
    }

    #[test]
    fn in_place_matches_copy_path_with_padded_linesize() {
        for (w, h) in [(100, 50), (101, 37), (654, 31), (1280, 8)] {
            let (src, src_ls) = make_frame(w, h);
            let padded = src_ls != w * 3;

            let mut old_mat = frame_to_mat_rgb24(&src, w, h);
            draw(&mut old_mat, w, h);
            let old_f = mat_to_frame_rgb24(old_mat, w, h).ok().unwrap();
            let old_px = pixels(old_f, w, h);

            let (out, mut new_mat) = frame_to_owned_mat_rgb24(&src, w, h).ok().unwrap();
            assert_eq!(
                new_mat.step1(0).unwrap() * new_mat.elem_size1(),
                unsafe { (*out.inner).linesize[0] } as usize,
                "Mat step must be the frame's linesize"
            );
            // The Mat's declared size is all that keeps OpenCV in bounds.
            assert_eq!(new_mat.rows(), h, "Mat rows must be the frame's height");
            assert_eq!(new_mat.cols(), w, "Mat cols must be the frame's width");
            draw(&mut new_mat, w, h);
            drop(new_mat);
            let new_px = pixels(out.inner, w, h);

            assert_eq!(
                old_px, new_px,
                "{w}x{h} (linesize {src_ls}, padded={padded}): in-place output differs"
            );
            unsafe {
                let mut p = old_f;
                ffi::av_frame_free(&mut p);
            }
        }
    }

    /// Every case above has equal source and destination strides, so a loop
    /// that used one for both would pass.
    #[test]
    fn in_place_copy_honors_unequal_strides() {
        let (w, h) = (100, 40);
        let (src, src_ls) = make_frame_with(w, h, w + 700, 0);
        let (out, mat) = frame_to_owned_mat_rgb24(&src, w, h).ok().unwrap();
        let dst_ls = unsafe { (*out.inner).linesize[0] };
        assert!(
            src_ls > dst_ls,
            "source stride {src_ls} must exceed the destination's {dst_ls}"
        );
        assert_eq!(
            mat.step1(0).unwrap() * mat.elem_size1(),
            dst_ls as usize,
            "Mat step must be the destination's linesize, not the source's"
        );
        assert_eq!(mat.rows(), h);
        assert_eq!(mat.cols(), w);
        drop(mat);
        assert_eq!(
            pixels(src.inner.inner, w, h),
            pixels(out.inner, w, h),
            "copy did not honour both strides"
        );
    }

    /// The fast path needs both strides packed, not just the destination's.
    #[test]
    fn in_place_copy_handles_packed_source_into_padded_dest() {
        let (w, h) = (100, 40);
        let (src, src_ls) = make_frame_with(w, h, w, 1);
        let (out, mat) = frame_to_owned_mat_rgb24(&src, w, h).ok().unwrap();
        let dst_ls = unsafe { (*out.inner).linesize[0] };
        assert!(
            src_ls == w * 3 && dst_ls > w * 3,
            "want a packed source ({src_ls}) and a padded destination ({dst_ls})"
        );
        drop(mat);
        assert_eq!(
            pixels(src.inner.inner, w, h),
            pixels(out.inner, w, h),
            "copy did not honour both strides"
        );
    }

    /// The premise the tests above rest on.
    #[test]
    fn padded_linesize_actually_occurs() {
        let (_f1280, ls1280) = make_frame(1280, 8);
        assert_eq!(
            ls1280,
            1280 * 3,
            "1280 wide is packed -- md5 tests miss padding"
        );
        let (_f100, ls100) = make_frame(100, 8);
        assert!(
            ls100 > 100 * 3,
            "expected padding at width 100, got {ls100}"
        );
        // `test_tos_cv2_filter_chain` scales to this width for the same reason.
        let (_f1276, ls1276) = make_frame(1276, 8);
        assert!(
            ls1276 > 1276 * 3,
            "expected padding at width 1276, got {ls1276}"
        );
    }
}
