mod draw;
mod edit;
mod parse_tex;
mod res;

use console::{style, Key, Term};
use plist::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, error::Error};

use crate::draw::{update_screen, Position};
use crate::edit::edit_value;
use crate::res::Resources;

fn on_resource(position: &Position) -> bool {
    if position.depth != 2 {
        false
    } else {
        let mut sec_sub = position.sec_key[0].clone();
        sec_sub.push_str(&position.sec_key[1]);
        match sec_sub.as_str() {
            "ACPIAdd" => true,
            "KernelAdd" => true,
            "MiscTools" => true,
            "UEFIDrivers" => true,
            _ => false,
        }
    }
}

fn process(config_plist: &PathBuf) -> Result<(), Box<dyn Error>> {
    let term = Term::stdout();
    term.set_title("octool");
    //    term.clear_screen()?;
    term.hide_cursor()?;

    let mut resources = Resources {
        acidanthera: serde_json::Value::Bool(false),
        dortania: serde_json::Value::Bool(false),
        octool_config: serde_json::Value::Bool(false),
        config_plist: plist::Value::Boolean(false),
        working_dir: env::current_dir()?,
        open_core_pkg: PathBuf::new(),
    };

    resources.octool_config = res::get_serde_json("octool_config_files/octool_config.json")?;
    let build_version = resources.octool_config["build_version"].as_str().unwrap();
    write!(&term, "build_version set to {}\r\n", build_version)?;

    write!(&term, "\r\nchecking acidanthera OpenCorePkg source\r\n")?;
    let path = Path::new(
        resources.octool_config["opencorepkg_path"]
            .as_str()
            .unwrap(),
    );
    let url = resources.octool_config["opencorepkg_url"].as_str().unwrap();
    let branch = resources.octool_config["opencorepkg_branch"]
        .as_str()
        .unwrap();
    res::clone_or_pull(url, path, branch)?;

    resources.config_plist = Value::from_file(&config_plist)
        .expect(format!("Didn't find valid plist at {:?}", config_plist).as_str());

    resources.acidanthera = res::get_serde_json("octool_config_files/acidanthera_config.json")?;

    write!(
        &term,
        "\r\nchecking dortania/build_repo/config.json\r\n"
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
    res::clone_or_pull(url, path, branch)?;

    resources.dortania =
        res::get_serde_json(path.parent().unwrap().join("config.json").to_str().unwrap())?;

    let path = res::update_local_res("OpenCorePkg", &resources.dortania, build_version)?;

    resources.open_core_pkg = path.parent().unwrap().to_path_buf();

    write!(
        &term,
        "\r\nChecking {:?} with latest acidanthera/ocvalidate\r\n",
        config_plist
    )?;

    let _status = Command::new(
        resources
            .open_core_pkg
            .join("Utilities/ocvalidate/ocvalidate"),
    )
    .arg(config_plist.clone())
    .status()?;

    write!(&term, "\r\ndone with init, any key to continue\r\n")?;
    let _ = term.read_key();

    let mut position = Position {
        file_name: config_plist.to_str().unwrap().to_string(),
        section_num: [0; 5],
        depth: 0,
        sec_key: Default::default(),
        item_clone: resources.config_plist.clone(),
        sec_length: [
            resources.config_plist.as_dictionary().unwrap().keys().len(),
            0,
            0,
            0,
            0,
        ],
    };

    update_screen(&mut position, &resources.config_plist, &term);
    let mut showing_info = false;

    loop {
        let key = term.read_key()?;
        match key {
            Key::Char('q') => {
                if showing_info {
                    showing_info = false;
                } else {
                    break;
                }
            }
            Key::ArrowUp | Key::Char('k') => position.up(),
            Key::ArrowDown | Key::Char('j') => position.down(),
            Key::ArrowLeft | Key::Char('h') => position.left(),
            Key::ArrowRight | Key::Char('l') => position.right(),
            Key::Home | Key::Char('t') => position.section_num[position.depth] = 0,
            Key::End | Key::Char('b') => {
                position.section_num[position.depth] = position.sec_length[position.depth] - 1
            }
            Key::Char(' ') => {
                if showing_info {
                    showing_info = false;
                } else {
                    edit_value(&position, &mut resources.config_plist, &term, true)?;
                }
            }
            Key::Enter | Key::Tab => {
                edit_value(&position, &mut resources.config_plist, &term, false)?
            }
            Key::Char('i') => {
                if !showing_info {
                    if on_resource(&position) {
                        res::show_res_path(&resources, &position);
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
                resources.config_plist.to_file_xml("test_out.plist")?;
                let _status = Command::new(
                    resources
                        .open_core_pkg
                        .join("Utilities/ocvalidate/ocvalidate"),
                )
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
            update_screen(&mut position, &resources.config_plist, &term);
        }
    }
    term.show_cursor()?;

    write!(&term, "\n\r\x1B[0J")?;

    Ok(())
}

fn main() {
    print!("\x1B[2J\x1B[H");
    let mut config_file = Path::new(&match env::args().nth(1) {
        Some(s) => s,
        None => "INPUT/config.plist".to_string(),
    })
    .to_owned();

    if !config_file.exists() {
        println!("Did not find config at {:?}", config_file);
        println!("Using OpenCorePkg/Docs/Sample.plist");
        config_file = Path::new("octool_config_files/OpenCorePkg/Docs/Sample.plist").to_owned();
    }

    match process(&config_file) {
        Ok(()) => (),
        Err(e) => print!("\r\n{:?}\r\n", e),
    }
}
