mod cargo;
mod config;
mod rust;

pub use self::cargo::CargoTarget;
use self::{cargo::CargoConfig, config::interactive_config_gen};
use crate::{
    android, ios,
    util::{self, FriendlyContains, IntoResult},
    Config,
};
use std::{path::Path, process::Command};

pub static STEPS: &'static [&'static str] = &["deps", "cargo", "android", "ios"];

#[derive(Debug)]
struct Skip {
    pub deps: bool,
    pub cargo: bool,
    pub hello_world: bool,
    pub android: bool,
    pub ios: bool,
}

impl From<Vec<String>> for Skip {
    fn from(skip: Vec<String>) -> Self {
        Skip {
            deps: skip.friendly_contains("deps"),
            cargo: skip.friendly_contains("cargo"),
            hello_world: skip.friendly_contains("hello_world"),
            android: skip.friendly_contains("android"),
            ios: skip.friendly_contains("ios"),
        }
    }
}

// TODO: Don't redo things if no changes need to be made
pub fn init(bike: &bicycle::Bicycle, force: bool, skip: Vec<String>) {
    if !Config::exists() {
        interactive_config_gen(bike);
        Config::recheck_path();
    }
    let skip = Skip::from(skip);
    if !skip.deps {
        install_deps(force);
    }
    if !skip.cargo {
        CargoConfig::generate().write();
    }
    if !skip.hello_world {
        rust::hello_world(bike, force).unwrap();
    }
    if !skip.android {
        android::project::create(bike).unwrap();
    }
    if !skip.ios {
        ios::project::create(bike).unwrap();
    }
}

// TODO: We should probably also try to install `rust-xcode-plugin`
fn install_deps(force: bool) {
    let xcodegen_found = util::command_present("xcodegen").expect("Failed to check for `xcodegen`");
    if !xcodegen_found || force {
        Command::new("brew")
            // reinstall works even if it's not installed yet,
            // and will upgrade if it's already installed!
            .args(&["reinstall", "xcodegen"])
            .status()
            .into_result()
            .expect("Failed to install `xcodegen`");
    }

    // Installing `ios-deploy` normally involves npm, even though it doesn't
    // use JavaScript at all... so, let's build it manually!
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dest = root.join("ios-deploy");
    let ios_deploy_found = dest.join("build/Release/ios-deploy").exists();
    if !ios_deploy_found || force {
        if dest.exists() {
            // this checkout is no longer necessary, but existing working trees
            // are still dirty from before... so, we can phase this out in the near future!
            util::git(&dest, &["checkout", "HEAD", "--", "ios-deploy.xcodeproj"])
                .expect("Failed to reset `ios-deploy` repo");
            util::git(&dest, &["pull", "--rebase", "origin", "master"])
                .expect("Failed to pull `ios-deploy` repo");
        } else {
            util::git(
                &root,
                &[
                    "clone",
                    "--depth",
                    "1",
                    "https://github.com/ios-control/ios-deploy",
                ],
            )
            .expect("Failed to checkout `ios-deploy` repo");
        }
        let project = dest.join("ios-deploy.xcodeproj");
        Command::new("xcodebuild")
            .arg("-project")
            .arg(&project)
            .status()
            .into_result()
            .expect("Failed to build `ios-deploy`");
    }
}
