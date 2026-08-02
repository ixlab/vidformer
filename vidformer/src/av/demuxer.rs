use log::*;
use num::Zero;
use num_rational::Rational64;
use rusty_ffmpeg::ffi;
use std::io::Read;
use std::io::Seek;
use std::{ptr, slice};

struct IoCtx {
    canary: u64, // We're doing some unsafe opaque pointer passing, so let's add a canary to make sure we didn't mess up. Valid value is 0xdeadbeef
    size: u64,
    reader: Box<dyn crate::io::ReadSeek>,
    err: Option<std::io::Error>,
}

unsafe extern "C" fn vidformer_avio_read_packet(
    opaque: *mut ::std::os::raw::c_void,
    buf: *mut u8,
    buf_size: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    let io_ctx = &mut *(opaque as *mut IoCtx);
    debug_assert_eq!(io_ctx.canary, 0xdeadbeef);

    if buf_size < 0 {
        return ffi::AVERROR_EXTERNAL;
    }
    let buf: &mut [u8] = unsafe { slice::from_raw_parts_mut(buf, buf_size as usize) };
    let read = io_ctx.reader.read(buf);
    match read {
        Ok(read) => {
            if read == 0 {
                ffi::AVERROR_EOF
            } else {
                read as i32
            }
        }
        Err(e) => {
            error!("Error reading packet: {}", e);
            if io_ctx.err.is_none() {
                io_ctx.err = Some(e);
            }
            ffi::AVERROR_EXTERNAL
        }
    }
}

unsafe extern "C" fn vidformer_avio_seek(
    opaque: *mut ::std::os::raw::c_void,
    offset: i64,
    whence: ::std::os::raw::c_int,
) -> i64 {
    let io_ctx = &mut *(opaque as *mut IoCtx);
    debug_assert_eq!(io_ctx.canary, 0xdeadbeef);

    // Never panic here — unwinding across the C ABI aborts the process.
    let whence_raw = whence as u32;
    if whence_raw & ffi::AVSEEK_SIZE != 0 {
        // libav way of asking for the size of the file
        return io_ctx.size as i64;
    }
    let whence = match whence_raw & !ffi::AVSEEK_FORCE {
        ffi::SEEK_CUR => std::io::SeekFrom::Current(offset),
        ffi::SEEK_END => std::io::SeekFrom::End(offset),
        ffi::SEEK_SET => std::io::SeekFrom::Start(offset as u64),
        _ => {
            error!("Unsupported seek whence ({})", whence);
            return ffi::AVERROR_EXTERNAL as i64;
        }
    };

    let seeked = io_ctx.reader.seek(whence);
    match seeked {
        Ok(seeked) => seeked as i64,
        Err(e) => {
            error!("Error seeking: {}", e);
            if io_ctx.err.is_none() {
                io_ctx.err = Some(e);
            }
            ffi::AVERROR_EXTERNAL as i64
        }
    }
}

pub struct Demuxer {
    pub format_context: *mut ffi::AVFormatContext,
    avio_context: *mut ffi::AVIOContext,
    #[allow(unused)] // We need to keep this alive since libav keeps it as an opaque pointer
    io_ctx: std::pin::Pin<std::boxed::Box<IoCtx>>,
    pub time_base: Rational64,
    pub codec: *const ffi::AVCodec,
    pub codec_parameters: *const ffi::AVCodecParameters,
    pub video_stream_index: Option<usize>,
    pub stream: *mut ffi::AVStream,
}

impl Demuxer {
    pub fn new(
        file_path: &str,
        stream_idx: usize,
        service: &crate::service::Service,
        file_size: u64,
        io_runtime_handle: &tokio::runtime::Handle,
        io_cache: Option<(&dyn crate::io::IoWrapper, &str)>,
    ) -> Result<Self, crate::Error> {
        let format_context = unsafe { ffi::avformat_alloc_context() };
        if format_context.is_null() {
            return Err(crate::Error::AVError(
                "could not allocate memory for Format Context".to_string(),
            ));
        }

        debug!("Opening {} for read", file_path);

        let op = service.blocking_operator(io_runtime_handle)?;

        let reader: opendal::BlockingReader = op.reader(file_path).map_err(|e| {
            if e.kind() == opendal::ErrorKind::NotFound {
                crate::Error::IOError(format!("File `{}` not found", file_path))
            } else {
                crate::Error::IOError(format!("OpenDAL error: {}", e))
            }
        })?;

        let reader: opendal::StdReader = match reader.into_std_read(0..file_size) {
            Ok(reader) => reader,
            Err(err) => {
                return Err(crate::Error::IOError(format!(
                    "OpenDAL failed to convert BlockingReader to StdReader: {}",
                    err
                )));
            }
        };

        let reader = match io_cache {
            Some((io_wrapper, fuid)) => io_wrapper.wrap(Box::new(reader), fuid),
            None => Box::new(std::io::BufReader::with_capacity(128 * 1024, reader)),
        };

        let io_ctx = IoCtx {
            canary: 0xdeadbeef,
            size: file_size,
            reader: Box::new(reader),
            err: None,
        };
        let io_ctx = Box::pin(io_ctx);
        let io_ctx_ptr =
            io_ctx.as_ref().get_ref() as *const IoCtx as *mut IoCtx as *mut ::std::os::raw::c_void;

        let avio_buffer_size = 16 * 1024;
        let avio_buffer: *mut std::ffi::c_void =
            unsafe { ffi::av_malloc(avio_buffer_size as usize) };

        let avio_context = unsafe {
            ffi::avio_alloc_context(
                avio_buffer as *mut u8,
                avio_buffer_size,
                0,
                io_ctx_ptr,
                Some(vidformer_avio_read_packet),
                None,
                Some(vidformer_avio_seek),
            )
        };

        if avio_context.is_null() {
            return Err(crate::Error::AVError(
                "could not allocate memory for AVIO Context".to_string(),
            ));
        }

        unsafe {
            (*format_context).pb = avio_context;
        }

        // `avformat_open_input` frees the format context on failure but never
        // touches `pb`, which libav has grown up to `probesize` while probing.
        let mut demuxer = Demuxer {
            format_context,
            avio_context,
            io_ctx,
            time_base: Rational64::new(0, 1),
            codec: ptr::null(),
            codec_parameters: ptr::null(),
            video_stream_index: None,
            stream: ptr::null_mut(),
        };

        let ret = unsafe {
            ffi::avformat_open_input(
                &mut demuxer.format_context,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        if ret != 0 {
            if ret == ffi::AVERROR_EXTERNAL {
                let err = match demuxer.io_ctx.err.as_ref() {
                    Some(err) => err,
                    None => {
                        return Err(crate::Error::IOError(
                            "IO error while opening media".to_string(),
                        ))
                    }
                };
                if err.kind() == std::io::ErrorKind::NotFound {
                    return Err(crate::Error::IOError(format!(
                        "File `{}` not found",
                        file_path
                    )));
                } else {
                    return Err(crate::Error::IOError(format!("OpenDAL error: {}", err)));
                }
            }

            let err_str = crate::util::av_strerror(ret);
            return Err(crate::Error::AVError(format!(
                "failed to open media format: {}",
                err_str
            )));
        }

        // TODO: This may decode a few frames, which could be slow. Maybe don't do that when actually running a spec?
        if unsafe { ffi::avformat_find_stream_info(demuxer.format_context, ptr::null_mut()) } < 0 {
            return Err(crate::Error::AVError(
                "could not get the stream info".to_string(),
            ));
        }

        let mut codec_ptr: *const ffi::AVCodec = ptr::null_mut();
        let mut codec_parameters_ptr: *const ffi::AVCodecParameters = ptr::null_mut();
        let mut video_stream_index = None;

        // libav leaves `streams` null for a container with no streams at all,
        // and `from_raw_parts_mut` requires non-null even at length 0.
        let nb_streams = unsafe { (*demuxer.format_context).nb_streams as usize };
        if stream_idx >= nb_streams {
            return Err(crate::Error::AVError(format!(
                "Stream index {} out of range (file has {} streams)",
                stream_idx, nb_streams
            )));
        }

        let streams =
            unsafe { slice::from_raw_parts_mut((*demuxer.format_context).streams, nb_streams) };

        for (i, stream) in streams
            .iter_mut()
            .map(|stream| unsafe { stream.as_mut() }.expect("null stream pointer"))
            .enumerate()
        {
            if i != stream_idx {
                stream.discard = ffi::AVDiscard_AVDISCARD_ALL;
                continue;
            }

            let local_codec_params = unsafe { stream.codecpar.as_ref() }.expect("codecpar is null");
            let local_codec =
                match unsafe { ffi::avcodec_find_decoder(local_codec_params.codec_id).as_ref() } {
                    Some(codec) => codec,
                    None => return Err(crate::Error::AVError("Unsupported codec".to_string())),
                };

            if local_codec_params.codec_type == ffi::AVMediaType_AVMEDIA_TYPE_VIDEO {
                if video_stream_index.is_none() {
                    video_stream_index = Some(i);
                    codec_ptr = local_codec;
                    codec_parameters_ptr = local_codec_params;
                }
            } else {
                // The requested stream is not a video stream
                let codec_type = match local_codec_params.codec_type {
                    ffi::AVMediaType_AVMEDIA_TYPE_AUDIO => "audio",
                    ffi::AVMediaType_AVMEDIA_TYPE_SUBTITLE => "subtitle",
                    ffi::AVMediaType_AVMEDIA_TYPE_DATA => "data",
                    ffi::AVMediaType_AVMEDIA_TYPE_ATTACHMENT => "attachment",
                    _ => "unknown",
                };
                return Err(crate::Error::AVError(format!(
                    "Stream {} is not a video stream (it is a {} stream). Please specify a valid video stream index.",
                    i, codec_type
                )));
            }
        }

        let time_base = crate::util::avrat_to_rat(&unsafe { (*streams[stream_idx]).time_base })?;
        debug_assert!(!time_base.is_zero());

        demuxer.time_base = time_base;
        demuxer.codec = codec_ptr;
        demuxer.codec_parameters = codec_parameters_ptr;
        demuxer.video_stream_index = video_stream_index;
        demuxer.stream = unsafe { streams[stream_idx].as_mut() }.unwrap();
        Ok(demuxer)
    }

    pub fn seek(&mut self, ts: &Rational64) -> Result<(), crate::Error> {
        let seek_ts = ts / self.time_base;
        assert_eq!(*seek_ts.denom(), 1);
        let seek_ts = *seek_ts.numer();

        debug!("Seeking to {}", ts);

        if unsafe { ffi::avformat_seek_file(self.format_context, 0, seek_ts, seek_ts, seek_ts, 0) }
            < 0
        {
            return Err(crate::Error::AVError("failed to seek file".to_string()));
        }

        Ok(())
    }

    pub fn read_packet(&mut self, packet: *mut ffi::AVPacket) -> Option<()> {
        loop {
            if unsafe { ffi::av_read_frame(self.format_context, packet) } >= 0 {
                // debug!(
                //     "DEMUX - Packet stream index {} pts = {} dts = {} key_frame = {} size = {}",
                //     unsafe { (*packet).stream_index },
                //     unsafe { (*packet).pts },
                //     unsafe { (*packet).dts },
                //     unsafe { (*packet).flags } as u32 & ffi::AV_PKT_FLAG_KEY != 0,
                //     unsafe { (*packet).size }
                // );

                // Is this packet from the correct stream?
                if unsafe { (*packet).stream_index } as usize == self.video_stream_index.unwrap() {
                    // If so, we have updated the pointer, so return
                    return Some(());
                } else {
                    // If not, unref the packet and keep trying
                    unsafe {
                        ffi::av_packet_unref(packet);
                    }
                    continue;
                }
            } else {
                return None;
            }
        }
    }

    pub fn close(&mut self) {
        self.free_resources();
    }

    /// Idempotent, so `close()` and then `Drop` is safe.
    fn free_resources(&mut self) {
        unsafe {
            if !self.format_context.is_null() {
                // The real field, so libav nulls it.
                ffi::avformat_close_input(&mut self.format_context);
            }

            if !self.avio_context.is_null() {
                ffi::av_freep(
                    &mut (*self.avio_context).buffer as *mut *mut u8
                        as *mut *mut std::os::raw::c_void
                        as *mut std::os::raw::c_void,
                );
                ffi::av_free(self.avio_context as *mut std::os::raw::c_void);
                self.avio_context = std::ptr::null_mut();
            }
        }
    }
}

impl Drop for Demuxer {
    fn drop(&mut self) {
        self.free_resources();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rusty_ffmpeg::ffi;

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

        let mut demuxer = Demuxer::new(
            TEST_VID,
            TEST_STREAM,
            &service,
            profile.file_size,
            io_runtime.handle(),
            None,
        )
        .unwrap();

        let packet = unsafe { ffi::av_packet_alloc().as_mut() }
            .expect("failed to allocated memory for AVPacket");

        let mut demuxed_packets = 0;
        while demuxer.read_packet(packet).is_some() {
            demuxed_packets += 1;
        }
        demuxer.close();

        assert_eq!(demuxed_packets, 17616);
    }
}
