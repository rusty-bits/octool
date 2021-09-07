mod build;
mod draw;
mod edit;
mod init;
mod parse_tex;
mod res;
mod snake;

use fs_extra::dir::{copy, CopyOptions};
use std::io::{stdin, stdout, Stdout, Write};
use std::path::{Path, PathBuf};
use std::{env, error::Error};
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::{IntoRawMode, RawTerminal};

use crate::build::build_output;
use crate::draw::{update_screen, Position};
use crate::edit::{delete_value, edit_value};
use crate::init::init;
use crate::res::Resources;
use crate::snake::snake;

fn process(
    config_plist: &PathBuf,
    current_dir: &PathBuf,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<(), Box<dyn Error>> {
    let stdin = stdin();
    let mut resources = Resources {
        acidanthera: serde_json::Value::Bool(false),
        dortania: serde_json::Value::Bool(false),
        octool_config: serde_json::Value::Bool(false),
        parents: serde_json::Value::Bool(false),
        other: res::get_serde_json("tool_config_files/other.json", stdout)?,
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
        parents: &serde_json::Value::Bool(false),
    };

    init(&config_plist, &mut resources, &mut position, stdout)?;
    position.parents = &resources.parents;

    writeln!(
        stdout,
        "\x1B[32mdone with init, \x1B[0;7mq\x1B[0;32m to quit, any other key to continue\x1B[0m\r"
    )?;
    let key = std::io::stdin().keys().next().unwrap().unwrap();

    if key != Key::Char('q') {
        update_screen(&mut position, &resources.config_plist, stdout)?;
        stdout.flush().unwrap();
        let mut showing_info = false;
        for key in stdin.keys() {
            let key = key.unwrap();
            match key {
                Key::Char('q') => {
                    if showing_info {
                        showing_info = false;
                    } else {
                        break;
                    }
                }
                Key::Char('G') => {
                    //                        term.clear_screen()?;
                    let build_okay = build_output(&resources, stdout)?;
                    writeln!(
                        stdout,
                        "\n\x1B[32mValidating\x1B[0m OUTPUT/EFI/OC/config.plist\r"
                    )?;
                    let config_okay = init::validate_plist(
                        &Path::new("OUTPUT/EFI/OC/config.plist").to_path_buf(),
                        &resources,
                        stdout,
                    )?;
                    if !build_okay || !config_okay {
                        writeln!(stdout, "\n\x1B[31mErrors occured while building OUTPUT/EFI, you should fix them before using it\x1B[0m\r")?;
                    } else {
                        writeln!(stdout, "\n\x1B[32mFinished building OUTPUT/EFI\x1B[0m\r")?;
                        if &env::current_dir().unwrap() != current_dir {
                            writeln!(stdout, "Copying OUTPUT EFI folder to this directory\r")?;
                            let mut options = CopyOptions::new();
                            options.overwrite = true;
                            copy("OUTPUT/EFI", current_dir, &options)?;
                        }
                    }
                    break;
                }
                Key::Char('p') => {
                    res::print_parents(&resources, stdout);
                    //                        let _ = term.read_key();
                }
                Key::Up | Key::Char('k') => position.up(),
                Key::Down | Key::Char('j') => position.down(),
                Key::Left | Key::Char('h') => position.left(),
                Key::Right | Key::Char('l') => position.right(),
                Key::Home | Key::Char('t') => position.section_num[position.depth] = 0,
                Key::End | Key::Char('b') => {
                    position.section_num[position.depth] = position.sec_length[position.depth] - 1
                }
                Key::Char(' ') => {
                    if !showing_info {
                        //                   showing_info = false;
                        //               } else {
                        edit_value(&position, &mut resources.config_plist, stdout, true)?;
                    }
                }
                Key::Char('\n') | Key::Char('\t') => {
                    edit_value(&position, &mut resources.config_plist, stdout, false)?
                }
                Key::Char('D') => {
                    if delete_value(&position, &mut resources.config_plist) {
                        position.delete();
                    }
                }
                Key::Char('i') => {
                    if !showing_info {
                        if position.is_resource() {
                            let _ = res::show_res_path(&resources, &position, stdout);
                            showing_info = true;
                        } else {
                            showing_info = parse_tex::show_info(&position, stdout)?;
                        }
                        write!(stdout, "{}\x1B[0K", "_".repeat(70))?;
                        stdout.flush()?;
                    } else {
                        showing_info = false;
                    }
                }
                Key::Char('M') => snake(stdout)?,
                Key::Char('s') => {
                    write!(stdout, "\r\n\x1B[0JSaving plist to test_out.plist\r\n\x1B[32mValidating\x1B[0m test_out.plist with acidanthera/ocvalidate\r\n")?;
                    resources.config_plist.to_file_xml("test_out.plist")?;
                    let _ = init::validate_plist(
                        &Path::new("test_out.plist").to_path_buf(),
                        &resources,
                        stdout,
                    )?;
                    break;
                }
                _ => (),
            }
            if key != Key::Char('i') && key != Key::Char(' ') {
                showing_info = false;
            }
            if !showing_info {
                update_screen(&mut position, &resources.config_plist, stdout)?;
                stdout.flush().unwrap();
            }
        }
    }

    write!(stdout, "\n\r\x1B[0J")?;
    stdout.flush()?;

    Ok(())
}

fn main() {
    let mut stdout = stdout().into_raw_mode().unwrap();
    write!(stdout, "{}", termion::clear::All).unwrap();
    write!(stdout, "{}", termion::cursor::Hide).unwrap();
    write!(stdout, "{}", termion::cursor::Goto(1, 1)).unwrap();

    let current_dir = env::current_dir().unwrap();

    #[cfg(not(debug_assertions))] // point to octool dir no matter where tool run from
    {
        let working_dir = env::current_exe().unwrap();
        if working_dir != current_dir {
            let working_dir = working_dir.parent().unwrap();
            env::set_current_dir(working_dir).unwrap();
        }
    }

    let mut config_file = Path::new(&match env::args().nth(1) {
        Some(s) => s,
        None => "INPUT/config.plist".to_string(),
    })
    .to_owned();

    if !config_file.exists() {
        write!(
            stdout,
            "\x1B[31mDid not find config at\x1B[0m {:?}\r\n",
            config_file
        )
        .unwrap();
        write!(stdout, "Using OpenCorePkg/Docs/Sample.plist\r\n").unwrap();
        config_file = Path::new("tool_config_files/OpenCorePkg/Docs/Sample.plist").to_owned();
    }
    stdout.flush().unwrap();

    match process(&config_file, &current_dir, &mut stdout) {
        Ok(()) => (),
        Err(e) => write!(stdout, "\n{:?}\r\n", e).unwrap(),
    }
    write!(stdout, "{}", termion::cursor::Show).unwrap();
}
