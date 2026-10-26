//! Compila apenas a ponte própria; o SDK do OCCT deve ser instalado separadamente.
use std::{env, error::Error, fs, path::PathBuf, process::Command};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=native");
    for name in [
        "OpenCASCADE_DIR",
        "CMAKE_PREFIX_PATH",
        "CMAKE_GENERATOR",
        "CXX",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    if env::var_os("CARGO_FEATURE_OCCT").is_none() {
        return Ok(());
    }
    if env::var("HOST")? != env::var("TARGET")? {
        return Err(
            "A ponte requer compilação nativa; use um runner Windows para validar Windows.".into(),
        );
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR ausente")?);
    let build = out.join("occt-bridge");
    let mut configure = Command::new("cmake");
    configure
        .args(["-S", "native", "-B"])
        .arg(&build)
        .arg("-DCMAKE_BUILD_TYPE=Release");
    if let Some(dir) = env::var_os("OpenCASCADE_DIR") {
        configure.arg(format!("-DOpenCASCADE_DIR={}", dir.to_string_lossy()));
    }
    if !configure.status()?.success() {
        return Err("SDK OCCT indisponível ou incompatível. Consulte docs/PROTOTYPE.md e configure OpenCASCADE_DIR.".into());
    }
    if !Command::new("cmake")
        .arg("--build")
        .arg(&build)
        .args(["--config", "Release", "--parallel", "2"])
        .status()?
        .success()
    {
        return Err("Falha ao compilar a ponte OCCT; consulte as mensagens do C++.".into());
    }
    println!(
        "cargo:rustc-link-search=native={}",
        build.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=nator_occt");
    for line in fs::read_to_string(build.join("nator-links.txt"))?.lines() {
        if let Some(path) = line.strip_prefix("path=") {
            println!("cargo:rustc-link-search=native={path}");
        }
        if let Some(lib) = line.strip_prefix("lib=") {
            println!("cargo:rustc-link-lib=dylib={lib}");
        }
    }
    match env::var("CARGO_CFG_TARGET_ENV")?.as_str() {
        "msvc" => {}
        _ if env::var("CARGO_CFG_TARGET_OS")? == "macos" => {
            println!("cargo:rustc-link-lib=dylib=c++")
        }
        _ => println!("cargo:rustc-link-lib=dylib=stdc++"),
    }
    Ok(())
}
