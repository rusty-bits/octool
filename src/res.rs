use crate::draw::Position;
use console::style;
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::Digest;

pub struct Resources {
    pub acidanthera: Value,
    pub dortania: Value,
    pub octool_config: Value,
    pub parents: Value,
    pub config_plist: plist::Value,
    pub working_dir: PathBuf,
    pub open_core_pkg: PathBuf,
}

pub fn get_or_update_local_parent(
    parent: &str,
    resources: &Value,
    build_version: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let url = resources[parent]["versions"][0]["links"][build_version]
        .as_str()
        .unwrap();
    let hash = resources[parent]["versions"][0]["hashes"][build_version]["sha256"]
        .as_str()
        .unwrap_or("");
    println!(
        "\x1B[32mchecking\x1B[0m local copy of {} binaries\x1B[0K",
        build_version
    );

    let path = Path::new("resources");
    let dir = Path::new(url).file_stem().unwrap();
    let file_name = Path::new(url).file_name().unwrap();
    let sum_file = path.join(dir).join("sum256");
    let git_file = path.join(dir).join(".git");
    let path = path.join(dir).join(file_name);

    match url.split('.').last().unwrap() {
        "zip" => {
            match File::open(&sum_file) {
                Ok(mut sum_file) => {
                    let mut sum = String::new();
                    sum_file.read_to_string(&mut sum)?;
                    println!("remote hash {}\x1B[0K\n  local sum {}\x1B[0K", hash, sum);
                    if sum != hash {
                        println!("\x1B[31mnew version found, downloading\x1B[0m\x1B[0K");
                        get_file_and_unzip(url, hash, &path)?;
                    } else {
                        println!("\x1B[32mAlready up to date.\x1B[0m\x1B[0K");
                    }
                }
                Err(e) => match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        println!(
                            "{:?} \x1B[31mnot found, downloading\x1B[0m\x1B[0K\n{}\x1B[0K",
                            dir, url
                        );
                        println!("remote hash {}\x1B[0K", hash);
                        get_file_and_unzip(url, hash, &path)?;
                    }
                    _ => panic!("{}", e),
                },
            }
        }
        "git" => {
            let branch = resources[parent]["branch"].as_str().unwrap_or("master");
            clone_or_pull(url, &git_file, branch)?;
        }
        _ => panic!("unknown parent type"),
    }
    Ok(path)
}

pub fn status(command: &str, args: &[&str]) -> Result<i32, Box<dyn Error>> {
    let out = Command::new(command).args(args).status()?;
    Ok(out.code().unwrap())
}

fn get_file_and_unzip(url: &str, hash: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(path.parent().unwrap())?;

    if status("curl", &["-L", "-o", path.to_str().unwrap(), url])? != 0 {
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
    )? != 0
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
        if status(
            "git",
            &["-C", path.parent().unwrap().to_str().unwrap(), "pull"],
        )? != 0
        {
            panic!("failed to update {:?}", path);
        }
    } else {
        println!(
            "{:?} \x1B[31mnot found\x1B[0m\x1B[0K\n Cloning from {:?}\x1B[0K",
            path.parent().unwrap(),
            url
        );
        if status(
            "git",
            &[
                "-C",
                path.to_str().unwrap().split('/').next().unwrap(),
                "clone",
                "--depth",
                "1",
                "--branch",
                branch,
                url,
            ],
        )? != 0
        {
            panic!("failed to clone {:?}", url);
        }
    };
    Ok(())
}

pub fn show_res_path(resources: &Resources, position: &Position) {
    let full_res: String;
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
    //    let ind_res = &ind_res;
    //    let stem = ind_res.split('.').next().unwrap();
    let parent = resources.parents[&ind_res]["parent"].as_str().unwrap_or("");

    println!(
        "\n{}\x1B[0K",
        style("the first found resource will be added to the OUTPUT/EFI").underlined()
    );
    println!("local\x1B[0K");

    res_exists(&resources.working_dir, "INPUT", &ind_res);

    let open_core_pkg = &resources.open_core_pkg;

    match section {
        "ACPI" => {
            let path = resources.octool_config["acpi_path"].as_str().unwrap();
            res_exists(open_core_pkg, path, &ind_res);
        }
        "Misc" => {
            let path = resources.octool_config["tools_path"].as_str().unwrap();
            res_exists(open_core_pkg, path, &ind_res);
        }
        "UEFI" => {
            let path = resources.octool_config["drivers_path"].as_str().unwrap();
            res_exists(open_core_pkg, path, &ind_res);
        }
        _ => (),
    }

    //    let acid_child = resources.acidanthera[&ind_res].clone();
    //    let parent = &acid_child["parent"];
    println!("\x1B[2K\nremote\x1B[0K");
    print!("{} in dortania_config \x1B[0K", parent);
    match &resources.dortania[parent]["versions"][0]["links"]["release"] {
        Value::String(url) => {
            println!("{}", style("true").green().to_string());
            let _ = get_or_update_local_parent(parent, &resources.dortania, "release");
            println!("{}\x1B[0K", style(url).green().to_string());
        }
        _ => println!("\x1B[31mfalse\x1B[0m"),
    }

    print!("\x1B[0K\n{} in acidanthera_config \x1B[0K", parent);
    match &resources.acidanthera[parent]["versions"][0]["links"]["release"] {
        Value::String(url) => {
            println!("{}\x1B[0K", style("true").green().to_string());
            let _ = get_or_update_local_parent(parent, &resources.acidanthera, "release");
            println!("{}\x1B[0K", style(url).green().to_string());
        }
        _ => println!("\x1B[31mfalse\x1B[0m"),
    }
    println!("\x1B[2K");
}

pub fn get_serde_json(path: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    print!("\x1B[0K\n\x1B[32mloading\x1B[0m {} ... ", path);
    let file = File::open(Path::new(path))?;
    let buf = BufReader::new(file);
    let v = serde_json::from_reader(buf)?;
    println!("\x1B[32mdone\x1B[0m\x1B[0K");
    Ok(v)
}

fn res_exists(open_core_pkg: &PathBuf, path: &str, ind_res: &str) {
    let path = open_core_pkg.join(path);
    println!(
        "inside {:?} dir?\x1B[0K {}",
        path,
        match path.join(ind_res).exists() {
            true => style("true").green(),
            false => style("false").red(),
        }
    );
}
