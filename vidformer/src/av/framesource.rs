use num::Zero;
use num_rational::Rational64;
use rusty_ffmpeg::ffi;

use crate::av::decoder::Decoder;
use crate::av::demuxer::Demuxer;

pub struct FrameSource {
    demuxer: Demuxer,
    decoder: Decoder,
    packet: *mut ffi::AVPacket,
    pub frame: *mut ffi::AVFrame,
    frames_needs_unref: bool,
}

impl FrameSource {
    pub fn new(
        vid_path: &str,
        stream: usize,
        seek_ts: &Rational64,
        service: &crate::service::Service,
        file_size: u64,
        io_runtime_handle: &tokio::runtime::Handle,
        io_cache: Option<(&dyn crate::io::IoWrapper, &str)>,
    ) -> Result<FrameSource, crate::Error> {
        let mut demuxer = crate::av::demuxer::Demuxer::new(
            vid_path,
            stream,
            service,
            file_size,
            io_runtime_handle,
            io_cache,
        )?;
        if !seek_ts.is_zero() {
            demuxer.seek(seek_ts)?;
        }

        let decoder = crate::av::decoder::Decoder::new(demuxer.codec, demuxer.codec_parameters)?;

        let frame = unsafe { ffi::av_frame_alloc().as_mut() }
            .expect("failed to allocate memory for AVFrame");
        let packet = unsafe { ffi::av_packet_alloc().as_mut() }
            .expect("failed to allocate memory for AVPacket");

        Ok(FrameSource {
            demuxer,
            decoder,
            packet,
            frame,
            frames_needs_unref: false,
        })
    }

    pub fn next_frame(&mut self) -> Result<Option<()>, crate::Error> {
        if self.frames_needs_unref {
            unsafe {
                ffi::av_frame_unref(self.frame);
            }
            self.frames_needs_unref = false;
        }

        // Always try to remove ready frames before reading more packets
        loop {
            match self.decoder.read_frame(self.frame)? {
                crate::av::decoder::DecoderResult::Frame => {
                    // we have a frame, yay!
                    self.frames_needs_unref = true;
                    return Ok(Some(()));
                }
                crate::av::decoder::DecoderResult::Again => {
                    // we need more packets
                    if self.demuxer.read_packet(self.packet).is_some() {
                        self.decoder.send_packet(self.packet);
                        unsafe {
                            ffi::av_packet_unref(self.packet);
                        }
                    } else {
                        // no more packets, flush the decoder and go until EOF
                        self.decoder.flush();
                    }
                }
                crate::av::decoder::DecoderResult::Eof => {
                    // we are done
                    return Ok(None);
                }
            }
        }
    }

    pub fn time_base(&self) -> &Rational64 {
        &self.demuxer.time_base
    }

    pub fn as_avframe(&self) -> std::sync::Arc<crate::dve::AVFrame> {
        std::sync::Arc::new(crate::dve::AVFrame::clone_avframe(self.frame))
    }
}

impl Drop for FrameSource {
    fn drop(&mut self) {
        if self.frames_needs_unref {
            unsafe {
                ffi::av_frame_unref(self.frame);
            }
        }
        unsafe {
            ffi::av_frame_free(&mut (self.frame as *mut _));
            ffi::av_packet_free(&mut (self.packet as *mut _));
        }
        self.decoder.close();
        self.demuxer.close();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    const TEST_VID: &str = "../tos_720p.mp4";
    const TEST_STREAM: usize = 0;

    #[test]
    fn tos_test() {
        let service = crate::service::Service::default();
        let profile = crate::source::SourceVideoStreamMeta::profile(
            "tos_720p",
            "../tos_720p.mp4",
            0,
            &service,
            None,
        )
        .unwrap();

        let io_runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();

        let mut decoded_frames = 0;
        let mut framesource = FrameSource::new(
            TEST_VID,
            TEST_STREAM,
            &0.into(),
            &service,
            profile.file_size,
            io_runtime.handle(),
            None,
        )
        .unwrap();

        while framesource.next_frame().unwrap().is_some() {
            decoded_frames += 1;
        }

        assert_eq!(decoded_frames, 17616);
    }
}
