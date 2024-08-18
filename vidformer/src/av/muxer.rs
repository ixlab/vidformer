use log::*;
use num_rational::Rational64;
use rusty_ffmpeg::ffi;
use std::ffi::CString;
use std::ptr;

pub struct Muxer {
    pub ofmt_ctx: *mut ffi::AVFormatContext,
    pub out_time_base: ffi::AVRational,
    pub out_stream: *mut ffi::AVStream,
    pub frames: usize,
}

impl Muxer {
    pub fn new(
        output_path: &str,
        codecpar: *mut ffi::AVCodecParameters,
        time_base: &Rational64,
    ) -> Result<Self, crate::Error> {
        let output_path = CString::new(output_path).unwrap();
        let mut ofmt_ctx: *mut ffi::AVFormatContext = ptr::null_mut();

        if unsafe {
            ffi::avformat_alloc_output_context2(
                &mut ofmt_ctx,
                ptr::null_mut(),
                ptr::null_mut(),
                output_path.as_ptr(),
            )
        } < 0
        {
            return Err(crate::Error::AVError(
                "Failed to allocate output context".to_string(),
            ));
        }

        unsafe {
            (*ofmt_ctx).debug = 1;
        }

        let out_stream = unsafe { ffi::avformat_new_stream(ofmt_ctx, ptr::null()) };
        if out_stream.is_null() {
            return Err(crate::Error::AVError(
                "Failed to allocate output stream".to_string(),
            ));
        }

        let time_base = crate::util::rat_to_avrat(time_base);
        unsafe {
            ffi::avcodec_parameters_free(&mut (*out_stream).codecpar); // free existing codec parameters, this is the only reference to them

            (*out_stream).codecpar = ffi::avcodec_parameters_alloc();
            if (*out_stream).codecpar.is_null() {
                return Err(crate::Error::AVError(
                    "Failed to allocate codec parameters".to_string(),
                ));
            }
            if ffi::avcodec_parameters_copy((*out_stream).codecpar, codecpar) < 0 {
                return Err(crate::Error::AVError(
                    "Failed to copy codec parameters".to_string(),
                ));
            }

            (*out_stream).time_base = time_base;
        }

        if unsafe { (*(*ofmt_ctx).oformat).flags } & ffi::AVFMT_NOFILE as i32 == 0
            && unsafe {
                ffi::avio_open(
                    &mut (*ofmt_ctx).pb,
                    output_path.as_ptr(),
                    ffi::AVIO_FLAG_WRITE as i32,
                )
            } < 0
        {
            return Err(crate::Error::AVError(
                "Failed to open output file".to_string(),
            ));
        }

        // show output on terminal
        unsafe {
            ffi::av_dump_format(ofmt_ctx, 0, output_path.as_ptr(), 1);
        }

        if unsafe { ffi::avformat_write_header(ofmt_ctx, ptr::null_mut()) } < 0 {
            return Err(crate::Error::AVError(
                "Failed to write output container header".to_string(),
            ));
        }

        Ok(Muxer {
            ofmt_ctx,
            out_time_base: unsafe { (*out_stream).time_base },
            out_stream,
            frames: 0,
        })
    }

    pub fn mux_packet(&mut self, packet: *mut ffi::AVPacket) -> Result<(), crate::Error> {
        let input_pts = unsafe { (*packet).pts };
        let input_dts = unsafe { (*packet).dts };

        unsafe {
            (*packet).pos = -1;
            (*packet).stream_index = (*self.out_stream).index;
            (*packet).duration = 0;
        }

        info!(
            "MUX - Write packet with pts {} and dts {} side_elements={} flags={} (output pts = {}/{} and dts = {}/{})",
            input_pts,
            input_dts,
            unsafe { (*packet).side_data_elems },
            unsafe { (*packet).flags },
            unsafe { (*packet).pts },
            unsafe { (*self.out_stream).time_base.den},
            unsafe { (*packet).dts },
            unsafe { (*self.out_stream).time_base.den},
        );

        // Todo: maybe optionally interleave?
        if unsafe { ffi::av_interleaved_write_frame(self.ofmt_ctx, packet) } < 0 {
            return Err(crate::Error::AVError(
                "Failed to write packet to output".to_string(),
            ));
        }

        self.frames += 1;

        Ok(())
    }

    pub fn close(self) -> Result<(), crate::Error> {
        info!("Closing muxer");

        // flush
        if unsafe { ffi::av_interleaved_write_frame(self.ofmt_ctx, ptr::null_mut()) } < 0 {
            return Err(crate::Error::AVError("Failed to flush output".to_string()));
        }

        unsafe {
            ffi::av_write_trailer(self.ofmt_ctx);
            ffi::avio_close((*self.ofmt_ctx).pb);
            ffi::avformat_free_context(self.ofmt_ctx);
        }

        Ok(())
    }
}
