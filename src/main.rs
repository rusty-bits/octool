mod build;
mod draw;
mod edit;
mod init;
mod parse_tex;
mod res;
mod snake;

use fs_extra::dir::{copy, CopyOptions};
use res::check_order;
use std::collections::HashMap;

use std::fs::{self, File, ReadDir};
use std::io::{stdout, BufReader, Stdout, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::{env, error::Error};

use crossterm::{
    cursor,
    event::{KeyCode, KeyModifiers},
    style::available_color_count,
    terminal, ExecutableCommand,
};

use crate::edit::read_key;
use crate::init::{guess_version, Manifest, Settings};
use crate::res::Resources;

const OCTOOL_VERSION: &str = &"v0.4.7 2022-07-31";

fn process(
    config_plist: &mut PathBuf,
    current_dir: &PathBuf,
    mut settings: &mut Settings,
    mut resources: &mut Resources,
    stdout: &mut Stdout,
) -> Result<(), Box<dyn Error>> {
    let mut found = vec![edit::Found::new()];
    let mut found_id: usize = 0;
    let mut res_name = String::new();

    init::init_oc_build(&mut resources, settings, stdout)?;
    init::init_plist(config_plist, &mut resources, settings, stdout)?;

    let mut key = KeyCode::Char('q');
    let mut key_mod;

    if !check_order(settings, resources, stdout, true) {
        write!(stdout, "\x1b[33mWARNING: Trouble(s) found in the Kernel > Add section:\x1b[0m\r\n either a missing \
                        dependency or a misordered resource\r\n go to the Kernel > Add section and use the 'O' command to \
                        attempt an automatic repair\r\n\r\n").unwrap();
    }

    write!(
        stdout,
        "\x1b[33mSUMMARY:\r\n\x1B[32moctool version\x1b[0m {}\r\n\
        \x1b[32mbuild_type set to\x1B[0m {}\r\n\x1B[32mbuild_version set to\x1B[0m {}\r\n",
        settings.octool_version, settings.build_type, settings.oc_build_version,
    )?;

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
            (key, key_mod) = read_key()?;
            match key {
                KeyCode::Char('q') => {
                    if showing_info {
                        showing_info = false;
                    } else {
                        if settings.modified == true {
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
                    if !check_order(settings, resources, stdout, true) {
                        write!(stdout, "\x1b[33mWARNING: Trouble(s) found in the Kernel > Add section:\x1b[0m\r\n either a missing \
                        dependency or a misordered resource\r\n go to the Kernel > Add section and use the 'O' command to \
                        attempt an automatic repair\r\n\r\n").unwrap();
                    }
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

                            delete_dir_contents(fs::read_dir(current_dir.join("EFI")));
                            fs::remove_dir_all(current_dir.join("EFI"))?;

                            let mut options = CopyOptions::new();
                            options.overwrite = true;
                            copy("OUTPUT/EFI", current_dir, &options)?;
                        }
                    }
                    break;
                }
                KeyCode::Char('a') => edit::add_item(settings, &mut resources, "", stdout),
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
                KeyCode::Char('O') => {
                    //limit 'O' command to Kernel>Add section
                    if settings.depth != 2
                        || settings.sec_key[0] != "Kernel"
                        || settings.sec_key[1] != "Add"
                    {
                        continue;
                    }
                    //will need Multi_Pass to check, add & sort resources (she knows it's a Multi_Pass)
                    write!(
                        stdout,
                        "\r\n\x1b[2K\x1b[32mChecking\x1b[0m for missing requirements and wrong order\r\n"
                    )
                    .unwrap();
                    write!(stdout, "\x1b[2K\r\n").unwrap();
                    let mut order_attempts = 0;
                    while !res::check_order(settings, resources, stdout, false) {
                        order_attempts += 1;
                        if order_attempts > 10 {
                            write!(
                                stdout,
                                "\x1b[2K\x1b[33mHmm, I just looped 10 times.  \
                                   Am I broken or is it just many fixes?\x1b[0m\r\n"
                            )
                            .unwrap();
                            break;
                        }
                    }
                    write!(stdout, "\x1b[2K\r\n\x1b[32mDone\x1b[0m\x1b[0K\r\n").unwrap();

                    //                    let _ = res::check_order(settings, &mut resources, stdout);

                    showing_info = true;
                }
                KeyCode::Char('p') => {
                    if edit::add_delete_value(settings, &mut resources.config_plist, true) {
                        settings.add();
                    }
                }
                KeyCode::Char('P') => {
                    res::purge_whole_plist(settings, resources, stdout);
                    showing_info = true;
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
                            yel = "\x1b[33m",
                            grn = "\x1b[32m",
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
                    let mut parent_res = "OpenCorePkg".to_string();
                    if settings.is_resource() {
                        // get parent name of selected resource
                        settings.res_name(&mut parent_res);
                        if let Some(p) = resources.resource_list[&parent_res]["parent"].as_str() {
                            parent_res = p.to_string();
                        } else {
                            write!(
                                stdout,
                                " \x1b[33mNo versions found for {}\x1b[0m\x1b[0K",
                                parent_res
                            )?;
                            stdout.flush()?;
                            showing_info = true;
                            parent_res = "".to_owned();
                        }
                    }
                    if parent_res.len() > 0 {
                        let mut versions = vec![];
                        let mut indexes = vec![];
                        res::get_parent_version_nums(
                            &parent_res,
                            &resources,
                            &mut versions,
                            &mut indexes,
                        );
                        let mut new_ver;
                        if versions.len() > 0 {
                            if &parent_res != "OpenCorePkg" {
                                let mut res_ver_name = String::new();
                                settings.res_name(&mut res_ver_name);
                                new_ver =
                                    match res::res_version(settings, &resources, &res_ver_name) {
                                        Some(s) => s,
                                        None => "".to_string(),
                                    }
                                // versions[0].split("---").next().unwrap().trim().to_owned();
                            } else {
                                new_ver = settings.oc_build_version.to_owned();
                                // reset resource versions if OpenCore version is changed
                                if resources.octool_config["reset_res_versions"]
                                    .as_bool()
                                    .unwrap_or(true)
                                {
                                    settings.resource_ver_indexes.clear();
                                }
                            }

                            write!(
                            stdout,
                            "\x1b[2K\r\n{}\r\x1B[2K\x1b[32mEnter or select {} version number:\x1b[0m {}\r\n\x1B[2K\x1B8",
                            cursor::Show,
                            &parent_res,
                            cursor::SavePosition,
                        )?;
                            edit::edit_string(&mut new_ver, Some(&versions), stdout)?;
                            if &parent_res == "OpenCorePkg" {
                                settings.oc_build_version = new_ver;
                                init::init_oc_build(&mut resources, settings, stdout)?;
                                if settings.oc_build_version == "not found" {
                                    stdout.flush()?;
                                    showing_info = true;
                                }
                            } else {
                                for (i, v) in versions.iter().enumerate() {
                                    if v.split("---").next().unwrap_or("").trim() == new_ver {
                                        settings.resource_ver_indexes.insert(
                                            parent_res.to_owned(),
                                            Manifest(
                                                indexes[i],
                                                resources.dortania[&parent_res]["versions"]
                                                    [indexes[i]]["commit"]["sha"]
                                                    .as_str()
                                                    .unwrap_or("no sha")
                                                    .to_owned(),
                                            ),
                                        );
                                    }
                                }
                            }
                        } else {
                            write!(
                                stdout,
                                " \x1b[33mNo versions found for parent resource {}\x1b[0m\x1b[0K",
                                parent_res
                            )?;
                            stdout.flush()?;
                            showing_info = true;
                        }
                    }
                    write!(stdout, "{}", cursor::Hide)?;
                }
                KeyCode::Char('R') => {
                    if settings.is_resource() {
                        let ext = match settings.sec_key[0].as_str() {
                            "ACPI" => ".aml",
                            "Kernel" => ".kext",
                            "Misc" => ".efi",
                            "UEFI" => ".efi",
                            _ => "",
                        };
                        let mut a = vec![];
                        for res in resources.resource_list.as_object().unwrap() {
                            if res.0.contains(ext) {
                                a.push(res.0);
                            }
                        }
                        a.sort();
                        write!(stdout, "{:?}", a)?;
                        stdout.flush()?;
                        read_key()?;
                    }
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
                            yel = "\x1b[33m",
                            grn = "\x1b[32m",
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
                KeyCode::Char('M') => {
                    res::merge_whole_plist(settings, resources, stdout, false);
                    stdout.flush().unwrap();
                    showing_info = true;
                }
                KeyCode::Char('I') => {
                    res::merge_whole_plist(settings, resources, stdout, true);
                    stdout.flush().unwrap();
                    showing_info = true;
                }
                KeyCode::Char('i') => {
                    if !showing_info {
                        let mut empty_vec = vec![];
                        if settings.is_resource() {
                            let _ = res::show_res_info(&mut resources, &mut settings, stdout);
                            showing_info = true;
                        } else {
                            showing_info = parse_tex::show_info(
                                &resources,
                                &settings,
                                false,
                                &mut empty_vec,
                                stdout,
                            )?;
                        }
                        if !showing_info && empty_vec.len() == 0 {
                            settings.res_name(&mut res_name);
                            write!(
                                stdout,
                                "\r\x1b[4m \x1b[33mno info found for{}\x1b[4m {}",
                                &settings.bg_col_info, res_name,
                            )?;
                            showing_info = true;
                        }
                        write!(stdout, "\x1b[0m")?;
                        stdout.flush()?;
                    } else {
                        showing_info = false;
                    }
                }
                KeyCode::Char('S') => {
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
                    let save_path = PathBuf::from("INPUT").join(&config_file);
                    write!(
                        stdout,
                        "\r\n\n\x1B[0JSaving copy of plist to INPUT directory\r\n\n\x1B[32m\
                           Validating\x1B[0m {} with {} Acidanthera/ocvalidate\r\n",
                        config_file, settings.oc_build_version,
                    )?;
                    resources.config_plist.to_file_xml(&save_path)?;

                    //save manifest
                    config_file.push_str(".man");
                    let manifest_path = PathBuf::from("INPUT").join(&config_file);
                    let manifest_file = match File::create(&manifest_path) {
                        Err(e) => panic!("Couldn't open {:?}: {}", &save_path, e),
                        Ok(f) => f,
                    };

                    let mut out_parent_shas = HashMap::<String, String>::default();
                    for v in &settings.resource_ver_indexes {
                        out_parent_shas.insert(v.0.to_owned(), v.1 .1.to_owned());
                    }
                    let man_out = (
                        &settings.build_type,
                        &settings.oc_build_version,
                        &out_parent_shas,
                        &resources.config_plist,
                    );
                    serde_json::to_writer(&manifest_file, &man_out)?;
                    //                    resources.config_plist.to_writer_xml(&manifest_file)?;

                    let _ = init::validate_plist(
                        &Path::new(&save_path).to_path_buf(),
                        &resources,
                        stdout,
                    )?;
                    if !check_order(settings, resources, stdout, true) {
                        write!(stdout, "\x1b[33mWARNING: Trouble(s) found in the Kernel > Add section:\x1b[0m\r\n either a missing \
                        dependency or a misordered resource\r\n go to the Kernel > Add section and use the 'O' command to \
                        attempt an automatic repair\r\n\r\n").unwrap();
                    }

                    showing_info = true;
                    settings.modified = false;
                }
                _ => (),
            }
            if key != KeyCode::Char('i')
                && key != KeyCode::Char(' ')
                && key != KeyCode::Char('s')
                && key != KeyCode::Char('V')
                && key != KeyCode::Char('M')
                && key != KeyCode::Char('P')
                && key != KeyCode::Char('O')
                && key != KeyCode::Char('I')
            {
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
    let current_dir = env::current_dir().expect("Finding current directory");
    let working_dir;

    #[cfg(not(debug_assertions))]
    {
        working_dir = env::current_exe()
            .unwrap()
            .parent()
            .expect("Didn't find working directory")
            .to_path_buf();
    }

    #[cfg(debug_assertions)]
    {
        working_dir = current_dir.to_owned();
    }
    env::set_current_dir(&working_dir).expect("Setting up environment");

    let mut resources = Resources {
        dortania: Default::default(),
        octool_config: Default::default(),
        config_differences: Default::default(),
        resource_list: Default::default(),
        other: Default::default(),
        config_plist: plist::Value::Boolean(false),
        sample_plist: plist::Value::Boolean(false),
        working_dir_path: Default::default(),
        open_core_binaries_path: Default::default(),
        open_core_source_path: Default::default(),
    };

    if !working_dir.join("INPUT").exists() {
        std::fs::create_dir_all(working_dir.join("INPUT")).expect("creating INPUT directory");
    }

    if !working_dir.join("tool_config_files").exists() {
        std::fs::create_dir_all(working_dir.join("tool_config_files"))
            .expect("creating tool_config_files directory");
    }

    if !working_dir
        .join("tool_config_files/octool_config.json")
        .exists()
    {
        let url = "https://raw.githubusercontent.com/rusty-bits/octool/main/tool_config_files/octool_config.json";
        let path = working_dir.join("tool_config_files/octool_config.json");
        res::curl_file(&url, &path).expect("getting latest octool_config file");
    }

    //load octool config file
    resources.octool_config =
        res::get_serde_json_quiet("tool_config_files/octool_config.json").unwrap();
    let latest_octool_ver = res::get_latest_ver(&resources).expect("finding version");

    let mut setup = Settings {
        held_item: None,
        build_type: "release".to_string(),
        oc_build_version: "latest".to_string(),
        octool_version: OCTOOL_VERSION.to_string(),
        show_info_url: resources.octool_config["show_url_in_info_screens"]
            .as_bool()
            .unwrap_or(true),
        can_expand: false,
        modified: false,
        bg_col: "\x1b[0;38;5;231;48;5;232m".to_string(),
        bg_col_info: if available_color_count() >= 256 {
            "\x1b[0;38;5;231;48;5;235m".to_string()
        } else {
            "\x1b[0;40m".to_string()
        },
        ..Default::default()
    };

    let mut stdout = stdout();

    if resources.octool_config["clobber_local_dyn_res_list"]
        .as_bool()
        .unwrap_or(true)
    {
        // get dynamic res list zip
        let url = resources.octool_config["octool_latest_dyn_res_list_url"]
            .as_str()
            .expect("getting url from config");
        let zip_path = &working_dir.join("tool_config_files/dyn_res_list.zip");
        res::curl_file(&url, &zip_path).expect("getting dynamic res list");

        // unzip dynamic res list
        let z_file = File::open(&zip_path).expect("opening zip file");
        let mut z_archive = zip::ZipArchive::new(z_file).expect("creating archive");
        match z_archive.extract(&working_dir.join("tool_config_files")) {
            Ok(_) => (), // leave zip file in place
            Err(e) => panic!("{:?}", e),
        }
    }

    //load config_differences
    resources.config_differences =
        res::get_serde_json_quiet("tool_config_files/config_differences.json").unwrap();

    let mut config_file = working_dir.join("INPUT/config.plist");
    let args = env::args().skip(1).collect::<Vec<String>>();
    let mut args = args.iter();
    loop {
        if let Some(arg) = args.next() {
            if arg.starts_with('-') {
                for c in arg.chars() {
                    match c {
                        'h' => {
                            write!(
                                stdout,
                                "SYNOPSIS\r\n\t./octool [options] [-o x.y.z] [config.plist]\r\n"
                            )
                            .unwrap();
                            write!(stdout, "OPTIONS\r\n\t-d  build debug version\n\t-h  print this help and exit\r\n\t-o x.y.z  \
                                     select OpenCore version number\r\n\t-v  show octool version info\r\n").unwrap();
                            std::process::exit(0);
                        }
                        'v' => {
                            write!(stdout, "\r\noctool {}", setup.octool_version).unwrap();
                            if latest_octool_ver > setup.octool_version {
                                write!(stdout, " \x1b[31mupdate available\x1b[0m").unwrap();
                            }
                            write!(stdout, "\r\n").unwrap();
                            if std::env::consts::OS == "macos" {
                                match res::status(
                                    "nvram",
                                    &["4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102:opencore-version"],
                                ) {
                                    Ok(s) => write!(
                                        stdout,
                                        "\r\ncurrent loaded OpenCore version\r\n{}",
                                        String::from_utf8_lossy(&s.stdout)
                                    )
                                    .unwrap(),
                                    Err(_) => (),
                                }
                            }
                            std::process::exit(0);
                        }
                        'o' => match args.next() {
                            Some(version) => setup.oc_build_version = version.to_owned(),
                            _ => {
                                write!(stdout,
                                    "\r\n\x1B[33mERROR:\x1b[0m You need to supply a version number \
                                    with the -o option\r\n"
                                )
                                .unwrap();
                                write!(stdout, "e.g. './octool -o \x1b[4m0.7.4\x1b[0m'\r\n")
                                    .unwrap();
                                std::process::exit(0);
                            }
                        },
                        'd' => setup.build_type = "debug".to_string(),
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
    stdout
        .execute(cursor::Hide)
        .unwrap()
        .execute(cursor::MoveTo(0, 0))
        .unwrap();

    write!(stdout, "\x1b[2J").unwrap();

    if latest_octool_ver > setup.octool_version {
        write!(
            stdout,
            "\x1b[33mNOTICE: Updated version of octool is available, it can be found at\r\n{}\r\n\
            Latest version of octool is \x1b[0m{}\x1b[33m you are\x1b[0m\r\n",
            resources.octool_config["octool_releases_url"]
                .as_str()
                .unwrap(),
            latest_octool_ver,
        )
        .unwrap();
        setup
            .octool_version
            .push_str(" \x1b[31mupdate available\x1b[0m");
    }

    #[cfg(debug_assertions)]
    {
        setup.octool_version.push_str(" debug");
    }

    write!(
        stdout,
        "octool {}\r\n",
        setup.octool_version
    )
    .unwrap();
    match init::init_static(&mut resources, &mut setup, &mut stdout) {
        Ok(_) => (),
        Err(e) => {
            write!(stdout, "\r\n\x1b[31mError:\x1b[0m while trying to initialize\r\n{:?}\r\nIs octool in it's proper folder?\r\n",
                   e).unwrap();
            stdout.execute(cursor::Show).unwrap();
            terminal::disable_raw_mode().unwrap();
            exit(1);
        }
    }

    if !config_file.exists() {
        write!(
            stdout,
            "\x1B[31mDid not find config at\x1B[0m {:?}\r\nWill use the latest Sample.plist from the OpenCorePkg\r\n",
            config_file
        )
        .unwrap();
        config_file = Path::new("").to_path_buf();
    } else {
        write!(stdout, "\r\n\x1b[32mUsing\x1b[0m {:?}\r\n", config_file).unwrap();
        if config_file.to_str().unwrap().ends_with(".man") {
            let manifest_file = match File::open(&config_file) {
                Err(e) => panic!("Couldn't open {:?}: {}", &config_file, e),
                Ok(f) => f,
            };
            let manifest_reader = BufReader::new(&manifest_file);

            let parent_shas: HashMap<String, String>;
            (
                setup.build_type,
                setup.oc_build_version,
                parent_shas,
                resources.config_plist,
            ) = serde_json::from_reader(manifest_reader).unwrap();
            for (parent, sha) in parent_shas {
                let mut i = 0;
                loop {
                    if let Some(v) =
                        resources.dortania[&parent]["versions"][i]["commit"]["sha"].as_str()
                    {
                        if v == &sha {
                            setup.resource_ver_indexes.insert(parent, Manifest(i, sha));
                            break;
                        }
                    } else {
                        break;
                    }
                    i += 1;
                }
            }

            config_file =
                PathBuf::from(config_file.to_str().unwrap().strip_suffix(".man").unwrap());
            /* do not write config.plist out until DATA as byte array issue is resolved
                        resources
                            .config_plist
                            .to_file_xml(&config_file)
                            .expect("writing config.plist");
            */
        }

        resources.config_plist = plist::Value::from_file(&config_file)
            .expect(format!("Didn't find valid plist at {:?}", config_file).as_str());
        if &setup.oc_build_version == "latest" {
            let first_diff;
            (setup.oc_build_version, first_diff) = guess_version(&resources);
            //use the latest version of OpenCore as a guess if there have been no changes to the
            //config.plist, this makes the assumption that the user wants to keep the OpenCore
            //version current, they can always use a Manifest or manually use an older version
            if first_diff
                && resources.octool_config["use_latest_oc_on_guess"]
                    .as_bool()
                    .unwrap_or(true)
            {
                setup.oc_build_version = resources.dortania["OpenCorePkg"]["versions"][0]
                    ["version"]
                    .as_str()
                    .unwrap()
                    .to_owned();
            }
            if &setup.oc_build_version == "" {
                // set to version befoce ocvalidate, maybe do something better in the future
                setup.oc_build_version = "0.5.9".to_string();
            }
            write!(stdout, "\x1b[33mGUESSING:\x1b[0m at OpenCore version of \x1b[33m{}\x1b[0m based on the input config.plist file\r\n\
                \tIf this is incorrect you can change the version used with the capital 'V' key on the next screen\r\n\
                \tor run octool with the -o option and provide an OpenCore version number\r\n\n", setup.oc_build_version ).unwrap();
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

fn delete_dir_contents(read_dir_res: Result<ReadDir, std::io::Error>) {
    if let Ok(dir) = read_dir_res {
        for entry in dir {
            if let Ok(entry) = entry {
                let path = entry.path();

                println!("removing {:?}", path);
                if path.exists() {
                    if path.is_dir() {
                        delete_dir_contents(fs::read_dir(&path));
                        fs::remove_dir_all(path).expect("Failed to remove a dir");
                    } else {
                        fs::remove_file(path).expect("Failed to remove a file");
                    }
                }
            };
        }
    };
}
