use directories::ProjectDirs;
use std::sync::LazyLock;

pub static PROJECT_DIRS: LazyLock<ProjectDirs> =
    LazyLock::new(|| ProjectDirs::from("com", "yob", "ssi").unwrap());
