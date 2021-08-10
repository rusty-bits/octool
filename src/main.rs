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

fn on_resource(position: &Position, sections: &Vec<String>) -> bool {
    if position.depth != 2 {
        false
    } else {
        let mut sec_sub = position.sec_key[0].clone();
        sec_sub.push_str(&position.sec_key[1]);
        sections.contains(&sec_sub)
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
        parents: serde_json::Value::Bool(false),
        config_plist: plist::Value::Boolean(false),
        working_dir: env::current_dir()?,
        open_core_pkg: PathBuf::new(),
    };

    let mut position = Position {
        file_name: String::new(),
        section_num: [0; 5],
        depth: 0,
        sec_key: Default::default(),
        item_clone: plist::Value::Boolean(false),
        sec_length: [0; 5],
        resource_sections: vec![],
    };

    resources.octool_config = res::get_serde_json("octool_config_files/octool_config.json")?;
    let build_version = resources.octool_config["build_version"].as_str().unwrap();
    write!(
        &term,
        "\x1B[32mbuild_version set to\x1B[0m {}\r\n",
        build_version
    )?;
    position.resource_sections =
        serde_json::from_value(resources.octool_config["resource_sections"].clone()).unwrap();
    write!(
        &term,
        "\x1B[32mplist resource sections\x1B[0m {:?}\r\n",
        position.resource_sections
    )?;

    write!(
        &term,
        "\r\n\x1B[32mchecking\x1B[0m acidanthera OpenCorePkg source\r\n"
    )?;
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
        "\r\n\x1B[32mchecking\x1B[0m dortania/build_repo/config.json\r\n"
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
    resources.parents =
        res::get_serde_json("octool_config_files/parents.json")?;

    write!(&term, "\r\n")?;
    let path = res::get_or_update_local_parent("OpenCorePkg", &resources.dortania, build_version)?;

    resources.open_core_pkg = path.parent().unwrap().to_path_buf();

    write!(
        &term,
        "\r\n\x1B[32mValidating\x1B[0m {:?} with latest acidanthera/ocvalidate\r\n",
        config_plist
    )?;

    if res::status(
        resources
            .open_core_pkg
            .join("Utilities/ocvalidate/ocvalidate")
            .to_str()
            .unwrap(),
        &[&config_plist.to_str().unwrap()],
    )? != 0
    {
        write!(
            &term,
            "\x1B[31mWARNING: Error(s) found in config.plist!\x1B[0m\r\n"
        )?;
    }

    position.file_name = config_plist.to_str().unwrap().to_owned();
    position.sec_length[0] = resources.config_plist.as_dictionary().unwrap().keys().len();

    write!(
        &term,
        "\r\n\x1B[32mdone with init, any key to continue\x1B[0m\r\n"
    )?;
    let _ = term.read_key();

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
                if !showing_info {
                    //                   showing_info = false;
                    //               } else {
                    edit_value(&position, &mut resources.config_plist, &term, true)?;
                }
            }
            Key::Enter | Key::Tab => {
                edit_value(&position, &mut resources.config_plist, &term, false)?
            }
            Key::Char('i') => {
                if !showing_info {
                    if on_resource(&position, &position.resource_sections) {
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
                write!(&term, "\r\n\x1B[0JSaving plist to test_out.plist\r\n\x1B[32mValidatinig\x1B[0m test_out.plist with acidanthera/ocvalidate\r\n")?;
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
        if key != Key::Char('i') && key != Key::Char(' ') {
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
        println!("\x1B[31mDid not find config at\x1B[0m {:?}", config_file);
        println!("Using OpenCorePkg/Docs/Sample.plist");
        config_file = Path::new("octool_config_files/OpenCorePkg/Docs/Sample.plist").to_owned();
    }

    match process(&config_file) {
        Ok(()) => (),
        Err(e) => print!("\r\n{:?}\r\n", e),
    }
}
