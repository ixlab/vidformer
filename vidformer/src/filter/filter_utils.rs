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
        (*f).format = ffi::AV_PIX_FMT_RGB24;

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
    debug_assert!(img.format == ffi::AV_PIX_FMT_RGB24);
    debug_assert_eq!(img.height, height);
    debug_assert_eq!(img.width, width);
    debug_assert_eq!(
        unsafe { (*(img.inner.inner)).format },
        ffi::AV_PIX_FMT_RGB24
    );
    debug_assert_eq!(unsafe { (*(img.inner.inner)).width }, width);
    debug_assert_eq!(unsafe { (*(img.inner.inner)).height }, height);

    let img: ffi::AVFrame = unsafe { *img.inner.inner };

    let mut mat =
        unsafe { opencv::core::Mat::new_rows_cols(height, width, opencv::core::CV_8UC3) }.unwrap();

    debug_assert!(mat.elem_size().unwrap() == 3);
    debug_assert!(mat.channels() == 3);
    debug_assert!(mat.size().unwrap().height == height);
    debug_assert!(mat.size().unwrap().width == width);
    debug_assert!(mat.is_continuous());

    if img.linesize[0] == width * 3 {
        // no padding, just copy the data
        unsafe {
            let src = img.data[0];
            let dst = mat.data_mut();
            std::ptr::copy_nonoverlapping(src, dst, width as usize * height as usize * 3);
        }
    } else {
        // there is padding, copy line by line
        debug_assert!(img.linesize[0] > width * 3);
        unsafe {
            let mut src = img.data[0];
            let mut dst = mat.data_mut();
            for _ in 0..height {
                std::ptr::copy_nonoverlapping(src, dst, width as usize * 3);
                src = src.add(img.linesize[0] as usize);
                dst = dst.add(width as usize * 3);
            }
        }
    }

    mat
}

pub(crate) fn frame_to_mat_gray8(img: &Frame, width: i32, height: i32) -> opencv::prelude::Mat {
    debug_assert!(img.format == ffi::AV_PIX_FMT_GRAY8);
    debug_assert_eq!(img.height, height);
    debug_assert_eq!(img.width, width);
    debug_assert_eq!(
        unsafe { (*(img.inner.inner)).format },
        ffi::AV_PIX_FMT_GRAY8
    );
    debug_assert_eq!(unsafe { (*(img.inner.inner)).width }, width);
    debug_assert_eq!(unsafe { (*(img.inner.inner)).height }, height);

    let img: ffi::AVFrame = unsafe { *img.inner.inner };

    let mut mat =
        unsafe { opencv::core::Mat::new_rows_cols(height, width, opencv::core::CV_8UC1) }.unwrap();

    debug_assert!(mat.elem_size().unwrap() == 1);
    debug_assert!(mat.channels() == 1);
    debug_assert!(mat.size().unwrap().height == height);
    debug_assert!(mat.size().unwrap().width == width);
    debug_assert!(mat.is_continuous());

    if img.linesize[0] == width {
        // no padding, just copy the data
        unsafe {
            let src = img.data[0];
            let dst = mat.data_mut();
            std::ptr::copy_nonoverlapping(src, dst, width as usize * height as usize);
        }
    } else {
        // there is padding, copy line by line
        debug_assert!(img.linesize[0] > width);
        unsafe {
            let mut src = img.data[0];
            let mut dst = mat.data_mut();
            for _ in 0..height {
                std::ptr::copy_nonoverlapping(src, dst, width as usize);
                src = src.add(img.linesize[0] as usize);
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
            unsafe { ffi::av_pix_fmt_count_planes(ffi::AV_PIX_FMT_RGB24) };
        assert_eq!(num_planes, 1);

        let width = 1920;
        let height = 1080;
        let size = (width as usize) * (height as usize) * 3;

        let frame_encoded_size_all_planes = unsafe {
            ffi::av_image_get_buffer_size(ffi::AV_PIX_FMT_RGB24, width, height, 1)
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
            unsafe { ffi::av_pix_fmt_count_planes(ffi::AV_PIX_FMT_GRAY8) };
        assert_eq!(num_planes, 1);

        let width = 1920;
        let height = 1080;
        let size = (width as usize) * (height as usize);

        let frame_encoded_size_all_planes = unsafe {
            ffi::av_image_get_buffer_size(ffi::AV_PIX_FMT_GRAY8, width, height, 1)
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
