# Short-term

* Replace de_mmio by mmio_decode ?
* Efs:create: Handle directory_address_mode.
* Callers of location_of_source: Fix arguments.
* Check for Directory.AddressMode > 2 (= 3), and error out if it is
* Do upper16.

# Important

* Connect via hubris ./drv/stm32h7-spi-server/src/main.rs
* Is crossing erase page boundary when writing to the flash handled? FIXME
* serde also create secondary directories ~
* psp directory entry high 2 bits of location are address mode    !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
* bhd directory entry high 2 bits of location are address mode   !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
  * Maybe change ValueOrLocation type?
    * Depends on directory header's address_mode; if that is 2, then entry address mode exists; otherwise it's 0.
      * AddressMode 0: X86 MMIO physical address; bits 63...56 are 0; so addr is 56 bits
      * AddressMode 1: relative address to entire BIOS image; might be 16M binary relative offset; bit 63~56: 0x40
      * AddressMode 2: relative address to PSP/BIOS directory; bit 63~56: 0x80
      * AddressMode 3 (!) relative address to slot N (only valid on the entry); bit 63~56: 0xC0
    * See #55758 page 120 for example
* bhddirectoryentry should NOT have a new_value !!! According to the documentation, no.
  It's actually just a location, and the weird two high bits (see above).
* Support directory headers somewhere else than the content
  * directory_beginning, contents_beginning: Figure out what exactly happens when base_address /= 0.  Does it mean the entries move there, too?
* Directory tables: update checksum (fletcher) less often (currently that's done on EVERY entry; better do it on drop maybe)

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

* Create secondary bios directory
  * Tests: secondary psp directory, secondary bios directory.

# Later after release

* Compression
* Make find_payload_empty_slot skip over zlib-compressed entries correctly
  (Entry.size is the UNCOMPRESSED size; new header is 256 byte (all 0, except offset 0x14 size 4: compressed image size excluding the header))
