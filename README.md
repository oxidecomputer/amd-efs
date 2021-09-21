# Purpose

This crate allows you to manipulate firmware, directly on flash.

# Usage

Add

    amd-efs = { path = "../amd-efs", default_features = false, features = [] }

to the `[dependencies]` block in your `Cargo.toml`.

To iterate, you can do:

    let efs = match Efs::<_, 0x1000, 0x2_0000>::load(storage) {
        Ok(efs) => {
            efs
        },
        Err(e) => {
            eprintln!("Error on load: {:x?}", e);
            std::process::exit(1);
        }
    };
    let efh = efs.embedded_firmware_structure(None)?;
    let psp_directory = efs.psp_directory(&efh)?;
    for directory in efs.bios_directories(&efh)? {
        println!("{:x?}", directory.header);
    }
