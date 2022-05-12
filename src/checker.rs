use anyhow::{anyhow, bail, Result};
use console::style;
use flate2::bufread::GzDecoder;
use progress_streams::ProgressReader;
use serde::Deserialize;
use std::{
    env::consts::ARCH,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::info;

const CURRENT_VERSION_DIRECTORY: &str = "/var/lib/vsdown/";
const CURRENT_VERSION_FILENAME: &str = "current_version";
const ANITYA_URL: &str = "https://release-monitoring.org/api/v2/versions/?project_id=243355";
const DOWNLOAD_VSCODE_URL: &str = "https://code.visualstudio.com/sha/download?build=stable&os=";
const VSCODE_PATH: &str = "/usr/lib";

const CODE_APPDATA_XML: &[u8] = include_bytes!("../res/code.appdata.xml");
const CODE_DESKTOP: &[u8] = include_bytes!("../res/code.desktop");
const CODE_URL_HANDLER_DESKTOP: &[u8] = include_bytes!("../res/code-url-handler.desktop");
const CODE_WORKSPACE_XML: &[u8] = include_bytes!("../res/code-workspace.xml");
const VSCODE_ICON: &[u8] = include_bytes!("../res/com.visualstudio.code.png");
const PATH_KV: &[(&str, &[u8])] = &[
    ("/usr/share/appdata/code.appdata.xml", CODE_APPDATA_XML),
    ("/usr/share/applications/code.desktop", CODE_DESKTOP),
    (
        "/usr/share/applications/code-url-handler.desktop",
        CODE_URL_HANDLER_DESKTOP,
    ),
    (
        "/usr/share/mine/packages/code-workspace.xml",
        CODE_WORKSPACE_XML,
    ),
    ("/usr/share/pixmaps/com.visualstudio.code.png", VSCODE_ICON),
];

const DIRECTORY_PATH: &[&str] = &[
    "/usr/share/appdata",
    "/usr/share/applications",
    "/usr/share/mine/packages",
    "/usr/share/pixmaps",
];

#[derive(Deserialize)]
struct AnityaVersion {
    latest_version: String,
}

macro_rules! make_progress_bar {
    ($msg:expr) => {
        concat!(
            "{spinner} [{bar:25.cyan/blue}] ",
            $msg,
            " ({bytes_per_sec}, eta {eta})"
        )
    };
}

pub fn update_checker() -> Result<()> {
    let lastest_version = get_lastest_version()?;
    let current_version = match get_current_version() {
        Ok(v) => v,
        Err(_) => {
            info!("You have no vsdown current version log! creating ...");
            std::fs::create_dir_all(CURRENT_VERSION_DIRECTORY)?;
            let mut f = std::fs::File::create(format!(
                "{}{}",
                CURRENT_VERSION_DIRECTORY, CURRENT_VERSION_FILENAME
            ))?;
            f.write_all(b"None")?;
            drop(f);

            "None".to_string()
        }
    };
    if current_version != lastest_version {
        bail!("Current version and lastest version not match! current version: {}, lastest version: {}", current_version, lastest_version)
    }

    Ok(())
}

fn get_lastest_version() -> Result<String> {
    info!("Getting vscode lastest version info ...");
    let json = reqwest::blocking::get(ANITYA_URL)?
        .error_for_status()?
        .json::<AnityaVersion>()?;

    Ok(json.latest_version)
}

fn get_current_version() -> Result<String> {
    let mut vsdown_ver_log = std::fs::File::open(format!(
        "{}{}",
        CURRENT_VERSION_DIRECTORY, CURRENT_VERSION_FILENAME
    ))?;
    let mut buf = Vec::new();
    vsdown_ver_log.read_to_end(&mut buf)?;
    if buf.is_empty() {
        bail!("Can not get current version!")
    }
    let s = std::str::from_utf8(&buf)?
        .to_string()
        .replace('\n', "")
        .replace(' ', "");

    Ok(s)
}

pub fn download_vscode() -> Result<()> {
    let arch = match ARCH {
        "x86_64" => "linux-x64",
        "aarch64" => "linux-arm64",
        _ => bail!("VSCode unsupport this arch!"),
    };
    info!("Downloading newest vscode tarball ...");
    let mut r =
        reqwest::blocking::get(format!("{}{}", DOWNLOAD_VSCODE_URL, arch))?.error_for_status()?;
    let length = r.content_length().unwrap_or(0);
    let progress_bar = indicatif::ProgressBar::new(length);
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template(make_progress_bar!("{bytes}/{total_bytes}")),
    );
    progress_bar.enable_steady_tick(500);
    let mut reader = ProgressReader::new(&mut r, |progress: usize| {
        progress_bar.inc(progress as u64);
    });
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    progress_bar.finish_and_clear();
    info!("Download finished! Decoding vscode xz tarball ...");
    let d = GzDecoder::new(&*buf);
    let mut tar = tar::Archive::new(d);
    tar.set_preserve_permissions(true);
    tar.set_preserve_ownerships(true);
    tar.unpack(VSCODE_PATH)?;
    let p = Path::new("/usr/lib/vscode");
    if p.is_dir() {
        std::fs::remove_dir_all(p)?;
    }
    std::fs::rename(format!("/usr/lib/VSCode-{}", arch), p)?;
    install_beyond()?;
    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(format!(
            "{}{}",
            CURRENT_VERSION_DIRECTORY, CURRENT_VERSION_FILENAME
        ))?;
    f.seek(SeekFrom::Start(0))?;
    f.write_all(get_lastest_version()?.as_bytes())?;

    Ok(())
}

fn install_beyond() -> Result<()> {
    remove_vscode()?;
    let p = Path::new("/usr/bin/vscode");
    std::os::unix::fs::symlink("/usr/lib/vscode/code", p)
        .map_err(|e| anyhow!("Could not create symlink! {}", e))?;
    info!("Installing metadata file ...");
    for i in DIRECTORY_PATH {
        std::fs::create_dir_all(i)
            .map_err(|e| anyhow!("Could not create directory {}, {}", i, e))?;
    }
    for (p, b) in PATH_KV {
        install_file_inner(p, b)?;
    }

    Ok(())
}

fn install_file_inner(p: &str, buf: &[u8]) -> Result<()> {
    let p = Path::new(p);
    if !p.exists() {
        let mut f = std::fs::File::create(p)?;
        f.write_all(buf)?;
    }

    Ok(())
}

pub fn remove_vscode() -> Result<()> {
    info!("Removing vscode ...");
    for (i, _) in PATH_KV {
        std::fs::remove_file(i)?;
    }
    std::fs::remove_dir_all("/usr/lib/vscode")?;
    std::fs::remove_file("/usr/bin/vscode")?;
    std::fs::remove_file(format!(
        "{}{}",
        CURRENT_VERSION_DIRECTORY, CURRENT_VERSION_FILENAME
    ))?;

    Ok(())
}
