use anyhow::{bail, Result};
use flate2::bufread::GzDecoder;
use progress_streams::ProgressReader;
use serde::Deserialize;
use std::{
    env::consts::ARCH,
    io::{Read, Seek, SeekFrom, Write},
};

const CURRENT_VERSION_DIRECTORY: &str = "/var/lib/vsdown/";
const CURRENT_VERSION_FILENAME: &str = "current_bersion";
const ANITYA_URL: &str = "https://release-monitoring.org/api/v2/versions/?project_id=243355";
const DOWNLOAD_VSCODE_URL: &str = "https://code.visualstudio.com/sha/download?build=stable&os=";
const VSCODE_PATH: &str = "/usr/lib";

#[derive(Deserialize)]
struct AnityaVersion {
    latest_version: String,
}

pub fn update_checker() -> Result<()> {
    let lastest_version = get_lastest_version()?;
    let current_version = get_current_version().unwrap_or_else(|_| {
        std::fs::create_dir_all(CURRENT_VERSION_DIRECTORY).ok();
        let mut f = std::fs::File::create(format!(
            "{}{}",
            CURRENT_VERSION_DIRECTORY, CURRENT_VERSION_FILENAME
        ))
        .unwrap();
        f.write_all(b"None").unwrap();
        drop(f);

        "None".to_string()
    });
    if current_version != lastest_version {
        bail!("Current version and lastest version not match!\ncurrent version: {}\nlastest version: {}", current_version, lastest_version)
    }

    Ok(())
}

fn get_lastest_version() -> Result<String> {
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
        .replace("\n", "")
        .replace(" ", "");

    Ok(s)
}

pub fn download_vscode() -> Result<()> {
    let arch = match ARCH {
        "x86_64" => "linux-x64",
        "aarch64" => "linux-arm64",
        _ => bail!("VSCode unsupport this arch!"),
    };
    let mut r =
        reqwest::blocking::get(format!("{}{}", DOWNLOAD_VSCODE_URL, arch))?.error_for_status()?;
    let length = r.content_length().unwrap();
    let progress_bar = indicatif::ProgressBar::new(length);
    progress_bar.enable_steady_tick(500);
    progress_bar.set_message("Downloading newest vscode tarball ...");
    let mut reader = ProgressReader::new(&mut r, |progress: usize| {
        progress_bar.inc(progress as u64);
    });
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    progress_bar.finish_with_message("Decoding vscode xz tarball ...");
    let d = GzDecoder::new(&*buf);
    let mut tar = tar::Archive::new(d);
    tar.set_preserve_permissions(true);
    tar.set_preserve_ownerships(true);
    tar.unpack(VSCODE_PATH)?;
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