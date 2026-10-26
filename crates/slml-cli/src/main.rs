//! The `slml` command.

mod project;
mod viewer;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use slml_render::Renderer;
use slml_syntax::model::Document;

const USAGE: &str = "\
slml — the SLML document language

USAGE:
    slml <file.slml>                         open the file in the viewer
    slml view <file.slml> [--scale N]        same as above
    slml check <file.slml>...                validate files, print errors
    slml render <file.slml> [-o out.png] [--width 900] [--scale 1]
                                             render the whole page to PNG

Viewer keys: scroll / arrows / PageUp / PageDown / Home / End,
             click links, Backspace = back, F5 = reload (also automatic on save)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            Ok(())
        }
        Some("--version" | "-V") => {
            println!(
                "slml {} (language {})",
                env!("CARGO_PKG_VERSION"),
                slml_syntax::analyze::SUPPORTED_VERSION
            );
            Ok(())
        }
        Some("check") => check(&args[1..]),
        Some("render") => render(&args[1..]),
        Some("view") => view(&args[1..]),
        Some(_) => view(&args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            if !msg.is_empty() {
                eprintln!("{msg}");
            }
            ExitCode::FAILURE
        }
    }
}

/// Reads and compiles a file; on failure returns all diagnostics, formatted.
pub fn load(path: &Path) -> Result<Document, String> {
    project::compile_file(path, &project::default_root(path))
}

struct Opts {
    files: Vec<PathBuf>,
    out: Option<PathBuf>,
    width: u32,
    scale: f32,
}

fn parse_opts(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts {
        files: Vec::new(),
        out: None,
        width: 900,
        scale: 1.0,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match a.as_str() {
            "-o" | "--out" => o.out = Some(value(a)?.into()),
            "--width" => {
                o.width = value(a)?
                    .parse()
                    .map_err(|_| "--width must be a whole number")?
            }
            "--scale" => {
                o.scale = value(a)?.parse().map_err(|_| "--scale must be a number")?;
                if !(0.25..=8.0).contains(&o.scale) {
                    return Err("--scale must be between 0.25 and 8".into());
                }
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown option {flag}\n\n{USAGE}"));
            }
            file => o.files.push(file.into()),
        }
    }
    if o.files.is_empty() {
        return Err(format!("no input file\n\n{USAGE}"));
    }
    Ok(o)
}

fn check(args: &[String]) -> Result<(), String> {
    let o = parse_opts(args)?;
    let mut failed = false;
    for f in &o.files {
        match load(f) {
            Ok(_) => println!("ok: {}", f.display()),
            Err(e) => {
                eprintln!("{e}\n");
                failed = true;
            }
        }
    }
    if failed { Err(String::new()) } else { Ok(()) }
}

fn render(args: &[String]) -> Result<(), String> {
    let o = parse_opts(args)?;
    let [file] = o.files.as_slice() else {
        return Err("render takes exactly one file".into());
    };
    let doc = load(file)?;
    let mut renderer = Renderer::new(project::base_dir(file));
    renderer.images = slml_render::images::ImageStore::with_root(
        project::base_dir(file),
        project::default_root(file),
    );
    let pixmap = renderer.render_page(&doc, o.width, o.scale);
    let out = o.out.unwrap_or_else(|| file.with_extension("png"));
    pixmap
        .save_png(&out)
        .map_err(|e| format!("error: cannot write {}: {e}", out.display()))?;
    println!(
        "wrote {} ({}x{})",
        out.display(),
        pixmap.width(),
        pixmap.height()
    );
    Ok(())
}

fn view(args: &[String]) -> Result<(), String> {
    let o = parse_opts(args)?;
    let [file] = o.files.as_slice() else {
        return Err("view takes exactly one file".into());
    };
    viewer::run(file.clone(), project::default_root(file), o.scale)
}
