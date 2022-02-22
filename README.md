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

# AMD boot process

The AMD boot process goes as follows:

* There's a pinstrap (`PspSoftFuseChain32MiBSpiDecoding`) that selects which 16 MiB half of a 32 MiB flash to use/have visible.
* Within a 32 MiB part, the PSP will first try to locate a valid EFH in the lower 16 MiB and then (if the former was unsuccessful) try to locate a valid EFH in the upper 16 MiB of it.  This search verifies both the magic signature of the EFH and the second_gen_efs bits (the latter basically chooses either Rome or Milan, one for each of the 16 MiB).
* Starting in this EFH, it locates the PSP directory table. That is either a PSP combo directory table, or a direct PSP directory table. The combo would allow further more detailed selection of one of multiple PSP directory tables depending on a filter stored with the combo directory entry.

Note: The PSP, at any time, sees only 16 MiB of the flash at once.  When using the legacy interface of x86 flash access, the same is true.

That means that you as the user of this crate have to decide which of these possible parts you want to manipulate. The crate will NOT guess.

If you just use a pass-through `storage`, that effectively puts the firmware in the lowest 16 MiB part. That might be what you want anyway.

It's also possible to instead provide a window that only shows one of the other `16 MiB` parts as `storage`, making `amd-efs` manipulate that part only.

Finally, you have to decide whether you want a PSP combo directory table or not. Depending on that, you have to call different functions of the `Efs` struct. If you don't know what that means, you don't want a PSP combo directory and rather want a PSP normal directory.
