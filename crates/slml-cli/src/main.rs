//! `slml`: the SLML compiler, preview server and viewer.
//!
//! Works like a classic compiler: `slml page.slml` writes `page.html`,
//! `-o` chooses the output and its extension chooses the format.

mod build;
mod project;
mod serve;
#[cfg(feature = "viewer")]
mod viewer;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use slml_render::Renderer;
use slml_render::images::ImageStore;

const USAGE: &str = "\
slml — compiler for SLML, the Strict Layout Markup Language

USAGE:
    slml <file.slml>... [-o <output>]     compile pages to HTML (page.slml → page.html)
    slml <folder> [-o <output-folder>]    compile a whole site (default output: <folder>/_site)
    slml check <file-or-folder>...        only check for errors
    slml serve [<folder>] [--port 4000]   preview in your web browser, reloads on save
    slml view <file.slml>                 open in the native SLML viewer
    slml render <file.slml> [-o out.png]  render to an image with the native renderer

OPTIONS:
    -o, --output <path>   output file or folder; `.png` renders an image, anything else is HTML
        --root <folder>   site root for paths starting with `/`
                          (default: the current folder if the file is inside it, else the file's folder)
        --width <px>      page width for PNG output (default 900)
        --scale <n>       pixel density for PNG output and the viewer (default 1)
        --port <n>        first port to try for `serve` (default 4000)
        --open            `serve`: also open the page in your browser
    -q, --quiet           print only errors
    -h, --help            show this help
    -V, --version         show the version

EXAMPLES:
    slml index.slml                       writes index.html (+ _slml/ with styles and fonts)
    slml site/ -o public/                 builds every page of site/ into public/
    slml serve site/ --open               live preview at http://127.0.0.1:4000/

Exit status is 0 on success and 1 if any file has errors.
Handbook: https://github.com/KubeKoslaw/slml (handbook/ folder)";

#[derive(Default)]
struct Args {
    command: Option<String>,
    inputs: Vec<PathBuf>,
    output: Option<PathBuf>,
    root: Option<PathBuf>,
    width: u32,
    scale: f32,
    port: u16,
    open: bool,
    quiet: bool,
}

fn parse_args(raw: Vec<String>) -> Result<Args, String> {
    let mut a = Args {
        width: 900,
        scale: 1.0,
        port: 4000,
        ..Args::default()
    };
    let mut it = raw.into_iter().peekable();
    if let Some(first) = it.peek()
        && matches!(
            first.as_str(),
            "check" | "serve" | "view" | "render" | "build"
        )
    {
        a.command = it.next();
    }
    while let Some(arg) = it.next() {
        let mut value = |name: &str| {
            it.next()
                .ok_or_else(|| format!("error: {name} needs a value"))
        };
        match arg.as_str() {
            "-o" | "--output" => a.output = Some(value(&arg)?.into()),
            "--root" => a.root = Some(value(&arg)?.into()),
            "--width" => {
                a.width = value(&arg)?
                    .parse()
                    .map_err(|_| "error: --width must be a whole number")?
            }
            "--scale" => {
                a.scale = value(&arg)?
                    .parse()
                    .map_err(|_| "error: --scale must be a number")?;
                if !(0.25..=8.0).contains(&a.scale) {
                    return Err("error: --scale must be between 0.25 and 8".into());
                }
            }
            "--port" => {
                a.port = value(&arg)?
                    .parse()
                    .map_err(|_| "error: --port must be a number from 1 to 65535")?
            }
            "--open" => a.open = true,
            "-q" | "--quiet" => a.quiet = true,
            flag if flag.starts_with('-') && flag.len() > 1 => {
                return Err(format!("error: unknown option {flag}\n\n{USAGE}"));
            }
            input => a.inputs.push(input.into()),
        }
    }
    Ok(a)
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    match raw.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Some("-V" | "--version") => {
            println!(
                "slml {} (language {})",
                env!("CARGO_PKG_VERSION"),
                slml_syntax::analyze::SUPPORTED_VERSION
            );
            return ExitCode::SUCCESS;
        }
        _ => {}
    }
    let result = parse_args(raw).and_then(|args| match args.command.as_deref() {
        Some("check") => check(&args),
        Some("serve") => serve(&args),
        Some("view") => view(&args),
        Some("render") => render(&args),
        _ => compile(&args),
    });
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

fn need_inputs(a: &Args) -> Result<(), String> {
    if a.inputs.is_empty() {
        return Err(format!("error: no input files\n\n{USAGE}"));
    }
    for i in &a.inputs {
        if !i.exists() {
            return Err(format!("error: {} does not exist", i.display()));
        }
    }
    Ok(())
}

fn single(a: &Args, what: &str) -> Result<PathBuf, String> {
    need_inputs(a)?;
    match a.inputs.as_slice() {
        [file] if file.is_file() => Ok(file.clone()),
        [_] => Err(format!("error: {what} takes a .slml file")),
        _ => Err(format!("error: {what} takes exactly one file")),
    }
}

fn root_for(a: &Args, file: &Path) -> PathBuf {
    a.root
        .clone()
        .unwrap_or_else(|| project::default_root(file))
}

/// The default action, like `cc file.c`: compile to HTML (or PNG).
fn compile(a: &Args) -> Result<(), String> {
    need_inputs(a)?;
    if let [dir] = a.inputs.as_slice()
        && dir.is_dir()
    {
        let out = a.output.clone().unwrap_or_else(|| dir.join("_site"));
        return match build::build_site(dir, &out, a.quiet) {
            0 => Ok(()),
            n => Err(format!("{n} file{} failed", if n == 1 { "" } else { "s" })),
        };
    }
    if let Some(dir) = a.inputs.iter().find(|i| i.is_dir()) {
        return Err(format!(
            "error: {} is a folder; compile a site on its own: slml {} -o <output-folder>",
            dir.display(),
            dir.display()
        ));
    }
    if a.output.is_some() && a.inputs.len() > 1 {
        return Err("error: -o can only be used with one input file".into());
    }
    if a.output
        .as_ref()
        .is_some_and(|o| o.extension().is_some_and(|e| e == "png"))
    {
        return render(a);
    }
    match build::compile_pages(&a.inputs, a.output.as_deref(), a.root.as_deref(), a.quiet) {
        0 => Ok(()),
        n => Err(format!("{n} file{} failed", if n == 1 { "" } else { "s" })),
    }
}

fn check(a: &Args) -> Result<(), String> {
    need_inputs(a)?;
    let mut files = Vec::new();
    for input in &a.inputs {
        if input.is_dir() {
            files.extend(
                build::site_files(input, &input.join("_site"))
                    .into_iter()
                    .filter(|f| f.extension().is_some_and(|e| e == "slml"))
                    .map(|f| (f, Some(input.clone()))),
            );
        } else {
            files.push((input.clone(), None));
        }
    }
    let mut failed = 0;
    for (file, site) in &files {
        let root = site.clone().unwrap_or_else(|| root_for(a, file));
        let src = std::fs::read_to_string(file).unwrap_or_default();
        // Libraries are checked by the pages that import them.
        if slml_syntax::is_library(&src) {
            match slml_syntax::parse(&src) {
                Ok(_) => continue,
                Err(d) => {
                    eprintln!("{}\n", d.render(&src, &file.display().to_string()));
                    failed += 1;
                    continue;
                }
            }
        }
        let result = project::compile_file(file, &root).and_then(|doc| {
            let broken = build::broken_links(&doc, file, &root);
            if broken.is_empty() {
                Ok(())
            } else {
                Err(broken.join("\n"))
            }
        });
        match result {
            Ok(()) => {
                if !a.quiet {
                    println!("ok: {}", file.display());
                }
            }
            Err(e) => {
                eprintln!("{e}\n");
                failed += 1;
            }
        }
    }
    match failed {
        0 => Ok(()),
        n => Err(format!("{n} file{} failed", if n == 1 { "" } else { "s" })),
    }
}

fn render(a: &Args) -> Result<(), String> {
    let file = single(a, "render")?;
    let root = root_for(a, &file);
    let doc = project::compile_file(&file, &root)?;
    let mut renderer = Renderer::new(project::base_dir(&file));
    renderer.images = ImageStore::with_root(project::base_dir(&file), &root);
    let pixmap = renderer.render_page(&doc, a.width, a.scale);
    let out = a
        .output
        .clone()
        .unwrap_or_else(|| file.with_extension("png"));
    pixmap
        .save_png(&out)
        .map_err(|e| format!("error: cannot write {}: {e}", out.display()))?;
    if !a.quiet {
        println!(
            "{} → {} ({}x{})",
            file.display(),
            out.display(),
            pixmap.width(),
            pixmap.height()
        );
    }
    Ok(())
}

#[cfg(feature = "viewer")]
fn view(a: &Args) -> Result<(), String> {
    let file = single(a, "view")?;
    viewer::run(file.clone(), root_for(a, &file), a.scale)
}

#[cfg(not(feature = "viewer"))]
fn view(_: &Args) -> Result<(), String> {
    Err("error: this slml was built without the native viewer; use `slml serve` instead".into())
}

fn serve(a: &Args) -> Result<(), String> {
    let root = match a.inputs.as_slice() {
        [] => PathBuf::from("."),
        [dir] if dir.is_dir() => dir.clone(),
        [file] if file.is_file() => project::base_dir(file),
        _ => return Err("error: serve takes one folder".into()),
    };
    serve::run(root, a.port, a.open)
}
