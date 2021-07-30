* Header is much smaller than a block
* So we need a stream over many sequential blocks
  for directory
  for reading/writing "files" in general
* Who has the current file position?
  Is position user-visible?  Probably not.  Maybe.
  Maybe even have pread, pwrite.
