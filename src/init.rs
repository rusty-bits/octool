use std::error::Error;
use std::io::{Read, Stdout, Write};
use std::path::{Path, PathBuf};

use plist::Value;

use crate::draw::Settings;
use crate::edit::{find, Found};
use crate::res::{self, Resources};

use crossterm::terminal;

/// load static resources into resources struct, shouldn't need to change even if user
/// changes opencore build version on the fly
pub fn init_static(
    resources: &mut Resources,
    settings: &mut Settings,
    stdout: &mut Stdout,
) -> Result<(), Box<dyn Error>> {
    //load octool config file
    resources.octool_config = res::get_serde_json("tool_config_files/octool_config.json", stdout)?;

    //load other resource file
    resources.other = res::get_serde_json("tool_config_files/other.json", stdout)?;

    // build resource section vector
    let config_res_sections: Vec<(String, String, String, String)> =
        serde_json::from_value(resources.octool_config["resource_sections"].clone()).unwrap();
    for (mut sec, sub, _, _) in config_res_sections {
        sec.push_str(&sub);
        settings.resource_sections.push(sec);
    }

    //load resources list file
    resources.resource_list = res::get_serde_json("tool_config_files/resource_list.json", stdout)?;

    //load dortania build_repo package
    write!(
        stdout,
        "\r\n\x1B[32mChecking\x1B[0m local dortania/build_repo/config.json\r\n"
    )?;
    let path = Path::new(
        resources.octool_config["dortania_config_path"]
            .as_str()
            .unwrap(),
    );
    let url = resources.octool_config["dortania_config_zip"]
        .as_str()
        .unwrap();
    if !path.exists() {
        write!(stdout, "\x1b[32mDownloading\x1B[0m latest config.json ... ")?;
        stdout.flush().unwrap();
        let path = path.parent().unwrap().join("builds.zip");
        res::curl_file(&url, &path)?;
        let z_file = std::fs::File::open(&path)?;
        let mut z_archive = zip::ZipArchive::new(z_file)?;
        match z_archive.extract(&path.parent().unwrap()) {
            Ok(_) => std::fs::remove_file(&path)?,
            Err(e) => panic!("{:?}", e),
        }
        write!(stdout, "\x1b[32mdone\x1b[0m\r\n")?
    } else {
        let path = path.parent().unwrap();
        let last_updated = resources.octool_config["dortania_last_updated"]
            .as_str()
            .unwrap();
        res::curl_file(&last_updated, &path.join("last.txt"))?;
        let mut old_date = Vec::new();
        let mut new_file = std::fs::File::open(&path.join("last.txt"))?;
        let mut new_date = Vec::new();
        new_file.read_to_end(&mut new_date).unwrap();
        std::fs::remove_file(&path.join("last.txt"))?;
        let last_up_path = &path.join("build-repo-builds/last_updated.txt");
        if last_up_path.exists() {
            let mut old_file = std::fs::File::open(&last_up_path)?;
            old_file.read_to_end(&mut old_date)?;
        };
        if old_date != new_date {
            write!(stdout, "\x1b[32mDownloading\x1B[0m latest config.json ... ")?;
            stdout.flush().unwrap();
            let path = path.join("builds.zip");
            res::curl_file(&url, &path)?;
            let z_file = std::fs::File::open(&path)?;
            let mut z_archive = zip::ZipArchive::new(z_file)?;
            match z_archive.extract(&path.parent().unwrap()) {
                Ok(_) => std::fs::remove_file(&path)?,
                Err(e) => panic!("{:?}", e),
            }
            write!(stdout, "\x1b[32mdone\x1b[0m\r\n")?
        } else {
            write!(stdout, "Already up to date.\r\n")?;
        }
    };
    resources.dortania = res::get_serde_json(path.join("config.json").to_str().unwrap(), stdout)?;

    Ok(())
}

/// load OpenCore binary packages and support files based on the version of
/// OpenCore that is selected, will change resources used on the fly if user
/// uses the 'V' command to change OC version #
pub fn init_oc_build(
    resources: &mut Resources,
    settings: &mut Settings,
    stdout: &mut Stdout,
) -> Result<(), Box<dyn Error>> {
    settings.oc_build_version_res_index = Default::default(); // reset oc_build_version to top of dortania
    settings.resource_ver_indexes = Default::default(); // clear out resource version indexes for dortania

    // test if version selected is latest version, don't try to download zip of latest
    // it doesn't exist yet
    let latest_ver = resources.dortania["OpenCorePkg"]["versions"][0]["version"]
        .as_str()
        .unwrap();
    if latest_ver == &settings.oc_build_version {
        settings.oc_build_version = "latest".to_owned();
    }

    if settings.oc_build_version == "latest" {
        settings.oc_build_version = resources.dortania["OpenCorePkg"]["versions"][0]["version"]
            .as_str()
            .unwrap()
            .to_owned();
        let path = Path::new(
            resources.octool_config["opencorepkg_path"]
                .as_str()
                .unwrap(),
        );
        resources.open_core_source_path = Path::new(&path).to_path_buf();
        let path = path.join("Docs");
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
        let url = resources.octool_config["current_configuration_tex"]
            .as_str()
            .unwrap();
        res::curl_file(&url, &path.join(&url.split('/').last().unwrap()))?;
        let url = resources.octool_config["current_sample_plist"]
            .as_str()
            .unwrap();
        res::curl_file(&url, &path.join(&url.split('/').last().unwrap()))?;
    } else {
        loop {
            if let Some(v) = resources.dortania["OpenCorePkg"]["versions"]
                [settings.oc_build_version_res_index]["version"]
                .as_str()
            {
                if v == settings.oc_build_version {
                    break;
                }
            } else {
                write!(
                    stdout,
                    "\r\n\x1b[33mERROR:\x1b[0m Version \x1b[32m{}\x1b[0m of OpenCorePkg not found in repos, please check your input\r\n\ne.g. './octool -o \x1b[4m0.7.4\x1b[0m'\n",
                    settings.oc_build_version
                )
                .unwrap();
                settings.oc_build_version = "not found".to_owned();
                return Ok(());
            }
            settings.oc_build_version_res_index += 1;
        }
        let mut path = "resources/OpenCorePkg-".to_owned();
        path.push_str(&settings.oc_build_version);
        resources.open_core_source_path = Path::new(&path).to_path_buf();
        path.push_str(".zip");
        let path = Path::new(&path).to_path_buf();

        let mut url = "https://github.com/acidanthera/OpenCorePkg/archive/refs/tags/".to_owned();
        url.push_str(&settings.oc_build_version);
        url.push_str(".zip");
        write!(
            stdout,
            "\x1B[32mChecking\x1B[0m OpenCorePkg {} source\r\n",
            settings.oc_build_version
        )?;
        if !resources.open_core_source_path.exists() {
            write!(
                stdout,
                "\x1B[32mDownloading\x1B[0m OpenCorePkg {} source from Acidanthera ... ",
                settings.oc_build_version
            )?;
            stdout.flush()?;
            res::get_file_and_unzip(&url, "", &path, stdout)?;
            write!(stdout, "\x1B[32mDone\x1B[0m\r\n")?;
        } else {
            write!(stdout, "Already up to date.\r\n")?;
        }
    }

    settings.resource_ver_indexes.insert(
        "OpenCorePkg".to_owned(),
        settings.oc_build_version_res_index,
    );
    settings.oc_build_date = resources.dortania["OpenCorePkg"]["versions"]
        [settings.oc_build_version_res_index]["date_built"]
        .as_str()
        .unwrap_or("")
        .to_owned();

    let sample_plist = &resources.open_core_source_path.join("Docs/Sample.plist");
    resources.sample_plist = Value::from_file(sample_plist)
        .expect(format!("Didn't find Sample.plist at {:?}", sample_plist).as_str());

    write!(
        stdout,
        "\r\n\x1B[32mChecking\x1B[0m local OpenCorePkg {} binaries\r\n",
        settings.oc_build_version
    )?;
    let path = res::get_or_update_local_parent(
        "OpenCorePkg",
        &resources.dortania,
        &settings.build_type,
        settings.resource_ver_indexes.get("OpenCorePkg").unwrap(),
        true,
        stdout,
    )?;

    match path {
        Some(p) => resources.open_core_binaries_path = p.parent().unwrap().to_path_buf(),
        _ => panic!("no OpenCorePkg found"),
    }

    Ok(())
}

/// load config.plist or use a Sample.plist if no valid INPUT plist given
/// and run plist through ocvalidate
pub fn init_plist(
    config_plist: &mut PathBuf,
    resources: &mut Resources,
    settings: &mut Settings,
    stdout: &mut Stdout,
) -> Result<(), Box<dyn Error>> {
    if !config_plist.exists() {
        *config_plist = resources
            .open_core_source_path
            .join("Docs/Sample.plist")
            .to_owned();
        resources.config_plist = Value::from_file(&config_plist)
            .expect(format!("Didn't find valid plist at {:?}", config_plist).as_str());
    }

    write!(
        stdout,
        "\n\x1B[32mValidating\x1B[0m {:?} with {} acidanthera/ocvalidate\r\n",
        config_plist, settings.oc_build_version,
    )?;
    validate_plist(&config_plist, &resources, stdout)?;

    //finish configuring settings
    settings.config_file_name = config_plist.to_str().unwrap().to_owned();
    settings.sec_length[0] = resources.config_plist.as_dictionary().unwrap().keys().len();
    let mut found_key = false;
    let keys: Vec<String> = resources
        .config_plist
        .as_dictionary()
        .unwrap()
        .keys()
        .map(|s| s.to_string())
        .collect();
    for (i, k) in keys.iter().enumerate() {
        if !found_key {
            //highlight first key that is not commented out #
            if !k.starts_with('#') {
                settings.sec_num[0] = i;
                found_key = true;
            }
        }
    }

    write!(
        stdout,
        "\x1B[32mbuild_type set to\x1B[0m {}\r\n\x1B[32mbuild_version set to\x1B[0m {}\r\n",
        settings.build_type, settings.oc_build_version,
    )?;

    Ok(())
}

/// run loaded config.plist through the corresponding ocvalidate utility if it
/// exists (no ocvalidate before oc 0.6.0, may be no ocvalidate available depending
/// on what OS is currently being run)
pub fn validate_plist(
    config_plist: &PathBuf,
    resources: &Resources,
    stdout: &mut Stdout,
) -> Result<bool, Box<dyn Error>> {
    let mut config_okay = true;
    let ocvalidate_bin = resources
        .open_core_binaries_path
        .join("Utilities/ocvalidate")
        .join(match std::env::consts::OS {
            "macos" => "ocvalidate",
            "windows" => "ocvalidate.exe",
            "linux" => "ocvalidate.linux",
            _ => "ocvalidate",
        });
    if ocvalidate_bin.exists() {
        let out = res::status(
            ocvalidate_bin.to_str().unwrap(),
            &[&config_plist.to_str().unwrap()],
        )?;
        terminal::disable_raw_mode()?;

        write!(stdout, "{}\r\n", String::from_utf8(out.stdout).unwrap())?;
        terminal::enable_raw_mode()?;

        if out.status.code().unwrap() != 0 {
            config_okay = false;
            write!(
                stdout,
                "\x1B[31mERROR: Problems(s) found in config.plist!\x1B[0m\r\n"
            )?;
            write!(stdout, "{}\r\n", String::from_utf8(out.stderr).unwrap())?;
        }
    } else {
        write!(
            stdout,
            "\r\n{:?}\r\n\x1b[33mocvalidate utility not found, skipping.\x1b[0m\r\n",
            ocvalidate_bin,
        )?;
    }
    Ok(config_okay)
}

/// run through vec of "config_differences" from tool_config_files/octool_config.json
/// if the current config.plist being worked on contains the field in the vec then
/// it is most likely to be the correct version of OpenCore
pub fn guess_version(resources: &Resources) -> String {
    let mut found = vec![Found::new()];
    let config_differences: Vec<(String, String, String, String)> =
        serde_json::from_value(resources.octool_config["config_differences"].clone()).unwrap();

    for (sec, sub, search, ver) in config_differences {
        find(&search, &resources.config_plist, &mut found);
        for result in &found {
            if result.keys.contains(&sec) && result.keys.contains(&sub) {
                return ver.to_owned();
            }
        }
    }
    "".to_string()
}
