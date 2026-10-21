//! Compile the actual Windows model/provider/cache bridge without the unrelated
//! network stack. Full desktop CI also compiles/runs the same tests in the engine.
#![cfg(windows)]
#![allow(dead_code)]
#[path = "../../twodrive-dev/src/local_files.rs"]
mod local_files;
#[path = "../../twodrive-dev/src/model.rs"]
mod model;
#[path = "../../twodrive-dev/src/provider.rs"]
mod provider;
