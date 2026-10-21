// Compile the actual desktop boundary against this lightweight adapter without
// starting the unrelated transport stack on a memory-constrained host.
extern crate twodrive_cfapi_probe as twodrive_local_files;
#[path = "../../twodrive-dev/src/desktop_local_files.rs"]
mod desktop_local_files;
