use std::path::{Path, PathBuf};

fn find_upwards(start: &Path, filename: &str) -> Option<PathBuf> {
    let mut dir = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent().unwrap_or(start).to_path_buf()
    };

    loop {
        let candidate = dir.join(filename);
        if candidate.exists() {
            return Some(candidate);
        }

        if !dir.pop() {
            break;
        }
    }

    None
}

pub fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let start = std::env::current_dir()?;
    let lock = find_upwards(&start, "Cargo.lock")
        .or_else(|| find_upwards(Path::new(env!("CARGO_MANIFEST_DIR")), "Cargo.lock"))
        .ok_or_else(|| -> Box<dyn std::error::Error> {
            Box::<dyn std::error::Error>::from(String::from(
                "Cargo.lock not found (run from within the repo or pass --config)",
            ))
        })?;

    Ok(lock.parent().unwrap_or(lock.as_path()).to_path_buf())
}
