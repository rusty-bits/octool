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

fn get_file_unzip(target: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(path.parent().unwrap())?;

    let mut out_file = File::create(&path)?;
    let _file_len = reqwest::blocking::get(target)?.copy_to(&mut out_file)?;

    unzip(path)?;
    print!("downloaded + unzipped\r\n");
    Ok(())
}

fn unzip(path: &Path) -> Result<(), Box<dyn Error>> {
    let file = File::open(path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    archive.extract(path.parent().unwrap())?;
    Ok(())
}

fn clone_pull(url: &str, path: &Path, branch: &str) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        print!(
            "found {:?}, checking for updates\r\n",
            path.file_name().unwrap()
        );
        let status = Command::new("git")
            .arg("-C")
            .arg(path.parent().unwrap())
            .arg("pull")
            .status();
    } else {
        print!(
            "{:?} not found\r\n Cloning from {:?}\r\n",
            path.file_name().unwrap(),
            url
        );
        let status = Command::new("git")
            .arg("-C")
            .arg("octool_files")
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg("--branch")
            .arg(branch)
            .arg(url)
            .status();
    };
    Ok(())
}

fn do_stuff() -> Result<(), Box<dyn Error>> {
    let term = Term::stdout();
    term.set_title("octool");
    term.clear_screen()?;
    term.hide_cursor()?;

    let path = Path::new("octool_files/octool_config.json");
    let f = File::open(path)?;
    let buf = BufReader::new(f);
    let octool_config: serde_json::Value = serde_json::from_reader(buf)?;

    let path = Path::new("octool_files/git_repo.json");
    let f = File::open(path)?;
    let buf = BufReader::new(f);
    let git_repo: serde_json::Value = serde_json::from_reader(buf)?;

    write!(&term, "checking for OpenCorePkg\r\n")?;
    let path = Path::new(octool_config["opencorepkg_path"].as_str().unwrap());
    let url = octool_config["opencorepkg_url"].as_str().unwrap();
    clone_pull(url, path, "master")?;

    write!(&term, "checking for build_repo/config.json\r\n")?;
    let path = Path::new(octool_config["build_repo_path"].as_str().unwrap());
    let url = octool_config["build_repo_url"].as_str().unwrap();
    clone_pull(url, path, "builds")?;

    let f = File::open(path)?;
    let buf = BufReader::new(f);
    let build_repo: serde_json::Value = serde_json::from_reader(buf)?;

    let url = build_repo["OpenCorePkg"]["versions"][0]["links"]["release"]
        .as_str()
        .unwrap();
    let sum = build_repo["OpenCorePkg"]["versions"][0]["hashes"]["release"]["sha256"]
        .as_str()
        .unwrap();
    write!(&term, "checking  {:?}\r\n", url)?;

    let path = Path::new("./resources");
    let dir = Path::new(url).file_stem().unwrap();
    let file_name = Path::new(url).file_name().unwrap();
    let path = path.join(dir).join(file_name);

    let mut file = File::open(&path)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    let hash = format!("{:x}", sha2::Sha256::digest(&data));

    write!(&term, " sum {}\r\nhash {}\r\n", sum, hash)?;
    if sum != hash {
        write!(&term, "new version found, downloading\r\n")?;
        get_file_unzip(url, &path)?;
    }
    let open_core_pkg = path.parent().unwrap();

    write!(&term, "pkg at {:?}\r\n", open_core_pkg)?;
    write!(&term, "done\r\n")?;

    let file = env::args().nth(1).unwrap_or("octool_files/OpenCorePkg/Docs/Sample.plist".to_string());

    let mut list =
        Value::from_file(&file).expect(format!("Didn't find plist at {}", file).as_str());

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
                let path = Path::new(&position.sec_key[position.depth]);
                let res = path.file_stem().unwrap().to_str().unwrap();
                write!(&term, " {}  \r\n\x1B[2K", res)?;
                write!(
                    &term,
                    "{:?}\r\n\x1B[0K",
                    build_repo[res]["versions"][0]["links"]["release"]
                )?;
                write!(
                    &term,
                    "{:?}\r\n\x1B[0K",
                    git_repo[&position.sec_key[position.depth]]
                )?;
                write!(
                    &term,
                    "{:?}\r\n\x1B[0K",
                    git_repo[res]["versions"][0]["links"]["release"]
                )?;
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
                list.to_file_xml("test1")?;
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

    write!(&term, "\n\r")?;

    let status = Command::new(
        open_core_pkg
            .join("Utilities")
            .join("ocvalidate")
            .join("ocvalidate"),
    )
    .arg("test1")
    .output()?;

    writeln!(&term, "\x1B[0J{}", String::from_utf8_lossy(&status.stdout))?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    match do_stuff() {
        Ok(()) => (),
        Err(e) => print!("Error {}\r\n", e),
    }
    Ok(())
}
