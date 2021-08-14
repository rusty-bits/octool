use crate::res::{get_res_path, status, Resources};

use fs_extra::copy_items;
use fs_extra::dir::{copy, CopyOptions};
use std::error::Error;
use std::io::{self, Write};

pub fn build_output(resources: &Resources) -> Result<(), Box<dyn Error>> {
    fs_extra::dir::remove("OUTPUT")?;
    std::fs::create_dir_all("OUTPUT/EFI")?;
    let mut has_open_canopy = false;

    let mut options = CopyOptions::new();
    options.overwrite = true;
    copy(&resources.open_core_pkg.join("X64/EFI"), "OUTPUT", &options)?;
    fs_extra::dir::remove("OUTPUT/EFI/OC/Drivers")?;
    fs_extra::dir::remove("OUTPUT/EFI/OC/Tools")?;
    std::fs::create_dir_all("OUTPUT/EFI/OC/Drivers")?;
    std::fs::create_dir_all("OUTPUT/EFI/OC/Tools")?;
    resources
        .config_plist
        .to_file_xml("OUTPUT/EFI/OC/config.plist")?;

    let mut from_paths = Vec::new();
    let kexts = resources.config_plist.as_dictionary().unwrap()["ACPI"]
        .as_dictionary()
        .unwrap()["Add"]
        .as_array()
        .unwrap();
    for val in kexts {
        let kext = val.as_dictionary().unwrap();
        if kext["Enabled"].as_boolean().unwrap() {
            let r = kext["Path"].as_string().unwrap().split('/').next().unwrap();
            from_paths.push(get_res_path(&resources, r, "ACPI", "release"));
        }
    }
    from_paths.sort();
    from_paths.dedup();
    copy_items(&from_paths, "OUTPUT/EFI/OC/ACPI", &options)?;

    let mut from_paths = Vec::new();
    let kexts = resources.config_plist.as_dictionary().unwrap()["Kernel"]
        .as_dictionary()
        .unwrap()["Add"]
        .as_array()
        .unwrap();
    for val in kexts {
        let kext = val.as_dictionary().unwrap();
        if kext["Enabled"].as_boolean().unwrap() {
            let r = kext["BundlePath"]
                .as_string()
                .unwrap()
                .split('/')
                .next()
                .unwrap();
            from_paths.push(get_res_path(&resources, r, "Kernel", "release"));
        }
    }
    from_paths.sort();
    from_paths.dedup();
    copy_items(&from_paths, "OUTPUT/EFI/OC/Kexts", &options)?;

    let mut from_paths = Vec::new();
    let kexts = resources.config_plist.as_dictionary().unwrap()["Misc"]
        .as_dictionary()
        .unwrap()["Tools"]
        .as_array()
        .unwrap();
    for val in kexts {
        let kext = val.as_dictionary().unwrap();
        if kext["Enabled"].as_boolean().unwrap() {
            let r = kext["Path"].as_string().unwrap().split('/').next().unwrap();
            from_paths.push(get_res_path(&resources, r, "Misc", "release"));
        }
    }
    from_paths.sort();
    from_paths.dedup();
    copy_items(&from_paths, "OUTPUT/EFI/OC/Tools", &options)?;

    let mut from_paths = Vec::new();
    let drivers = resources.config_plist.as_dictionary().unwrap()["UEFI"]
        .as_dictionary()
        .unwrap()["Drivers"]
        .as_array()
        .unwrap();
    for val in drivers {
        let driver = val.as_string().unwrap().to_string();
        if !driver.starts_with('#') {
            if &driver == "OpenCanopy.efi" {
                has_open_canopy = true;
            }
            from_paths.push(get_res_path(&resources, &driver, "UEFI", "release"));
        }
    }
    from_paths.sort();
    from_paths.dedup();
    copy_items(&from_paths, "OUTPUT/EFI/OC/Drivers", &options)?;

    if has_open_canopy {
        copy(
            "resources/OcBinaryData/Resources",
            "OUTPUT/EFI/OC",
            &options,
        )?;
    }
    match resources.config_plist.as_dictionary().unwrap()["Misc"]
        .as_dictionary()
        .unwrap()["Security"]
        .as_dictionary()
        .unwrap()["Vault"]
        .as_string()
        .unwrap()
    {
        "Secure" => {
            println!("found Misc->Security->Vault set to Secure");
            print!("\x1B[32mComputing\x1B[0m vault.plist ... ");
            io::stdout().flush()?;
            let _ = status(
                &resources
                    .open_core_pkg
                    .join("Utilities/CreateVault/create_vault.sh")
                    .to_str()
                    .unwrap(),
                &["OUTPUT/EFI/OC/."],
            );
            let _ = status(
                &resources
                    .open_core_pkg
                    .join("Utilities/CreateVault/RsaTool")
                    .to_str()
                    .unwrap(),
                &[
                    "-sign",
                    "OUTPUT/EFI/OC/vault.plist",
                    "OUTPUT/EFI/OC/vault.sig",
                    "OUTPUT/EFI/OC/vault.pub",
                ],
            );
            println!("\x1B[32mdone\x1B[0m");
            print!("\x1b[32mSigning\x1B[0m OpenCore.efi ... ");
            io::stdout().flush()?;
            let out = status("strings", &["-a", "-t", "d", "OUTPUT/EFI/OC/OpenCore.efi"])?;
            let mut offset = 0;
            for line in String::from_utf8(out.stdout).unwrap().lines() {
                let (off, s) = line.split_once(' ').unwrap();
                if s == "=BEGIN OC VAULT=" {
                    offset = off.parse::<i32>().unwrap() + 16;
                }
            }
            let mut seek = "seek=".to_string();
            seek.push_str(&offset.to_string());
            let _ = status(
                "dd",
                &[
                    "of=OUTPUT/EFI/OC/OpenCore.efi",
                    "if=OUTPUT/EFI/OC/vault.pub",
                    "bs=1",
                    &seek,
                    "count=528",
                    "conv=notrunc",
                ],
            );
            std::fs::remove_file("OUTPUT/EFI/OC/vault.pub")?;
            println!("\x1B[32mdone\x1B[0m");
        }
        _ => (),
    }
    /*
    517104
    resources/OpenCore-0.7.3-RELEASE/Utilities/CreateVault/create_vault.sh OUTPUT/EFI/OC/.
    resources/OpenCore-0.7.3-RELEASE/Utilities/CreateVault/RsaTool -sign OUTPUT/EFI/OC/vault.plist OUTPUT/EFI/OC/vault.sig OUTPUT/EFI/OC/vault.pub
    off=$(($(strings -a -t d OUTPUT/EFI/OC/OpenCore.efi | grep "=BEGIN OC VAULT=" | cut -f1 -d' ')+16))
    dd of=OUTPUT/EFI/OC/OpenCore.efi if=OUTPUT/EFI/OC/vault.pub bs=1 seek=$off count=528 conv=notrunc
    rm OUTPUT/EFI/OC/vault.pub
    */
    Ok(())
}
