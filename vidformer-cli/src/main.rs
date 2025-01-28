use clap::{Parser, Subcommand};
use num_rational::Rational64;
use std::{collections::BTreeMap, fs::File, io::Write};

use vidformer::{filter, run, sir, source, spec};

mod bench;
mod yrden;

pub fn simple_source(
    ident: &str,
) -> Result<vidformer::source::SourceVideoStreamMeta, vidformer::Error> {
    let split = ident.split(':').collect::<Vec<&str>>();
    assert_eq!(split.len(), 3);
    let (name, path, stream) = (split[0], split[1], split[2].parse::<usize>().unwrap());
    let fs_service = vidformer::service::Service::default();
    source::SourceVideoStreamMeta::profile(name, path, stream, &fs_service, &None)
}

pub fn opendal_source(
    name: &str,
    path: &str,
    stream: usize,
    service: Option<&vidformer::service::Service>,
) -> Result<vidformer::source::SourceVideoStreamMeta, vidformer::Error> {
    if let Some(service) = service {
        source::SourceVideoStreamMeta::profile(name, path, stream, service, &None)
    } else {
        let fs_service = vidformer::service::Service::default();
        source::SourceVideoStreamMeta::profile(name, path, stream, &fs_service, &None)
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: ArgCmd,
}

#[derive(Parser, Debug)]
struct ProfileCmd {
    #[clap(long)]
    name: String,

    #[clap(long)]
    vid_path: String,

    #[clap(long)]
    stream: usize,

    #[clap(long)]
    out_path: Option<String>,
}

#[derive(Parser, Debug)]
struct ValidateCmd {
    #[clap(long)]
    name: String,

    #[clap(long)]
    vid_path: String,

    #[clap(long)]
    stream: usize,
}

#[derive(Parser, Debug)]
struct YrdenCmd {
    #[clap(long, default_value = "8000")]
    port: u16,

    #[clap(long)]
    hls_prefix: Option<String>,

    #[clap(long)]
    print_url: bool,

    #[clap(long, default_value = "/tmp/yrden.db")]
    db_path: String,
}

#[derive(Parser, Debug)]
struct BenchmarkCmd {
    #[clap(long)]
    benches_glob: String,

    #[clap(long, default_value = "5")]
    runs: usize,

    #[clap(long, default_value = "1")]
    warmup_runs: usize,

    #[clap(long)]
    out_path: String,
}

#[derive(Subcommand, Debug)]
enum ArgCmd {
    Profile(ProfileCmd),
    X,
    Yrden(YrdenCmd),
    Benchmark(BenchmarkCmd),
    Validate(ValidateCmd),
    Codecs,
}

fn cmd_profile(opt: &ProfileCmd) {
    let fs_service = vidformer::service::Service::default();
    let stream_meta = source::SourceVideoStreamMeta::profile(
        &opt.name,
        &opt.vid_path,
        opt.stream,
        &fs_service,
        &None,
    )
    .unwrap();
    let profile_data = source::create_profile_file(&[stream_meta]);

    if let Some(out_path) = &opt.out_path {
        assert_ne!(out_path, &opt.vid_path);
        let json = serde_json::to_string(&profile_data).unwrap();
        let mut file = File::create(out_path).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    } else {
        println!("{}", serde_json::to_string_pretty(&profile_data).unwrap());
    }
}

fn cmd_validate(opt: &ValidateCmd) {
    let fs_service = vidformer::service::Service::default();
    source::SourceVideoStreamMeta::validate(
        &opt.name,
        &opt.vid_path,
        opt.stream,
        &fs_service,
        &None,
    );
}

fn cmd_x() {
    const NUM_FRAMES: i64 = 3;

    struct MySpec {}
    impl spec::Spec for MySpec {
        fn domain(&self, _context: &dyn spec::SpecContext) -> Vec<num_rational::Rational64> {
            (0..NUM_FRAMES).map(|i| Rational64::new(i, 24)).collect()
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

    let fs_service = vidformer::service::Service::default();
    let filters: BTreeMap<String, Box<dyn filter::Filter>> = BTreeMap::new();
    let sources =
        vec![
            source::SourceVideoStreamMeta::profile("tos", "tos_720p.mp4", 0, &fs_service, &None)
                .unwrap(),
        ];
    let context: vidformer::Context = vidformer::Context::new(sources, filters, None);

    let dve_config = vidformer::Config {
        decode_pool_size: 10000,
        decoder_view: usize::MAX,
        decoders: 2,
        filterers: 2,

        output_width: 1280,
        output_height: 720,
        output_pix_fmt: "yuv420p".to_string(),

        encoder: None,
        format: None,
    };

    let spec: Box<dyn spec::Spec> = Box::new(MySpec {});

    let output_path = "mytestout.mp4";

    let spec = std::sync::Arc::new(spec);
    let context = std::sync::Arc::new(context);
    let dve_config = std::sync::Arc::new(dve_config);

    println!("Running spec...");
    let stats = run(&spec, output_path, &context, &dve_config, &None).unwrap();

    dbg!(&stats);

    // assert_eq!(stats.max_decoder_count, 1);
    // assert_eq!(stats.frames_written, NUM_FRAMES as usize);

    assert!(std::path::Path::new(output_path).exists());
}

fn default_filters() -> BTreeMap<String, Box<dyn filter::Filter>> {
    let mut filters: BTreeMap<String, Box<dyn filter::Filter>> = BTreeMap::new();
    filters.extend(vidformer::filter::builtin::filters());
    filters.extend(vidformer::filter::cv2::filters());
    filters
}

fn cmd_codecs() {
    let codecs = vidformer::codecs();

    let longest_name = codecs.iter().map(|c| c.name.len()).max().unwrap();
    let longest_long_name = codecs.iter().map(|c| c.long_name.len()).max().unwrap();
    let longest_format_id_pretty_name = codecs
        .iter()
        .map(|c| c.format_id_pretty_name.len())
        .max()
        .unwrap();

    println!(
        "{:<longest_name$} {:<longest_long_name$} {:<longest_format_id_pretty_name$} D E",
        "Name",
        "Long Name",
        "Format ID",
        longest_name = longest_name,
        longest_long_name = longest_long_name,
        longest_format_id_pretty_name = longest_format_id_pretty_name,
    );

    for codec in codecs {
        println!(
            "{:<longest_name$} {:<longest_long_name$} {:<longest_format_id_pretty_name$} {} {}",
            codec.name,
            codec.long_name,
            codec.format_id_pretty_name,
            if codec.has_decoder { "D" } else { "-" },
            if codec.has_encoder { "E" } else { "-" },
            longest_name = longest_name,
            longest_long_name = longest_long_name,
            longest_format_id_pretty_name = longest_format_id_pretty_name,
        );
    }
}

fn main() {
    pretty_env_logger::init();
    vidformer::init();

    let args = Args::parse();

    match args.cmd {
        ArgCmd::Profile(opt) => cmd_profile(&opt),
        ArgCmd::Validate(opt) => cmd_validate(&opt),
        ArgCmd::X => cmd_x(),
        ArgCmd::Yrden(opt) => yrden::cmd_yrden(&opt),
        ArgCmd::Benchmark(opt) => bench::cmd_benchmark(&opt),
        ArgCmd::Codecs => cmd_codecs(),
    }
}
