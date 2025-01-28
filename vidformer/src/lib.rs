//! [vidformer](https://github.com/ixlab/vidformer) is a core video synthesis/transformation library.
//! It handles the movement, control flow, and processing of video and conventional (non-video) data.
//!
//! **Quick links:**
//! * [📦 Crates.io](https://crates.io/crates/vidformer)
//! * [📘 Documentation](https://ixlab.github.io/vidformer/vidformer/)
//! * [🧑‍💻 Source Code](https://github.com/ixlab/vidformer/tree/main/vidformer/)

pub mod filter;
pub mod io;
pub mod service;
pub mod sir;
pub mod source;
pub mod spec;

pub(crate) mod av;
mod dve;
mod pool;
mod util;

pub use dve::{
    create_spec_hls, run, validate, Config, Context, EncoderConfig, Error, Range, RangeTsFormat,
    Stats,
};
pub use util::{codecs, init, CodecDescriptor};
