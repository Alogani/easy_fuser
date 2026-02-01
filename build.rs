use std::env;
use std::fs;
use std::path::Path;

use askama::Template;

mod templates;
use crate::templates::*;

pub enum Modes {
    Serial,
    Parallel,
    Async
}

impl Modes {
    fn as_str(&self) -> &str {
        match self {
            Modes::Serial => "serial",
            Modes::Parallel => "parallel",
            Modes::Async => "async",
        }
    }
}


fn main() -> std::io::Result<()> {
    let template_dir = "templates";
    println!("cargo:rerun-if-changed={template_dir}");
    for entry in std::fs::read_dir(template_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR not set");

    for (mode, send_sync) in [(Modes::Serial, false), (Modes::Parallel, true)] {
        let mode = mode.as_str();
        let mode_dir = Path::new(&out_dir).join(mode);
        fs::create_dir_all(&mode_dir)?;

        let content = FuseDriverTemplate { mode, send_sync }.render()?;
        fs::write(
            mode_dir.join("fuse_driver.rs"),
            content
        )?;
    }

    Ok(())
}
