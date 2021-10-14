use std::error::Error;
use std::io::{Stdout, Write};
use std::path::{Path, PathBuf};

use plist::Value;
use termion::raw::RawTerminal;

use crate::draw::Settings;
use crate::res::{self, Resources};

pub fn init(
    config_plist: &PathBuf,
    resources: &mut Resources,
    settings: &mut Settings,
    stdout: &mut RawTerminal<Stdout>,
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
        "\n\x1B[32mChecking local\x1B[0m dortania/build_repo/config.json\r\n"
    )?;
    let path = Path::new(
        resources.octool_config["dortania_config_path"]
            .as_str()
            .unwrap(),
    );
    let url = resources.octool_config["dortania_config_url"]
        .as_str()
        .unwrap();
    let branch = resources.octool_config["dortania_config_branch"]
        .as_str()
        .unwrap();
    res::clone_or_pull(url, path, branch, stdout)?;
    resources.dortania = res::get_serde_json(
        path.parent().unwrap().join("config.json").to_str().unwrap(),
        stdout,
    )?;

    // test if version selected is latest version, don't try to download zip of latest
    // it doesn't exist yet, clone it instead
    let test_ver = resources.dortania["OpenCorePkg"]["versions"][0]["version"]
        .as_str()
        .unwrap();
    if test_ver == &settings.oc_build_version {
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
        let url = resources.octool_config["opencorepkg_url"].as_str().unwrap();
        let branch = resources.octool_config["opencorepkg_branch"]
            .as_str()
            .unwrap();
        res::clone_or_pull(url, path, branch, stdout)?;
        resources.open_core_source_path = Path::new(&path).parent().unwrap().to_path_buf();
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
                    "Version {} of OpenCorePkg not found in repos, please check your input",
                    settings.oc_build_version
                )
                .unwrap();
                settings.oc_build_version = "not found".to_owned();
                return Ok(());
            }
            settings.oc_build_version_res_index += 1;
        }
        let mut path = "tool_config_files/OpenCorePkg-".to_owned();
        path.push_str(&settings.oc_build_version);
        resources.open_core_source_path = Path::new(&path).to_path_buf();
        path.push_str(".zip");
        let path = Path::new(&path).to_path_buf();

        let mut url = "https://github.com/acidanthera/OpenCorePkg/archive/refs/tags/".to_owned();
        url.push_str(&settings.oc_build_version);
        url.push_str(".zip");
        if !resources.open_core_source_path.exists() {
            res::get_file_and_unzip(&url, "", &path, stdout)?;
        }
    }

    settings
        .resource_ver_indexes
        .insert("OpenCorePkg".to_owned(), settings.oc_build_version_res_index);
settings.oc_build_date = resources.dortania["OpenCorePkg"]["versions"][settings.oc_build_version_res_index]["date_built"].as_str().unwrap_or("").to_owned();

    write!(
        stdout,
        "\x1B[32mbuild_type set to\x1B[0m {}\r\n\x1B[32mbuild_version set to\x1B[0m {}\r\n",
        settings.build_type, settings.oc_build_version,
    )?;

    resources.config_plist = Value::from_file(&config_plist)
        .expect(format!("Didn't find valid plist at {:?}", config_plist).as_str());
    let sample_plist = &resources.open_core_source_path.join("Docs/Sample.plist");
    resources.sample_plist = Value::from_file(sample_plist)
        .expect(format!("Didn't find Sample.plist at {:?}", sample_plist).as_str());
//    resources.acidanthera =
//        res::get_serde_json("tool_config_files/acidanthera_config.json", stdout)?;

    write!(stdout, "\r\n\x1B[32mChecking\x1B[0m local OpenCorePkg\r\n")?;
    let path = res::get_or_update_local_parent(
        "OpenCorePkg",
        &resources.dortania,
        &settings.build_type,
        settings.resource_ver_indexes.get("OpenCorePkg").unwrap(),
        stdout,
    )?;

    match path {
        Some(p) => resources.open_core_binaries_path = p.parent().unwrap().to_path_buf(),
        _ => panic!("no OpenCorePkg found"),
    }

    write!(
        stdout,
        "\n\x1B[32mValidating\x1B[0m {:?} with latest acidanthera/ocvalidate\r\n",
        config_plist
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
    Ok(())
}

pub fn validate_plist(
    config_plist: &PathBuf,
    resources: &Resources,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<bool, Box<dyn Error>> {
    let mut config_okay = true;
    let out = res::status(
        resources
            .open_core_binaries_path
            .join("Utilities/ocvalidate/ocvalidate")
            .to_str()
            .unwrap(),
        &[&config_plist.to_str().unwrap()],
    )?;
    stdout.suspend_raw_mode()?;
    write!(stdout, "{}\r\n", String::from_utf8(out.stdout).unwrap())?;
    stdout.activate_raw_mode()?;
    if out.status.code().unwrap() != 0 {
        config_okay = false;
        write!(
            stdout,
            "\x1B[31mERROR: Problems(s) found in config.plist!\x1B[0m\r\n"
        )?;
        write!(stdout, "{}\r\n", String::from_utf8(out.stderr).unwrap())?;
    }
    Ok(config_okay)
}
