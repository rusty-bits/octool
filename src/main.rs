mod build;
mod draw;
mod edit;
mod init;
mod parse_tex;
mod res;

use console::{style, Key, Term};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, error::Error};

use crate::build::build_output;
use crate::draw::{update_screen, Position};
use crate::edit::edit_value;
use crate::init::init;
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
    term.clear_screen()?;
    term.hide_cursor()?;

    let mut resources = Resources {
        acidanthera: serde_json::Value::Bool(false),
        dortania: serde_json::Value::Bool(false),
        octool_config: serde_json::Value::Bool(false),
        parents: serde_json::Value::Bool(false),
        other: res::get_serde_json("tool_config_files/other.json")?,
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
        build_type: "release".to_string(),
    };

    init(&config_plist, &mut resources, &mut position)?;
    println!(
        "\x1B[32mdone with init, \x1B[0;7mq\x1B[0;32m to quit, any other key to continue\x1B[0m"
    );

    if term.read_key()? != Key::Char('q') {
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
                Key::Char('G') => {
                    term.clear_screen()?;
                    build_output(&resources)?;
                    println!("\x1B[32mValidating\x1B[0m OUTPUT/EFI/OC/config.plist");
                    init::validate_plist(
                        &Path::new("OUTPUT/EFI/OC/config.plist").to_path_buf(),
                        &resources,
                    )?;
                    break;
                }
                Key::Char('p') => {
                    res::print_parents(&resources);
                    let _ = term.read_key();
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
                            let _ = res::show_res_path(&resources, &position);
                            showing_info = true;
                        } else {
                            showing_info = parse_tex::show_info(&position, &term);
                        }
                        write!(
                            &term,
                            "{}\x1B[0K",
                            style(" ".repeat(70)).underlined()
                        )?;
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
        config_file = Path::new("tool_config_files/OpenCorePkg/Docs/Sample.plist").to_owned();
    }

    match process(&config_file) {
        Ok(()) => (),
        Err(e) => print!("\r\n{:?}\r\n", e),
    }
}
