use crate::draw::Position;
use console::style;
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::Digest;

pub struct Resources {
    pub acidanthera: Value,
    pub dortania: Value,
    pub octool_config: Value,
    pub config_plist: plist::Value,
    pub working_dir: PathBuf,
    pub open_core_pkg: PathBuf,
}

pub fn update_local_res(
    res: &str,
    resources: &Value,
    build_version: &str,
) -> Result<PathBuf, Box<dyn Error>> {
    let url = resources[res]["versions"][0]["links"][build_version]
        .as_str()
        .unwrap();
    let hash = resources[res]["versions"][0]["hashes"][build_version]["sha256"]
        .as_str()
        .unwrap();
    println!(
        "\nchecking for dortania {} version\n{:?}",
        build_version, url
    );

    let path = Path::new("./resources");
    let dir = Path::new(url).file_stem().unwrap();
    let file_name = Path::new(url).file_name().unwrap();
    let path = path.join(dir).join(file_name);
    match File::open(&path) {
        Ok(mut f) => {
            let mut data = Vec::new();
            f.read_to_end(&mut data).unwrap();
            let sum = format!("{:x}", sha2::Sha256::digest(&data));
            println!("remote hash {}\n  local sum {}", hash, sum);
            if sum != hash {
                println!("new version found, downloading");
                get_file_and_unzip(url, hash, &path)?;
            } else {
                println!("Already up to date.");
            }
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                println!("{:?} not found, downloading from\n{}", dir, url);
                println!("remote hash {}", hash);
                get_file_and_unzip(url, hash, &path)?;
            }
            _ => panic!("{}", e),
        },
    }
    Ok(path)
}

fn status(command: &str, args: &[&str]) -> Result<i32, Box<dyn Error>> {
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
    println!("  local sum {}", sum);
    if sum != hash {
        panic!("Sum of {:?} does not match {}", path, hash);
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

    print!("downloaded + unzipped\r\n");
    Ok(())
}

pub fn clone_or_pull(url: &str, path: &Path, branch: &str) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        print!(
            "found {:?}, checking for updates\r\n",
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
        print!(
            "{:?} not found\r\n Cloning from {:?}\r\n",
            path.parent().unwrap(),
            url
        );
        if status(
            "git",
            &[
                "-C",
                "octool_config_files",
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
    if position.sec_key[0].as_str() == "UEFI" && position.sec_key[1].as_str() == "Drivers" {
        full_res = resources
            .config_plist
            .as_dictionary()
            .unwrap()
            .get("UEFI")
            .unwrap()
            .as_dictionary()
            .unwrap()
            .get("Drivers")
            .unwrap()
            .as_array()
            .unwrap()
            .get(position.sec_key[2].parse::<usize>().unwrap())
            .unwrap()
            .as_string()
            .unwrap()
            .to_string();
    } else {
        full_res = position.sec_key[position.depth].clone();
    }
    let ind_res: &str = full_res.split('/').collect::<Vec<&str>>().last().unwrap();
    let stem: Vec<&str> = ind_res.split('.').collect();
    println!();

    println!(
        "inside {:?} INPUT dir?\x1B[0K {}\x1B[0K\n\x1B[2K",
        resources.working_dir,
        match Path::new("INPUT").join(ind_res).exists() {
            true => style("true").green(),
            false => style("false").red(),
        }
    );
    
    let open_core_pkg = &resources.open_core_pkg;
    let acpi_path = resources.octool_config["acpi_path"].as_str().unwrap();
    println!(
        "inside {:?} AcpiSamples dir?\x1B[0K {}\n\x1B[2K",
        open_core_pkg,
        match Path::new(open_core_pkg)
            .join(acpi_path)
            .join(ind_res)
            .exists()
        {
            true => style("true").green(),
            false => style("false").red(),
        }
    );

    let drivers_path = resources.octool_config["drivers_path"].as_str().unwrap();
    println!(
        "inside {:?} Drivers dir?\x1B[0K {}\n\x1B[2K",
        open_core_pkg,
        match Path::new(&open_core_pkg)
            .join(drivers_path)
            .join(ind_res)
            .exists()
        {
            true => style("true").green(),
            false => style("false").red(),
        }
    );

    let tools_path = resources.octool_config["tools_path"].as_str().unwrap();
    println!(
        "inside {:?} Tools dir?\x1B[0K {}\n\x1B[2K",
        open_core_pkg,
        match Path::new(&open_core_pkg)
            .join(tools_path)
            .join(ind_res)
            .exists()
        {
            true => style("true").green(),
            false => style("false").red(),
        }
    );

    println!("{} in dortania_config\x1B[0K", stem[0]);
    println!(
        "{}\x1B[0K\n\x1B[2K",
        match &resources.dortania[stem[0]]["versions"][0]["links"]["release"] {
            Value::String(s) => style(s).green().to_string(),
            _ => style("false").red().to_string(),
        }
    );

    let acid_child = resources.acidanthera[ind_res].clone();
    println!("{} in acidanthera_config\x1B[0K", ind_res);
    //    print!("{:?}\x1B[0K\n\x1B[2K", acid_child);
    match &acid_child["parent"] {
        Value::String(s) => {
            //            println!("parent {} in acidanthera_config\x1B[0K", s);
            println!(
                "{}\x1B[0K",
                match &resources.acidanthera[s]["versions"][0]["links"]["release"] {
                    Value::String(s) => style(s).green().to_string(),
                    _ => panic!("not String!"),
                }

            );
            let p = match &acid_child["path"] {
                Value::String(s) => s,
                _ => "",
            };
            if p.len() > 0 {
                println!("inside path {}\x1B[0K", style(p).green());
            }
        }
        _ => println!("{}\x1B[0K", style("false").red().to_string()),
    }
}

pub fn get_serde_json(path: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    print!("\r\nloading {} ... ", path);
    let file = File::open(Path::new(path))?;
    let buf = BufReader::new(file);
    let v = serde_json::from_reader(buf)?;
    println!("done");
    Ok(v)
}
