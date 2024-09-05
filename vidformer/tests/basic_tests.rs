use num_rational::Rational64;
use std::collections::BTreeMap;
use vidformer::{sir::FrameSource, *};

macro_rules! test_output_path {
    ($name:ident) => {
        &format!("/tmp/{}.mp4", stringify!($name))
    };
}

// tests write a lot of specs directly in the AST
// let's create some macros to make it easier:

macro_rules! int {
    ($i:expr) => {
        sir::Expr::Data(sir::DataExpr::Int($i))
    };
}

// macro_rules! string {
//     ($s:expr) => {
//         sir::Expr::Data(sir::DataExpr::String($s.to_string()))
//     };
// }

// macro_rules! bool {
//     ($b:expr) => {
//         sir::Expr::Data(sir::DataExpr::Bool($b))
//     };
// }

macro_rules! frame {
    ($video:expr, $index:expr) => {
        sir::Expr::Frame(sir::FrameExpr::Source(FrameSource::new(
            $video.to_string(),
            $index,
        )))
    };
}

macro_rules! iloc_index {
    ($i:expr) => {
        sir::IndexConst::ILoc($i)
    };
}

// macro_rules! t_index {
//     ($t:expr) => {
//         sir::IndexConst::T($t)
//     };
// }

macro_rules! filter {
    ($name:ident; $($arg:expr),*; $($key:ident = $value:expr),*) => {
        {
            let args: Vec<sir::Expr> = vec![$($arg),*];
            let mut kwargs: BTreeMap<String, sir::Expr> = BTreeMap::new();
            $(
                kwargs.insert(stringify!($key).to_string(), $value);
            )*
            sir::FrameExpr::Filter(sir::FilterExpr {
                name: stringify!($name).to_string(),
                args,
                kwargs,
            })
        }
    };
}

#[test]
fn test_placeholder() {
    struct MySpec {}
    impl spec::Spec for MySpec {
        fn domain(&self, _context: &dyn spec::SpecContext) -> Vec<num_rational::Rational64> {
            (0..24 * 3).map(|i| Rational64::new(i, 24)).collect()
        }

        fn render(
            &self,
            _context: &dyn spec::SpecContext,
            _t: &num_rational::Rational64,
        ) -> sir::FrameExpr {
            filter!(PlaceholderFrame; ; width=int!(1920), height=int!(1080))
        }
    }

    let mut filters: BTreeMap<String, Box<dyn filter::Filter>> = BTreeMap::new();
    filters.insert(
        "PlaceholderFrame".to_string(),
        Box::new(crate::filter::builtin::PlaceholderFrame {}),
    );

    let sources = vec![];
    let arrays = BTreeMap::new();
    let context: vidformer::Context = vidformer::Context::new(sources, arrays, filters);

    let dve_config = vidformer::Config {
        decode_pool_size: 10,
        decoder_view: usize::MAX,
        decoders: u16::MAX as usize,
        filterers: 4,

        output_width: 1920,
        output_height: 1080,
        output_pix_fmt: "yuv420p".to_string(),

        encoder: None,
        format: None,
    };

    let spec: Box<dyn spec::Spec> = Box::new(MySpec {});

    let output_path = test_output_path!(test_placeholder);

    let spec = std::sync::Arc::new(spec);
    let context = std::sync::Arc::new(context);
    let dve_config = std::sync::Arc::new(dve_config);
    let stats = run_spec(&spec, output_path, &context, &dve_config, &None).unwrap();
    assert_eq!(stats.max_decoder_count, 0);
    assert_eq!(stats.frames_written, 24 * 3);

    assert!(std::path::Path::new(output_path).exists());
}

#[test]
fn test_bad_resolution() {
    struct MySpec {}
    impl spec::Spec for MySpec {
        fn domain(&self, _context: &dyn spec::SpecContext) -> Vec<num_rational::Rational64> {
            (0..10).map(|i| Rational64::new(i, 24)).collect()
        }

        fn render(
            &self,
            _context: &dyn spec::SpecContext,
            _t: &num_rational::Rational64,
        ) -> sir::FrameExpr {
            filter!(PlaceholderFrame; ; width=int!(1280), height=int!(1080)) // 1280 instead of 1920
        }
    }

    let mut filters: BTreeMap<String, Box<dyn filter::Filter>> = BTreeMap::new();
    filters.insert(
        "PlaceholderFrame".to_string(),
        Box::new(crate::filter::builtin::PlaceholderFrame {}),
    );

    let sources = vec![];
    let arrays = BTreeMap::new();
    let context: vidformer::Context = vidformer::Context::new(sources, arrays, filters);

    let dve_config = vidformer::Config {
        decode_pool_size: 10,
        decoder_view: usize::MAX,
        decoders: u16::MAX as usize,
        filterers: 4,

        output_width: 1920,
        output_height: 1080,
        output_pix_fmt: "yuv420p".to_string(),

        encoder: None,
        format: None,
    };

    let spec: Box<dyn spec::Spec> = Box::new(MySpec {});

    let spec = std::sync::Arc::new(spec);
    let context = std::sync::Arc::new(context);
    let dve_config = std::sync::Arc::new(dve_config);
    assert!(matches!(
        run_spec(
            &spec,
            test_output_path!(test_bad_resolution),
            &context,
            &dve_config,
            &None
        ),
        Err(Error::InvalidOutputFrameType)
    ));
}

#[test]
fn test_non_existant_source() {
    struct MySpec {}
    impl spec::Spec for MySpec {
        fn domain(&self, _context: &dyn spec::SpecContext) -> Vec<num_rational::Rational64> {
            (0..10).map(|i| Rational64::new(i, 24)).collect()
        }

        fn render(
            &self,
            _context: &dyn spec::SpecContext,
            _t: &num_rational::Rational64,
        ) -> sir::FrameExpr {
            filter!(PlaceholderFrame; frame!("non-existant-source", iloc_index!(0)) ; width=int!(1920), height=int!(1080))
        }
    }

    let mut filters: BTreeMap<String, Box<dyn filter::Filter>> = BTreeMap::new();
    filters.insert(
        "PlaceholderFrame".to_string(),
        Box::new(crate::filter::builtin::PlaceholderFrame {}),
    );

    let sources = vec![];
    let arrays = BTreeMap::new();
    let context: vidformer::Context = vidformer::Context::new(sources, arrays, filters);

    let dve_config = vidformer::Config {
        decode_pool_size: 10,
        decoder_view: usize::MAX,
        decoders: u16::MAX as usize,
        filterers: 4,

        output_width: 1920,
        output_height: 1080,
        output_pix_fmt: "yuv420p".to_string(),

        encoder: None,
        format: None,
    };

    let spec: Box<dyn spec::Spec> = Box::new(MySpec {});

    let spec = std::sync::Arc::new(spec);
    let context = std::sync::Arc::new(context);
    let dve_config = std::sync::Arc::new(dve_config);

    let ret = run_spec(
        &spec,
        test_output_path!(test_non_existant_source),
        &context,
        &dve_config,
        &None,
    );
    dbg!(&ret);
    assert!(matches!(ret, Err(Error::SourceNotFound(_))));
}

#[test]
fn test_no_source_file() {
    struct MySpec {}
    impl spec::Spec for MySpec {
        fn domain(&self, _context: &dyn spec::SpecContext) -> Vec<num_rational::Rational64> {
            (0..1).map(|i| Rational64::new(i, 1)).collect()
        }

        fn render(
            &self,
            _context: &dyn spec::SpecContext,
            t: &num_rational::Rational64,
        ) -> sir::FrameExpr {
            sir::FrameExpr::Source(sir::FrameSource::new(
                "gone".to_string(),
                sir::IndexConst::T(*t),
            ))
        }
    }

    let mut filters: BTreeMap<String, Box<dyn filter::Filter>> = BTreeMap::new();
    filters.insert(
        "PlaceholderFrame".to_string(),
        Box::new(crate::filter::builtin::PlaceholderFrame {}),
    );

    let sources = vec![source::SourceVideoStreamMeta {
        name: "gone".to_string(),
        codec: "h264".to_string(),
        stream_idx: 0,
        service: vidformer::service::Service::default(),
        file_size: 4 * 1024 * 1024,
        resolution: (1920, 1080),
        pix_fmt: "yuv420p".to_string(),
        ts: vec![Rational64::new(0, 1)],
        keys: vec![Rational64::new(0, 1)],
        file_path: "something_fake.mp4".to_string(),
    }];
    let arrays = BTreeMap::new();
    let context: vidformer::Context = vidformer::Context::new(sources, arrays, filters);

    let dve_config = vidformer::Config {
        decode_pool_size: 10,
        decoder_view: usize::MAX,
        decoders: u16::MAX as usize,
        filterers: 4,

        output_width: 1920,
        output_height: 1080,
        output_pix_fmt: "yuv420p".to_string(),

        encoder: None,
        format: None,
    };

    let spec: Box<dyn spec::Spec> = Box::new(MySpec {});

    let spec = std::sync::Arc::new(spec);
    let context = std::sync::Arc::new(context);
    let dve_config = std::sync::Arc::new(dve_config);

    let ret = run_spec(
        &spec,
        test_output_path!(test_non_existant_source),
        &context,
        &dve_config,
        &None,
    );
    dbg!(&ret);
    assert!(matches!(ret, Err(Error::IOError(_))));
    assert_eq!(
        ret.unwrap_err().to_string(),
        "IO error: File `something_fake.mp4` not found"
    );
}

const NUM_FRAMES: i64 = 30 * 24;
struct ClipSpec {
    num_frames: i64,
}
impl spec::Spec for ClipSpec {
    fn domain(&self, _context: &dyn spec::SpecContext) -> Vec<num_rational::Rational64> {
        (0..self.num_frames)
            .map(|i| Rational64::new(i, 24))
            .collect()
    }

    fn render(
        &self,
        _context: &dyn spec::SpecContext,
        t: &num_rational::Rational64,
    ) -> sir::FrameExpr {
        sir::FrameExpr::Source(sir::FrameSource::new(
            "tos".to_string(),
            sir::IndexConst::T(*t),
        ))
    }
}

fn tos_context() -> std::sync::Arc<vidformer::Context> {
    let fs_service = vidformer::service::Service::default();

    let filters: BTreeMap<String, Box<dyn filter::Filter>> = BTreeMap::new();
    let sources =
        vec![
            source::SourceVideoStreamMeta::profile("tos", "../tos_720p.mp4", 0, &fs_service)
                .unwrap(),
        ];
    let arrays = BTreeMap::new();
    let context: vidformer::Context = vidformer::Context::new(sources, arrays, filters);
    std::sync::Arc::new(context)
}

#[test]
fn test_tos_transcode_1dec() {
    let context = tos_context();
    let dve_config = std::sync::Arc::new(vidformer::Config {
        decode_pool_size: 10,
        decoder_view: usize::MAX,
        decoders: 1,
        filterers: 4,

        output_width: 1280,
        output_height: 720,
        output_pix_fmt: "yuv420p".to_string(),

        encoder: None,
        format: None,
    });
    let spec: std::sync::Arc<Box<dyn spec::Spec>> = std::sync::Arc::new(Box::new(ClipSpec {
        num_frames: NUM_FRAMES,
    }));
    let output_path = test_output_path!(test_tos_transcode_1dec);
    let stats = run_spec(&spec, output_path, &context, &dve_config, &None).unwrap();

    assert_eq!(stats.max_decoder_count, 1);
    assert_eq!(stats.frames_written, NUM_FRAMES as usize);
    assert!(stats.frames_decoded >= NUM_FRAMES as usize);

    assert!(std::path::Path::new(output_path).exists());
}

#[test]
fn test_tos_transcode_2dec() {
    let context = tos_context();
    let dve_config = std::sync::Arc::new(vidformer::Config {
        decode_pool_size: 10,
        decoder_view: usize::MAX,
        decoders: 2,
        filterers: 4,

        output_width: 1280,
        output_height: 720,
        output_pix_fmt: "yuv420p".to_string(),

        encoder: None,
        format: None,
    });
    let spec: std::sync::Arc<Box<dyn spec::Spec>> = std::sync::Arc::new(Box::new(ClipSpec {
        num_frames: NUM_FRAMES,
    }));
    let output_path = test_output_path!(test_tos_transcode_2dec);
    let stats = run_spec(&spec, output_path, &context, &dve_config, &None).unwrap();

    assert!(stats.max_decoder_count >= 1 && stats.max_decoder_count <= 2);
    assert_eq!(stats.frames_written, NUM_FRAMES as usize);
    assert!(stats.frames_decoded >= NUM_FRAMES as usize);

    assert!(std::path::Path::new(output_path).exists());
}

#[test]
fn test_tos_transcode_4dec() {
    let context = tos_context();
    let dve_config = std::sync::Arc::new(vidformer::Config {
        decode_pool_size: 10,
        decoder_view: usize::MAX,
        decoders: 4,
        filterers: 4,

        output_width: 1280,
        output_height: 720,
        output_pix_fmt: "yuv420p".to_string(),

        encoder: None,
        format: None,
    });
    let spec: std::sync::Arc<Box<dyn spec::Spec>> = std::sync::Arc::new(Box::new(ClipSpec {
        num_frames: NUM_FRAMES,
    }));
    let output_path = test_output_path!(test_tos_transcode_4dec);
    let stats = run_spec(&spec, output_path, &context, &dve_config, &None).unwrap();

    assert!(stats.max_decoder_count >= 1 && stats.max_decoder_count <= 4);
    assert_eq!(stats.frames_written, NUM_FRAMES as usize);
    assert!(stats.frames_decoded >= NUM_FRAMES as usize);

    assert!(std::path::Path::new(output_path).exists());
}

#[test]
fn test_tos_transcode_manydec() {
    let context = tos_context();
    let dve_config = std::sync::Arc::new(vidformer::Config {
        decode_pool_size: 10,
        decoder_view: usize::MAX,
        decoders: 100,
        filterers: 4,

        output_width: 1280,
        output_height: 720,
        output_pix_fmt: "yuv420p".to_string(),

        encoder: None,
        format: None,
    });
    let spec: std::sync::Arc<Box<dyn spec::Spec>> = std::sync::Arc::new(Box::new(ClipSpec {
        num_frames: NUM_FRAMES,
    }));
    let output_path = test_output_path!(test_tos_transcode_manydec);
    let stats = run_spec(&spec, output_path, &context, &dve_config, &None).unwrap();

    assert!(stats.max_decoder_count >= 1 && stats.max_decoder_count <= 100);
    assert_eq!(stats.frames_written, NUM_FRAMES as usize);
    assert!(stats.frames_decoded >= NUM_FRAMES as usize);

    assert!(std::path::Path::new(output_path).exists());
}

#[test]
fn test_tos_transcode_1dec_1pool() {
    let context = tos_context();
    let dve_config = std::sync::Arc::new(vidformer::Config {
        decode_pool_size: 1,
        decoder_view: usize::MAX,
        decoders: 1,
        filterers: 4,

        output_width: 1280,
        output_height: 720,
        output_pix_fmt: "yuv420p".to_string(),

        encoder: None,
        format: None,
    });
    let spec: std::sync::Arc<Box<dyn spec::Spec>> =
        std::sync::Arc::new(Box::new(ClipSpec { num_frames: 2 * 24 })); // make sure we only need one source GOP
    let output_path = test_output_path!(test_tos_transcode_1dec_1pool);
    let stats = run_spec(&spec, output_path, &context, &dve_config, &None).unwrap();

    assert_eq!(stats.max_decoder_count, 1);
    assert_eq!(stats.decoders_created, 1); // this is just a basic streaming edit. if our algorithm works it should just decode the one needed GOP
    assert_eq!(stats.frames_written, 2 * 24 as usize);
    assert!(stats.frames_decoded >= 2 * 24 as usize);

    assert!(std::path::Path::new(output_path).exists());
}
