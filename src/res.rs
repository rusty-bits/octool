use crate::draw::Position;
use serde_json::Value;
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
    pub acidanthera: Value,
    pub dortania: Value,
    pub octool_config: Value,
    pub resource_list: Value,
    pub other: Value,
    pub config_plist: plist::Value,
    pub sample_plist: plist::Value,
    pub working_dir: PathBuf,
    pub open_core_pkg: PathBuf,
}

pub fn get_or_update_local_parent(
    parent: &str,
    single_resource: &Value,
    build_type: &str,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<Option<PathBuf>, Box<dyn Error>> {
    let url = single_resource[parent]["versions"][0]["links"][build_type]
        .as_str()
        .unwrap_or("");
    if url == "" {
        return Ok(None);
    }
    let hash = single_resource[parent]["versions"][0]["hashes"][build_type]["sha256"]
        .as_str()
        .unwrap_or("");
    write!(
        stdout,
        "\x1B[32mchecking local\x1B[0m [{}] {}\x1B[0K\r\n",
        build_type, parent
    )?;

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
                } else {
                    write!(stdout, "Already up to date.\x1B[0K\r\n")?;
                }
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

pub fn status(command: &str, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(command).args(args).output()?)
}

fn get_file_and_unzip(
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
        != 0
    {
        panic!("failed to unzip {:?}", path);
    }
    Ok(())
}

pub fn clone_or_pull(
    url: &str,
    path: &Path,
    branch: &str,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        write!(
            stdout,
            "\x1B[32mfound\x1B[0m {:?}, checking for updates\x1B[0K\r\n",
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

pub fn show_res_path(resources: &Resources, position: &Position, stdout: &mut RawTerminal<Stdout>) {
    let mut res_path: Option<PathBuf>;
    let section = position.sec_key[0].as_str();
    let mut ind_res = String::new();
    position.res_name(&mut ind_res);
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

    res_path = res_exists(&resources.working_dir, "INPUT", &ind_res, stdout);

    let open_core_pkg = &resources.open_core_pkg;

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
        match &resources.dortania[parent]["versions"][0]["links"][&position.build_type] {
            Value::String(url) => {
                write!(stdout, "{}true\r\n", color::Fg(color::Green)).unwrap();
                write!(stdout, "{}\x1B[0m\x1B[0K\r\n", url).unwrap();
                if res_path == None {
                    res_path = get_or_update_local_parent(
                        parent,
                        &resources.dortania,
                        &position.build_type,
                        stdout,
                    )
                    .unwrap();
                }
            }
            _ => write!(stdout, "{}false\x1B[0m\r\n", color::Fg(color::Red)).unwrap(),
        }

        write!(
            stdout,
            "\x1B[0K\n{} in Acidanthera Releases? \x1B[0K",
            parent
        )
        .unwrap();
        match &resources.acidanthera[parent]["versions"][0]["links"][&position.build_type] {
            Value::String(url) => {
                write!(stdout, "{}true\x1B[0K\r\n", color::Fg(color::Green)).unwrap();
                write!(stdout, "{}\x1B[0m\x1B[0K\r\n", url).unwrap();
                if res_path == None {
                    res_path = get_or_update_local_parent(
                        parent,
                        &resources.acidanthera,
                        &position.build_type,
                        stdout,
                    )
                    .unwrap();
                }
            }
            _ => write!(stdout, "\x1B[31mfalse\x1B[0m\r\n").unwrap(),
        }

        write!(stdout, "\x1B[0K\n{} in other? \x1B[0K", parent).unwrap();
        match &resources.other[parent]["versions"][0]["links"][&position.build_type] {
            Value::String(url) => {
                write!(stdout, "{}true\x1B[0K\r\n", color::Fg(color::Green)).unwrap();
                write!(stdout, "{}\x1B[0m\x1B[0K\r\n", url).unwrap();
                if res_path == None {
                    res_path = get_or_update_local_parent(
                        parent,
                        &resources.other,
                        &position.build_type,
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
            "\x1B[33mNo parent found for resource, skipping prebuilt repos\x1B[0m\x1B[0J\r\n"
        )
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

pub fn get_serde_json(
    path: &str,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<serde_json::Value, Box<dyn Error>> {
    write!(
        stdout,
        "\x1B[0K\n\x1B[32mloading\x1B[0m {} ... \x1B[0K",
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

pub fn print_parents(resources: &Resources, stdout: &mut RawTerminal<Stdout>) {
    let build_type = resources.octool_config["build_type"]
        .as_str()
        .unwrap_or("release");
    let m: HashMap<String, Value> = serde_json::from_value(resources.dortania.to_owned()).unwrap();
    for (name, val) in m {
        write!(
            stdout,
            "name: {} {:?}\r\n",
            name, val["versions"][0]["links"][build_type]
        )
        .unwrap();
    }
    let m: HashMap<String, Value> =
        serde_json::from_value(resources.acidanthera.to_owned()).unwrap();
    for (name, val) in m {
        write!(
            stdout,
            "name: {} {:?}\r\n",
            name, val["versions"][0]["links"][build_type]
        )
        .unwrap();
    }
}

pub fn get_res_path(
    resources: &Resources,
    ind_res: &str,
    section: &str,
    stdout: &mut RawTerminal<Stdout>,
) -> Option<String> {
    let mut res_path: Option<PathBuf>;
    let parent = resources.resource_list[&ind_res]["parent"]
        .as_str()
        .unwrap_or("");
    let build_type = resources.octool_config["build_type"]
        .as_str()
        .unwrap_or("release");
    let mut path = resources.working_dir.join("INPUT").join(ind_res);
    if path.exists() {
        write!(stdout, "\x1B[33mUsing INPUT folder copy for \x1B[32m{}\x1B[0m\r\n", ind_res).unwrap();
        res_path = Some(path.clone());
    } else {
        res_path = None;
    }
    if res_path == None {
        let open_core_pkg = &resources.open_core_pkg;
        match section {
            "ACPI" => {
                path = open_core_pkg
                    .join(resources.octool_config["acpi_path"].as_str().unwrap())
                    .join(&ind_res)
            }
            "Misc" => {
                path = open_core_pkg
                    .join(resources.octool_config["tools_path"].as_str().unwrap())
                    .join(&ind_res);
            }
            "UEFI" => {
                path = open_core_pkg
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
        res_path =
            get_or_update_local_parent(parent, &resources.dortania, build_type, stdout).unwrap();
    }
    if res_path == None {
        res_path =
            get_or_update_local_parent(parent, &resources.acidanthera, build_type, stdout).unwrap();
    }
    if res_path == None {
        res_path =
            get_or_update_local_parent(parent, &resources.other, build_type, stdout).unwrap();
    }
    match res_path {
        None => None,
        Some(p) => {
            let out = status(
                "find",
                &[p.parent().unwrap().to_str().unwrap(), "-name", &ind_res],
            )
            .unwrap();
            Some(
                String::from_utf8(out.stdout)
                    .unwrap()
                    .lines()
                    .last()
                    .unwrap()
                    .to_owned(),
            )
        }
    }
}
