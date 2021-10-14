use crate::draw::Settings;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read, Stdout, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use termion::raw::RawTerminal;
use termion::{color, style};

use sha2::Digest;

pub struct Resources {
//    pub acidanthera: serde_json::Value, // Acidanthera parent child json
    pub dortania: serde_json::Value,    // Dortania builds config.json file
    pub octool_config: serde_json::Value, // config file for octool itself
    pub resource_list: serde_json::Value, // list linking resources to their parents
    pub other: serde_json::Value,       // list of other party parent/childs
    pub config_plist: plist::Value,     // current active config.plist
    pub sample_plist: plist::Value,     // latest Sample.plist
    pub working_dir_path: PathBuf,      // location of octool and files
    pub open_core_binaries_path: PathBuf, // location of the OpenCorePkg binariesg
    pub open_core_source_path: PathBuf, // location of OpenCore source files
                                        //    pub resource_ver_indexes: HashMap<&'static str, usize>,
}

pub fn get_or_update_local_parent(
    parent: &str,
    single_resource: &serde_json::Value,
    build_type: &str,
    build_index: &usize,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<Option<PathBuf>, Box<dyn Error>> {
    let url = single_resource[parent]["versions"][build_index]["links"][build_type]
        .as_str()
        .unwrap_or("");
    if url == "" {
        return Ok(None);
    }
    let hash = single_resource[parent]["versions"][build_index]["hashes"][build_type]["sha256"]
        .as_str()
        .unwrap_or("");
    /*    write!(
            stdout,
            "\x1B[32mlocal\x1B[0m [{}] {}\x1B[0K ",
            build_type, parent
        )?;
    */
    let path = Path::new("resources");
    let mut dir = Path::new(url).file_stem().unwrap().to_str().unwrap();
    if dir.ends_with(".kext") {
        dir = &dir[0..dir.len() - 5];
    }
    let file_name = Path::new(url).file_name().unwrap();
    let sum_file = path.join(dir).join("sum256");
    let git_file = path.join(dir).join(".git");
    let path = path.join(dir).join(file_name);

    match url.split('.').last().unwrap() {
        "zip" => match File::open(&sum_file) {
            Ok(mut sum_file) => {
                let mut sum = String::new();
                sum_file.read_to_string(&mut sum)?;
                if sum != hash {
                    write!(
                        stdout,
                        "remote hash {}\x1B[0K\r\n  local sum {}\x1B[0K\r\n",
                        hash, sum
                    )?;
                    write!(
                        stdout,
                        "{yel}new version found, {grn}Downloading\x1B[0m\x1B[0K\r\n",
                        yel = color::Fg(color::Yellow),
                        grn = color::Fg(color::Green),
                    )?;
                    get_file_and_unzip(url, hash, &path, stdout)?;
                } /*  else {
                      write!(stdout, "Already up to date.\x1B[0K\r\n")?;
                  }   */
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    write!(
                        stdout,
                        "{:?} {yel}local copy not found, {grn}Downloading\x1B[0m\x1B[0K\r\n",
                        dir,
                        yel = color::Fg(color::Yellow),
                        grn = color::Fg(color::Green),
                    )?;
                    write!(stdout, "remote hash {}\x1B[0K\r\n", hash)?;
                    get_file_and_unzip(url, hash, &path, stdout)?;
                }
                _ => panic!("{}", e),
            },
        },
        "git" => {
            let branch = single_resource[parent]["branch"]
                .as_str()
                .unwrap_or("master");
            clone_or_pull(url, &git_file, branch, stdout)?;
        }
        _ => panic!("unknown parent type"),
    }
    Ok(Some(path))
}

/// Runs `command` with included args and returns the result or Err
/// # example
/// ```
/// let stats = status("git", ["fetch", "--all"]);
/// ```
pub fn status(command: &str, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(command).args(args).output()?)
}

pub fn get_file_and_unzip(
    url: &str,
    hash: &str,
    path: &Path,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(path.parent().unwrap())?;

    if status("curl", &["-L", "-o", path.to_str().unwrap(), url])?
        .status
        .code()
        .unwrap()
        != 0
    {
        panic!("failed to get {:?}", path);
    }
    if hash != "" {
        let mut f = File::open(path)?;
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        let sum = format!("{:x}", sha2::Sha256::digest(&data));
        write!(stdout, "  local sum {}\x1B[0K\r\n", sum)?;
        if sum != hash {
            panic!("Sum of {:?} does not match {}", path, hash);
        } else {
            let sum_file = path.parent().unwrap().join("sum256");
            let mut sum_file = File::create(sum_file)?;
            sum_file.write_all(sum.as_bytes())?;
        }
    }

    if status(
        "unzip",
        &[
            "-o",
            "-q",
            path.to_str().unwrap(),
            "-d",
            path.parent().unwrap().to_str().unwrap(),
        ],
    )?
    .status
    .code()
    .unwrap()
        == 0
    {
        std::fs::remove_file(&path)?;
    } else {
        panic!("failed to unzip {:?}", path);
    }
    Ok(())
}

/// If url-repo does not already exist locally, clone the repo into `path`
/// If it already exist, do a `git pull` on it
pub fn clone_or_pull(
    url: &str,
    path: &Path,
    branch: &str,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        write!(
            stdout,
            "\x1B[32mFound\x1B[0m {:?}, checking for updates\x1B[0K\r\n",
            path.parent().unwrap()
        )?;
        let out = status(
            "git",
            &[
                "-c",
                "color.ui=always",
                "-C",
                path.parent().unwrap().to_str().unwrap(),
                "pull",
            ],
        )?;

        if out.status.code().unwrap() != 0 {
            panic!("failed to update {:?}", path);
        } else {
            stdout.suspend_raw_mode()?;
            write!(stdout, "{}\x1B[0K\r\n", String::from_utf8(out.stdout)?)?;
            stdout.activate_raw_mode()?;
        }
    } else {
        write!(
            stdout,
            "{:?} \x1B[33mlocal copy not found, \x1B[32mCloning\x1B[0m\x1B[0K\r\n{:?}\x1B[0K\r\n",
            path.parent().unwrap(),
            url
        )?;
        let out = status(
            "git",
            &[
                "-c",
                "color.ui=always",
                "-C",
                path.to_str().unwrap().split('/').next().unwrap(),
                "clone",
                "--progress",
                "--depth",
                "1",
                "--branch",
                branch,
                url,
            ],
        )?;

        if out.status.code().unwrap() != 0 {
            panic!("failed to clone {:?}", url);
        } else {
            stdout.suspend_raw_mode()?;
            write!(stdout, "{}\x1B[0K", String::from_utf8(out.stderr)?)?;
            stdout.activate_raw_mode()?;
        }
    };
    Ok(())
}

/// Show the origin and local location, if any, of the currently highlighted item
/// lastly, show which resource will be used in the build
pub fn show_res_path(resources: &Resources, settings: &Settings, stdout: &mut RawTerminal<Stdout>) {
    let mut res_path: Option<PathBuf>;
    let section = settings.sec_key[0].as_str();
    let mut ind_res = String::new();
    settings.res_name(&mut ind_res);
    let parent = resources.resource_list[&ind_res]["parent"]
        .as_str()
        .unwrap_or("");

    write!(
        stdout,
        "\r\n{}the first found resource will be used in the OUTPUT/EFI{}\x1B[0K\r\n",
        style::Underline,
        style::Reset,
    )
    .unwrap();

    res_path = res_exists(&resources.working_dir_path, "INPUT", &ind_res, stdout);

    let open_core_pkg = &resources.open_core_binaries_path;

    if res_path == None {
        let path;
        match section {
            "ACPI" => {
                path = resources.octool_config["acpi_path"].as_str().unwrap();
            }
            "Misc" => {
                path = resources.octool_config["tools_path"].as_str().unwrap();
            }
            "UEFI" => {
                path = resources.octool_config["drivers_path"].as_str().unwrap();
            }
            _ => path = "",
        }
        res_path = res_exists(open_core_pkg, path, &ind_res, stdout);
    }

    if parent.len() > 0 {
        write!(stdout, "\x1B[2K\r\n").unwrap();
        write!(stdout, "{} in Dortania Builds? \x1B[0K", parent).unwrap();
        match &resources.dortania[parent]["versions"][0]["links"][&settings.build_type] {
            serde_json::Value::String(url) => {
                write!(stdout, "{}true\r\n", color::Fg(color::Green)).unwrap();
                write!(stdout, "{}\x1B[0m\x1B[0K\r\n", url).unwrap();
                if res_path == None {
                    res_path = get_or_update_local_parent(
                        parent,
                        &resources.dortania,
                        &settings.build_type,
                        &0,
                        stdout,
                    )
                    .unwrap();
                }
            }
            _ => write!(stdout, "{}false\x1B[0m\r\n", color::Fg(color::Red)).unwrap(),
        }

/*        write!(
            stdout,
            "\x1B[0K\n{} in Acidanthera Releases? \x1B[0K",
            parent
        )
        .unwrap();
        match &resources.acidanthera[parent]["versions"][0]["links"][&settings.build_type] {
            serde_json::Value::String(url) => {
                write!(stdout, "{}true\x1B[0K\r\n", color::Fg(color::Green)).unwrap();
                write!(stdout, "{}\x1B[0m\x1B[0K\r\n", url).unwrap();
                if res_path == None {
                    res_path = get_or_update_local_parent(
                        parent,
                        &resources.acidanthera,
                        &settings.build_type,
                        &0,
                        stdout,
                    )
                    .unwrap();
                }
            }
            _ => write!(stdout, "\x1B[31mfalse\x1B[0m\r\n").unwrap(),
        }
*/
        write!(stdout, "\x1B[0K\n{} in other? \x1B[0K", parent).unwrap();
        match &resources.other[parent]["versions"][0]["links"][&settings.build_type] {
            serde_json::Value::String(url) => {
                write!(stdout, "{}true\x1B[0K\r\n", color::Fg(color::Green)).unwrap();
                write!(stdout, "{}\x1B[0m\x1B[0K\r\n", url).unwrap();
                if res_path == None {
                    res_path = get_or_update_local_parent(
                        parent,
                        &resources.other,
                        &settings.build_type,
                        &0,
                        stdout,
                    )
                    .unwrap();
                }
            }
            _ => write!(stdout, "{}false\x1B[0m\r\n", color::Fg(color::Red)).unwrap(),
        }
    } else {
        write!(
            stdout,
            "\x1B[33m{} not found in tool_config_files/resource_list.json, skipping prebuilt repos\x1B[0m\x1B[0J\r\n"
        , &ind_res )
        .unwrap();
    }
    write!(stdout, "\x1B[2K\r\n").unwrap();
    match res_path {
        None => write!(stdout, "\x1B[31mNo local resource found\x1B[0m\x1B[0K\r\n").unwrap(),
        Some(p) => {
            write!(
                stdout,
                "\x1B[32mlocal path to resource that will be used\x1B[0m\x1B[0K\r\n"
            )
            .unwrap();
            let out = status(
                "find",
                &[p.parent().unwrap().to_str().unwrap(), "-name", &ind_res],
            )
            .unwrap();
            write!(
                stdout,
                "{}\r\n",
                String::from_utf8(out.stdout)
                    .unwrap()
                    .lines()
                    .last()
                    .unwrap()
                    .to_owned()
            )
            .unwrap();
        }
    }
}

/// Read the `path` file into a `serde_json::Value`
pub fn get_serde_json(
    path: &str,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<serde_json::Value, Box<dyn Error>> {
    write!(
        stdout,
        "\x1B[0K\n\x1B[32mLoading\x1B[0m {} ... \x1B[0K",
        path
    )?;
    stdout.flush()?;
    let file = File::open(Path::new(path))?;
    let buf = BufReader::new(file);
    let v = serde_json::from_reader(buf)?;
    write!(stdout, "\x1B[32mdone\x1B[0m\r\n")?;
    Ok(v)
}

fn res_exists(
    open_core_pkg: &PathBuf,
    path: &str,
    ind_res: &str,
    stdout: &mut RawTerminal<Stdout>,
) -> Option<PathBuf> {
    let path = open_core_pkg.join(path).join(ind_res);
    if path.exists() {
        write!(
            stdout,
            "inside {:?} dir?\x1B[0K {}true\x1B[0m\r\n",
            path.parent().unwrap(),
            color::Fg(color::Green)
        )
        .unwrap();
        Some(path)
    } else {
        write!(
            stdout,
            "inside {:?} dir?\x1B[0K {}false\x1B[0m\r\n",
            path.parent().unwrap(),
            color::Fg(color::Red)
        )
        .unwrap();
        None
    }
}

pub fn res_version(settings: &mut Settings, resources: &Resources, res: &str) -> String {
    let mut ver = String::new();
    if let Some(parent_res) = resources.resource_list[res]["parent"].as_str() {
        match settings.resource_ver_indexes.get(parent_res) {
            Some(p_index) => {
                if let Some(v) =
                    resources.dortania[parent_res]["versions"][p_index]["version"].as_str()
                {
                    ver = v.to_owned();
                } else {
                    ver = "".to_owned();
                }
            }
            None => {
                let mut p_index = 0;
                loop {
                    if let Some(date) =
                        resources.dortania[parent_res]["versions"][p_index]["date_built"].as_str()
                    {
                        if date[..11] <= settings.oc_build_date[..11] {
                            settings
                                .resource_ver_indexes
                                .insert(parent_res.to_owned(), p_index);
                            if let Some(s) = resources.dortania[parent_res]["versions"][p_index]
                                ["version"]
                                .as_str()
                            {
                                ver = s.to_owned();
                            } else {
                                ver = "".to_owned();
                            }
                            break;
                        } else {
                            p_index += 1;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        ver
    } else {
        "".to_owned()
    }
}

/// Print the Dortania and Acidanthera top level parent child
pub fn print_parents(resources: &Resources, stdout: &mut RawTerminal<Stdout>) {
    let build_type = resources.octool_config["build_type"]
        .as_str()
        .unwrap_or("release");
    let m: HashMap<String, serde_json::Value> =
        serde_json::from_value(resources.dortania.to_owned()).unwrap();
    for (name, val) in m {
        write!(
            stdout,
            "name: {} {:?}\r\n",
            name, val["versions"][0]["links"][build_type]
        )
        .unwrap();
    }
/*    let m: HashMap<String, serde_json::Value> =
        serde_json::from_value(resources.acidanthera.to_owned()).unwrap();
    for (name, val) in m {
        write!(
            stdout,
            "name: {} {:?}\r\n",
            name, val["versions"][0]["links"][build_type]
        )
        .unwrap();
    }*/
}

/// this seems redundant to the `show_res_path` function, can I combine or eliminate?
pub fn get_res_path(
    settings: &Settings,
    resources: &Resources,
    ind_res: &str,
    section: &str,
    stdout: &mut RawTerminal<Stdout>,
) -> Option<String> {
    let mut from_input = false;
    let mut res_path: Option<PathBuf>;
    let parent = resources.resource_list[&ind_res]["parent"]
        .as_str()
        .unwrap_or("");
    let mut path = resources.working_dir_path.join("INPUT").join(ind_res);
    if path.exists() {
        from_input = true;
        res_path = Some(path.clone());
    } else {
        res_path = None;
    }
    if res_path == None {
        match section {
            "ACPI" => {
                path = resources
                    .open_core_binaries_path
                    .join(resources.octool_config["acpi_path"].as_str().unwrap())
                    .join(&ind_res)
            }
            "Misc" => {
                path = resources
                    .open_core_binaries_path
                    .join(resources.octool_config["tools_path"].as_str().unwrap())
                    .join(&ind_res);
            }
            "UEFI" => {
                path = resources
                    .open_core_binaries_path
                    .join(resources.octool_config["drivers_path"].as_str().unwrap())
                    .join(&ind_res);
            }
            _ => (),
        }
        if path.exists() {
            res_path = Some(path);
        }
    }
    if res_path == None {
        res_path = get_or_update_local_parent(
            parent,
            &resources.dortania,
            &settings.build_type,
            &settings.resource_ver_indexes.get(parent).unwrap_or(&0),
            stdout,
        )
        .unwrap();
    }
/*    if res_path == None {
        res_path = get_or_update_local_parent(
            parent,
            &resources.acidanthera,
            &settings.build_type,
            &0,
            stdout,
        )
        .unwrap();
    }*/
    if res_path == None {
        res_path =
            get_or_update_local_parent(parent, &resources.other, &settings.build_type, &0, stdout)
                .unwrap();
    }
    match res_path {
        None => None,
        Some(p) => {
            let out = status(
                "find",
                &[p.parent().unwrap().to_str().unwrap(), "-name", &ind_res],
            )
            .unwrap();
            let out = String::from_utf8(out.stdout).unwrap().trim().to_owned();
            if from_input {
                write!(
                    stdout,
                    "\x1B[33mUsing \x1B[0m{}\x1B[33m copy from INPUT folder\x1B[0m\r\n",
                    ind_res
                )
                .unwrap();
            } else {
                write!(stdout, "{}\r\n", out).unwrap();
            }
            Some(out)
        }
    }
}
