use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=shim/shim.c");
    println!("cargo:rerun-if-changed=vendor");

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let aom_source = manifest.join("vendor/avif/ext/aom");
    let aom_lists = aom_source.join("CMakeLists.txt");
    if !aom_lists.is_file() {
        println!(
            "cargo::error=the libaom source is missing from the crate: {} not found",
            aom_lists.display()
        );
        return;
    }

    // libaom's encoder is unusably slow unoptimised, so it is always built
    // for release, whatever profile Cargo is building.
    let mut config = cmake::Config::new("vendor/avif");
    config.profile("Release");

    if env::var("CARGO_CFG_TARGET_ENV").unwrap() == "msvc" {
        // With a Visual Studio generator the cmake crate replaces
        // CMAKE_<LANG>_FLAGS_RELEASE with flags that leave out /O2 and
        // /DNDEBUG (rust-lang/cmake-rs#240), so they are set in full here.
        let features = env::var("CARGO_CFG_TARGET_FEATURE").unwrap_or_default();
        let (crt, runtime) = if features.split(',').any(|f| f == "crt-static") {
            ("/MT", "MultiThreaded")
        } else {
            ("/MD", "MultiThreadedDLL")
        };
        let flags = format!("{crt} /O2 /Ob2 /DNDEBUG");
        config
            // A project asking for CMake 3.15 or later takes its C runtime
            // from this and not from the flags, and defaults to the DLL.
            .define("CMAKE_POLICY_DEFAULT_CMP0091", "NEW")
            .define("CMAKE_MSVC_RUNTIME_LIBRARY", runtime)
            .define("CMAKE_C_FLAGS_RELEASE", &flags)
            .define("CMAKE_CXX_FLAGS_RELEASE", &flags)
            // libavif applies the first configuration's flags to libaom's
            // targets, and in a multi-configuration generator that would be
            // Debug.
            .define("CMAKE_CONFIGURATION_TYPES", "Release");
    }

    let dst = config
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("CMAKE_INSTALL_LIBDIR", "lib")
        .define("AVIF_CODEC_AOM", "LOCAL")
        .define("AVIF_CODEC_AOM_DECODE", "OFF")
        .define("AVIF_CODEC_AOM_ENCODE", "ON")
        .define("AVIF_LIBYUV", "OFF")
        .define("AVIF_LIBSHARPYUV", "OFF")
        .define("AVIF_LIBXML2", "OFF")
        .define("AVIF_JPEG", "OFF")
        .define("AVIF_ZLIBPNG", "OFF")
        .define("AVIF_BUILD_APPS", "OFF")
        .define("AVIF_BUILD_TESTS", "OFF")
        .define("AVIF_BUILD_EXAMPLES", "OFF")
        .define("AVIF_ENABLE_WERROR", "OFF")
        // libavif's LocalAom.cmake sets FETCHCONTENT_SOURCE_DIR_AOM, but its
        // content is named libaom, so the override it means is this one.
        // Fully disconnected makes a missing source an error rather than a
        // download.
        .define("FETCHCONTENT_SOURCE_DIR_LIBAOM", &aom_source)
        .define("FETCHCONTENT_FULLY_DISCONNECTED", "ON")
        .build();

    record_build_facts(&dst.join("build/CMakeCache.txt"));

    let include = dst.join("include");
    cc::Build::new()
        .file("shim/shim.c")
        .include(&include)
        .compile("avif_shim");

    let lib: PathBuf = dst.join("lib");
    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=static=avif");
    println!("cargo:include={}", include.display());

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os == "linux" {
        println!("cargo:rustc-link-lib=m");
    }
}

fn record_build_facts(cache: &Path) {
    let cache = fs::read_to_string(cache).expect("CMake left no cache");
    let value = |name: &str| {
        cache.lines().find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.split_once(':').map_or(key, |(key, _)| key) == name).then_some(value.trim())
        })
    };
    // A multi-configuration generator leaves CMAKE_BUILD_TYPE empty and builds
    // the first of CMAKE_CONFIGURATION_TYPES, which is what libavif gives aom.
    let build_type = value("CMAKE_BUILD_TYPE")
        .filter(|t| !t.is_empty())
        .or_else(|| value("CMAKE_CONFIGURATION_TYPES")?.split(';').next())
        .unwrap_or_default();
    let config_flags = value(&format!("CMAKE_C_FLAGS_{}", build_type.to_uppercase()));
    let c_flags = [value("CMAKE_C_FLAGS"), config_flags]
        .into_iter()
        .flatten()
        .filter(|f| !f.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    println!("cargo:rustc-env=AVIF_AOM_BUILD_TYPE={build_type}");
    println!("cargo:rustc-env=AVIF_AOM_C_FLAGS={c_flags}");
}
