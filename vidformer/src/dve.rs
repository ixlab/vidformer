use crate::av;
use crate::filter;
use crate::filter::Frame;
use crate::pool::Pool;
use crate::sir;

use crate::spec::Spec;
use crate::util;
use log::*;
use num::integer::Roots;
use num_rational::Rational64;
use parking_lot::Condvar;
use parking_lot::Mutex;
use rusty_ffmpeg::ffi;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::ffi::CString;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Source `{0}` not found")]
    SourceNotFound(String),
    #[error("Index `{1:?}` out of bounds on source `{0}`")]
    IndexOutOfBounds(String, sir::IndexConst),
    #[error("Missing filter arg")]
    MissingFilterArg,
    #[error("Invalid filter arg type on `{0}`. Expected `{1}`, got `{2}`")]
    InvalidFilterArgType(String, String, String),
    #[error("Invalid filter arg value `{0}`: `{1}`")]
    InvalidFilterArgValue(String, String),
    #[error("Filter internal error: {0}")]
    FilterInternalError(String),
    #[error("Invalid output frame type")]
    InvalidOutputFrameType,
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("AV error: {0}")]
    AVError(String),
    #[error("IO error: {0}")]
    IOError(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}

#[derive(Ord, Eq, PartialEq, PartialOrd, Clone, Debug)]
pub(crate) struct SourceRef {
    video: String,
}

impl SourceRef {
    pub fn new(video: &str) -> Self {
        SourceRef {
            video: video.to_string(),
        }
    }
}

impl std::fmt::Display for SourceRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.video)
    }
}

/// A context for a spec
///
/// This is a collection of sources, filters, and arrays that are used to execute a DVE spec.
/// It's a pretty simple layout here, but if you really want to incorporate vidformer deep into a VDBMS, you
/// gut this object and make the sources and arrays backed directly through your DB.
pub struct Context {
    pub(crate) sources: BTreeMap<SourceRef, crate::source::SourceVideoStreamMeta>,
    pub(crate) filters: BTreeMap<String, Box<dyn crate::filter::Filter>>,
    pub(crate) arrays: BTreeMap<String, Box<dyn crate::array::Array>>,
}

#[derive(Debug)]

pub(crate) struct AVFrame {
    pub(crate) inner: *mut ffi::AVFrame,
}

impl AVFrame {
    pub(crate) fn clone_avframe(frame: *mut ffi::AVFrame) -> AVFrame {
        let new_frame = unsafe { ffi::av_frame_clone(frame) };
        AVFrame { inner: new_frame }
    }
}

unsafe impl Send for AVFrame {}
unsafe impl Sync for AVFrame {}

impl Drop for AVFrame {
    fn drop(&mut self) {
        unsafe { ffi::av_frame_free(&mut self.inner) }
    }
}

pub(crate) struct EmptySpecCtx;
impl crate::spec::SpecContext for EmptySpecCtx {}

impl Context {
    pub fn new(
        source_files: Vec<crate::source::SourceVideoStreamMeta>,
        arrays: BTreeMap<String, Box<dyn crate::array::Array>>,
        filters: BTreeMap<String, Box<dyn crate::filter::Filter>>,
    ) -> Context {
        let mut sources = BTreeMap::new();
        for source in source_files {
            sources.insert(SourceRef::new(&source.name), source);
        }
        Context {
            sources,
            arrays,
            filters,
        }
    }

    pub(crate) fn spec_ctx(&self) -> impl crate::spec::SpecContext {
        EmptySpecCtx
    }

    pub(crate) fn get_gop_frames(
        &self,
        source: &SourceRef,
        gop_idx: usize,
    ) -> BTreeSet<Rational64> {
        let stream_meta = &self.sources.get(source).unwrap();
        let mut out = BTreeSet::new();

        let gop_start_ts = stream_meta.keys[gop_idx];
        let next_gop_start_ts = if gop_idx >= stream_meta.keys.len() - 1 {
            None
        } else {
            Some(stream_meta.keys[gop_idx + 1])
        };

        // TODO: Use binary search to speed up
        for frame_ts in &stream_meta.ts {
            if frame_ts >= &gop_start_ts {
                if next_gop_start_ts.is_some() && frame_ts >= next_gop_start_ts.as_ref().unwrap() {
                    break;
                }
                out.insert(*frame_ts);
            }
        }

        out
    }

    pub(crate) fn resolve_frame_source(
        &self,
        frame_source: &crate::sir::FrameSource,
    ) -> Result<(SourceRef, Rational64), Error> {
        let source_name: &String = &frame_source.video;
        let sourceref = SourceRef::new(source_name);
        if let Some(source) = self.sources.get(&sourceref) {
            match &frame_source.index {
                crate::sir::IndexConst::T(t) => {
                    // make sure the timestamp exists
                    match source.ts.binary_search(t) {
                        Ok(_) => Ok((sourceref, *t)),
                        Err(_) => Err(Error::IndexOutOfBounds(
                            source_name.clone(),
                            frame_source.index.clone(),
                        )),
                    }
                }
                crate::sir::IndexConst::ILoc(k) => {
                    if let Some(t) = source.ts.get(*k) {
                        Ok((SourceRef::new(&frame_source.video), *t))
                    } else {
                        Err(Error::IndexOutOfBounds(
                            source_name.clone(),
                            frame_source.index.clone(),
                        ))
                    }
                }
            }
        } else {
            Err(Error::SourceNotFound(source_name.clone()))
        }
    }
}

#[derive(Debug)]
pub(crate) struct DecoderState {
    pub source: SourceRef,
    pub gop_idx: usize,
    pub future_frames: BTreeSet<Rational64>,
    pub past_frames: BTreeSet<Rational64>,
}

impl DecoderState {
    pub(crate) fn future_iframerefs(&self) -> BTreeSet<IFrameRef> {
        self.future_frames
            .iter()
            .map(|t| IFrameRef {
                sourceref: self.source.clone(),
                pts: *t,
            })
            .collect()
    }
}

pub(crate) fn run_decoder(
    context: &Context,
    stat: &StatRunner,
    source: &SourceRef,
    gop_idx: usize,
    pool: &Arc<(Mutex<Pool>, Condvar)>,
    decoder_id: String,
    io_runtime_handle: &tokio::runtime::Handle,
) -> Result<(), Error> {
    let stream_meta = &context.sources.get(source).unwrap();
    let stream_service = &stream_meta.service;

    let mut framesource = av::framesource::FrameSource::new(
        &stream_meta.file_path,
        stream_meta.stream_idx,
        &stream_meta.keys[gop_idx],
        stream_service,
        stream_meta.file_size,
        io_runtime_handle,
    )?;

    while framesource.next_frame()?.is_some() {
        stat.frames_decoded
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        unsafe {
            debug!(
                "DECODE - Frame (type={}, size={} bytes, pts={} key_frame={}, dts={})",
                char::from(ffi::av_get_picture_type_char((*framesource.frame).pict_type) as u8),
                (*framesource.frame).pkt_size,
                (*framesource.frame).pts,
                (*framesource.frame).key_frame,
                (*framesource.frame).coded_picture_number
            );
        }
        let frame_t =
            Rational64::new(unsafe { (*framesource.frame).pts }, 1) * framesource.time_base();
        let iframeref = IFrameRef {
            sourceref: source.clone(),
            pts: frame_t,
        };

        loop {
            let mut pool_ref = pool.0.lock();
            if pool_ref.terminate_decoders {
                return Ok(());
            }

            {
                let decoder_state = pool_ref.decoders.get_mut(&decoder_id).unwrap();
                debug_assert!(decoder_state.source == *source);
                debug_assert!(decoder_state.gop_idx == gop_idx);

                if !decoder_state.future_frames.contains(&iframeref.pts) {
                    warn!(
                        "Decoded frame pts={} not extected in {}:gop{}",
                        iframeref.pts, source, gop_idx
                    );
                    return Err(Error::AVError(format!(
                        "Decoded frame not expected, source {} gop {} not encoded correctly!",
                        source, gop_idx
                    )));
                }
            }

            if pool_ref.should_stall(&decoder_id) {
                if pool_ref.should_decoder_abandon(&decoder_id) {
                    pool_ref.decoders.remove(&decoder_id);
                    pool_ref
                        .finished_unjoined_decoders
                        .insert(decoder_id.clone());
                    pool.1.notify_all();
                    return Ok(());
                } else {
                    pool.1.wait(&mut pool_ref);
                    continue;
                }
            }

            let avframe = framesource.as_avframe();
            debug!("Adding frame {}:{} to pool", source, iframeref.pts);
            pool_ref.decoded(&decoder_id, iframeref.clone(), avframe);

            let decoder_state = pool_ref.decoders.get_mut(&decoder_id).unwrap();
            decoder_state.future_frames.remove(&iframeref.pts);
            decoder_state.past_frames.insert(iframeref.pts);

            // we've mutated the pool
            pool.1.notify_all();

            if decoder_state.future_frames.is_empty() {
                pool_ref.decoders.remove(&decoder_id);
                pool_ref
                    .finished_unjoined_decoders
                    .insert(decoder_id.clone());
                return Ok(());
            } else {
                break;
            }
        }
    }

    Err(Error::AVError("Decoder ran out of frames".to_string()))
}

struct FilterTask {
    gen: usize,
    oframe_expr: crate::sir::FrameExpr,
    dep_frames: BTreeMap<IFrameRef, Arc<AVFrame>>,
}

struct FilterTaskResult {
    gen: usize,
    oframe: Arc<AVFrame>,
}

fn run_filter(
    context: &Context,
    config: &Config,
    _stat: &StatRunner,
    input_channel: crossbeam_channel::Receiver<Option<FilterTask>>,
    output_channel: crossbeam_channel::Sender<FilterTaskResult>,
) -> Result<(), Error> {
    loop {
        match input_channel.recv() {
            Ok(Some(filter_task)) => {
                debug!("Filtering gen {}", filter_task.gen);
                let oframe = render_frame(
                    context,
                    config,
                    &filter_task.oframe_expr,
                    &filter_task.dep_frames,
                )?;

                let result = FilterTaskResult {
                    gen: filter_task.gen,
                    oframe,
                };

                output_channel.send(result).unwrap();
            }
            Ok(None) => {
                debug!("Filter recieved kill signal");
                return Ok(());
            }
            Err(crossbeam_channel::RecvError) => {
                return Err(Error::Unknown("Filter channel closed".to_string()));
            }
        }
    }
}

#[derive(Ord, Eq, PartialEq, PartialOrd, Clone, Debug)]
pub struct IFrameRef {
    pub sourceref: SourceRef,
    pub pts: Rational64,
}

/// Config for a spesific run of a vidformer spec
///
/// This is a collection of settings that are used to execute a spec.
/// This combines information about the output with internal performance knobs.
///
/// The max number of frames in memory is roughly decoders + decode_pool_size + decoder_view
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// The number of frames which can be fit in the decode pool
    pub decode_pool_size: usize,
    /// How many output frames can be active at once
    /// This also limits how many frames can be in the filters + encode buffer at once
    pub decoder_view: usize,
    /// How many decoders can be active at once
    pub decoders: usize,
    /// How many filter threads to run
    pub filterers: usize,

    pub output_width: usize,
    pub output_height: usize,
    pub output_pix_fmt: String, // needs to be an AVPixelFormat

    /// Configuration to use for output encoder
    pub encoder: Option<EncoderConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EncoderConfig {
    pub codec_name: String,

    pub opts: Vec<(String, String)>,
}

impl EncoderConfig {
    pub(crate) fn avcodec(&self) -> Result<&'static ffi::AVCodec, Error> {
        let codec_name = &self.codec_name;
        let codec_name_cstr = CString::new(codec_name.clone()).unwrap();

        let av_codec =
            unsafe { ffi::avcodec_find_encoder_by_name(codec_name_cstr.as_ptr()).as_ref() };
        match av_codec {
            Some(av_codec) => Ok(av_codec),
            None => Err(Error::ConfigError(format!(
                "Failed to find encoder for `{}` codec",
                codec_name
            ))),
        }
    }
}

impl Config {
    pub(crate) fn expected_output_type(&self) -> filter::FrameType {
        filter::FrameType {
            width: self.output_width,
            height: self.output_height,
            format: util::pixel_fmt_str_to_av_pix_fmt(self.output_pix_fmt.as_str()).unwrap(),
        }
    }
}

/// Stats from a run of a vidformer spec
#[derive(Debug, Serialize)]
pub struct Stats {
    pub max_decoder_count: usize,
    pub max_encode_buffer_size: usize,
    pub decoders_created: usize,
    pub frames_written: usize,
    pub frames_decoded: usize,
    pub runtime: std::time::Duration,
}

pub(crate) struct StatRunner {
    max_decoder_count: std::sync::atomic::AtomicUsize,
    max_encode_buffer_size: std::sync::atomic::AtomicUsize,
    decoders_created: std::sync::atomic::AtomicUsize,
    frames_written: std::sync::atomic::AtomicUsize,
    frames_decoded: std::sync::atomic::AtomicUsize,
    start_time: std::time::Instant,
}

impl StatRunner {
    fn new() -> Self {
        StatRunner {
            max_decoder_count: std::sync::atomic::AtomicUsize::new(0),
            max_encode_buffer_size: std::sync::atomic::AtomicUsize::new(0),
            decoders_created: std::sync::atomic::AtomicUsize::new(0),
            frames_written: std::sync::atomic::AtomicUsize::new(0),
            frames_decoded: std::sync::atomic::AtomicUsize::new(0),
            start_time: std::time::Instant::now(),
        }
    }

    fn stats(&self) -> Stats {
        Stats {
            max_decoder_count: self
                .max_decoder_count
                .load(std::sync::atomic::Ordering::SeqCst),
            max_encode_buffer_size: self
                .max_encode_buffer_size
                .load(std::sync::atomic::Ordering::SeqCst),
            decoders_created: self
                .decoders_created
                .load(std::sync::atomic::Ordering::SeqCst),
            frames_written: self
                .frames_written
                .load(std::sync::atomic::Ordering::SeqCst),
            frames_decoded: self
                .frames_decoded
                .load(std::sync::atomic::Ordering::SeqCst),
            runtime: std::time::Instant::now() - self.start_time,
        }
    }
}

pub(crate) fn type_frame(
    context: &Context,
    _config: &Config,
    frame: &crate::sir::FrameExpr,
) -> Result<filter::FrameType, Error> {
    match frame {
        crate::sir::FrameExpr::Source(s) => {
            let (sourceref, _t) = context.resolve_frame_source(s)?;
            let source = context.sources.get(&sourceref).unwrap();
            Ok(filter::FrameType {
                width: source.resolution.0,
                height: source.resolution.1,
                format: util::pixel_fmt_str_to_av_pix_fmt(source.pix_fmt.as_str()).unwrap(),
            })
        }
        crate::sir::FrameExpr::Filter(f) => {
            let filter = context.filters.get(&f.name);

            if filter.is_none() {
                return Err(Error::Unknown(format!("Filter `{}` not found", f.name)));
            }
            let filter = filter.unwrap();

            let mut args = Vec::new();
            let mut kwargs = BTreeMap::new();

            for arg in &f.args {
                match arg {
                    crate::sir::Expr::Frame(frame) => {
                        let frame = type_frame(context, _config, frame)?;
                        args.push(filter::ValType::Frame(frame));
                    }
                    crate::sir::Expr::Data(data) => {
                        args.push(crate::filter::ValType::from_expr(data, context));
                    }
                }
            }
            for (k, v) in &f.kwargs {
                match v {
                    crate::sir::Expr::Frame(frame) => {
                        let frame = type_frame(context, _config, frame)?;
                        kwargs.insert(k.clone(), filter::ValType::Frame(frame));
                    }
                    crate::sir::Expr::Data(data) => {
                        kwargs.insert(k.clone(), crate::filter::ValType::from_expr(data, context));
                    }
                }
            }

            Ok(filter.filter_type(&args, &kwargs)?)
        }
    }
}

fn render_frame(
    context: &Context,
    _config: &Config,
    frame: &crate::sir::FrameExpr,
    loaded_frames: &BTreeMap<IFrameRef, Arc<AVFrame>>,
) -> Result<Arc<AVFrame>, Error> {
    info!("Rendering frame {}", frame);
    match frame {
        crate::sir::FrameExpr::Source(s) => {
            let (source_ref, t) = context.resolve_frame_source(s)?;
            let frame_ref = IFrameRef {
                sourceref: source_ref,
                pts: t,
            };
            let frame = loaded_frames.get(&frame_ref).unwrap();
            Ok(frame.clone())
        }
        crate::sir::FrameExpr::Filter(f) => {
            let filter = context.filters.get(&f.name).unwrap();
            let mut args = Vec::new();
            let mut kwargs = BTreeMap::new();

            // TODO: Very easy to parallelize these two loops
            for arg in &f.args {
                match arg {
                    crate::sir::Expr::Frame(frame) => {
                        let frame = render_frame(context, _config, frame, loaded_frames)?;
                        args.push(crate::filter::Val::Frame(Frame::new_arc(frame)));
                    }
                    crate::sir::Expr::Data(data) => {
                        args.push(crate::filter::Val::from_expr(data, context));
                    }
                }
            }
            for (k, v) in &f.kwargs {
                match v {
                    crate::sir::Expr::Frame(frame) => {
                        let frame = render_frame(context, _config, frame, loaded_frames)?;
                        kwargs.insert(k.clone(), crate::filter::Val::Frame(Frame::new_arc(frame)));
                    }
                    crate::sir::Expr::Data(data) => {
                        kwargs.insert(k.clone(), crate::filter::Val::from_expr(data, context));
                    }
                }
            }

            let oframe = filter.filter(&args, &kwargs)?;

            Ok(oframe.into_avframe())
        }
    }
}

/// Select whether the output timestamps are local to the ranged segment or the entire output
///
/// - `SegmentLocal`: The output timestamps are in the same timebase as the ranged segment. Used for VOD.
/// - `StreamLocal`: The output timestamps are in the same timebase as the entire output.
#[derive(Clone)]
pub enum RangeTsFormat {
    SegmentLocal,
    StreamLocal,
}

/// A range spesifier
///
/// `start` and `end` are the start and end of the range in the timebase of the input, inclusive.
#[derive(Clone)]
pub struct Range {
    pub start: Rational64,
    pub end: Rational64,
    pub ts_format: RangeTsFormat,
}

struct EncodeBuffer {
    members: Vec<(usize, Arc<AVFrame>)>,
    terminate_encoder: bool,
}

impl EncodeBuffer {
    fn new() -> Self {
        EncodeBuffer {
            members: Vec::new(),
            terminate_encoder: false,
        }
    }
}

struct ExecContext {
    output_path: String,
    context: Arc<Context>,
    config: Arc<Config>,

    stat: Arc<StatRunner>,
    process_span: Arc<sir::ProcessSpan>,

    pool: Arc<(Mutex<Pool>, Condvar)>,
    output_time_base: num_rational::Ratio<i64>,

    to_filter_channel: (
        crossbeam_channel::Sender<Option<FilterTask>>,
        crossbeam_channel::Receiver<Option<FilterTask>>,
    ),
    from_filter_channel: (
        crossbeam_channel::Sender<FilterTaskResult>,
        crossbeam_channel::Receiver<FilterTaskResult>,
    ),

    decoder_count: Arc<AtomicI64>,
    io_runtime: tokio::runtime::Runtime,

    filtering_gens: BTreeSet<usize>,
    frames_post_filtering: usize,

    encode_buffer: Arc<(Mutex<EncodeBuffer>, Condvar)>,
    dec_join_handles: BTreeMap<String, std::thread::JoinHandle<Result<(), Error>>>,
    filter_join_handles: Vec<std::thread::JoinHandle<Result<(), Error>>>,
}

impl ExecContext {
    fn run(mut self) -> Result<Stats, Error> {
        let enc_thread = {
            let encode_buffer = self.encode_buffer.clone();
            let output_path = self.output_path.to_string();
            let config = self.config.clone();
            let process_span = self.process_span.clone();
            let stat = self.stat.clone();
            let output_time_base = self.output_time_base;

            std::thread::spawn(move || {
                let r = encoder_thread(
                    config,
                    stat,
                    output_time_base,
                    output_path,
                    process_span,
                    encode_buffer,
                );

                debug!("Enc thread ended");
                r
            })
        };

        for _i in 0..self.config.filterers {
            let context = self.context.clone();
            let config = self.config.clone();
            let stat = self.stat.clone();
            let reciever = self.to_filter_channel.1.clone();
            let sender = self.from_filter_channel.0.clone();
            let filter_thread =
                std::thread::spawn(move || run_filter(&context, &config, &stat, reciever, sender));
            self.filter_join_handles.push(filter_thread);
        }

        let mut return_err: Option<Error> = Option::None;

        'control_loop: loop {
            debug_assert!(return_err.is_none());

            // Create new decoders
            let r = self.create_new_decoders();
            if let Err(e) = r {
                return_err = Some(e);
                break 'control_loop;
            }

            // Join finished decoders
            let r = self.join_finished_decoders();
            match r {
                Ok(request_early_stop) if request_early_stop => break,
                Ok(_) => {}
                Err(e) => {
                    return_err = Some(e);
                    break 'control_loop;
                }
            }
            // Create filters
            let r = self.send_to_filters();
            if let Err(e) = r {
                return_err = Some(e);
                break 'control_loop;
            }

            // Process filtered frames
            let r = self.recieve_filtered_frames();
            if let Err(e) = r {
                return_err = Some(e);
                break 'control_loop;
            }

            // Check if encoder finished
            {
                if enc_thread.is_finished() {
                    // This means the encoder had an error and we should stop
                    break;
                }
            }

            // Check if any filter threads panicked
            {
                let finished_i = self
                    .filter_join_handles
                    .iter()
                    .position(|h| h.is_finished());
                // for filter_join_handle in self.filter_join_handles.iter() {
                //     if filter_join_handle.is_finished() {
                //         return_err = Some(Error::Unknown("Filter thread panicked".to_string()));
                //         break 'control_loop;
                //     }
                // }
                if let Some(finished_i) = finished_i {
                    let thread_return = self.filter_join_handles.remove(finished_i).join();

                    match thread_return {
                        Ok(Ok(_)) => {
                            return_err =
                                Some(Error::Unknown("Filter thread finished early?".to_string()));
                            break 'control_loop;
                        }
                        Ok(Err(e)) => {
                            return_err = Some(e);
                            break 'control_loop;
                        }
                        Err(e) => {
                            return_err = Some(Error::Unknown(format!(
                                "Filter thread {} panicked: {:?}",
                                finished_i, e
                            )));
                            break 'control_loop;
                        }
                    }
                }
            }

            if self.frames_post_filtering == self.process_span.frames.len() {
                break;
            }

            std::thread::sleep(std::time::Duration::from_micros(250));
        }

        // If we are here either all frames have been filtered or we have an error
        debug!("Main exec loop finished");

        // Wait for all decoders to finish
        {
            let mut pool_ref = self.pool.0.lock();
            pool_ref.terminate_decoders = true;
            self.pool.1.notify_all();
        }

        while let Some((_decoder_id, dec_join_handle)) = self.dec_join_handles.pop_first() {
            match dec_join_handle.join() {
                Ok(decoder_result) => {
                    match decoder_result {
                        Ok(_) => {} // Thread finished successfully
                        Err(e) => {
                            if return_err.is_none() {
                                return_err = Some(e);
                            }
                        }
                    }
                }
                Err(e) => {
                    if return_err.is_none() {
                        return_err =
                            Some(Error::Unknown(format!("Decoder thread panicked: {:?}", e)));
                    }
                    break;
                }
            }
        }
        debug!("All decoders finished");

        // Wait for all filters to finish
        for _i in 0..self.config.filterers {
            self.to_filter_channel.0.send(None).unwrap();
        }
        while let Some(filter_join_handle) = self.filter_join_handles.pop() {
            match filter_join_handle.join() {
                Ok(filter_result) => {
                    match filter_result {
                        Ok(_) => {} // Thread finished successfully
                        Err(e) => {
                            if return_err.is_none() {
                                return_err = Some(e);
                            }
                        }
                    }
                }
                Err(e) => {
                    if return_err.is_none() {
                        return_err =
                            Some(Error::Unknown(format!("Filter thread panicked: {:?}", e)));
                    }
                    break;
                }
            }
        }
        debug!("All filters finished");

        // Wait for encoder to finish
        if return_err.is_some() {
            let mut encode_buffer_ref = self.encode_buffer.0.lock();
            // If we have an error kill the encoder now
            encode_buffer_ref.terminate_encoder = true;
            self.encode_buffer.1.notify_one();
        }
        match enc_thread.join() {
            Ok(enc_result) => {
                match enc_result {
                    Ok(_) => {} // Thread finished successfully
                    Err(e) => {
                        assert!(return_err.is_none());
                        return_err = Some(e);
                    }
                }
            }
            Err(e) => {
                assert!(return_err.is_none());
                return_err = Some(Error::Unknown(format!("Encoder thread panicked: {:?}", e)));
            }
        }
        debug!("Encoder finished");

        match return_err {
            Some(e) => Err(e),
            None => Ok(self.stat.stats()),
        }
    }

    fn recieve_filtered_frames(&mut self) -> Result<(), Error> {
        loop {
            match self.from_filter_channel.1.try_recv() {
                Ok(result) => {
                    debug!("Recieved gen {} from filter", result.gen);
                    debug_assert!(self.filtering_gens.contains(&result.gen));
                    self.filtering_gens.remove(&result.gen);

                    let oframe = result.oframe;
                    let gen = result.gen;

                    {
                        let mut pool_ref = self.pool.0.lock();
                        debug_assert!(pool_ref.is_gen_ready(gen));
                        pool_ref.finish_gen(gen);
                        self.pool.1.notify_all();
                    }

                    {
                        let mut encode_buffer_ref = self.encode_buffer.0.lock();
                        encode_buffer_ref.members.push((gen, oframe));
                        let encode_buffer_size = encode_buffer_ref.members.len();
                        self.stat
                            .max_encode_buffer_size
                            .fetch_max(encode_buffer_size, Ordering::SeqCst);

                        self.encode_buffer.1.notify_one();
                    }

                    self.frames_post_filtering += 1;
                }
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    return Ok(());
                }
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    return Err(Error::Unknown("Filter channel closed".to_string()));
                }
            }
        }
    }

    fn join_finished_decoders(&mut self) -> Result<bool, Error> {
        let mut pool_ref = self.pool.0.lock();
        while let Some(finished_decoder_id) = pool_ref.finished_unjoined_decoders.pop_first() {
            let dec_join_handle = self.dec_join_handles.remove(&finished_decoder_id).unwrap();
            match dec_join_handle.join() {
                Ok(decoder_result) => {
                    match decoder_result {
                        Ok(_) => {} // Thread finished successfully
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    return Err(Error::Unknown(format!("Decoder thread panicked: {:?}", e)));
                }
            }
            self.pool.1.notify_all();
        }

        // now that we've removed all decoders which reported themselves as finished check for any panicked decoders
        for (decoder_id, dec_join_handle) in self.dec_join_handles.iter() {
            if dec_join_handle.is_finished() {
                info!(
                    "Found a unexpected finished decoder thread: {:?}",
                    decoder_id
                );
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn create_new_decoders(&mut self) -> Result<(), Error> {
        if self.decoder_count.load(Ordering::SeqCst) < self.config.decoders as i64 {
            debug_assert!(self.decoder_count.load(Ordering::SeqCst) >= 0);
            let mut pool_ref = self.pool.0.lock();
            let new_decoder = pool_ref.new_decoder_gop();
            if let Some((sourceref, gop_idx)) = new_decoder {
                let source = sourceref.clone();
                let decoder_id = crate::util::rand_uuid();
                let decooder_id_join_handle_copy = decoder_id.clone();
                let pool = self.pool.clone();
                let context = self.context.clone();
                let stat = self.stat.clone();
                let io_runtime_handle = self.io_runtime.handle().clone();

                let num_decoders = self.decoder_count.fetch_add(1, Ordering::SeqCst) + 1;
                stat.max_decoder_count
                    .fetch_max(num_decoders as usize, Ordering::SeqCst);
                stat.decoders_created.fetch_add(1, Ordering::SeqCst);
                let decoder_count = self.decoder_count.clone();

                let decoder_state = DecoderState {
                    source: source.clone(),
                    gop_idx,
                    future_frames: context.get_gop_frames(&source, gop_idx),
                    past_frames: BTreeSet::new(),
                };
                pool_ref.decoders.insert(decoder_id.clone(), decoder_state);
                self.pool.1.notify_all();

                debug!("Creating decoder {decoder_id} on {source}:gop{gop_idx}");

                let dec_join_handle = std::thread::Builder::new()
                    .name("decoder".to_string())
                    .spawn(move || {
                        let decoder_result = run_decoder(
                            &context,
                            &stat,
                            &source,
                            gop_idx,
                            &pool,
                            decoder_id.clone(),
                            &io_runtime_handle,
                        );
                        // TODO: Handle error
                        decoder_count.fetch_add(-1, Ordering::SeqCst);
                        debug!("Decoder {decoder_id} finished");
                        decoder_result
                    })
                    .unwrap();

                self.dec_join_handles
                    .insert(decooder_id_join_handle_copy, dec_join_handle);
            }
        }

        Ok(())
    }

    fn send_to_filters(&mut self) -> Result<(), Error> {
        loop {
            let gen_to_filter = {
                let mut out = None;
                let pool_ref = self.pool.0.lock();
                for active_gen in pool_ref.active_gens() {
                    if !self.filtering_gens.contains(&active_gen)
                        && pool_ref.is_gen_ready(active_gen)
                    {
                        out = Some(active_gen);
                        break;
                    }
                }
                out
            };

            if gen_to_filter.is_none() {
                break;
            }
            let gen_to_filter = gen_to_filter.unwrap();
            self.filtering_gens.insert(gen_to_filter);

            let frame_deps = {
                let pool_ref = self.pool.0.lock();
                pool_ref.get_ready_gen_frames(gen_to_filter)
            };

            let filter_task = FilterTask {
                gen: gen_to_filter,
                oframe_expr: self.process_span.frames[gen_to_filter].clone(),
                dep_frames: frame_deps,
            };

            debug!("Sending gen {} to filter", gen_to_filter);

            self.to_filter_channel.0.send(Some(filter_task)).unwrap();
        }

        Ok(())
    }
}

/// Execute a vidformer spec
pub fn run_spec(
    spec: &Arc<Box<dyn Spec>>,
    output_path: &str,
    context: &Arc<Context>,
    config: &Arc<Config>,
    range: &Option<Range>,
) -> Result<Stats, Error> {
    if config.decoders > u16::MAX as usize {
        // yes this is arbitrary, but bad things happen if do something like usize::MAX because we track counts with a i64 internally
        return Err(Error::ConfigError(
            "Decoders must be less than u16::MAX".to_string(),
        ));
    }

    // Make sure the encoder config is valid
    if let Some(enc_cfg) = &config.encoder {
        let _avcodec: &ffi::AVCodec = enc_cfg.avcodec()?;
    }

    let stat = Arc::new(StatRunner::new());
    let expected_output_type = config.expected_output_type();

    let process_span = Arc::new(crate::sir::ProcessSpan::create(
        spec.as_ref().as_ref(),
        context,
        range,
    ));

    // Type check frames
    for oframe in &process_span.frames {
        let frame_type = type_frame(context, config, oframe)?;
        if frame_type != expected_output_type {
            return Err(Error::InvalidOutputFrameType);
        }
    }

    let (pool, output_time_base) = build_pool(&process_span, config, context)?;

    let pool = Arc::new((Mutex::new(pool), Condvar::new()));

    let decoder_count = Arc::new(AtomicI64::new(0));

    let filtering_gens: BTreeSet<usize> = BTreeSet::new();
    let encode_buffer = Arc::new((Mutex::new(EncodeBuffer::new()), Condvar::new()));
    let dec_join_handles: BTreeMap<String, std::thread::JoinHandle<Result<(), Error>>> =
        BTreeMap::new();
    let filter_join_handles: Vec<std::thread::JoinHandle<Result<(), Error>>> = Vec::new();

    let io_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2.max(config.decoders.sqrt())) // guess a good number of threads to use for I/O
        .enable_all()
        .build()
        .unwrap();

    let exec_contex = ExecContext {
        output_path: output_path.to_string(),
        context: context.clone(),
        config: config.clone(),
        stat: stat.clone(),
        process_span,
        pool,
        output_time_base,
        to_filter_channel: crossbeam_channel::unbounded(),
        from_filter_channel: crossbeam_channel::unbounded(),
        decoder_count,
        io_runtime,
        filtering_gens,
        frames_post_filtering: 0,
        encode_buffer,
        dec_join_handles,
        filter_join_handles,
    };

    exec_contex.run()
}

fn encoder_thread(
    config: Arc<Config>,
    stat: Arc<StatRunner>,
    output_time_base: num_rational::Ratio<i64>,
    output_path: String,
    process_span: Arc<sir::ProcessSpan>,
    encode_buffer: Arc<(Mutex<EncodeBuffer>, Condvar)>,
) -> Result<(), Error> {
    let mut encoder = av::encoder::Encoder::new(&config, &output_time_base)?;

    let mut encoder_codec_params = unsafe { ffi::avcodec_parameters_alloc() };
    // copy from encoder context
    if unsafe { ffi::avcodec_parameters_from_context(encoder_codec_params, encoder.codec_ctx) } < 0
    {
        return Err(Error::AVError(
            "Encoder failed to copy codec params to codec context".to_string(),
        ));
    }

    let mut muxer = av::muxer::Muxer::new(&output_path, encoder_codec_params, &output_time_base)?;
    let muxer_time_base = crate::util::avrat_to_rat(&muxer.out_time_base);
    let encoder_to_muxer_ts_multiplier: num_rational::Ratio<i64> =
        encoder.time_base / muxer_time_base;

    if *encoder_to_muxer_ts_multiplier.denom() != 1 {
        return Err(Error::AVError(format!(
            "Can't operate with encoder timebase {} and muxer timebase {}. Are you using mkv?",
            encoder.time_base, muxer_time_base
        )));
    }

    let mut oframe_next = 0;

    loop {
        // Encode frames
        {
            if oframe_next >= process_span.frames.len() {
                break;
            }

            let mut encode_buffer_ref = encode_buffer.0.lock();

            if encode_buffer_ref.terminate_encoder {
                break;
            }

            let target_index = encode_buffer_ref
                .members
                .iter()
                .position(|(gen, _)| *gen == oframe_next);

            if let Some(target_index) = target_index {
                let (gen, frame) = encode_buffer_ref.members.remove(target_index);

                let pts = match process_span.output_ts_offset {
                    Some(offset) => process_span.ts[gen] - offset,
                    None => process_span.ts[gen],
                };
                encoder.encode(&pts, &frame)?;
                oframe_next += 1;
            } else {
                encode_buffer.1.wait(&mut encode_buffer_ref);
                continue;
            }
        }

        // Mux frames
        if let Some(packet) = encoder.get_packet() {
            unsafe {
                (*packet).pts *= *encoder_to_muxer_ts_multiplier.numer();
                (*packet).dts *= *encoder_to_muxer_ts_multiplier.numer();
            }

            stat.frames_written
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            muxer.mux_packet(packet)?;
        }
    }

    encoder.flush()?;

    // Mux frames
    while let Some(packet) = encoder.get_packet() {
        unsafe {
            (*packet).pts *= *encoder_to_muxer_ts_multiplier.numer();
            (*packet).dts *= *encoder_to_muxer_ts_multiplier.numer();
        }

        stat.frames_written
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        muxer.mux_packet(packet)?;
    }

    encoder.close();
    muxer.close()?;

    unsafe {
        ffi::avcodec_parameters_free(&mut encoder_codec_params);
    }

    Ok(())
}

fn build_pool(
    process_span: &sir::ProcessSpan,
    config: &Arc<Config>,
    context: &Arc<Context>,
) -> Result<(Pool, num_rational::Ratio<i64>), Error> {
    let mut iframes_per_oframe: Vec<BTreeSet<IFrameRef>> =
        Vec::<BTreeSet<IFrameRef>>::with_capacity(process_span.frames.len());
    let mut iframe_refs_in_out_idx: BTreeMap<IFrameRef, BTreeSet<usize>> = BTreeMap::new();

    for (oframe_i, oframe) in process_span.frames.iter().enumerate() {
        let mut frame_deps = std::collections::BTreeSet::new();
        oframe.add_deps(&mut frame_deps);
        assert!(
            frame_deps.len() <= config.decode_pool_size,
            "OFrame {} has too many dependencies ({}) for decode pool size {}",
            oframe_i,
            frame_deps.len(),
            config.decode_pool_size
        );

        let mut iframe_refs = BTreeSet::new();
        for dep in &frame_deps {
            let (source_ref, t) = context.resolve_frame_source(dep)?;

            let iframe_ref = IFrameRef {
                sourceref: source_ref.clone(),
                pts: t,
            };
            iframe_refs.insert(iframe_ref.clone());

            iframe_refs_in_out_idx
                .entry(iframe_ref)
                .or_default()
                .insert(oframe_i);
        }
        iframes_per_oframe.push(iframe_refs);
    }
    debug_assert_eq!(iframes_per_oframe.len(), process_span.frames.len());

    let output_time_base = {
        let mut lcm = *process_span.ts[0].denom();
        for t in &process_span.ts {
            lcm = num::integer::lcm(lcm, *t.denom());
        }
        Rational64::new(1, lcm)
    };

    let pool = crate::pool::Pool::new(
        iframes_per_oframe,
        iframe_refs_in_out_idx,
        context.clone(),
        config.clone(),
    )?;

    Ok((pool, output_time_base))
}

/// Create [HLS](https://en.wikipedia.org/wiki/HTTP_Live_Streaming) artifacts for a spec
///
/// * `spec` - The spec to create HLS artifacts for
/// * `host_prefix` - The prefix to use for the host (e.g., `https://example.com:8000`)
/// * `context` - The context to use for the spec
/// * `config` - The config to use for the spec
///
/// Returns a tuple of:
/// * The namespace for the HLS artifacts
/// * The text of the m3u8 playlist file
/// * The text of the m3u8 stream file
/// * A vector of tuples of the start and end timestamps of each segment
pub fn create_spec_hls(
    spec: &dyn Spec,
    host_prefix: &str,
    context: &Context,
    _config: &Config,
) -> (String, String, String, Vec<(Rational64, Rational64)>) {
    let namespace = format!("vod-{}", crate::util::rand_uuid());

    let _playlist_path = format!("{namespace}/playlist.m3u8");
    let playlist_text = format!(
        "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=640000\n{}/{}/stream.m3u8\n",
        host_prefix, namespace
    );

    let stream_ts = sir::spec_domain(spec, context);
    let frame_rate = crate::spec::get_framerate(spec);
    let segment_duration = 2;

    let mut stream_text = format!("#EXTM3U\n#EXT-X-PLAYLIST-TYPE:VOD\n#EXT-X-TARGETDURATION:{segment_duration}\n#EXT-X-VERSION:4\n#EXT-X-MEDIA-SEQUENCE:0\n");
    let mut ranges = Vec::new();

    // todo fix off-by-one error
    for segment_id in 0..=(stream_ts.len() - 1) / (frame_rate * segment_duration) {
        let stream_ts_path = format!("segment-{segment_id}.ts");
        stream_text +=
            &format!("#EXTINF:{segment_duration}.0,\n{host_prefix}/{namespace}/{stream_ts_path}\n");

        let range_start = stream_ts[segment_id * frame_rate * segment_duration];
        let range_end = if (segment_id + 1) * frame_rate * segment_duration - 1 < stream_ts.len() {
            stream_ts[(segment_id + 1) * frame_rate * segment_duration - 1]
        } else {
            stream_ts[stream_ts.len() - 1]
        };

        ranges.push((range_start, range_end));
    }

    stream_text += "#EXT-X-ENDLIST\n";
    let _stream_path = format!("{namespace}/stream.m3u8");

    (namespace, playlist_text, stream_text, ranges)
}
