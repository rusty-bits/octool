//use crate::res::status;
use crate::res::{self, Resources};

use fs_extra::copy_items;
use fs_extra::dir::{copy, CopyOptions};
use std::error::Error;

pub fn build_output(resources: &Resources) -> Result<(), Box<dyn Error>> {
    fs_extra::dir::remove("OUTPUT")?;
    std::fs::create_dir_all("OUTPUT/EFI")?;

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
            from_paths.push(res::get_res_path(&resources, r, "ACPI", "release"));
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
            from_paths.push(res::get_res_path(&resources, r, "Kernel", "release"));
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
            from_paths.push(res::get_res_path(&resources, r, "Misc", "release"));
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
            from_paths.push(res::get_res_path(&resources, &driver, "UEFI", "release"));
        }
    }
    from_paths.sort();
    from_paths.dedup();

    copy_items(&from_paths, "OUTPUT/EFI/OC/Drivers", &options)?;
    copy("resources/OcBinaryData/Resources", "OUTPUT/EFI/OC", &options)?;

    Ok(())
}
