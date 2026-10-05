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
  * `FUSE_OVER_IO_URING` (bit 41) is the flag that opts into it, and has no
    `KernelCaps`/`FsCaps` counterpart either
* Add a mocking system that simulates a FUSE device so users can write proper unit tests for `Fs` impls
* Add `#[inline]` appropriately throughout the crate
* Update public documentation to not expand on implementation details that are irrelevant to the user
  * Meaning, public doc comments shouldn't explain why something is implemented in a particular way,
    it should stick to explaining behaviors that impact the user. This can sometimes include
    implementation details, but only when those details effect how the user should use the item.
* Document the test requirements in `README.md`
  * The `fusermount` tests mount for real, so they need `fusermount` or `fusermount3`
    installed (Ubuntu's `fuse3` package carries both) and a usable `/dev/fuse`
* Test the `MountOpt::AllowRoot` refusal end to end
  * `src/access.rs` decides the policy and `handle_req` applies it, both unit tested,
    but nothing exercises a real request from a foreign uid actually being refused
  * Wants either the mocking system above, or a namespace with two uids mapped:
    `unshare --map-user=<n>` drops capabilities on exec, so it needs `newuidmap`
    or a root-mapped parent that forks a child which drops to the other uid
* Check `RawHeader`'s layout against a live kernel
  * `the_header_matches_fuse_in_header` pins the offsets against `struct fuse_in_header`,
    but nothing confirms a real kernel agrees. INIT cannot do it: the kernel sends it with
    `nocreds`, so `uid`/`gid`/`pid` are zero before kernel commit `794e811d1443` and only
    `pid` is filled after it
  * Wants a request that carries credentials, which means completing the INIT handshake
    first and then provoking a real operation -- the mocking system above would do it
* Close the remaining INIT flag gaps. Each of these is a wire bit in `init_flags.rs`
  with no `KernelCaps` or `FsCaps` counterpart, so the kernel's offer is invisible and
  `ReplyInitFlags::negotiate` can never set it
  * `FUSE_SUBMOUNTS` (bit 27) is the one already half-exposed: `AttrsFlags::SUBMOUNT` and
    `Attrs::is_submount_root` let a filesystem mark a submount root today, with no way to
    learn whether the kernel will act on it. It is silently ignored when it won't
  * `FUSE_REQUEST_TIMEOUT` (bit 42) is nearly wired: `negotiate` already takes a
    `request_timeout` argument and `request_timeout` is in the `fuse_init_out` reply
    struct, but the only caller passes `false` and no `Config` setter reaches it
  * `FUSE_SECURITY_CTX` (bit 32) and `FUSE_CREATE_SUPP_GROUP` (bit 34) both append data to
    `create`/`mkdir`/`symlink`/`mknod` as request extensions, sized by the header's
    `total_extlen` -- which `RawHeader` discards as `_unused`. One mechanism, so one job
  * `FUSE_HAS_RESEND` (bit 39) lets the kernel resend a pending request, marked with
    `FUSE_UNIQUE_RESEND` (bit 63 of `unique`). Nothing masks that bit, which stays safe
    only while the flag is unreachable
  * `FUSE_ABORT_ERROR` (bit 21) makes a read after an administrative abort return
    `ECONNABORTED` instead of `ENODEV`, letting a server tell an abort from the filesystem
    going away. Unrelated to `ConnCaps::ABORT`, which is about whether the connection can
    be aborted at all
  * `FUSE_SPLICE_READ`/`WRITE`/`MOVE` (bits 7-9) advertise that the device supports
    `splice(2)`, for a server wanting zero-copy device I/O. A performance option rather
    than missing behaviour
  * Deliberately not capabilities, so this audit need not be repeated: `FILE_OPS` (bit 2,
    "not yet supported" in the kernel's own header), `BIG_WRITES`, `MAX_PAGES`, `INIT_EXT`
    and `INIT_RESERVED` (plumbing, or always set by `negotiate`), and `MAP_ALIGNMENT` with
    `HAS_INODE_DAX` (DAX, out of scope)
* Implement `FUSE_ALLOW_IDMAP` (protocol 7.41), which lets the filesystem be mounted
  with a per-mount uid/gid translation (`mount_setattr(2)` with `MOUNT_ATTR_IDMAP`)
  * The wire bit is already in `KernelInitFlags` and `ReplyInitFlags` (`init_flags.rs`),
    but there is no `KernelCaps`, `FsCaps` or `Feature` counterpart, so the kernel's offer
    is invisible and `ReplyInitFlags::negotiate` can never set it. Parsed, unreachable
  * Three constraints come with it:
    * The kernel fails the handshake outright unless the mount has `default_permissions`
      (`ok = false` in `fs/fuse/inode.c`), rather than ignoring the flag
    * uid/gid are then sent only on inode-creation requests (`MKNOD`, `SYMLINK`, `MKDIR`,
      `TMPFILE`, `CREATE`, and `RENAME2` with `RENAME_WHITEOUT`), translated through the
      mount's idmap. Every other request carries `FUSE_INVALID_UIDGID` (`fs/fuse/req.c`),
      which is why `default_permissions` is required: the server can no longer decide
    * So it cannot coexist with `MountOpt::AllowRoot`. `src/access.rs` would refuse nearly
      everything, an invalid uid being neither the owner's nor root's, and the mount would
      look mysteriously broken rather than misconfigured. The two have to be rejected
      together at mount or handshake time
  * The `Fs` API hands every request a uid and gid, so it would need to express their
    absence rather than report `-1`
* Implement `DirectMount` for the BSDs (`src/mount/direct/bsd.rs`)
  * `tests/direct.rs` already covers it; the FreeBSD CI step is commented out in
    `.github/workflows/ci.yml` and wants a `getmntinfo(3)` counterpart to
    `the_mount_table_describes_the_mount`
