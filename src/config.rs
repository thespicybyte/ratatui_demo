use std::path::PathBuf;
use std::sync::LazyLock;

pub static PROJECT_NAME: LazyLock<String> =
    LazyLock::new(|| env!("CARGO_CRATE_NAME").to_uppercase().to_string());
pub fn get_data_dir() -> PathBuf {
    PathBuf::from(".")
    // let directory = if let Some(s) = DATA_FOLDER.clone() {
    //     s
    // } else if let Some(proj_dirs) = project_directory() {
    //     proj_dirs.data_local_dir().to_path_buf()
    // } else {
    //     PathBuf::from(".").join(".data")
    // };
    // directory
}
