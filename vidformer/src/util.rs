use num_rational::Rational64;
use rusty_ffmpeg::ffi;
use std::ffi::CStr;
use std::ffi::CString;

pub(crate) fn avrat_to_rat(avr: &ffi::AVRational) -> Rational64 {
    Rational64::new(avr.num as i64, avr.den as i64)
}

pub(crate) fn rat_to_avrat(rat: &Rational64) -> ffi::AVRational {
    ffi::AVRational {
        num: *rat.numer() as i32,
        den: *rat.denom() as i32,
    }
}

pub fn rand_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Instantiate FFmpeg logging
pub fn init() {
    unsafe {
        // TODO: Should be WARNING, but it keeps throwing sws warnings
        ffi::av_log_set_level(ffi::AV_LOG_ERROR as i32);
    }
}

#[derive(Debug)]
pub struct CodecDescriptor {
    pub name: String,
    pub long_name: String,
    pub format_id_pretty_name: String,
    pub has_decoder: bool,
    pub has_encoder: bool,
}

/// List all available codecs
pub fn codecs() -> Vec<CodecDescriptor> {
    let mut out = Vec::new();
    unsafe {
        let mut codec: *const ffi::AVCodecDescriptor = std::ptr::null();
        while {
            codec = ffi::avcodec_descriptor_next(codec);
            !codec.is_null()
        } {
            let codec = &*codec;
            if codec.type_ != ffi::AVMediaType_AVMEDIA_TYPE_VIDEO {
                continue;
            }

            let name = CString::new(CStr::from_ptr(codec.name).to_bytes()).unwrap();
            let long_name = CString::new(CStr::from_ptr(codec.long_name).to_bytes()).unwrap();
            let id = codec.id;
            let format_id_pretty_name = ffi::avcodec_get_name(id);
            let format_id_pretty_name =
                CString::new(CStr::from_ptr(format_id_pretty_name).to_bytes()).unwrap();

            let has_decoder = !ffi::avcodec_find_decoder(id).is_null();
            let has_encoder = !ffi::avcodec_find_encoder(id).is_null();

            out.push(CodecDescriptor {
                name: name.to_str().unwrap().to_string(),
                long_name: long_name.to_str().unwrap().to_string(),
                format_id_pretty_name: format_id_pretty_name.to_str().unwrap().to_string(),
                has_decoder,
                has_encoder,
            });
        }
    }

    out
}

pub(crate) fn fmt_av_codec(format: &ffi::AVCodec) -> String {
    let name = CString::new(unsafe { CStr::from_ptr(format.name) }.to_bytes()).unwrap();
    let long_name = CString::new(unsafe { CStr::from_ptr(format.long_name) }.to_bytes()).unwrap();
    let id = format.id;
    let format_id_pretty_name = unsafe {
        let format_id_pretty_name = ffi::avcodec_get_name(id);
        CString::new(CStr::from_ptr(format_id_pretty_name).to_bytes()).unwrap()
    };
    format!(
        "{}/{}/{}",
        name.to_str().unwrap(),
        long_name.to_str().unwrap(),
        format_id_pretty_name.to_str().unwrap()
    )
}

pub(crate) fn pixel_fmt_str_to_av_pix_fmt(s: &str) -> Result<ffi::AVPixelFormat, String> {
    let s = CString::new(s).unwrap();
    let fmt = unsafe { ffi::av_get_pix_fmt(s.as_ptr()) };
    if fmt == ffi::AVPixelFormat_AV_PIX_FMT_NONE {
        Err(format!("Invalid pixel format: {}", s.to_str().unwrap()))
    } else {
        Ok(fmt)
    }
}

pub(crate) fn pixel_fmt_str(n: i32) -> &'static str {
    unsafe {
        let fmt = ffi::av_get_pix_fmt_name(n);
        CStr::from_ptr(fmt).to_str().unwrap()
    }
}

pub(crate) fn libav_error_str(err: i32) -> String {
    let mut buf = [0u8; 1024];
    unsafe {
        ffi::av_strerror(err, buf.as_mut_ptr() as *mut i8, buf.len());
    }
    String::from_utf8_lossy(&buf).to_string()
}
