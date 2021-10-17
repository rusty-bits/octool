This is the `INPUT` folder  

Place your `config.plist` in here along with any custom files you want included in your `EFI`  

for example  
- `SSDT-XXX.aml` files specific to your build  
- `USBMap.kext` or similar if you have mapped your USB ports with the USBMap.command or USBToolBox  
- custom theme files for `OpenCanopy`  
- other custom drivers and kexts, e.g. things like `Solarflare10GbE.kext` that I need for my ethernet  

basically, any files that are not part of the Dortania builds or listed in the `tool_config_files/resource_list.json`
