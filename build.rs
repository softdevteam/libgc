use std::env;
use std::path::PathBuf;
use std::process::Command;

use which::which;

const BOEHM_REPO: &str = "https://github.com/ivmai/bdwgc.git";
const BOEHM_ATOMICS_REPO: &str = "https://github.com/ivmai/libatomic_ops.git";
const BOEHM_DIR: &str = "bdwgc";
const BUILD_DIR: &str = ".libs";

#[cfg(not(all(target_pointer_width = "64", target_arch = "x86_64")))]
compile_error!("Requires x86_64 with 64 bit pointer width.");
static POINTER_MASK: &str = "-DPOINTER_MASK=0xFFFFFFFFFFFFFFF8";

fn run<F>(name: &str, mut configure: F)
where
    F: FnMut(&mut Command) -> &mut Command,
{
    let mut command = Command::new(name);
    let configured = configure(&mut command);
    if !configured.status().is_ok() {
        panic!("failed to execute {:?}", configured);
    }
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let mut boehm_src = PathBuf::from(out_dir);
    boehm_src.push(BOEHM_DIR);

    if !boehm_src.exists() {
        run("git", |cmd| {
            cmd.arg("clone").arg(BOEHM_REPO).arg(&boehm_src)
        });

        run("git", |cmd| {
            cmd.arg("clone")
                .arg(BOEHM_ATOMICS_REPO)
                .current_dir(&boehm_src)
        });

        env::set_current_dir(&boehm_src).unwrap();

        run("./autogen.sh", |cmd| cmd);
        run("./configure", |cmd| {
            cmd.arg("--enable-static")
                .arg("--disable-shared")
                .env("CFLAGS", POINTER_MASK)
        });

        let cpus = num_cpus::get();
        let make_bin = match which("gmake") {
            Ok(_) => "gmake",
            Err(_) => "make",
        };
        run(make_bin, |cmd| cmd.arg("-j").arg(format!("{}", cpus)));
    }

    let mut libpath = PathBuf::from(&boehm_src);
    libpath.push(BUILD_DIR);

    println!(
        "cargo:rustc-link-search=native={}",
        &libpath.as_path().to_str().unwrap()
    );
    println!("cargo:rustc-link-lib=static=gc");
}
