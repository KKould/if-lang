use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use if_lang::eval::{eval_program_with_builtins, BuiltinFn};
use if_lang::lexer::Lexer;
use if_lang::lower::lower_program;
use if_lang::parser::parse_program;
use if_lang::validate::validate_program;
use libloading::Library;

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage();
        std::process::exit(2);
    }

    let command = args.remove(0);

    let (source, builtins, loaded_libs) = match command.as_str() {
        "check" | "run" => {
            let path = match args.get(0) {
                Some(p) => p.as_str(),
                None => {
                    eprintln!("missing file path");
                    print_usage();
                    std::process::exit(2);
                }
            };
            let source = match read_source(path) {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("failed to read source: {err}");
                    std::process::exit(1);
                }
            };
            let builtins = HashMap::new();
            (source, builtins, Vec::new())
        }
        "extra" => {
            let extra_path = match args.get(0) {
                Some(p) => p.as_str(),
                None => {
                    eprintln!("missing extra file path");
                    print_usage();
                    std::process::exit(2);
                }
            };
            let source_path = match args.get(1) {
                Some(p) => p.as_str(),
                None => {
                    eprintln!("missing source file path");
                    print_usage();
                    std::process::exit(2);
                }
            };
            let source = match read_source(source_path) {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("failed to read source: {err}");
                    std::process::exit(1);
                }
            };
            let mut builtins = HashMap::new();
            let mut loaded = Vec::new();
            match load_extra(extra_path, &mut builtins) {
                Ok(lib) => loaded.push(lib),
                Err(err) => {
                    eprintln!("failed to load extra: {err}");
                    std::process::exit(1);
                }
            }
            (source, builtins, loaded)
        }
        _ => {
            eprintln!("unknown command: {command}");
            print_usage();
            std::process::exit(2);
        }
    };

    let tokens = Lexer::new(&source).lex_all();
    let surface = match parse_program(&tokens) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("parse error: {} at {}", err.message, err.position);
            std::process::exit(1);
        }
    };

    if let Err(err) = validate_program(&surface) {
        eprintln!("validation error: {} at {}", err.message, err.position);
        std::process::exit(1);
    }

    let core = lower_program(surface);

    match command.as_str() {
        "check" => {
            println!("ok");
        }
        "run" => {
            match eval_program_with_builtins(&core, &builtins) {
                Ok(Some(value)) => println!("{value:?}"),
                Ok(None) => println!("ok"),
                Err(err) => {
                    eprintln!("eval error: {}", err.message);
                    std::process::exit(1);
                }
            }
        }
        "extra" => {
            match eval_program_with_builtins(&core, &builtins) {
                Ok(Some(value)) => println!("{value:?}"),
                Ok(None) => println!("ok"),
                Err(err) => {
                    eprintln!("eval error: {}", err.message);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("unknown command: {command}");
            print_usage();
            std::process::exit(2);
        }
    }

    drop(builtins);
    drop(loaded_libs);
}

fn read_source(path: &str) -> io::Result<String> {
    if path == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        fs::read_to_string(path)
    }
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  if_lang check <file>");
    eprintln!("  if_lang run <file>");
    eprintln!("  if_lang extra <dylib|rs> <file>");
}

type RegisterFn = unsafe extern "C" fn(*mut HashMap<String, BuiltinFn>);

fn load_extra(path: &str, builtins: &mut HashMap<String, BuiltinFn>) -> io::Result<Library> {
    let path = Path::new(path);
    let dylib_path = if is_rust_source(path) {
        build_extra_dylib(path)?
    } else {
        path.to_path_buf()
    };
    load_extra_library(&dylib_path, builtins)
}

fn load_extra_library(
    path: &Path,
    builtins: &mut HashMap<String, BuiltinFn>,
) -> io::Result<Library> {
    let lib = unsafe { Library::new(path) }.map_err(to_io_error)?;
    unsafe {
        let register: libloading::Symbol<RegisterFn> =
            lib.get(b"if_lang_register").map_err(to_io_error)?;
        register(builtins as *mut _);
    }
    Ok(lib)
}

fn to_io_error(err: libloading::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err.to_string())
}

fn is_rust_source(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("rs"))
        .unwrap_or(false)
}

fn build_extra_dylib(rs_path: &Path) -> io::Result<PathBuf> {
    if !rs_path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("extra source not found: {}", rs_path.display()),
        ));
    }
    let (target_dir, profile) = resolve_target_profile()?;
    let deps_dir = target_dir.join(&profile).join("deps");
    let rlib = find_if_lang_rlib(&deps_dir)?;
    let out_dir = target_dir.join("if_lang_extra").join(&profile);
    fs::create_dir_all(&out_dir)?;
    let stem = rs_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "invalid extra file name"))?;
    let out_path = out_dir.join(dylib_name(stem));

    let status = Command::new("rustc")
        .arg("--crate-type")
        .arg("cdylib")
        .arg("--edition")
        .arg("2024")
        .arg(rs_path)
        .arg("-o")
        .arg(&out_path)
        .arg("--extern")
        .arg(format!("if_lang={}", rlib.display()))
        .arg("-L")
        .arg(format!("dependency={}", deps_dir.display()))
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("rustc failed with status {status}"),
        ));
    }

    Ok(out_path)
}

fn resolve_target_profile() -> io::Result<(PathBuf, String)> {
    let exe = env::current_exe()?;
    if let Ok(dir) = env::var("CARGO_TARGET_DIR") {
        let target_dir = PathBuf::from(dir);
        if let Ok(relative) = exe.strip_prefix(&target_dir) {
            if let Some(profile) = relative
                .components()
                .next()
                .and_then(|c| c.as_os_str().to_str())
            {
                return Ok((target_dir, profile.to_string()));
            }
        }
        return Ok((target_dir, "debug".to_string()));
    }

    for ancestor in exe.ancestors() {
        if ancestor.file_name().and_then(|n| n.to_str()) == Some("target") {
            let relative = exe.strip_prefix(ancestor).unwrap_or(&exe);
            let profile = relative
                .components()
                .next()
                .and_then(|c| c.as_os_str().to_str())
                .unwrap_or("debug");
            return Ok((ancestor.to_path_buf(), profile.to_string()));
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "unable to locate target directory",
    ))
}

fn find_if_lang_rlib(deps_dir: &Path) -> io::Result<PathBuf> {
    let mut best: Option<(SystemTime, PathBuf)> = None;
    for entry in fs::read_dir(deps_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if !(name.starts_with("libif_lang") || name.starts_with("if_lang")) {
            continue;
        }
        if !name.ends_with(".rlib") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        match &best {
            Some((best_time, _)) if *best_time >= modified => {}
            _ => best = Some((modified, path)),
        }
    }
    if let Some((_, path)) = best {
        Ok(path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "if_lang rlib not found in {} (build the project first)",
                deps_dir.display()
            ),
        ))
    }
}

fn dylib_name(stem: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{stem}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{stem}.dylib")
    } else {
        format!("lib{stem}.so")
    }
}
