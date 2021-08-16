use crate::draw::Position;
use console::style;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sha2::Digest;

pub struct Resources {
    pub acidanthera: Value,
    pub dortania: Value,
    pub octool_config: Value,
    pub parents: Value,
    pub other: Value,
    pub config_plist: plist::Value,
    pub working_dir: PathBuf,
    pub open_core_pkg: PathBuf,
}

pub fn get_or_update_local_parent(
    parent: &str,
    single_resource: &Value,
    build_type: &str,
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
    println!(
        "\x1B[32mchecking\x1B[0m local [{}] {}\x1B[0K",
        build_type, parent
    );

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
                println!("remote hash {}\x1B[0K\n  local sum {}\x1B[0K", hash, sum);
                if sum != hash {
                    println!("\x1B[32mnew version found, Downloading\x1B[0m\x1B[0K");
                    get_file_and_unzip(url, hash, &path)?;
                } else {
                    println!("Already up to date.\x1B[0K");
                }
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => {
                    println!(
                        "{:?} \x1B[33mlocal copy not found, \x1B[32mDownloading\x1B[0m\x1B[0K",
                        dir
                    );
                    println!("remote hash {}\x1B[0K", hash);
                    get_file_and_unzip(url, hash, &path)?;
                }
                _ => panic!("{}", e),
            },
        },
        "git" => {
            let branch = single_resource[parent]["branch"].as_str().unwrap_or("master");
            clone_or_pull(url, &git_file, branch)?;
        }
        _ => panic!("unknown parent type"),
    }
    Ok(Some(path))
}

pub fn status(command: &str, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(command).args(args).output()?)
}

fn get_file_and_unzip(url: &str, hash: &str, path: &Path) -> Result<(), Box<dyn Error>> {
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
    println!("  local sum {}\x1B[0K", sum);
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

pub fn clone_or_pull(url: &str, path: &Path, branch: &str) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        println!(
            "\x1B[32mfound\x1B[0m {:?}, checking for updates\x1B[0K",
            path.parent().unwrap()
        );
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
            println!("{}\x1B[0K", String::from_utf8(out.stdout)?);
        }
    } else {
        println!(
            "{:?} \x1B[33mlocal copy not found, \x1B[32mCloning\x1B[0m\x1B[0K\n{:?}\x1B[0K",
            path.parent().unwrap(),
            url
        );
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
            println!("{}\x1B[0K", String::from_utf8(out.stderr)?);
        }
    };
    Ok(())
}

pub fn show_res_path(resources: &Resources, position: &Position) {
    let full_res: String;
    let mut res_path: Option<PathBuf>;
    let section = position.sec_key[0].as_str();
    if section == "UEFI" {
        full_res = position.item_clone.as_string().unwrap().to_string();
    } else {
        full_res = position.sec_key[position.depth].clone();
    }
    let mut ind_res = full_res
        .split('/')
        .collect::<Vec<&str>>()
        .last()
        .unwrap()
        .to_string();
    if ind_res.starts_with('#') {
        ind_res.remove(0);
    }
    let parent = resources.parents[&ind_res]["parent"].as_str().unwrap_or("");

    println!(
        "\n{}\x1B[0K",
        style("the first found resource will be added to the OUTPUT/EFI").underlined()
    );

    res_path = res_exists(&resources.working_dir, "INPUT", &ind_res);

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
        res_path = res_exists(open_core_pkg, path, &ind_res);
    }

    if parent.len() > 0 {
        println!("\x1B[2K");
        print!("{} in Dortania Builds? \x1B[0K", parent);
        match &resources.dortania[parent]["versions"][0]["links"][&position.build_type] {
            Value::String(url) => {
                println!("{}", style("true").green().to_string());
                println!("{}\x1B[0K", style(url).green().to_string());
                if res_path == None {
                    res_path = get_or_update_local_parent(
                        parent,
                        &resources.dortania,
                        &position.build_type,
                    )
                    .unwrap();
                }
            }
            _ => println!("\x1B[31mfalse\x1B[0m"),
        }

        print!("\x1B[0K\n{} in Acidanthera Releases? \x1B[0K", parent);
        match &resources.acidanthera[parent]["versions"][0]["links"][&position.build_type] {
            Value::String(url) => {
                println!("{}\x1B[0K", style("true").green().to_string());
                println!("{}\x1B[0K", style(url).green().to_string());
                if res_path == None {
                    res_path = get_or_update_local_parent(
                        parent,
                        &resources.acidanthera,
                        &position.build_type,
                    )
                    .unwrap();
                }
            }
            _ => println!("\x1B[31mfalse\x1B[0m"),
        }

        print!("\x1B[0K\n{} in other? \x1B[0K", parent);
        match &resources.other[parent]["versions"][0]["links"][&position.build_type] {
            Value::String(url) => {
                println!("{}\x1B[0K", style("true").green().to_string());
                println!("{}\x1B[0K", style(url).green().to_string());
                if res_path == None {
                    res_path =
                        get_or_update_local_parent(parent, &resources.other, &position.build_type)
                            .unwrap();
                }
            }
            _ => println!("\x1B[31mfalse\x1B[0m"),
        }
    } else {
        println!("\x1B[33mNo parent found for resource, skipping prebuilt repos\x1B[0m\x1B[0J");
    }
    println!("\x1B[2K");
    match res_path {
        None => println!("\x1B[31mNo local resource found\x1B[0m\x1B[0K"),
        Some(p) => {
            println!("\x1B[32mlocal path to resource\x1B[0m\x1B[0K");
            let out = status(
                "find",
                &[p.parent().unwrap().to_str().unwrap(), "-name", &ind_res],
            )
            .unwrap();
            println!(
                "{}",
                String::from_utf8(out.stdout)
                    .unwrap()
                    .lines()
                    .last()
                    .unwrap()
                    .to_owned()
            );
        }
    }
}

pub fn get_serde_json(path: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    print!("\x1B[0K\n\x1B[32mloading\x1B[0m {} ... \x1B[0K", path);
    std::io::stdout().flush()?;
    let file = File::open(Path::new(path))?;
    let buf = BufReader::new(file);
    let v = serde_json::from_reader(buf)?;
    println!("\x1B[32mdone\x1B[0m");
    Ok(v)
}

fn res_exists(open_core_pkg: &PathBuf, path: &str, ind_res: &str) -> Option<PathBuf> {
    let path = open_core_pkg.join(path).join(ind_res);
    if path.exists() {
        println!(
            "inside {:?} dir?\x1B[0K {}",
            path.parent().unwrap(),
            style("true").green()
        );
        Some(path)
    } else {
        println!(
            "inside {:?} dir?\x1B[0K {}",
            path.parent().unwrap(),
            style("false").red()
        );
        None
    }
}

pub fn print_parents(resources: &Resources) {
    let build_type = resources.octool_config["build_type"].as_str().unwrap_or("release");
    let m: HashMap<String, Value> = serde_json::from_value(resources.dortania.to_owned()).unwrap();
    for (name, val) in m {
        println!(
            "name: {} {:?}",
            name, val["versions"][0]["links"][build_type]
        );
    }
    let m: HashMap<String, Value> =
        serde_json::from_value(resources.acidanthera.to_owned()).unwrap();
    for (name, val) in m {
        println!(
            "name: {} {:?}",
            name, val["versions"][0]["links"][build_type]
        );
    }
}

pub fn get_res_path(
    resources: &Resources,
    ind_res: &str,
    section: &str,
) -> Option<String> {
    let mut res_path: Option<PathBuf>;
    let parent = resources.parents[&ind_res]["parent"].as_str().unwrap_or("");
    let build_type = resources.octool_config["build_type"].as_str().unwrap_or("release");
    let mut path = resources.working_dir.join("INPUT").join(ind_res);
    if path.exists() {
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
        res_path = get_or_update_local_parent(parent, &resources.dortania, build_type).unwrap();
    }
    if res_path == None {
        res_path = get_or_update_local_parent(parent, &resources.acidanthera, build_type).unwrap();
    }
    if res_path == None {
        res_path = get_or_update_local_parent(parent, &resources.other, build_type).unwrap();
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
