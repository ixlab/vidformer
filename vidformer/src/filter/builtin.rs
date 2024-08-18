//! vidformer built-in filters

use crate::dve::AVFrame;
use crate::dve::Error;
use crate::filter::*;
use rusty_ffmpeg::ffi;
use std::collections::BTreeMap;

mod ipc;
pub use ipc::IPC;

macro_rules! get_kwarg_or_err {
    ($map:expr, $arg:expr, Int) => {
        $map.get($arg)
            .and_then(|val| val.as_int())
            .ok_or(Error::MissingFilterArg)?
    };
    ($map:expr, $arg:expr, Str) => {
        $map.get($arg)
            .and_then(|val| val.as_str())
            .ok_or(Error::MissingFilterArg)?
    };
    ($map:expr, $arg:expr, Bool) => {
        $map.get($arg)
            .and_then(|val| val.as_bool())
            .ok_or(Error::MissingFilterArg)?
    };
    ($map:expr, $arg:expr, Frame) => {
        $map.get($arg)
            .and_then(|val| val.as_frame())
            .ok_or(Error::MissingFilterArg)?
    };
}

pub struct PlaceholderFrame {}
impl super::Filter for PlaceholderFrame {
    fn filter(
        &self,
        _args: &[Val],
        kwargs: &BTreeMap<String, Val>,
    ) -> Result<Frame, crate::dve::Error> {
        let width = get_kwarg_or_err!(kwargs, "width", Int);
        let height: i64 = get_kwarg_or_err!(kwargs, "height", Int);

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
        _args: &[ValType],
        kwargs: &BTreeMap<String, ValType>,
    ) -> Result<FrameType, Error> {
        let width = get_kwarg_or_err!(kwargs, "width", Int);
        let height: i64 = get_kwarg_or_err!(kwargs, "height", Int);

        Ok(FrameType::new(
            width as usize,
            height as usize,
            ffi::AVPixelFormat_AV_PIX_FMT_YUV420P,
        ))
    }
}
