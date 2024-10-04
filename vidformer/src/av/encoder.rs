use crate::dve::AVFrame;
use crate::util;
use log::*;
use num_rational::Rational64;
use rusty_ffmpeg::ffi;
use std::ffi::CString;
use std::ptr;

use crate::dve::Config;

pub struct Encoder {
    pub(crate) codec_ctx: *mut ffi::AVCodecContext,
    pub(crate) packet: *mut ffi::AVPacket,
    pub(crate) time_base: Rational64,
    pub(crate) flushed: bool,
}

impl Encoder {
    pub(crate) fn new(config: &Config, time_base: &Rational64) -> Result<Self, crate::Error> {
        let codec = match &config.encoder {
            Some(enc_cfg) => enc_cfg.avcodec()?,
            None => {
                let codec = unsafe { ffi::avcodec_find_encoder(ffi::AVCodecID_AV_CODEC_ID_H264) };
                match unsafe { codec.as_ref() } {
                    Some(codec) => codec,
                    None => panic!("Failed to find default h264 encoder"),
                }
            }
        };

        debug!("Encoder has codec {}", util::fmt_av_codec(codec));

        let codec_ctx: *mut ffi::AVCodecContext = unsafe { ffi::avcodec_alloc_context3(codec) };
        if codec_ctx.is_null() {
            return Err(crate::Error::AVError(
                "Failed to allocate codec context".to_string(),
            ));
        }

        let default_opts = &[("preset".to_string(), "ultrafast".to_string())];

        let opts: &[(String, String)] = match &config.encoder {
            Some(enc_cfg) => &enc_cfg.opts,
            None => default_opts,
        };

        for (opt_k, opt_v) in opts {
            let opt_k_cstr = CString::new(opt_k.clone()).unwrap();
            let opt_v_cstr = CString::new(opt_v.clone()).unwrap();

            let ret = unsafe {
                ffi::av_opt_set(
                    (*codec_ctx).priv_data,
                    opt_k_cstr.as_ptr(),
                    opt_v_cstr.as_ptr(),
                    0,
                )
            };
            if ret < 0 {
                return Err(crate::Error::AVError(format!(
                    "Failed to set encoder opt `{}` to `{}`",
                    opt_k, opt_v
                )));
            }
        }

        let pix_fmt_name = CString::new(config.output_pix_fmt.clone()).unwrap();
        let output_pix_fmt = unsafe { ffi::av_get_pix_fmt(pix_fmt_name.as_ptr()) };
        if output_pix_fmt == ffi::AVPixelFormat_AV_PIX_FMT_NONE {
            return Err(crate::Error::ConfigError(format!(
                "Failed to find output pix fmt `{}`",
                config.output_pix_fmt
            )));
        }

        // If supported pix_fmts are known make sure the output pix_fmt is supported
        if !codec.pix_fmts.is_null() {
            let mut found = false;
            for i in 0.. {
                let pix_fmt = unsafe { *codec.pix_fmts.add(i) };
                if pix_fmt == -1 {
                    break;
                }
                if pix_fmt == output_pix_fmt {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(crate::Error::ConfigError(format!(
                    "Output pix_fmt `{}` not supported by encoder",
                    config.output_pix_fmt
                )));
            }
        }

        let av_time_base = crate::util::rat_to_avrat(time_base);
        unsafe {
            (*codec_ctx).height = config.output_height as i32;
            (*codec_ctx).width = config.output_width as i32;
            (*codec_ctx).time_base = av_time_base;
            (*codec_ctx).gop_size = 10; // TODO: Don't set?
            (*codec_ctx).pix_fmt = output_pix_fmt;
            (*codec_ctx).flags |= ffi::AV_CODEC_FLAG_GLOBAL_HEADER as i32;
        }

        if unsafe { ffi::avcodec_open2(codec_ctx, codec, ptr::null_mut()) } < 0 {
            return Err(crate::Error::AVError("Failed to open encoder".to_string()));
        }

        let packet = unsafe { ffi::av_packet_alloc() };
        if packet.is_null() {
            return Err(crate::Error::AVError(
                "Failed to allocate packet".to_string(),
            ));
        }

        Ok(Encoder {
            codec_ctx,
            packet,
            time_base: *time_base,
            flushed: false,
        })
    }

    pub(crate) fn encode(&mut self, pts: &Rational64, frame: &AVFrame) -> Result<(), crate::Error> {
        // TODO: Do this elsewhere?
        if unsafe { ffi::av_frame_make_writable(frame.inner) } < 0 {
            return Err(crate::Error::AVError(
                "Failed to make frame writable".to_string(),
            ));
        }
        unsafe {
            // It's none of our business what the input frame type is
            // Also, we don't want the encoder to complain if something looks weird
            (*frame.inner).pict_type = ffi::AVPictureType_AV_PICTURE_TYPE_NONE;
        }

        let time_scaled = pts / self.time_base;
        assert!(*time_scaled.denom() == 1);
        debug!("ENCODE - Set packet pts to {}", *time_scaled.numer());
        unsafe {
            (*frame.inner).pts = *time_scaled.numer();
            (*frame.inner).pkt_dts = 0;
        }

        let ret = unsafe { ffi::avcodec_send_frame(self.codec_ctx, frame.inner) };
        if ret < 0 {
            let error = ffi::av_err2str(ret);
            return Err(crate::Error::AVError(format!(
                "Failed to send frame to encoder: {}",
                error
            )));
        }

        Ok(())
    }

    pub(crate) fn flush(&mut self) -> Result<(), crate::Error> {
        assert!(!self.flushed);
        self.flushed = true;
        if unsafe { ffi::avcodec_send_frame(self.codec_ctx, ptr::null_mut()) } < 0 {
            return Err(crate::Error::AVError("Failed to flush encoder".to_string()));
        }

        Ok(())
    }

    pub(crate) fn get_packet(&mut self) -> Option<*mut ffi::AVPacket> {
        if unsafe { ffi::avcodec_receive_packet(self.codec_ctx, self.packet) } >= 0 {
            Some(self.packet)
        } else {
            None
        }
    }

    pub(crate) fn close(&mut self) {
        assert!(self.flushed);
        assert!(self.get_packet().is_none());

        info!("Closing encoder");
        unsafe {
            ffi::av_packet_free(&mut self.packet);
            ffi::avcodec_free_context(&mut self.codec_ctx);
        }
    }
}
