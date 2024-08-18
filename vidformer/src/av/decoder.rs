use log::*;
use rusty_ffmpeg::ffi;
use std::ptr;

pub struct Decoder {
    pub codec_context: *mut ffi::AVCodecContext,
}

pub enum DecoderResult {
    Frame,
    Again,
    Eof,
}

impl Decoder {
    pub fn new(
        codec: *const ffi::AVCodec,
        codec_parameters: *const ffi::AVCodecParameters,
    ) -> Result<Self, crate::Error> {
        let codec_context = unsafe { ffi::avcodec_alloc_context3(codec) };

        if codec_context.is_null() {
            return Err(crate::Error::AVError(
                "Failed to allocate memory for codec context".to_string(),
            ));
        }

        let response =
            unsafe { ffi::avcodec_parameters_to_context(codec_context, codec_parameters) };

        if response < 0 {
            return Err(crate::Error::AVError(
                "Failed to copy codec parameters to codec context".to_string(),
            ));
        }

        let response = unsafe { ffi::avcodec_open2(codec_context, codec, ptr::null_mut()) };
        if response < 0 {
            return Err(crate::Error::AVError(
                "Decoder failed to open codec".to_string(),
            ));
        }

        Ok(Decoder { codec_context })
    }

    pub fn send_packet(&mut self, packet: *mut ffi::AVPacket) {
        let response = unsafe { ffi::avcodec_send_packet(self.codec_context, packet) };

        if response < 0 {
            error!("Error sending packet to decoder");
        }
    }

    pub fn flush(&mut self) {
        let response = unsafe { ffi::avcodec_send_packet(self.codec_context, ptr::null_mut()) };
        if response < 0 {
            error!("Error flushing decoder");
        }
    }

    pub fn read_frame(&mut self, frame: *mut ffi::AVFrame) -> Result<DecoderResult, crate::Error> {
        let response = unsafe { ffi::avcodec_receive_frame(self.codec_context, frame) };

        if response == ffi::AVERROR(ffi::EAGAIN) {
            Ok(DecoderResult::Again)
        } else if response == ffi::AVERROR_EOF {
            Ok(DecoderResult::Eof)
        } else if response < 0 {
            Err(crate::Error::AVError(
                "Error while receiving a frame from the decoder.".to_string(),
            ))
        } else {
            Ok(DecoderResult::Frame)
        }
    }

    pub fn close(&self) {
        unsafe {
            ffi::avcodec_free_context(&mut (self.codec_context as *mut _));
        }
    }
}
