
# Important

* Connect via hubris ./drv/stm32h7-spi-server/src/main.rs
* Is crossing page boundary when writing to the flash handled?
* bhddirectoryentry should not have a new_value !!!
* serde also create secondary directories ~
* psp directory entry high 2 bits of location are address mode    !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
* bios directory entry high 2 bits of location are address mode   !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
* Support directory headers somewhere else than the content
  * directory_beginning, contents_beginning: Figure out what exactly happens when base_address /= 0.  Does it mean the entries move there, too?
* Directory tables: update checksum (fletcher) less often (currently that's done on EVERY entry; better do it on drop maybe)

# Tests

* Tests: secondary psp directory, secondary bios directory.

# Convenience and Resilience

* Make it actually export `DirectoryAdditionalInfo` to JSON
  * Then handle the `0` special case
* `Serde` helper structs necessarily use `modular-bitfield`'s too-large `InOut` registers (for example `u8` for `B5`). Therefore, it's possible for the user to set a way-too-large value using JSON.
  * Fixing this would require adapting `modular-bitfield`'s derive macro, and using `ux_serde` or whatever new thing is now en vogue.
* DirectoryAdditionalInfo: Most are in 4 kiB units and have strange bit count. Maybe make those (max_size, base_address) nicer.
* BhdDirectoryEntryAttrs: instance is u4; sub_program is u3; rom_id is u2.
  * Making instance, sub_program and rom_id enumerated would prevent those.
    * But that's a workaround.
* modular-bitfield: generate_specifier_for: "let in_out =" too coarse-grained.
  * Maybe adapt that.  Otherwise we have funny problems using the result of getters to store into JSON--since the JSON type is actually the right size (!).
* Don't allow creation of directory entry that are already there (unique key = (type, subprogram, instance)) ; what about rom_id ?!
* bios_directories: Return Err directly if appropriate
* Make rom_id more useful?  (maybe not?)

# Later with Flash

* Cache about 5 flash blocks;  Cache can be transparent to FlashRead, FlashWrite interface
  * last_recenty_used
  * write, dirty

# Later if we need it

* Use https://docs.rs/schemars/latest/schemars/ to autogenerate JSON Schema
* Create secondary bios directory
