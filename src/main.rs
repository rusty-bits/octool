mod draw;
mod edit;
mod parse_tex;

use serde_json;

use console::{Key, Term};
use plist::Value;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process::Command;
use std::{env, error::Error};

use sha2::Digest;

use draw::{update_screen, Position};
use edit::edit_value;

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

fn do_stuff() -> Result<(), Box<dyn Error>> {
    let term = Term::stdout();
    term.set_title("octool");
    term.clear_screen()?;
    term.hide_cursor()?;

    let octool_config = get_serde("octool_config_files/octool_config.json")?;
    let build_version = octool_config["build_version"].as_str().unwrap();
    write!(&term, "build_version set to {}\r\n", build_version)?;

    let acidanthera_config = get_serde("octool_config_files/acidanthera_config.json")?;

    write!(&term, "\r\nchecking for acidanthera OpenCorePkg\r\n")?;
    let path = Path::new(octool_config["opencorepkg_path"].as_str().unwrap());
    let url = octool_config["opencorepkg_url"].as_str().unwrap();
    let branch = octool_config["opencorepkg_branch"].as_str().unwrap();
    clone_pull(url, path, branch)?;

    write!(
        &term,
        "\r\nchecking for dortania/build_repo/config.json\r\n"
    )?;
    let path = Path::new(octool_config["dortania_config_path"].as_str().unwrap());
    let url = octool_config["dortania_config_url"].as_str().unwrap();
    let branch = octool_config["dortania_config_branch"].as_str().unwrap();
    clone_pull(url, path, branch)?;

    let dortania_config = get_serde(path.parent().unwrap().join("config.json").to_str().unwrap())?;

    let url = dortania_config["OpenCorePkg"]["versions"][0]["links"][build_version]
        .as_str()
        .unwrap();
    let hash = dortania_config["OpenCorePkg"]["versions"][0]["hashes"][build_version]["sha256"]
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

    let mut list =
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
        section: [0; 5],
        depth: 0,
        sec_key: Default::default(),
        item_clone: list.clone(),
        sec_length: [list.as_dictionary().unwrap().keys().len(), 0, 0, 0, 0],
    };

    update_screen(&mut position, &list, &term);
    let mut showing_info = false;

    loop {
        let key = term.read_key()?;
        match key {
            Key::Escape | Key::Char('q') => break,
            Key::ArrowUp | Key::Char('k') => position.up(),
            Key::ArrowDown | Key::Char('j') => position.down(),
            Key::ArrowLeft | Key::Char('h') => position.left(),
            Key::ArrowRight | Key::Char('l') => position.right(),
            Key::Home => position.section[position.depth] = 0,
            Key::End => position.section[position.depth] = position.sec_length[position.depth] - 1,
            Key::Char(' ') => edit_value(&position, &mut list, &term, true)?,
            Key::Enter | Key::Tab => edit_value(&position, &mut list, &term, false)?,
            Key::Char('g') => {
                let full_res = position.sec_key[position.depth].clone();
                let ind_res: &str = full_res.split('/').collect::<Vec<&str>>().last().unwrap();
                let stem: Vec<&str> = ind_res.split('.').collect();
                write!(&term, "\r\n {} - {}\x1B[0K\r\n", stem[0], ind_res)?;
                write!(&term, "inside INPUT dir?\x1B[0K\r\n {:?}\x1B[0K\r\n\x1B[2K\r\n", Path::new("INPUT").join(ind_res).exists())?;
                write!(&term, "{} in dortania_config\x1B[0K\r\n", stem[0])?;
                write!(
                    &term,
                    "{:?}\x1B[0K\r\n\x1B[2K\r\n",
                    dortania_config[stem[0]]["versions"][0]["links"]["release"]
                )?;
                let acid_child = acidanthera_config[ind_res].clone();
                write!(&term, "{} in acidanthera_config\x1B[0K\r\n", ind_res)?;
                write!(&term, "{:?}\x1B[0K\r\n\x1B[2K\r\n", acid_child)?;
                match acid_child["parent"].to_owned() {
                    serde_json::Value::String(s) => {
                        write!(&term, "parent {} in acidanthera_config\x1B[0K\r\n", s)?;
                        write!(
                            &term,
                            "{:?}\x1B[0K\r\n",
                            acidanthera_config[s]["versions"][0]["links"]["release"]
                        )?;
                    }
                    _ => (),
                }
                write!(&term, "\x1B[2K")?;
                let _ = term.read_key()?;
            }
            Key::Char('i') => {
                if !showing_info {
                    parse_tex::show_info(&position, &term);
                    showing_info = true;
                } else {
                    showing_info = false;
                }
            }
            Key::Char('s') => {
                write!(&term, "\r\n\x1B[0JSaving plist to test_out.plist\r\nChecking test_out.plist with acidanthera/ocvalidate\r\n")?;
                list.to_file_xml("test_out.plist")?;
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
            update_screen(&mut position, &list, &term);
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
