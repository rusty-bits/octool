mod build;
mod draw;
mod edit;
mod init;
mod parse_tex;
mod res;
mod snake;

use crossterm::event::KeyModifiers;
use fs_extra::dir::{copy, CopyOptions};
use std::io::{stdout, Stdout, Write};
use std::path::{Path, PathBuf};
use std::{env, error::Error};

use crossterm::ExecutableCommand;
use crossterm::{cursor, event::KeyCode, terminal};

use edit::read_key;

use crate::init::guess_version;

fn process(
    config_plist: &mut PathBuf,
    current_dir: &PathBuf,
    settings: &mut draw::Settings,
    mut resources: &mut res::Resources,
    stdout: &mut Stdout,
) -> Result<(), Box<dyn Error>> {
    let mut found = vec![edit::Found::new()];
    let mut found_id: usize = 0;
    let mut res_name = String::new();

    init::init_oc_build(&mut resources, settings, stdout)?;
    init::init_plist(config_plist, &mut resources, settings, stdout)?;

    let mut key = KeyCode::Char('q');
    let mut key_mod;

    if settings.oc_build_version != "not found" {
        write!(
        stdout,
        "\r\n\x1B[32mdone with init, \x1B[0;7mq\x1B[0;32m to quit, any other key to continue\x1B[0m\r"
    )?;

        stdout.flush()?;
        key = read_key()?.0;
    }

    if key != KeyCode::Char('q') {
        let mut showing_info = false;
        loop {
            if !showing_info {
                draw::update_screen(settings, &mut resources, stdout)?;
                stdout.flush().unwrap();
            }
            //            (key, key_mod) = read_key()?; // feature not in stable yet, issue #71126
            let key_and_mods = read_key()?;
            key = key_and_mods.0;
            key_mod = key_and_mods.1;
            // TODO: add option to change version of single kext, efi, etc...
            match key {
                KeyCode::Char('q') => {
                    if showing_info {
                        showing_info = false;
                    } else {
                        if settings.modified {
                            write!(stdout, "\r\n\x1B[33;7mNOTICE:\x1B[0m changes have been made to the plist file\
                                   \x1B[0K\r\n capital 'Q' to quit without saving, any other key will cancel\
                                   \x1B[0K\r\n\x1B[2K").unwrap();
                            stdout.flush().unwrap();
                            match read_key()?.0 {
                                KeyCode::Char('Q') => break,
                                _ => (),
                            }
                        } else {
                            break;
                        }
                    }
                }
                KeyCode::Char('G') => {
                    let build_okay = build::build_output(settings, &resources, stdout)?;
                    writeln!(
                        stdout,
                        "\n\x1B[32mValidating\x1B[0m OUTPUT/EFI/OC/config.plist\r"
                    )?;
                    let config_okay = init::validate_plist(
                        &Path::new("OUTPUT/EFI/OC/config.plist").to_path_buf(),
                        &resources,
                        stdout,
                    )?;
                    let mut config_file = PathBuf::from(&settings.config_file_name)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    if !config_file.starts_with("last_built_") {
                        let mut tmp = "last_built_".to_string();
                        tmp.push_str(&config_file);
                        config_file = tmp.to_owned();
                    }
                    let save_file = PathBuf::from("INPUT").join(&config_file);
                    write!(
                        stdout,
                        "\r\n\x1B[0JSaving copy of plist as INPUT/{}\r\n\x1B[0K",
                        config_file
                    )
                    .unwrap();
                    resources.config_plist.to_file_xml(&save_file)?;
                    if !build_okay || !config_okay {
                        writeln!(
                            stdout,
                            "\n\x1B[31mErrors occured while building OUTPUT/EFI, \
                                 you should fix them before using it\x1B[0m\r"
                        )?;
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
                KeyCode::Char('a') => edit::add_item(settings, &mut resources, stdout),
                KeyCode::Char('f') => {
                    found = vec![];
                    found_id = 0;
                    settings.find_string = String::new();
                    write!(
                        stdout,
                        "{}\r\x1B[2KEnter search term: {}\r\n\x1B[2K\x1B8",
                        cursor::Show,
                        cursor::SavePosition,
                    )
                    .unwrap();
                    edit::edit_string(&mut settings.find_string, None, stdout).unwrap();
                    write!(stdout, "{}", cursor::Hide).unwrap();
                    if settings.find_string.len() > 0 {
                        edit::find(&settings.find_string, &resources.config_plist, &mut found);
                        if found.len() == 1 {
                            settings.depth = found[0].level;
                            settings.sec_num = found[0].section;
                            settings.find_string = String::new();
                            found_id = 0;
                        } else if found.len() > 1 {
                            let mut selection = 1;
                            write!(stdout, "\r\n\x1B[2K\x1B7").unwrap();
                            loop {
                                write!(stdout, "\x1B8").unwrap();
                                for (i, f) in found.iter().enumerate() {
                                    let mut fk = f.keys.iter();
                                    write!(
                                        stdout,
                                        "  {}{}",
                                        if i == selection - 1 { "\x1B[7m" } else { "" },
                                        fk.next().unwrap()
                                    )
                                    .unwrap();
                                    for next_key in fk {
                                        write!(stdout, "->{}", next_key).unwrap();
                                    }
                                    write!(stdout, "\x1B[0m\r\n\x1B[2K").unwrap();
                                }
                                stdout.flush().unwrap();
                                match read_key()?.0 {
                                    KeyCode::Up => {
                                        if selection > 1 {
                                            selection -= 1;
                                        }
                                    }
                                    KeyCode::Down => {
                                        if selection < found.len() {
                                            selection += 1;
                                        }
                                    }
                                    KeyCode::Enter => break,
                                    KeyCode::Esc => {
                                        selection = 0;
                                        break;
                                    }
                                    _ => (),
                                }
                            }
                            found_id = selection;
                            if selection > 0 {
                                settings.depth = found[selection - 1].level;
                                settings.sec_num = found[selection - 1].section;
                            }
                        } else {
                            settings.find_string = String::new();
                        }
                    }
                }
                KeyCode::Char('n') => {
                    if found_id > 0 {
                        found_id += 1;
                        if found_id > found.len() {
                            found_id = 1;
                        }
                        settings.depth = found[found_id - 1].level;
                        settings.sec_num = found[found_id - 1].section;
                    }
                }
                KeyCode::Char('p') => {
                    if edit::add_delete_value(settings, &mut resources.config_plist, true) {
                        settings.add();
                    }
                }
                KeyCode::Char('v') => {
                    if key_mod == KeyModifiers::CONTROL
                        && edit::add_delete_value(settings, &mut resources.config_plist, true)
                    {
                        settings.add();
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => settings.up(),
                KeyCode::Down | KeyCode::Char('j') => settings.down(),
                KeyCode::Left | KeyCode::Char('h') => settings.left(),
                KeyCode::Right | KeyCode::Char('l') => settings.right(),
                KeyCode::Home | KeyCode::Char('t') => settings.sec_num[settings.depth] = 0,
                KeyCode::End | KeyCode::Char('b') => {
                    settings.sec_num[settings.depth] = settings.sec_length[settings.depth] - 1
                }
                // TODO: special check for driver section for OC 0.7.2 and earlier, uses # to enable/disable
                KeyCode::Char(' ') => {
                    if !showing_info {
                        edit::edit_value(
                            settings,
                            &mut resources.config_plist,
                            None,
                            stdout,
                            true,
                            false,
                        )?;
                    }
                }
                KeyCode::Enter | KeyCode::Tab => {
                    write!(stdout, "\x1b8\r\n")?;
                    let mut valid_values = vec![];
                    parse_tex::show_info(&resources, &settings, true, &mut valid_values, stdout)?;
                    edit::edit_value(
                        settings,
                        &mut resources.config_plist,
                        if valid_values.len() > 0 {
                            Some(&valid_values)
                        } else {
                            None
                        },
                        stdout,
                        false,
                        false,
                    )?;
                }
                KeyCode::Char('K') => edit::edit_value(
                    settings,
                    &mut resources.config_plist,
                    None,
                    stdout,
                    false,
                    true,
                )?,
                KeyCode::Char('D') => {
                    if settings.sec_length[settings.depth] > 0 {
                        if edit::add_delete_value(settings, &mut resources.config_plist, false) {
                            settings.delete();
                        }
                    }
                }
                KeyCode::Char('x') => {
                    if key_mod == KeyModifiers::CONTROL {
                        if settings.sec_length[settings.depth] > 0 {
                            if edit::add_delete_value(settings, &mut resources.config_plist, false)
                            {
                                settings.delete();
                            }
                        }
                    }
                }
                KeyCode::Char('d') => {
                    if settings.sec_length[settings.depth] > 0 {
                        write!(stdout,"\r\n{und}Press{res} '{grn}d{res}' again to remove {yel}{obj}{res}, \
                               any other key to cancel.{clr}\r\n{yel}You can use '{grn}p{yel}' to place {obj} \
                               back into plist{res}{clr}\r\n{clr}",
                            obj = &settings.sec_key[settings.depth],
                            yel = "\x1b[32m",
                            grn = "\x1b[33m",
                            und = "\x1b[4m",
                            res = "\x1b[0m",
                            clr = "\x1b[0K",
                        )?;
                        stdout.flush()?;
                        if read_key()?.0 == KeyCode::Char('d') {
                            if edit::add_delete_value(settings, &mut resources.config_plist, false)
                            {
                                settings.delete();
                            }
                        }
                    }
                }
                KeyCode::Char('y') => {
                    edit::extract_value(settings, &mut resources.config_plist, false, true);
                }
                KeyCode::Char('c') => {
                    if key_mod == KeyModifiers::CONTROL {
                        edit::extract_value(settings, &mut resources.config_plist, false, true);
                    }
                }
                KeyCode::Char('V') => {
                    write!(
                        stdout,
                        "\x1b[2K\r\n{}\r\x1B[2KEnter version number: {}\r\n\x1B[2K\x1B8",
                        cursor::Show,
                        cursor::SavePosition,
                    )?;
                    edit::edit_string(&mut settings.oc_build_version, None, stdout)?;
                    write!(stdout, "{}", cursor::Hide)?;
                    init::init_oc_build(&mut resources, settings, stdout)?;
                }
                KeyCode::Char('r') => {
                    if settings.depth < 4 {
                        let mut obj = String::new();
                        for i in 0..settings.depth + 1 {
                            obj.push_str(&settings.sec_key[i]);
                            obj.push(' ');
                        }
                        write!(stdout,"\r\n{und}Press{res} '{grn}r{res}' again to reset {yel}{obj}{res}to \
                               the Sample.plist values, any other key to cancel.{clr}\r\n{yel}You can use \
                               '{grn}p{yel}' to place old {grn}{cur}{yel} back into plist if needed{res}{clr}\r\n{clr}",
                            obj = &obj,
                            cur = &settings.sec_key[settings.depth],
                            yel = "\x1b[32m",
                            grn = "\x1b[33m",
                            und = "\x1b[4m",
                            res = "\x1b[0m",
                            clr = "\x1b[0K",
                        )?;
                        stdout.flush()?;
                        if read_key()?.0 == KeyCode::Char('r') {
                            if edit::extract_value(settings, &resources.config_plist, false, true) {
                                settings.modified = true;
                                let tmp_item = settings.held_item.clone();
                                let tmp_key = settings.held_key.clone();
                                if edit::extract_value(
                                    settings,
                                    &resources.sample_plist,
                                    false,
                                    true,
                                ) {
                                    let _ = edit::add_delete_value(
                                        settings,
                                        &mut resources.config_plist,
                                        true,
                                    );
                                }
                                settings.held_key = tmp_key.to_owned();
                                settings.held_item = tmp_item;
                            }
                        }
                    }
                }
                KeyCode::Char('m') => {
                    //    it might not make sense to merge an array, maybe use 'r'eset instead?
                    let initial_depth = settings.depth;
                    let initial_key = settings.held_key.to_owned();
                    let initial_item = settings.held_item.to_owned();
                    if !settings.can_expand && settings.depth > 0 {
                        settings.depth -= 1;
                    }
                    if edit::extract_value(settings, &resources.config_plist, false, true) {
                        match settings.held_item.clone().unwrap() {
                            plist::Value::Dictionary(mut d) => {
                                stdout.flush()?;
                                if edit::extract_value(
                                    settings,
                                    &resources.sample_plist,
                                    false,
                                    false,
                                ) {
                                    stdout.flush().unwrap();
                                    match settings.held_item.clone().unwrap() {
                                        plist::Value::Dictionary(d2) => {
                                            for (k, v) in d2 {
                                                if !d.contains_key(&k) {
                                                    d.insert(k.to_owned(), v.to_owned());
                                                }
                                            }
                                        }
                                        _ => (),
                                    }
                                    let _ = edit::add_delete_value(
                                        settings,
                                        &mut resources.config_plist,
                                        false,
                                    );
                                    d.sort_keys();
                                    settings.held_item =
                                        Some(plist::Value::Dictionary(d.to_owned()));
                                    let _ = edit::add_delete_value(
                                        settings,
                                        &mut resources.config_plist,
                                        true,
                                    );
                                    if initial_depth != settings.depth {
                                        settings.sec_length[initial_depth] = d.len();
                                    }
                                }
                            }
                            _ => (),
                        }
                    }
                    settings.held_key = initial_key;
                    settings.held_item = initial_item;
                    settings.depth = initial_depth;
                    settings.modified = true;
                }
                KeyCode::Char('i') => {
                    if !showing_info {
                        if settings.is_resource() {
                            let _ = res::show_res_path(&resources, &settings, stdout);
                            showing_info = true;
                        } else {
                            let mut empty_vec = vec![];
                            showing_info = parse_tex::show_info(
                                &resources,
                                &settings,
                                false,
                                &mut empty_vec,
                                stdout,
                            )?;
                        }
                        write!(stdout, "\x1b[4m{}\x1b[0m\x1B[0K", " ".repeat(70))?;
                        if !showing_info {
                            settings.res_name(&mut res_name);
                            write!(
                                stdout,
                                "\r\x1b[4m \x1b[33mno info found for\x1b[0;4m {}\x1b[0m",
                                res_name,
                            )?;
                            showing_info = true;
                        }
                        stdout.flush()?;
                    } else {
                        showing_info = false;
                    }
                }
                KeyCode::Char('M') => {
                    snake::snake(stdout)?;
                    read_key()?;
                    write!(stdout, "\x1B[2J")?;
                }
                KeyCode::Char('s') => {
                    let mut config_file = PathBuf::from(&settings.config_file_name)
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    if !config_file.starts_with("modified_") {
                        let mut tmp = "modified_".to_string();
                        tmp.push_str(&config_file);
                        config_file = tmp.to_owned();
                    }
                    let save_file = PathBuf::from("INPUT").join(&config_file);
                    write!(
                        stdout,
                        "\r\n\n\x1B[0JSaving copy of plist to INPUT directory\r\n\n\x1B[32m\
                           Validating\x1B[0m {} with {} Acidanthera/ocvalidate\r\n",
                        config_file, settings.oc_build_version,
                    )?;
                    resources.config_plist.to_file_xml(&save_file)?;
                    settings.modified = false;
                    let _ = init::validate_plist(
                        &Path::new(&save_file).to_path_buf(),
                        &resources,
                        stdout,
                    )?;
                    showing_info = true;
                }
                _ => (),
            }
            if key != KeyCode::Char('i') && key != KeyCode::Char(' ') && key != KeyCode::Char('s') {
                showing_info = false;
            }
        }
    }

    write!(stdout, "\n\r\x1B[0J")?;
    stdout.flush()?;

    #[cfg(debug_assertions)]
    {
        println!("debug:   HashMap {:?}", settings.resource_ver_indexes);
    }

    Ok(())
}

fn main() {
    let current_dir = env::current_dir().expect("Didn't find current directory");

    let working_dir;
    let ver;
    #[cfg(not(debug_assertions))]
    {
        working_dir = env::current_exe()
            .unwrap()
            .parent()
            .expect("Didn't find working directory")
            .to_path_buf();
        ver = "0.3.4";
    }

    #[cfg(debug_assertions)]
    {
        working_dir = current_dir.to_owned();
        ver = "0.3.4 debug";
    }
    env::set_current_dir(&working_dir).expect("Unable to set environment");

    let mut setup = draw::Settings {
        config_file_name: String::new(),
        sec_num: [0; 5],
        depth: 0,
        sec_key: Default::default(),
        item_instructions: String::new(),
        held_item: None,
        held_key: Default::default(),
        sec_length: [0; 5],
        resource_sections: vec![],
        build_type: "release".to_string(),
        oc_build_version: "latest".to_string(),
        oc_build_date: String::new(),
        oc_build_version_res_index: Default::default(),
        resource_ver_indexes: Default::default(),
        can_expand: false,
        find_string: Default::default(),
        modified: false,
    };

    let mut resources = res::Resources {
        dortania: serde_json::json!(null),
        octool_config: serde_json::json!(null),
        resource_list: serde_json::json!(null),
        other: serde_json::json!(null),
        config_plist: plist::Value::Boolean(false),
        sample_plist: plist::Value::Boolean(false),
        working_dir_path: env::current_dir().unwrap(),
        open_core_binaries_path: PathBuf::new(),
        open_core_source_path: PathBuf::new(),
    };

    let mut config_file = working_dir.join("INPUT/config.plist");
    let args = env::args().skip(1).collect::<Vec<String>>();
    let mut args = args.iter();
    loop {
        if let Some(arg) = args.next() {
            if arg.starts_with('-') {
                for c in arg.chars() {
                    match c {
                        'o' => match args.next() {
                            Some(version) => setup.oc_build_version = version.to_owned(),
                            _ => {
                                println!(
                                    "\n\x1B[33mERROR:\x1b[0m You need to supply a version number \
                                    with the -o option\n"
                                );
                                println!("e.g. './octool -o \x1b[4m0.7.4\x1b[0m'\n");
                                std::process::exit(0);
                            }
                        },
                        'v' => {
                            println!("\noctool v{}", ver);
                            if std::env::consts::OS == "macos" {
                                match res::status(
                                    "nvram",
                                    &["4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102:opencore-version"],
                                ) {
                                    Ok(s) => println!(
                                        "\ncurrent loaded OpenCore version\n{}",
                                        String::from_utf8_lossy(&s.stdout)
                                    ),
                                    Err(_) => (),
                                }
                            }
                            std::process::exit(0);
                        }
                        'd' => setup.build_type = "debug".to_string(),
                        'h' => {
                            println!("SYNOPSIS\n\t./octool [options] [-o x.y.z] [config.plist]\n");
                            println!("OPTIONS\n\t-d  build debug version\n\t-h  print this help and exit\n\t-o x.y.z  \
                                     select OpenCore version number\n\t-v  show octool version info");
                            std::process::exit(0);
                        }
                        _ => (),
                    }
                }
            } else {
                config_file = current_dir.join(arg);
            }
        } else {
            break;
        }
    }

    terminal::enable_raw_mode().unwrap();

    let mut stdout = stdout();

    write!(stdout, "\x1b[2J").unwrap();

    stdout
        .execute(cursor::Hide)
        .unwrap()
        .execute(cursor::MoveTo(0, 0))
        .unwrap();

    init::init_static(&mut resources, &mut setup, &mut stdout).unwrap();

    if !config_file.exists() {
        write!(
            stdout,
            "\x1B[31mDid not find config at\x1B[0m {:?}\r\nWill use the latest Sample.plist from the OpenCorePkg\r\n",
            config_file
        )
        .unwrap();
        config_file = Path::new("").to_path_buf();
    } else {
        write!(stdout, "\r\nUsing {:?}\r\n", config_file).unwrap();
        resources.config_plist = plist::Value::from_file(&config_file)
            .expect(format!("Didn't find valid plist at {:?}", config_file).as_str());
        if &setup.oc_build_version == "latest" {
            setup.oc_build_version = guess_version(&resources);
            write!(stdout, "\x1b[33mGUESSING:\x1b[0m at OpenCore version of \x1b[33m{}\x1b[0m based on the input config.plist file\r\n\
                \tIf this is incorrect you can change the version used with the capital 'V' key in the editor\r\n\
                \tor run octool with the -o option and provide an OpenCore version number\r\n\n",
        setup.oc_build_version ).unwrap();
        }
    }
    stdout.flush().unwrap();

    match process(
        &mut config_file,
        &current_dir,
        &mut setup,
        &mut resources,
        &mut stdout,
    ) {
        Ok(()) => (),
        Err(e) => eprintln!("\r\n\x1B[31mERROR:\x1B[0m while processing plist: {:?}", e),
    }

    stdout.execute(cursor::Show).unwrap();

    terminal::disable_raw_mode().unwrap();
}
