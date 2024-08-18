use super::*;
use glob::glob;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use vidformer::run_spec;

#[derive(Deserialize)]
struct DveBench {
    spec: String,
    sources: Vec<(String, String, usize)>,
    config: vidformer::Config,
}

impl DveBench {
    fn from_json_file(path: &str) -> Self {
        let json_body = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&json_body).unwrap()
    }

    fn split(
        self,
    ) -> (
        Arc<Box<dyn vidformer::spec::Spec>>,
        Arc<vidformer::Context>,
        Arc<vidformer::Config>,
    ) {
        let spec = {
            let spec_json_body = std::fs::read_to_string(&self.spec).unwrap();
            let spec: spec::JsonSpec = serde_json::from_str(&spec_json_body).unwrap();
            Box::new(spec) as Box<dyn spec::Spec>
        };

        let sources = self
            .sources
            .into_iter()
            .map(|(name, path, stream)| {
                source::SourceVideoStreamMeta::load_meta(
                    &name,
                    stream,
                    &vidformer::service::Service::default(),
                    &path,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();

        let filters = default_filters();
        let arrays = BTreeMap::new();
        let context = vidformer::Context::new(sources, arrays, filters);

        (Arc::new(spec), Arc::new(context), Arc::new(self.config))
    }
}

#[derive(serde::Serialize)]
struct BenchmarkStat {
    bench: String,
    run: usize,
    config: vidformer::Config,
    stats: vidformer::Stats,
}

pub(crate) fn cmd_benchmark(opt: &BenchmarkCmd) {
    let mut outfile = std::fs::File::create(&opt.out_path).unwrap();
    for entry in glob(&opt.benches_glob).expect("Failed to use specs glob pattern") {
        let entry = entry.unwrap();
        let (spec, context, config) = DveBench::from_json_file(entry.to_str().unwrap()).split();

        println!("Running spec {} benches", entry.display());
        for _i in 0..opt.warmup_runs {
            run_spec(&spec, "/tmp/output.mp4", &context, &config, &None).unwrap();
        }

        for i in 0..opt.runs {
            let stat = run_spec(&spec, "/tmp/output.mp4", &context, &config, &None).unwrap();

            let benchmark_stat = BenchmarkStat {
                bench: entry.display().to_string(),
                config: (*config).clone(),
                run: i,
                stats: stat,
            };

            let json = serde_json::to_string(&benchmark_stat).unwrap();
            writeln!(outfile, "{}", json).unwrap();
            outfile.flush().unwrap();
        }
    }

    println!("Done!");
}
