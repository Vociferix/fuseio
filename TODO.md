Items to implement or revisit:

* Support more mount options (`MountOpt`)
  * macFUSE uses `iosize` to set buffer sizes
  * Might need a `MountOpt::Custom(OsString)`
    * Possibly also `MountOpt::CustomKv(OsString, OsString)`, but `MountOpt::Custom("{key}={value}")` could suffice
* Pull the latest commits of `libfuse` and `macFUSE` to check for newer unimplemented features/ops
  * The local kernel tree is already at 7.46 while the crate advertises 7.45
  * 7.45 added `FUSE_NOTIFY_PRUNE` (notify code 9), which isn't implemented
  * 7.46 is entirely io_uring transport work (`FUSE_IO_URING_CMD_ADD_QUEUE`,
    bufpool, zero-copy), so 7.45 stays honest until that's considered
* Add a mocking system that simulates a FUSE device so users can write proper unit tests for `Fs` impls
* Add `#[inline]` appropriately throughout the crate
* Update public documentation to not expand on implementation details that are irrelevant to the user
  * Meaning, public doc comments shouldn't explain why something is implemented in a particular way,
    it should stick to explaining behaviors that impact the user. This can sometimes include
    implementation details, but only when those details effect how the user should use the item.
* Document the test requirements in `README.md`
  * The `fusermount` tests mount for real, so they need `fusermount` or `fusermount3`
    installed (Ubuntu's `fuse3` package carries both) and a usable `/dev/fuse`
