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
    buf_reader: std::io::BufReader<opendal::StdReader>,
    err: Option<std::io::Error>,
}

unsafe extern "C" fn vidformer_avio_read_packet(
    opaque: *mut ::std::os::raw::c_void,
    buf: *mut u8,
    buf_size: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    let io_ctx = &mut *(opaque as *mut IoCtx);
    debug_assert_eq!(io_ctx.canary, 0xdeadbeef);

    let buf: &mut [u8] = unsafe { slice::from_raw_parts_mut(buf, buf_size as usize) };
    let read = io_ctx.buf_reader.read(buf);
    match read {
        Ok(read) => read as i32,
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

    let whence = match whence as u32 {
        ffi::SEEK_CUR => std::io::SeekFrom::Current(offset),
        ffi::SEEK_END => std::io::SeekFrom::End(offset),
        ffi::SEEK_SET => std::io::SeekFrom::Start(offset as u64),
        ffi::AVSEEK_SIZE => {
            // whence == AVSEEK_SIZE is the libav way of asking for the size of the file
            return io_ctx.size as i64;
        }
        ffi::AVSEEK_FORCE => panic!("AVSEEK_FORCE is not supported"), // libav way of saying "seek even if you have to reopen the file"
        _ => panic!("invalid seek whence ({})", whence,),
    };

    let seeked = io_ctx.buf_reader.seek(whence);
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
    ) -> Result<Self, crate::Error> {
        let mut format_context = unsafe { ffi::avformat_alloc_context() };
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
        let buffered_reader = std::io::BufReader::new(reader);

        let io_ctx = IoCtx {
            canary: 0xdeadbeef,
            size: file_size,
            buf_reader: buffered_reader,
            err: None,
        };
        let io_ctx = Box::pin(io_ctx);
        let io_ctx_ptr =
            io_ctx.as_ref().get_ref() as *const IoCtx as *mut IoCtx as *mut ::std::os::raw::c_void;

        let avio_buffer_size = 128 * 1024; // 128KiB buffer, probably good enough for now
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

        let ret = unsafe {
            ffi::avformat_open_input(
                &mut format_context,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        if ret != 0 {
            if ret == ffi::AVERROR_EXTERNAL {
                let err = io_ctx.err.as_ref().unwrap();
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
        if unsafe { ffi::avformat_find_stream_info(format_context, ptr::null_mut()) } < 0 {
            return Err(crate::Error::AVError(
                "could not get the stream info".to_string(),
            ));
        }

        let mut codec_ptr: *const ffi::AVCodec = ptr::null_mut();
        let mut codec_parameters_ptr: *const ffi::AVCodecParameters = ptr::null_mut();
        let mut video_stream_index = None;

        let streams = unsafe {
            slice::from_raw_parts_mut(
                (*format_context).streams,
                (*format_context).nb_streams as usize,
            )
        };

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
                unsafe { ffi::avcodec_find_decoder(local_codec_params.codec_id).as_ref() }
                    .expect("ERROR unsupported codec!");

            if local_codec_params.codec_type == ffi::AVMediaType_AVMEDIA_TYPE_VIDEO {
                if video_stream_index.is_none() {
                    video_stream_index = Some(i);
                    codec_ptr = local_codec;
                    codec_parameters_ptr = local_codec_params;
                }
            } else {
                println!("Stream {i} is not a video");
                continue;
            }
        }

        let time_base = crate::util::avrat_to_rat(&unsafe { (*streams[stream_idx]).time_base });
        debug_assert!(!time_base.is_zero());

        Ok(Demuxer {
            format_context,
            avio_context,
            io_ctx,
            time_base,
            codec: codec_ptr,
            codec_parameters: codec_parameters_ptr,
            video_stream_index,
            stream: unsafe { streams[stream_idx].as_mut() }.unwrap(),
        })
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

    pub fn close(&self) {
        unsafe {
            ffi::avformat_close_input(&mut (self.format_context as *mut _));

            // Free the avio buffer. libav and us share memory control over it, so its probably not even the one we allocated.
            // Sorry that pointer casting is so ugly, it makes sure to pass the pointer to the buffer pointer, not the buffer pointer itself.
            // Probably a better way to do this which doesn't involve using the mut keyword six times in one expression. ¯\_(ツ)_/¯
            ffi::av_freep(
                &mut (*self.avio_context).buffer as *mut *mut u8 as *mut *mut std::os::raw::c_void
                    as *mut std::os::raw::c_void,
            );

            ffi::av_free(self.avio_context as *mut std::os::raw::c_void);
        }
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
