# Important

* Support directory headers somewhere else than the content
  * directory_beginning, contents_beginning: Figure out what exactly happens when base_address /= 0.  Does it mean the entries move there, too?
* Directory tables: update checksum (fletcher)

# Tests

* Tests: secondary psp directory, secondary bios directory.

# Convenience and Resilience

* Don't allow creation of entry that are already there (unique key = ?)
* bios_directories: Return Err directly if appropriate
* Make rom_id more useful?  (maybe not?)
* Make it possible to add ELF image, see https://github.com/oxidecomputer/rfd/tree/0215/rfd/0215
  * In general, both APCB and ELF need postprocessing (checksum and load address, respectively)

# Naming

* Condensing the names: "Efs" and "Bhd" be treated as opaque acronyms
  * "Bios" -> "Bhd"
  * "efs" instead of "embedded_firmware_header"

# Later with Flash

* Cache about 5 flash blocks;  Cache can be transparent to FlashRead, FlashWrite interface
  * last_recenty_used
  * write, dirty

# Later if we need it

* Create secondary bios directory
