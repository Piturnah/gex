//! The build script is used to get the most recent commit hash in order to differentiate between
//! dev versions.

use std::process::Command;

fn main() {
    if env!("CARGO_PKG_VERSION").ends_with("-dev") {
        let hash = Command::new("git")
            .args(["log", "HEAD", "--pretty=format:%h %ai", "-n", "1"])
            .output()
            .expect("couldn't get commit hash pointed to by HEAD");

        println!(
            "cargo:rustc-env=GEX_VERSION={} ({})",
            env!("CARGO_PKG_VERSION"),
            std::str::from_utf8(&hash.stdout).expect("malformed stdout from `git log`")
        );
    } else {
        println!("cargo:rustc-env=GEX_VERSION={}", env!("CARGO_PKG_VERSION"));
    }
}
