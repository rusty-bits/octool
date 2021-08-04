mod draw;
mod edit;
mod parse_tex;
mod res;

use serde_json;

use console::{Key, Term, style};
use plist::Value;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process::Command;
use std::{env, error::Error};

use sha2::Digest;

use draw::{update_screen, Position};
use edit::edit_value;
use res::show_res_path;
use res::Resources;

fn status(command: &str, args: &[&str]) -> Result<i32, Box<dyn Error>> {
    let out = Command::new(command).args(args).status()?;
    Ok(out.code().unwrap())
}

fn get_file_unzip(url: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(path.parent().unwrap())?;

    if status("curl", &["-L", "-o", path.to_str().unwrap(), url])? != 0 {
        panic!("failed to get {:?}", path);
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

fn clone_pull(url: &str, path: &Path, branch: &str) -> Result<(), Box<dyn Error>> {
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

fn get_serde(path: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    print!("\r\nloading {} ... ", path);
    let file = File::open(Path::new(path))?;
    let buf = BufReader::new(file);
    let v = serde_json::from_reader(buf)?;
    println!("done");
    Ok(v)
}

fn on_resource(position: &Position) -> bool {
    if position.depth != 2 {
        false
    } else {
        let mut sec_sub = position.sec_key[0].clone();
        sec_sub.push_str(&position.sec_key[1]);
        match sec_sub.as_str() {
            "ACPIAdd" => true,
            "KernelAdd" => true,
            "UEFIDrivers" => true,
            _ => false,
        }
    }
}

fn do_stuff() -> Result<(), Box<dyn Error>> {
    let term = Term::stdout();
    term.set_title("octool");
    term.clear_screen()?;
    term.hide_cursor()?;

    let mut resources = Resources {
        acidanthera: serde_json::Value::Bool(false),
        dortania: serde_json::Value::Bool(false),
        octool_config: serde_json::Value::Bool(false),
        config: plist::Value::Boolean(false),
    };

    resources.octool_config = get_serde("octool_config_files/octool_config.json")?;
    let build_version = resources.octool_config["build_version"].as_str().unwrap();
    write!(&term, "build_version set to {}\r\n", build_version)?;

    resources.acidanthera = get_serde("octool_config_files/acidanthera_config.json")?;

    write!(&term, "\r\nchecking for acidanthera OpenCorePkg\r\n")?;
    let path = Path::new(
        resources.octool_config["opencorepkg_path"]
            .as_str()
            .unwrap(),
    );
    let url = resources.octool_config["opencorepkg_url"].as_str().unwrap();
    let branch = resources.octool_config["opencorepkg_branch"]
        .as_str()
        .unwrap();
    clone_pull(url, path, branch)?;

    write!(
        &term,
        "\r\nchecking for dortania/build_repo/config.json\r\n"
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
    clone_pull(url, path, branch)?;

    resources.dortania = get_serde(path.parent().unwrap().join("config.json").to_str().unwrap())?;

    let url = resources.dortania["OpenCorePkg"]["versions"][0]["links"][build_version]
        .as_str()
        .unwrap();
    let hash = resources.dortania["OpenCorePkg"]["versions"][0]["hashes"][build_version]["sha256"]
        .as_str()
        .unwrap();
    write!(
        &term,
        "\r\nchecking for dortania {} version\r\n{:?}\r\n",
        build_version, url
    )?;

    let path = Path::new("./resources");
    let dir = Path::new(url).file_stem().unwrap();
    let file_name = Path::new(url).file_name().unwrap();
    let path = path.join(dir).join(file_name);

    match File::open(&path) {
        Ok(mut f) => {
            let mut data = Vec::new();
            f.read_to_end(&mut data).unwrap();
            let sum = format!("{:x}", sha2::Sha256::digest(&data));
            write!(&term, "remote hash {}\r\nlocal sum   {}\r\n", hash, sum)?;
            if sum != hash {
                write!(&term, "new version found, downloading\r\n")?;
                get_file_unzip(url, &path)?;
            } else {
                write!(&term, "Already up to date.\r\n")?;
            }
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                write!(
                    &term,
                    "{:?} not found, downloading from\r\n{}\r\n",
                    dir, url
                )?;
                get_file_unzip(url, &path)?;
            }
            _ => panic!("{}", e),
        },
    }

    let open_core_pkg = path.parent().unwrap();

    let file = env::args()
        .nth(1)
        .unwrap_or("octool_config_files/OpenCorePkg/Docs/Sample.plist".to_string());

    resources.config =
        Value::from_file(&file).expect(format!("Didn't find plist at {}", file).as_str());

    write!(
        &term,
        "\r\nChecking input config.plist with latest acidanthera/ocvalidate\r\n"
    )?;
    let _status = Command::new(open_core_pkg.join("Utilities/ocvalidate/ocvalidate"))
        .arg(file.clone())
        .status()?;

    write!(&term, "\r\ndone with init, any key to continue\r\n")?;
    let _ = term.read_key();

    let mut position = Position {
        file_name: file.to_owned(),
        section_num: [0; 5],
        depth: 0,
        sec_key: Default::default(),
        item_clone: resources.config.clone(),
        sec_length: [resources.config.as_dictionary().unwrap().keys().len(), 0, 0, 0, 0],
    };

    update_screen(&mut position, &resources.config, &term);
    let mut showing_info = false;

    loop {
        let key = term.read_key()?;
        match key {
            Key::Escape | Key::Char('q') => break,
            Key::ArrowUp | Key::Char('k') => position.up(),
            Key::ArrowDown | Key::Char('j') => position.down(),
            Key::ArrowLeft | Key::Char('h') => position.left(),
            Key::ArrowRight | Key::Char('l') => position.right(),
            Key::Home | Key::Char('t') => position.section_num[position.depth] = 0,
            Key::End | Key::Char('b') => {
                position.section_num[position.depth] = position.sec_length[position.depth] - 1
            }
            Key::Char(' ') => edit_value(&position, &mut resources.config, &term, true)?,
            Key::Enter | Key::Tab => edit_value(&position, &mut resources.config, &term, false)?,
            Key::Char('i') => {
                if !showing_info {
                    if on_resource(&position) {
                        show_res_path(&resources, &position);
                        showing_info = true;
                    } else {
                        showing_info = parse_tex::show_info(&position, &term);
                    }
                    write!(&term, "{}\x1B[0K", style(" ".repeat(70)).underlined())?;
                } else {
                    showing_info = false;
                }
            }
            Key::Char('s') => {
                write!(&term, "\r\n\x1B[0JSaving plist to test_out.plist\r\nChecking test_out.plist with acidanthera/ocvalidate\r\n")?;
                resources.config.to_file_xml("test_out.plist")?;
                let _status = Command::new(open_core_pkg.join("Utilities/ocvalidate/ocvalidate"))
                    .arg("test_out.plist")
                    .status()?;
                break;
            }

            _ => (),
        }
        if key != Key::Char('i') {
            showing_info = false;
        }
        if !showing_info {
            update_screen(&mut position, &resources.config, &term);
        }
    }
    term.show_cursor()?;

    write!(&term, "\n\r\x1B[0J")?;

    Ok(())
}

fn main() {
    match do_stuff() {
        Ok(()) => (),
        Err(e) => print!("\r\n{:?}\r\n", e),
    }
}
