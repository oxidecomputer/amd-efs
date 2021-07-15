
# Flash access crate

* QSPI
  * Note: QSPI is MMIO mapped for reading, but not for writing
* rw block size
* erase block size (is bigger than rw block size)

# amd-efh crate

* Access blocks, not bytes, not stream
  * Maybe maintain Flash allocation bitmap maybe?
    * Maybe just wing it
* When to erase
  * diff
  * or just only touch things as you go
  * Skip erasing for all-1 ?
* Directory: needs stream abstraction
  * over a certain number of (consecutive) blocks; growable?!
* Minimal entry body size seems to be 0x100
