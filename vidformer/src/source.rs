use log::*;
use num_rational::Rational64;
use rusty_ffmpeg::ffi;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::CStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceFileMeta {
    streams: Vec<SourceVideoStreamMeta>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceVideoStreamMeta {
    pub name: String,
    pub file_path: String,
    pub stream_idx: usize,
    pub service: crate::service::Service,
    pub file_size: u64,
    pub resolution: (usize, usize),
    pub codec: String,
    pub pix_fmt: String,
    pub ts: Vec<Rational64>,
    pub keys: Vec<Rational64>,
}

pub fn create_profile_file(streams: &[SourceVideoStreamMeta]) -> SourceFileMeta {
    let streams = streams.to_vec();
    SourceFileMeta { streams }
}

impl SourceVideoStreamMeta {
    pub fn profile(
        source_name: &str,
        vid_path: &str,
        stream: usize,
        service: &crate::service::Service,
    ) -> Result<Self, crate::dve::Error> {
        let io_runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();

        let file_size = {
            let op = service.blocking_operator(io_runtime.handle())?;
            let file_stat = op.stat(vid_path);

            let file_stat = match file_stat {
                Ok(stat) => stat,
                Err(e) if e.kind() == opendal::ErrorKind::NotFound => {
                    return Err(crate::dve::Error::IOError(format!(
                        "File `{}` not found",
                        vid_path
                    )));
                }
                Err(e) => {
                    return Err(crate::dve::Error::AVError(format!(
                        "Failed to stat file {}: {}",
                        vid_path, e
                    )));
                }
            };

            assert!(file_stat.is_file());
            file_stat.content_length()
        };

        let mut demuxer = crate::av::demuxer::Demuxer::new(
            vid_path,
            stream,
            service,
            file_size,
            io_runtime.handle(),
        )?;

        let codec_name = unsafe {
            CStr::from_ptr(ffi::avcodec_get_name(
                (*((*demuxer.stream).codecpar)).codec_id,
            ))
            .to_str()
            .unwrap()
            .to_string()
        };

        let resolution = unsafe {
            (
                (*((*demuxer.stream).codecpar)).width as usize,
                (*((*demuxer.stream).codecpar)).height as usize,
            )
        };

        let time_base = crate::util::avrat_to_rat(unsafe { &(*demuxer.stream).time_base });

        let pix_fmt = unsafe { (*((*demuxer.stream).codecpar)).format };
        let pix_fmt_name = unsafe {
            CStr::from_ptr(ffi::av_get_pix_fmt_name(pix_fmt))
                .to_str()
                .unwrap()
                .to_string()
        };

        let packet = unsafe { ffi::av_packet_alloc().as_mut() }
            .expect("failed to allocated memory for AVPacket");

        let mut pts_array = Vec::new();
        let mut key_array = Vec::new();

        while demuxer.read_packet(packet).is_some() {
            debug_assert!(packet.flags as u32 & ffi::AV_PKT_FLAG_CORRUPT == 0);
            trace!(
                "AVPacket [pts {}, dts {}, duration {}, key={}]",
                packet.pts,
                packet.dts,
                packet.duration,
                packet.flags as u32 & ffi::AV_PKT_FLAG_KEY != 0
            );

            // make sure the first frame is a keyframe
            if pts_array.is_empty() {
                assert!(packet.flags as u32 & ffi::AV_PKT_FLAG_KEY != 0);
            }

            // make sure a keyframe is always the new max pts
            if packet.flags as u32 & ffi::AV_PKT_FLAG_KEY != 0 && !pts_array.is_empty() {
                assert!(packet.pts > pts_array[pts_array.len() - 1]);
            }

            // make sure a non-keyframe is always past the most recent keyframe
            if packet.flags as u32 & ffi::AV_PKT_FLAG_KEY == 0 && !key_array.is_empty() {
                assert!(packet.pts > key_array[key_array.len() - 1]);
            }

            match pts_array.binary_search(&packet.pts) {
                Ok(_) => {
                    return Err(crate::dve::Error::AVError(format!(
                        "Duplicate pts {}",
                        packet.pts
                    )));
                }
                Err(idx) => {
                    pts_array.insert(idx, packet.pts);
                }
            }
            if packet.flags as u32 & ffi::AV_PKT_FLAG_KEY != 0 {
                assert!(key_array.is_empty() || key_array[key_array.len() - 1] < packet.pts); // keyframes should always go in order
                key_array.push(packet.pts);
            }

            unsafe { ffi::av_packet_unref(packet) };
        }

        unsafe {
            ffi::av_packet_free(&mut (packet as *mut _));
        }
        demuxer.close();

        let pts_array = pts_array
            .iter()
            .map(|&x| Rational64::new(x, 1) * time_base)
            .collect();

        let key_array: Vec<Rational64> = key_array
            .iter()
            .map(|&x| Rational64::new(x, 1) * time_base)
            .collect();

        Ok(SourceVideoStreamMeta {
            name: source_name.to_string(),
            codec: codec_name,
            stream_idx: stream,
            service: service.clone(),
            file_size,
            resolution,
            pix_fmt: pix_fmt_name,
            ts: pts_array,
            keys: key_array,
            file_path: vid_path.to_string(),
        })
    }

    pub fn validate(
        source_name: &str,
        vid_path: &str,
        stream: usize,
        service: &crate::service::Service,
    ) {
        let io_runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();

        // First profile
        let source_profile =
            SourceVideoStreamMeta::profile(source_name, vid_path, stream, service).unwrap();

        // Do a full decode
        {
            let mut decoded_frames = 0;
            let mut framesource = crate::av::framesource::FrameSource::new(
                vid_path,
                stream,
                &0.into(),
                service,
                source_profile.file_size,
                io_runtime.handle(),
            )
            .unwrap();
            while framesource.next_frame().unwrap().is_some() {
                decoded_frames += 1;
            }
            assert_eq!(decoded_frames, source_profile.ts.len());
        }

        // Decode each GOP individually, check we see expected behavior
        {
            let mut decoded_frames = 0;
            for (i, key) in source_profile.keys.iter().enumerate() {
                let mut framesource = crate::av::framesource::FrameSource::new(
                    vid_path,
                    stream,
                    key,
                    service,
                    source_profile.file_size,
                    io_runtime.handle(),
                )
                .unwrap();

                let mut expected_frame_ts: BTreeSet<&Rational64> =
                    source_profile.gop_times(i).iter().collect();
                let mut first_frame = true;

                while framesource.next_frame().unwrap().is_some() {
                    decoded_frames += 1;
                    let ts = Rational64::new(unsafe { (*framesource.frame).pts }, 1)
                        * framesource.time_base();

                    if first_frame {
                        assert_eq!(ts, *key);
                        first_frame = false;
                    }

                    assert!(expected_frame_ts.remove(&ts));
                    if expected_frame_ts.is_empty() {
                        break;
                    }
                }

                assert!(expected_frame_ts.is_empty());
            }

            assert_eq!(decoded_frames, source_profile.ts.len());
        }
    }

    fn gop_times(&self, gop: usize) -> &[Rational64] {
        debug_assert!(gop < self.keys.len());
        let start_i = self.ts.binary_search(&self.keys[gop]).unwrap();
        let end_i = if gop == self.keys.len() - 1 {
            self.ts.len()
        } else {
            self.ts.binary_search(&self.keys[gop + 1]).unwrap()
        };

        &self.ts[start_i..end_i]
    }
}
