Items to implement or revisit:

* Implement `FUSE_MONITOR` (macFUSE 5 only)
* Support more mount options (`MountOpt`)
  * macFUSE uses `iosize` to set buffer sizes
  * Might need a `MountOpt::Custom(OsString)`
    * Possibly also `MountOpt::CustomKv(OsString, OsString)`, but `MountOpt::Custom("{key}={value}")` could suffice
* Pull the latest commits of `libfuse` and `macFUSE` to check for newer unimplemented features/ops
* Add a mocking system that simulates a FUSE device so users can write proper unit tests for `Fs` impls
* Provide API(s) for filesystems to check if features are support for the current protocol version and configuration
  * Probably a method or set of methods on `Context`
  * E.g. `ctx.supports(Feature::IncrementEpoch)`
* Add `#[inline]` appropriately throughout the crate
