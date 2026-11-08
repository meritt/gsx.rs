mod corpus;

use std::fmt::Display;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::{env, fs};

use serde_json::{Value, json};

const USAGE: &str = "usage: cargo xtask header [--check] | wasm | xcframework | wheel | replay [npm|swift|python]... \
                     | dist [<dir>] [--wheels <dir>] | pin [--check] [<dir>] | corpus | bench [--wasm] [--out <file>]";

const RELEASES: &str = "https://github.com/meritt/gsx.rs/releases/download";

const BINARYEN: &str = "133";

const WASM_OPT_FLAGS: [&str; 3] = ["-Os", "--strip-debug", "--strip-producers"];

const WASM_GZIP_BUDGET: u64 = 365_000;

const GLUE_MARKER: &str = "/* @glue */";

const MACOS: &str = "27.0";

const WHEEL_MACOS: &str = "11.0";

const SOURCE_DATE_EPOCH: &str = "315532800";

const APPLE_TARGETS: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];

const LINUX_WHEELS: [&str; 4] =
    ["manylinux_2_28_aarch64", "manylinux_2_28_x86_64", "musllinux_1_2_aarch64", "musllinux_1_2_x86_64"];

const CORPUS: [&str; 2] = ["testdata/corpus/document.jsonl", "testdata/corpus/diagnostics.jsonl"];
const SELECTION: &str = "testdata/corpus/targets/x-plain.jsonl";
const PROJECTION: &str = "target/staging/projection.jsonl";

type Result<T = ()> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["header"] => header(false),
        ["header", "--check"] => header(true),
        ["wasm"] => wasm().map(drop),
        ["xcframework"] => xcframework().map(drop),
        ["wheel"] => wheel().map(drop),
        ["replay", bindings @ ..] => replay(bindings),
        ["dist"] => dist(None, None),
        ["dist", "--wheels", wheels] => dist(None, Some(Path::new(wheels))),
        ["dist", dir] => dist(Some(Path::new(dir)), None),
        ["dist", dir, "--wheels", wheels] => dist(Some(Path::new(dir)), Some(Path::new(wheels))),
        ["pin"] => pin(false, None),
        ["pin", "--check"] => pin(true, None),
        ["pin", "--check", dir] => pin(true, Some(Path::new(dir))),
        ["pin", dir] => pin(false, Some(Path::new(dir))),
        ["corpus"] => regenerate(),
        ["bench"] => bench(None, false),
        ["bench", "--wasm"] => bench(None, true),
        ["bench", "--out", out] => bench(Some(out), false),
        ["bench", "--wasm", "--out", out] => bench(Some(out), true),
        _ => Err(USAGE.to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

fn header(check: bool) -> Result {
    let ffi = root().join("crates/gsx-ffi");
    let path = ffi.join("include/gsx.h");
    let config = cbindgen::Config::from_file(ffi.join("cbindgen.toml"))?;
    let bindings = cbindgen::Builder::new()
        .with_crate(&ffi)
        .with_config(config)
        .generate()
        .map_err(|e| format!("cbindgen: {e}"))?;
    if check {
        let mut fresh = Vec::new();
        bindings.write(&mut fresh);
        if fs::read(&path).map_err(at(path.display()))? != fresh {
            return Err(format!("{} is stale: run `cargo xtask header`", path.display()));
        }
    } else {
        bindings.write_to_file(&path);
    }
    Ok(())
}

fn wasm() -> Result<PathBuf> {
    let root = root();
    let version = version()?;
    let manifest = root.join("packages/npm/package.json");
    let npm: Value = read_json(&manifest)?;
    if npm["version"] != version.as_str() {
        return Err(format!("{} has version {}, Cargo.toml {version}", manifest.display(), npm["version"]));
    }

    let bindgen = tool("WASM_BINDGEN", Some(root.join("target/tools/bin/wasm-bindgen")), "wasm-bindgen");
    let expected = locked_version("wasm-bindgen")?;
    let found = capture(Command::new(&bindgen).arg("--version"))?;
    if found != format!("wasm-bindgen {expected}") {
        return Err(format!(
            "{} is {found}, Cargo.lock has wasm-bindgen {expected}: \
             cargo install wasm-bindgen-cli --version ={expected} --locked --root target/tools",
            bindgen.display()
        ));
    }
    let opt = tool("WASM_OPT", None, "wasm-opt");
    let found = capture(Command::new(&opt).arg("--version"))?;
    if !found.starts_with(&format!("wasm-opt version {BINARYEN}")) {
        return Err(format!("{} is {found}, the package is built with binaryen {BINARYEN}", opt.display()));
    }

    run(release_cargo()?.args([
        "build",
        "-p",
        "gsx-wasm",
        "--profile",
        "wasm",
        "--locked",
        "--target",
        "wasm32-unknown-unknown",
    ]))?;
    let module = build_dir().join("wasm32-unknown-unknown/wasm/gsx_wasm.wasm");
    let bindgen_out = root.join("target/staging/bindgen");
    fresh_dir(&bindgen_out)?;
    run(Command::new(&bindgen)
        .args(["--target", "no-modules", "--no-typescript", "--out-dir"])
        .arg(&bindgen_out)
        .arg(&module))?;

    let dist = root.join("packages/npm/dist");
    fresh_dir(&dist)?;
    run(Command::new(&opt)
        .args(WASM_OPT_FLAGS)
        .arg(bindgen_out.join("gsx_wasm_bg.wasm"))
        .arg("-o")
        .arg(dist.join("gsx.wasm")))?;

    let glue = bindgen_out.join("gsx_wasm.js");
    let glue = read(&glue)?;
    if !glue.starts_with("let wasm_bindgen = ") {
        return Err("the wasm-bindgen glue does not start with `let wasm_bindgen = `".to_owned());
    }
    let template = root.join("packages/npm/src/gsx.js");
    let text = read(&template)?;
    if text.matches(GLUE_MARKER).count() != 1 || text.matches("'@VERSION@'").count() != 1 {
        return Err(format!("{} needs one {GLUE_MARKER} and one '@VERSION@'", template.display()));
    }
    let text = text.replace(GLUE_MARKER, glue.trim_end()).replace("'@VERSION@'", &format!("'{version}'"));
    write(&dist.join("gsx.js"), &text)?;
    copy(&root.join("packages/npm/src/gsx.d.ts"), &dist.join("gsx.d.ts"))?;
    copy(&root.join("schema/gsx-1.schema.json"), &dist.join("gsx-1.schema.json"))?;
    copy(&root.join("LICENSE"), &dist.join("LICENSE"))?;
    copy(&manifest, &dist.join("package.json"))?;
    Ok(dist)
}

fn xcframework() -> Result<PathBuf> {
    let root = root();
    header(true)?;
    let mut libraries = Vec::new();
    for target in APPLE_TARGETS {
        run(ffi_build("staticlib")?.env("MACOSX_DEPLOYMENT_TARGET", MACOS).args(["--target", target]))?;
        libraries.push(build_dir().join(format!("{target}/release/libgsx_ffi.a")));
    }

    let work = root.join("target/staging/xcframework");
    fresh_dir(&work)?;
    let headers = work.join("include/GSXFFI");
    let include = root.join("crates/gsx-ffi/include");
    for file in ["gsx.h", "module.modulemap"] {
        copy(&include.join(file), &headers.join(file))?;
    }
    let library = work.join("libgsx.a");
    lipo(&libraries, &library)?;

    let framework = root.join("packages/swift/GSXFFI.xcframework");
    if framework.exists() {
        fs::remove_dir_all(&framework).map_err(at(framework.display()))?;
    }
    run(Command::new("xcodebuild")
        .arg("-create-xcframework")
        .arg("-library")
        .arg(&library)
        .arg("-headers")
        .arg(work.join("include"))
        .arg("-output")
        .arg(&framework))?;
    Ok(framework)
}

fn wheel() -> Result<PathBuf> {
    let root = root();
    let version = version()?;
    let pyproject = root.join("packages/python/pyproject.toml");
    let text = read(&pyproject)?;
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("{key} = \"")).and_then(|v| v.strip_suffix('"')))
            .ok_or_else(|| format!("{} has no {key}", pyproject.display()))
    };
    if field("version")? != version {
        return Err(format!("{} has version {}, Cargo.toml {version}", pyproject.display(), field("version")?));
    }
    let summary = field("description")?;

    let work = root.join("target/staging/wheel");
    fresh_dir(&work)?;
    let (library, platform) = if cfg!(target_os = "macos") {
        let mut libraries = Vec::new();
        for target in APPLE_TARGETS {
            run(ffi_build("cdylib")?.env("MACOSX_DEPLOYMENT_TARGET", WHEEL_MACOS).args([
                "--target",
                target,
                "--",
                "-C",
                "link-arg=-Wl,-install_name,@rpath/libgsx.dylib",
            ]))?;
            libraries.push(build_dir().join(format!("{target}/release/libgsx_ffi.dylib")));
        }
        let library = work.join("libgsx.dylib");
        lipo(&libraries, &library)?;
        (library, format!("macosx_{}_universal2", WHEEL_MACOS.replace('.', "_")))
    } else if cfg!(target_os = "linux") {
        linux_library(&work)?
    } else {
        return Err(format!("wheels are built on macOS and Linux, not {}", env::consts::OS));
    };

    let tag = format!("py3-none-{platform}");
    let tree = work.join("tree");
    let info = format!("gsx-{version}.dist-info");
    copy(&root.join("packages/python/src/gsx"), &tree.join("gsx"))?;
    copy(&library, &tree.join("gsx").join(library.file_name().ok_or("library has no name")?))?;
    copy(&root.join("LICENSE"), &tree.join(&info).join("licenses/LICENSE"))?;
    write(
        &tree.join(&info).join("METADATA"),
        &format!(
            "Metadata-Version: 2.4\nName: gsx\nVersion: {version}\nSummary: {summary}\n\
             License-Expression: MIT\nLicense-File: LICENSE\n\
             Project-URL: Repository, https://github.com/meritt/gsx.rs\n"
        ),
    )?;
    write(
        &tree.join(&info).join("WHEEL"),
        &format!("Wheel-Version: 1.0\nGenerator: gsx-xtask\nRoot-Is-Purelib: false\nTag: {tag}\n"),
    )?;

    let mut files = files(&tree)?;
    files.retain(|name| !name.contains("__pycache__"));
    let mut record = String::new();
    for name in &files {
        let path = tree.join(name);
        let hex = capture(Command::new("sha256sum").arg(&path))?;
        let hex = hex.split_whitespace().next().ok_or_else(|| format!("shasum {}", path.display()))?;
        let size = fs::metadata(&path).map_err(at(path.display()))?.len();
        record.extend([name.as_str(), ",sha256=", &base64url(hex)?, ",", &size.to_string(), "\n"]);
    }
    let record_name = format!("{info}/RECORD");
    record.extend([record_name.as_str(), ",,\n"]);
    write(&tree.join(&record_name), &record)?;
    files.push(record_name);

    let out = root.join("packages/python/dist");
    fresh_dir(&out)?;
    let wheel = out.join(format!("gsx-{version}-{tag}.whl"));
    zip(&tree, &files, &wheel)?;
    Ok(wheel)
}

fn linux_library(work: &Path) -> Result<(PathBuf, String)> {
    let musl = cfg!(target_env = "musl");
    let mut command = ffi_build("cdylib")?;
    command.args(["--", "-C", "link-arg=-Wl,-soname,libgsx.so"]);
    run(&mut command)?;
    let library = work.join("libgsx.so");
    copy(&build_dir().join("release/libgsx_ffi.so"), &library)?;
    let (family, libc) = if musl {
        let output = Command::new("ldd").output().map_err(at("ldd"))?;
        let text = String::from_utf8_lossy(&output.stderr).into_owned();
        ("musllinux", text.lines().find_map(|line| line.strip_prefix("Version ")).map(str::to_owned))
    } else {
        let text = capture(Command::new("getconf").arg("GNU_LIBC_VERSION"))?;
        ("manylinux", text.strip_prefix("glibc ").map(str::to_owned))
    };
    let libc = libc.ok_or_else(|| format!("no {family} C library version"))?;
    let version: Vec<&str> = libc.split('.').take(2).collect();
    Ok((library, format!("{family}_{}_{}", version.join("_"), env::consts::ARCH)))
}

struct Artifacts {
    wasm_gzip: u64,
    out: PathBuf,
    npm: String,
    xcf: String,
    wheel: String,
    checksum: String,
}

fn artifacts() -> Result<Artifacts> {
    let out = staging()?;
    let cases = cases()?;
    let (npm, wasm_gzip) = npm_package(&out, &cases)?;
    let (xcf, checksum) = swift_package(&out, &cases)?;
    let wheel = python_package(&out, &cases)?;
    Ok(Artifacts { wasm_gzip, out, npm, xcf, wheel, checksum })
}

fn replay(bindings: &[&str]) -> Result {
    if bindings.is_empty() {
        return artifacts().map(drop);
    }
    let out = staging()?;
    let cases = cases()?;
    for binding in bindings {
        match *binding {
            "npm" => npm_package(&out, &cases).map(drop)?,
            "swift" => swift_package(&out, &cases).map(drop)?,
            "python" => python_package(&out, &cases).map(drop)?,
            other => return Err(format!("replay: no binding {other}; npm, swift or python")),
        }
    }
    Ok(())
}

fn staging() -> Result<PathBuf> {
    let out = root().join("target/staging/release");
    fresh_dir(&out)?;
    Ok(out)
}

fn npm_package(out: &Path, cases: &[corpus::Case]) -> Result<(String, u64)> {
    let stage = wasm()?;
    let wasm_gzip = gzip_size(&stage.join("gsx.wasm"))?;
    println!("gsx.wasm: {wasm_gzip} bytes in gzip -9, budget {WASM_GZIP_BUDGET}");
    if wasm_gzip > WASM_GZIP_BUDGET {
        return Err(format!("gsx.wasm is {wasm_gzip} bytes in gzip, over the budget of {WASM_GZIP_BUDGET}"));
    }
    let npm = format!("gsx-v{}-npm.tgz", version()?);
    pack(&stage, &out.join(&npm))?;
    check_npm(&out.join(&npm), cases)?;
    Ok((npm, wasm_gzip))
}

fn swift_package(out: &Path, cases: &[corpus::Case]) -> Result<(String, String)> {
    let framework = xcframework()?;
    let xcf = format!("gsx-v{}-xcframework.zip", version()?);
    let parent = framework.parent().ok_or("the XCFramework has no parent")?;
    let names = files(parent)?.into_iter().filter(|name| name.starts_with("GSXFFI.xcframework/")).collect::<Vec<_>>();
    zip(parent, &names, &out.join(&xcf))?;
    let checksum = capture(Command::new("swift").args(["package", "compute-checksum"]).arg(out.join(&xcf)))?;
    check_swift(&out.join(&xcf), cases)?;
    Ok((xcf, checksum))
}

fn python_package(out: &Path, cases: &[corpus::Case]) -> Result<String> {
    let built = wheel()?;
    let wheel = built.file_name().and_then(|n| n.to_str()).ok_or("the wheel has no name")?.to_owned();
    copy(&built, &out.join(&wheel))?;
    check_python(&out.join(&wheel), cases)?;
    Ok(wheel)
}

fn dist(dir: Option<&Path>, wheels: Option<&Path>) -> Result {
    let version = version()?;
    let dest = dir_or_default(dir, &version)?;
    if dest.exists() && fs::read_dir(&dest).map_err(at(dest.display()))?.next().is_some() {
        return Err(format!("{} is not empty", dest.display()));
    }
    let Artifacts { wasm_gzip, out, npm, xcf, wheel, checksum } = artifacts()?;
    let mut wheel_names = vec![wheel];
    if let Some(wheels) = wheels {
        wheel_names.extend(linux_wheels(wheels, &version, &out)?);
    }

    let files: Vec<&str> =
        [npm.as_str(), xcf.as_str()].into_iter().chain(wheel_names.iter().map(String::as_str)).collect();
    let sums = capture(Command::new("shasum").args(["-a", "256"]).args(&files).current_dir(&out))?;
    write(&out.join("SHA256SUMS"), &format!("{sums}\n"))?;
    let sha = |name: &str| -> Result<String> {
        sums.lines()
            .find_map(|line| line.strip_suffix(name).map(|hash| hash.trim().to_owned()))
            .ok_or_else(|| format!("no SHA-256 for {name}"))
    };
    if checksum != sha(&xcf)? {
        return Err(format!("swift package compute-checksum {xcf} is {checksum}, not its SHA-256"));
    }
    let size = |name: &str| fs::metadata(out.join(name)).map(|m| m.len()).map_err(at(out.join(name).display()));

    let mut entries = vec![
        json!({
            "name": npm,
            "product": "npm",
            "channel": "npm",
            "package": "@meritt/gsx-wasm",
            "wasm_gzip": wasm_gzip,
            "size": size(&npm)?,
            "sha256": sha(&npm)?,
        }),
        json!({
            "name": xcf,
            "product": "xcframework",
            "channel": "swiftpm",
            "module": "GSXFFI",
            "targets": APPLE_TARGETS,
            "macos": MACOS,
            "header": "gsx.h",
            "size": size(&xcf)?,
            "sha256": sha(&xcf)?,
            "swiftpm_checksum": checksum,
        }),
    ];
    for wheel in &wheel_names {
        entries.push(json!({
            "name": wheel,
            "product": "wheel",
            "channel": "pypi",
            "package": "gsx",
            "size": size(wheel)?,
            "sha256": sha(wheel)?,
        }));
    }
    let manifest = json!({
        "name": "gsx",
        "version": version,
        "commit": git(&["rev-parse", "HEAD"])?,
        "dirty": !git(&["status", "--porcelain", "--untracked-files=no"])?
            .is_empty(),
        "files": entries,
        "swift": {
            "package": "GSX",
            "product": "GSX",
            "macos": MACOS,
            "binary_target": format!("{RELEASES}/v{version}/{xcf}"),
            "checksum": checksum,
        },
        "channels": {
            "npm": "https://registry.npmjs.org",
            "swiftpm": "https://github.com/meritt/gsx.rs",
            "pypi": "https://pypi.org/project/gsx",
            "crates.io": "gsx-core",
        },
        "tools": {
            "rustc": capture(rustc().arg("-V"))?,
            "wasm-bindgen": locked_version("wasm-bindgen")?,
            "binaryen": BINARYEN,
            "xcode": capture(Command::new("xcodebuild").arg("-version"))?.lines().next().unwrap_or_default(),
        },
    });
    let manifest = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    write(&out.join("release-manifest.json"), &format!("{manifest}\n"))?;

    fs::create_dir_all(&dest).map_err(at(dest.display()))?;
    for name in files.iter().copied().chain(["SHA256SUMS", "release-manifest.json"]) {
        copy(&out.join(name), &dest.join(name))?;
    }
    println!("{}", dest.display());
    Ok(())
}

fn linux_wheels(dir: &Path, version: &str, out: &Path) -> Result<Vec<String>> {
    let prefix = format!("gsx-{version}-py3-none-");
    let mut names = Vec::new();
    for entry in fs::read_dir(dir).map_err(at(dir.display()))? {
        let name = entry
            .map_err(at(dir.display()))?
            .file_name()
            .into_string()
            .map_err(|name| format!("{}: {} is not UTF-8", dir.display(), name.display()))?;
        let platform = name.strip_prefix(&prefix).and_then(|rest| rest.strip_suffix(".whl"));
        match platform {
            Some(platform) if LINUX_WHEELS.contains(&platform) => names.push(name),
            _ => return Err(format!("{}: {name} is not a gsx {version} Linux wheel", dir.display())),
        }
    }
    names.sort();
    if names.len() != LINUX_WHEELS.len() {
        return Err(format!("{}: {} Linux wheels, expected {}", dir.display(), names.len(), LINUX_WHEELS.len()));
    }
    for name in &names {
        copy(&dir.join(name), &out.join(name))?;
    }
    Ok(names)
}

fn pack(from: &Path, tgz: &Path) -> Result {
    let root = root();
    let staged = root.join("target/staging/npm");
    fresh_dir(&staged)?;
    copy(from, &staged)?;
    let pack = root.join("target/staging/pack");
    fresh_dir(&pack)?;
    run(Command::new("pnpm").args(["pack", "--pack-destination"]).arg(&pack).current_dir(&staged))?;
    let packed = fs::read_dir(&pack)
        .map_err(at(pack.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(at(pack.display()))?;
    let [archive] = packed.as_slice() else {
        return Err("pnpm pack did not produce one archive".to_owned());
    };
    copy(&archive.path(), tgz)
}

fn pin(check: bool, dir: Option<&Path>) -> Result {
    let version = version()?;
    let dir = dir_or_default(dir, &version)?;
    let path = dir.join("release-manifest.json");
    let manifest: Value = read_json(&path)?;
    if manifest["version"] != version.as_str() {
        return Err(format!("{} is not a set of version {version}", dir.display()));
    }
    let file = manifest["files"]
        .as_array()
        .and_then(|files| files.iter().find(|f| f["product"] == "xcframework"))
        .ok_or_else(|| format!("{} lists no xcframework", dir.display()))?;
    let (Some(name), Some(sha), Some(checksum)) =
        (file["name"].as_str(), file["sha256"].as_str(), file["swiftpm_checksum"].as_str())
    else {
        return Err(format!("{} has an incomplete xcframework entry", dir.display()));
    };
    if sha != checksum {
        return Err(format!("{name}: SwiftPM checksum {checksum} is not its SHA-256 {sha}"));
    }
    let path = root().join("Package.swift");
    let text = read(&path)?;
    let pinned = pin_swift(&text, &format!("{RELEASES}/v{version}/{name}"), checksum)?;
    if check {
        if pinned != text {
            return Err(format!("Package.swift does not pin {name} of {}: run `cargo xtask pin`", dir.display()));
        }
        return Ok(());
    }
    write(&path, &pinned)?;
    println!("Package.swift pins {name} {checksum}");
    Ok(())
}

fn dir_or_default(dir: Option<&Path>, version: &str) -> Result<PathBuf> {
    match dir {
        Some(dir) => std::path::absolute(dir).map_err(at(dir.display())),
        None => Ok(root().join(format!("dist/v{version}"))),
    }
}

fn pin_swift(text: &str, url: &str, checksum: &str) -> Result<String> {
    let mut found = (false, false);
    let pinned: Vec<String> = text
        .lines()
        .map(|line| {
            if line.starts_with("    url: \"") && !found.0 {
                found.0 = true;
                format!("    url: \"{url}\",")
            } else if line.starts_with("    checksum: \"") && !found.1 {
                found.1 = true;
                format!("    checksum: \"{checksum}\"")
            } else {
                line.to_owned()
            }
        })
        .collect();
    if found != (true, true) {
        return Err("Package.swift has no `let release = (url:, checksum:)` to pin".to_owned());
    }
    Ok(pinned.join("\n") + "\n")
}

fn corpus_args(command: &mut Command) -> &mut Command {
    command.args(corpus_files())
}

fn corpus_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = CORPUS.iter().map(|file| root().join(file)).collect();
    files.push(root().join(PROJECTION));
    files
}

fn cases() -> Result<Vec<corpus::Case>> {
    let projection = root().join(PROJECTION);
    let parent = projection.parent().ok_or("the projection cases have no directory")?;
    fs::create_dir_all(parent).map_err(at(parent.display()))?;
    corpus::projection(&CORPUS.map(|file| root().join(file)), &root().join(SELECTION), &projection)?;
    corpus::load(&corpus_files())
}

fn check_npm(tgz: &Path, cases: &[corpus::Case]) -> Result {
    let root = root();
    let consumer = root.join("target/staging/consumer/npm");
    fresh_dir(&consumer)?;
    write(&consumer.join("package.json"), "{ \"private\": true, \"type\": \"module\" }\n")?;
    run(Command::new("pnpm").args(["add", "--offline", "--ignore-scripts"]).arg(tgz).current_dir(&consumer))?;
    copy(&root.join("packages/npm/test/replay.mjs"), &consumer.join("replay.mjs"))?;
    let out = consumer.join("replay.jsonl");
    run(corpus_args(Command::new("node").arg("replay.mjs").arg(&out)).current_dir(&consumer))?;
    corpus::compare("npm", cases, &out, false)
}

fn check_swift(zip: &Path, cases: &[corpus::Case]) -> Result {
    let root = root();
    let consumer = root.join("target/staging/consumer/swift");
    fresh_dir(&consumer)?;
    let package = consumer.join("gsx");
    for item in ["Package.swift", "packages/swift/Sources", "packages/swift/Tests"] {
        copy(&root.join(item), &package.join(item))?;
    }
    copy(zip, &package.join("GSXFFI.xcframework.zip"))?;
    let manifest = package.join("Package.swift");
    let text = read(&manifest)?;
    let remote = r#".binaryTarget(name: "GSXFFI", url: release.url, checksum: release.checksum)"#;
    if text.matches(remote).count() != 1 {
        return Err(format!("Package.swift needs one `{remote}`"));
    }
    write(&manifest, &text.replace(remote, r#".binaryTarget(name: "GSXFFI", path: "GSXFFI.xcframework.zip")"#))?;
    run(Command::new("swift").arg("test").arg("--package-path").arg(&package))?;

    let app = consumer.join("app");
    write(
        &app.join("Package.swift"),
        &format!(
            "// swift-tools-version: 6.2\n\
             import PackageDescription\n\n\
             let package = Package(\n    \
                 name: \"App\",\n    \
                 platforms: [.macOS(\"{MACOS}\")],\n    \
                 dependencies: [.package(path: \"../gsx\")],\n    \
                 targets: [.executableTarget(name: \"App\", dependencies: [.product(name: \"GSX\", package: \"gsx\")])]\n\
             )\n"
        ),
    )?;
    copy(&root.join("packages/swift/Replay/main.swift"), &app.join("Sources/App/main.swift"))?;
    let out = consumer.join("replay.jsonl");
    run(corpus_args(
        Command::new("swift").args(["run", "-c", "release", "--package-path"]).arg(&app).arg("App").arg(&out),
    ))?;
    corpus::compare("swift", cases, &out, true)
}

fn check_python(wheel: &Path, cases: &[corpus::Case]) -> Result {
    let root = root();
    let consumer = root.join("target/staging/consumer/python");
    fresh_dir(&consumer)?;
    let python = env::var("GSX_PYTHON").unwrap_or_else(|_| "python3".to_owned());
    let venv = consumer.join("venv");
    run(Command::new(&python).args(["-m", "venv"]).arg(&venv))?;
    let python = venv.join("bin/python");
    run(Command::new(&python).args(["-m", "pip", "install", "--quiet", "--no-index", "--no-deps"]).arg(wheel))?;
    copy(&root.join("packages/python/test/replay.py"), &consumer.join("replay.py"))?;
    let out = consumer.join("replay.jsonl");
    run(corpus_args(Command::new(&python).arg("-I").arg(consumer.join("replay.py")).arg(&out)))?;
    corpus::compare("python", cases, &out, false)
}

fn base64url(hex: &str) -> Result<String> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2).unwrap_or("??"), 16).map_err(|e| format!("{hex}: {e}")))
        .collect::<Result<Vec<u8>>>()?;
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
        }
    }
    Ok(out)
}

fn files(dir: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            for entry in fs::read_dir(&path).map_err(at(path.display()))? {
                stack.push(entry.map_err(at(path.display()))?.path());
            }
        } else {
            let name = path.strip_prefix(dir).map_err(at(path.display()))?;
            files.push(name.to_str().ok_or_else(|| format!("{} is not UTF-8", path.display()))?.to_owned());
        }
    }
    files.sort();
    Ok(files)
}

fn zip(dir: &Path, files: &[String], to: &Path) -> Result {
    let epoch: u64 = SOURCE_DATE_EPOCH.parse().map_err(|e| format!("SOURCE_DATE_EPOCH: {e}"))?;
    let times = fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(epoch));
    for name in files {
        let path = dir.join(name);
        fs::File::open(&path).and_then(|file| file.set_times(times)).map_err(at(path.display()))?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).map_err(at(path.display()))?;
    }
    if to.exists() {
        fs::remove_file(to).map_err(at(to.display()))?;
    }
    let mut command = Command::new("zip");
    command.args(["-X", "-D", "-q", "-@"]).arg(to).current_dir(dir).env("TZ", "UTC").stdin(Stdio::piped());
    eprintln!("+ {}", show(&command));
    let mut child = command.spawn().map_err(at(show(&command)))?;
    let mut stdin = child.stdin.take().ok_or("zip has no stdin")?;
    stdin.write_all(format!("{}\n", files.join("\n")).as_bytes()).map_err(at(show(&command)))?;
    drop(stdin);
    let status = child.wait().map_err(at(show(&command)))?;
    if status.success() { Ok(()) } else { Err(format!("{} failed: {status}", show(&command))) }
}

fn gzip_size(path: &Path) -> Result<u64> {
    Ok(output(Command::new("gzip").arg("-9c").arg(path))?.len() as u64)
}

fn regenerate() -> Result {
    corpus::regenerate(&root().join("testdata/corpus"))?;
    run(cargo()
        .args(["test", "-p", "gsx-core", "--test", "editor", "--locked", "--", "matches_syntax_and_editor_corpus"])
        .env("GSX_BLESS", "1"))
}

fn bench(out: Option<&str>, wasm_build: bool) -> Result {
    let root = root();
    let out = if let Some(out) = out {
        env::current_dir().map_err(|e| e.to_string())?.join(out)
    } else {
        let commit = git(&["rev-parse", "--short=7", "HEAD"])?;
        let name = if wasm_build { "gsx-wasm" } else { "gsx" };
        root.join(format!("testdata/bench/baseline/{name}-{commit}.json"))
    };
    if wasm_build {
        let glue = wasm()?.join("gsx.js");
        run(Command::new("node").arg(root.join("packages/npm/test/bench.mjs")).arg(&glue).arg(&out))?;
    } else {
        run(cargo()
            .args(["bench", "-p", "gsx-core", "--bench", "parse", "--locked"])
            .env("CRITERION_HOME", root.join("target/criterion"))
            .env("GSX_BENCH_REPORT", &out))?;
    }
    run(Command::new("node").arg(root.join("tools/bench/summary.mjs")).arg(&out).arg(out.with_extension("md")))
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("xtask sits in the workspace root").to_path_buf()
}

fn build_dir() -> PathBuf {
    root().join(env::var_os("CARGO_TARGET_DIR").unwrap_or_else(|| "target".into()))
}

fn version() -> Result<String> {
    let text = capture(cargo().args(["metadata", "--format-version", "1", "--no-deps", "--locked"]))?;
    let metadata: Value = serde_json::from_str(&text).map_err(|e| format!("cargo metadata: {e}"))?;
    metadata["packages"]
        .as_array()
        .and_then(|packages| packages.iter().find(|p| p["name"] == "gsx-core"))
        .and_then(|p| p["version"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| "cargo metadata has no gsx-core".to_owned())
}

fn locked_version(name: &str) -> Result<String> {
    let id = capture(cargo().args(["pkgid", "--locked", "-p", name]))?;
    id.rsplit_once('@').map(|(_, version)| version.to_owned()).ok_or_else(|| format!("cargo pkgid {name}: {id}"))
}

fn cargo() -> Command {
    let mut command = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command.current_dir(root());
    command
}

fn rustc() -> Command {
    let mut command = Command::new(env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()));
    command.current_dir(root());
    command
}

fn git(args: &[&str]) -> Result<String> {
    capture(Command::new("git").args(args).current_dir(root()))
}

fn ffi_build(crate_type: &str) -> Result<Command> {
    let mut command = release_cargo()?;
    command.args(["rustc", "-p", "gsx-ffi", "--lib", "--crate-type", crate_type, "--release", "--locked"]);
    Ok(command)
}

fn lipo(libraries: &[PathBuf], output: &Path) -> Result {
    run(Command::new("lipo").arg("-create").args(libraries).arg("-output").arg(output))
}

fn release_cargo() -> Result<Command> {
    let sysroot = capture(rustc().args(["--print", "sysroot"]))?;
    let version = capture(rustc().arg("-vV"))?;
    let commit = version
        .lines()
        .find_map(|line| line.strip_prefix("commit-hash: "))
        .ok_or_else(|| format!("rustc -vV has no commit-hash: {version}"))?;
    let home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .ok_or("neither CARGO_HOME nor HOME is set")?;
    let mut flags = vec![
        format!("--remap-path-prefix={}=/gsx", root().display()),
        format!("--remap-path-prefix={}=/cargo", home.display()),
        format!("--remap-path-prefix={sysroot}/lib/rustlib/src/rust=/rustc/{commit}"),
    ];
    if cfg!(target_env = "musl") {
        flags.push("-Ctarget-feature=-crt-static".to_owned());
    }
    let mut command = cargo();
    command.env("SOURCE_DATE_EPOCH", SOURCE_DATE_EPOCH).env("CARGO_ENCODED_RUSTFLAGS", flags.join("\x1f"));
    Ok(command)
}

fn tool(var: &str, local: Option<PathBuf>, name: &str) -> PathBuf {
    env::var_os(var).map(PathBuf::from).or_else(|| local.filter(|path| path.is_file())).unwrap_or_else(|| name.into())
}

fn run(command: &mut Command) -> Result {
    eprintln!("+ {}", show(command));
    let status = command.status().map_err(at(show(command)))?;
    if status.success() { Ok(()) } else { Err(format!("{} failed: {status}", show(command))) }
}

fn output(command: &mut Command) -> Result<Vec<u8>> {
    let output = command.stderr(Stdio::inherit()).output().map_err(at(show(command)))?;
    if !output.status.success() {
        return Err(format!("{} failed: {}", show(command), output.status));
    }
    Ok(output.stdout)
}

fn capture(command: &mut Command) -> Result<String> {
    String::from_utf8(output(command)?).map(|s| s.trim().to_owned()).map_err(at(show(command)))
}

fn show(command: &Command) -> String {
    let mut words = vec![command.get_program().to_string_lossy()];
    words.extend(command.get_args().map(|arg| arg.to_string_lossy()));
    words.join(" ")
}

fn fresh_dir(path: &Path) -> Result {
    if path.exists() {
        fs::remove_dir_all(path).map_err(at(path.display()))?;
    }
    fs::create_dir_all(path).map_err(at(path.display()))
}

fn copy(from: &Path, to: &Path) -> Result {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(at(parent.display()))?;
    }
    if from.is_dir() {
        fs::create_dir_all(to).map_err(at(to.display()))?;
        for entry in fs::read_dir(from).map_err(at(from.display()))? {
            let entry = entry.map_err(at(from.display()))?;
            copy(&entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(from, to).map(drop).map_err(at(from.display()))
    }
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(at(path.display()))
}

fn read_json(path: &Path) -> Result<Value> {
    serde_json::from_str(&read(path)?).map_err(at(path.display()))
}

fn write(path: &Path, text: &str) -> Result {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(at(parent.display()))?;
    }
    fs::write(path, text).map_err(at(path.display()))
}

fn at<E: Display>(what: impl Display) -> impl FnOnce(E) -> String {
    let what = what.to_string();
    move |error| format!("{what}: {error}")
}
