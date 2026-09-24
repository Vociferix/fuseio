# Protocol correctness review

This review covers request decoding, reply encoding, notifications, the handshake,
server dispatch and the `Fs` defaults. It compares them against libfuse
(`~/devel/libfuse`) and macFUSE (`~/devel/macfuse`, branches `master` = macFUSE 4 /
libfuse 2.9 and `support/libfuse-3.18` = macFUSE 5). DAX and CUSE are out of scope.

Each item has a matching `TODO` comment at the location given. Line numbers are as
of this review.

## Critical: wrong behaviour on current kernels

1. ~~**Interrupted requests never get a reply**~~ — **addressed.** Interrupts now
   signal the handler through the cancellation API on `Req` (`is_cancelled`,
   `cancelled`, `run_until_cancelled`), so the handler always gets to reply, or
   are ignored entirely (`ignore_interrupts`). Aborting was dropped as a footgun.
   Token reuse is safe: the interrupt path holds a clone, so `try_reset` can't
   recycle a token that is still reachable. Follow-ups are items 30–35.

2. ~~**`SETXATTR` value is one byte short, and an empty value panics**~~ —
   **fixed.** The value's end is computed from the key's NUL
   (`key_end + 1 + size`, with a `checked_add` against a malformed `size`), so
   values arrive whole and an empty one decodes to `&[]`. Covered by tests in
   `setxattr.rs`.

3. ~~**macOS `SETXATTR` layout is wrong**~~ — **fixed.** `Raw` now carries
   `position` and its padding on macOS (16 bytes, asserted), matching
   `getxattr.rs`, so the key is read at the right offset. The resource-fork
   offset is exposed as `SetXattrReq::offset()`, which is 0 off macOS, and the
   decoder tests build the macOS body under `cfg`.

4. ~~**Entry and delete notifications are always rejected**~~ — **fixed.**
   `namelen` now comes from `IoBufferWithNul::len_without_nul`, so it counts the
   name alone while the payload still carries the terminator, which is what the
   kernel's `size == sizeof(out) + namelen + 1` check expects. Wire-level tests
   in `notify/entry.rs` and `notify/delete.rs` pin the lengths, the payload, the
   notify code and the expire flag.

5. ~~**`STATX` reply layout is wrong**~~ — **fixed.** `spare` is `[u64; 2]`, so
   the fields line up and the reply is the 304 bytes the kernel expects
   (`fuse_out_header` + `fuse_statx_out`), asserted at compile time along with
   `StatX` (176) and `StatXTime` (16). `fs_device_number` now writes `dev_*`.
   `__spare2` stays in the encoder's `Out` rather than `StatX`, to keep the
   user-held struct small. Tests check the reply size, field offsets, mask
   accumulation and the two device numbers.

6. ~~**`InodeAttrs` can't set permission bits**~~ — **fixed.** `InodeAttrs::mode`
   sets the permission bits (including setuid/setgid/sticky) and `kind` sets only
   the `S_IFMT` bits, so the two compose in either order and neither can corrupt
   the other. Tests read the encoded `mode` field on both layouts. Device numbers
   moved to item 36.

7. ~~**`COPY_FILE_RANGE` (47) reply is encoded as `COPY_FILE_RANGE_64`'s**~~ —
   **fixed.** Opcode 47 now replies with `Write` (`fuse_write_out`) and 53 with
   `CopyFileRange` (`fuse_copy_file_range_out`). `CopyFileRange::decode` clamps
   the requested length to `0xfffff000` as libfuse does, `decode_64` passes it
   through, and the two `Body` variants share the type while the opcode picks the
   decoder. A filesystem that reports more than `u32::MAX` on opcode 47 gets a
   logged EIO rather than a truncated count.

8. ~~**Out-of-range error values hang the caller**~~ — **fixed.** Replies clamp
   the errno to `1..=MAX_ERRNO` (133) through `Error::wire_errno`, logging and
   substituting EIO otherwise, and `from_raw_os_error` rejects anything outside
   that range so most `Error`s are valid by construction. The handshake's error
   replies share the same path.
   - 133 rather than Linux's 511, because the kernels checked disagree and two of
     them reject *before* matching the reply to its request, which is what hangs
     the caller: Linux allows 1..=511 (`fs/fuse/dev.c`, `oh.error <= -512`);
     FreeBSD allows 1..=`ELAST` (97) and quietly substitutes EIO above that, but
     under the `linux_errnos` mount option only translates 1..=133
     (`LINUX_ELAST`) and fails the write otherwise; the old osxfuse kext checks
     nothing. 133 is the highest errno any of them defines, so nothing valid is
     refused.

9. ~~**`Created::attr_ttl` sets the entry TTL**~~ — **fixed.** It calls
   `attrs_ttl`, and is itself renamed `attrs_ttl` to match `Entry`. Tests check
   that the two timeouts land in their own fields in either order, and that the
   reply is `entry_out + open_out` long on both layouts.

## High: incorrect semantics or platform behaviour

10. ~~**`setattr` kill-suid/sgid bits**~~ — **fixed.** `remove_suid()` and
    `remove_sgid()` are replaced by `remove_suid_sgid()`, reading the single
    `FATTR_KILL_SUIDGID` (bit 11), matching `Write::remove_suid_sgid()`. The
    mislabelled bit comments are corrected.
    - Verified against all three kernels: bit 11 is the highest `FATTR_*` in
      Linux's `include/uapi/linux/fuse.h:381`, FreeBSD's `fuse_kernel.h:287` and
      libfuse's own copy, and nothing defines bit 12. libfuse's
      `FUSE_SET_ATTR_KILL_SUID`/`_SGID` (bits 11 and 12) mirror the Linux VFS's
      internal `ATTR_*` numbering, so its `_SGID` is dead even in libfuse, whose
      only use of it is a pass-through mask.
    - Who sends it: Linux only, with `HANDLE_KILLPRIV_V2`, on a non-directory
      chown and on a truncate without `CAP_FSETID` (`fs/fuse/dir.c:2234,2243`).
      FreeBSD never sets it; the macOS kext predates it.
    - Also fixed a typo while here: `WriteReq::remove_suid_guid` is now
      `remove_suid_sgid`.

11. ~~**macOS `setattr` ctime**~~ — **fixed.** On macOS the Darwin change time
    (bit 29) decodes into the portable `ctime` field and `ctime()` accepts either
    flag, as macFUSE 5's library does. `chgtime()` is gone, since it would return
    exactly the same value. `bkuptime()` and `crtime()` stay, having no portable
    equivalent.

12. ~~**Pre-1970 timestamps**~~ — **fixed.** A new `proto::time` module converts
    both directions, treating the seconds as the signed value they are and
    keeping the nanosecond remainder moving forward, as `timespec` does: half a
    second before the epoch is `-1` seconds plus 500 ms. `setattr` decodes
    through it, and `attr`, `statx` and `xtimes` encode through it, so a pre-1970
    time no longer fails with EINVAL or silently becomes 1970. `StatXTime.secs`
    is now `i64`, matching `fuse_sx_time`.
    - Its own tests round-trip the epoch, later times, whole and fractional
      pre-epoch times, the two's-complement bits, an out-of-range remainder and
      the extremes of `i64`. Decode and encode are also covered end to end
      through `SetAttr` and `InodeAttrs`.

13. ~~**`Fs` defaults differ from libfuse where it matters**~~ — **fixed.**
    `open` (and so `opendir`) hands out a zero file handle, `close` succeeds to
    match it, and `statfs` reports `FsAttrs::new()`'s placeholders. Each carries
    a doc comment saying why ENOSYS would be wrong there. Every other ENOSYS
    default is left alone: for those the kernels treat it as "not supported" and
    stop asking.
    - Not unit-tested: reaching a default needs a `Req`, which needs a live
      `Server` and device fd. The planned mock device is what would cover these.

14. ~~**Worker threads can block on poll-based backends**~~ — **fixed.**
    `DevFuse::bind` sets `O_NONBLOCK`, so the device is blocking while the
    handshake reads it synchronously and non-blocking once the runtime owns it.
    Each worker binds its own clone, which covers both the Linux clone ioctl (a
    separate open file) and the `dup` fallback (a shared one).
    - Confirmed compio doesn't do this itself: `AsyncFd::new` goes to
      `Attacher::new` and then the driver's `attach`, which is a no-op on the
      poll driver; its only `set_nonblocking` calls are for accepted sockets.
    - `/dev/fuse` implements `.poll` (`fs/fuse/dev.c:2415`), so io_uring arms
      poll on EAGAIN rather than failing, and compio's poll driver already maps
      EAGAIN back to "wait for readability".
    - `TODO(e2e)`: kqueue support for `/dev/macfuseN` and the FSKit socket is
      still unverified.

15. ~~**macFUSE 5 `FUSE_MONITOR` (60)**~~ — **fixed.** The opcode is defined and
    answered with no reply on macOS, matching libfuse. Decoding its
    `fuse_monitor_in {flags, padding}` body and exposing it is still optional.

16. ~~**Replies to no-reply requests on decode failure**~~ — **fixed.** FORGET,
    BATCH_FORGET, macOS MONITOR and NOTIFY_REPLY no longer get an error reply,
    and a NOTIFY_REPLY that fails to decode now fails the waiting
    `Context::get_cache` instead of leaving it hung.

17. ~~**`Read::decode` length check is inverted**~~ — **fixed.** The check now
    rejects a body that is too short rather than one that is too long, and a
    `RawCompat` covers the 24-byte pre-7.9 body (no lock owner, no open flags),
    as `Write` already did. Tests cover both layouts, the lock-owner flag, a
    short body and a longer one.

## Medium

18. ~~**Default `max_write` is 16 MiB**~~ — **fixed.** The default is 1 MiB,
    matching what Linux will actually send (`max_pages` defaults to 256) and what
    libfuse uses, so a worker's read buffer is 1 MiB + 4 KiB rather than 16 MiB.
    A const assertion now pins `BUF_HEADER_SIZE` to at least the request and
    write headers, which is what Linux requires before it will fill a buffer
    (`fs/fuse/dev.c:1569`, else the read fails with EINVAL).
    - I did **not** clamp `max_write` to 32 pages when MAX_PAGES isn't
      negotiated, which libfuse effectively does. That suits libfuse's fixed
      buffer but would hurt here: FreeBSD doesn't implement MAX_PAGES
      (`fuse_internal.c:1121`) and chunks writes by `max_write` alone
      (`fuse_io.c:357`), so clamping would cut its writes to 128 KiB. The macOS
      kext sizes I/O from its own `iosize` mount option instead. Linux bounds
      writes by `max_pages` as well as `max_write`, so an over-large value only
      ever cost us buffer space.

19. ~~**Read replies aren't size-checked**~~ — **fixed.** A read reply longer
    than the size requested is now logged and answered with EIO, rather than sent
    for the kernel to reject with an unexplained EIO, matching the ioctl and
    getxattr paths. Rejecting rather than truncating, since returning more than
    was asked for is a filesystem bug worth surfacing.
    - Not unit-tested, for the same reason as item 13: the handler needs a live
      `Server`. `read_dir` and `read_dir_plus` need no such check, as their
      buffers are built against the requested capacity.

20. ~~**Compatibility with old protocol versions is broken in several places**~~
    — **fixed.** Enumerated from the kernel's own changelog
    (`include/uapi/linux/fuse.h`) and libfuse's `proto_minor` checks, rather than
    only the cases first spotted:
    - **Before 7.9**, `fuse_attr` ends before `blksize`: `InodeAttrsCompat` now
      carries `nlink`, `uid`, `gid` and `rdev` (and the Darwin fields), so the
      entry and attr replies report `FUSE_COMPAT_ENTRY_OUT_SIZE` (120, or 136 on
      macOS) and `FUSE_COMPAT_ATTR_OUT_SIZE` (96/112). Const assertions pin all
      four, plus that the old layout is a prefix of the current one.
    - **Before 7.12**, CREATE sends a `fuse_open_in` carrying the mode in its
      second field and no umask, with the name 8 bytes earlier.
    - **Before 7.8**, RELEASE has no release flags and no lock owner.
    - **Before 7.7**, FLUSH has no lock owner.
    - **Before 7.9**, the lock bodies have no `lk_flags`, so no lock is a flock.
    - Already correct and re-checked: WRITE and READ (<7.9), MKNOD and MKDIR
      (<7.12), GETATTR (<7.9), STATFS (<7.4), negative entries (<7.4), and the
      INIT reply sizes (<7.5, <7.23). `fuse_setattr_in` needs no compat path: its
      `lock_owner` replaced a padding field at the same offset, and it is ignored
      here anyway.
    - Each path has decoder tests for both layouts; disabling all four compat
      branches fails exactly those tests, and shortening the compat attr layout
      fails the build.
    - Still only exercised against synthetic requests. A VM running an old kernel
      would be the way to test these for real.

21. ~~**flock requests drop the lock owner**~~ — **fixed.** A flock keeps its
    owner, which names the open file and is what a later RELEASE with
    `FLOCK_UNLOCK` carries, and `FlockReq` exposes `lock_owner()`.
    - The lock body's `pid` is now a distinct `Tgid` type, exposed as `tgid()` on
      the flock and POSIX lock requests and on the `PosixLock`/`Flock` replies.
      The kernels disagree with the request header here: Linux puts a *thread*
      id in the header (`fs/fuse/req.c:13`) and a *thread group* id in a lock
      (`fs/fuse/file.c:2533`), while FreeBSD and macOS put a process id in both.
      With one type for both, comparing them looked reasonable and silently
      failed for threaded callers on Linux; now it doesn't compile. `Req::pid()`
      documents the difference.
    - A kernel sends `pid` 0 when releasing a lock, which now reads as `None`.
      The wrappers previously called `unwrap_unchecked()` on that `Option`, which
      would have been unsound once unlocks produced `None`.

22. **`FsAttrs` default `frsize` is 0** (`src/proto/response/statfs.rs:38`)
    - FreeBSD's fusefs uses `frsize` as `f_bsize`, so the filesystem likely shows
      as zero-sized there. This is from memory; verify on FreeBSD.
    - Defaulting `frsize` to `bsize` is harmless everywhere.

23. **Attributes-only inode invalidation isn't possible**
    (`src/proto/notify/inval_inode.rs:17`)
    - A negative offset invalidates attributes only, and there's no way to
      express one, so every invalidation also drops cached data.
    - `range()` also overflows for an inclusive end of `u64::MAX`.

24. **Notification caveats** (`src/context.rs:110`)
    - `expire_entry` silently becomes a full invalidation on kernels without
      EXPIRE_ONLY (before 7.38). libfuse returns ENOSYS.
    - No notification checks the minimum protocol version it needs (inval 7.12,
      store/retrieve 7.15, delete 7.18).
    - Worth documenting: on Linux, inval_entry/delete lock the parent directory, so
      sending one from a handler that is operating on that directory deadlocks.

## Low

25. **macOS open-reply flags are missing** (`src/types/opened_flags.rs:12`)
    - `FOPEN_PURGE_ATTR` (1 << 30) and `FOPEN_PURGE_UBC` (1 << 31) aren't in
      `OpenedFlags`.

26. **`FileRange` edge cases** (`src/types/file_range.rs:42`)
    - `5..5` becomes `5..=4`, so `len()` underflows.
    - `0..0` becomes a one-byte range.
    - `len()` overflows for `0..=u64::MAX`.

27. **A 0-byte device read spins forever** (`src/server.rs:253`)
    - Real `/dev/fuse` never returns 0 bytes, but the planned socket-based mock
      does once its peer closes.

28. **Truncating casts in small replies**
    - `Write` and `XattrLen` cast `usize` to `u32` without checking.
    - `Write`'s decoder doesn't check that the payload is at least `size` bytes.

29. **Minor API inconsistencies**
    - `Create::mode()` keeps `S_IFREG`, while `MkNod::mode()` masks the type bits.
    - `Symlink::ino()` is the parent directory.
    - `Poll::decode` returns EINVAL for a short body where every other decoder
      returns EPROTO.

## Follow-ups from the interrupt work

30. ~~**Cancellation panics the waiting future**~~ — **fixed.** `poll` clears its
    key instead of removing it when `cancelled` is set, and `drop` uses
    `try_remove`, so a stale key is ignored. Verified with scratch tests: cancel
    then poll, cancel then drop unpolled, the `run_until_cancelled` race, two
    waiters where one is dropped first, and reuse after `try_reset`.
    - Stale keys can't collide with reused ones either: between `cancel` and
      `try_reset` nothing new is inserted (`poll` returns ready without
      registering), and `try_reset` only runs when no other token clone, and
      therefore no live waiter, exists.

31. ~~**`cancel()` holds the borrow across `Waker::wake`**~~ — **fixed.** The slab
    is taken out before waking, so no borrow is held across user code. Nothing
    can be lost when the slab is put back, because after `cancel` sets the flag
    `poll` returns ready without inserting.

32. ~~**Early interrupts are dropped**~~ — **fixed.** An interrupt for a request
    this worker isn't running is recorded in `EarlyInterrupts`, and the request
    cancels itself the moment it registers. The three-state `OpenReq` marker and
    the `ClearInterrupt` message were replaced by that set, which removes the
    ordering cases they existed for: stale ids are harmless because the kernel
    never reuses a `unique`.

33. ~~**`Blocked` markers leak**~~ — **fixed.** `EarlyInterrupts` holds at most
    `MAX_EARLY_INTERRUPTS` (64) ids per worker and drops the oldest when full, so
    an interrupt that loses its race with a reply can't accumulate. Dropping one
    only means it isn't honoured, which is what happened before any of them were
    recorded.

34. ~~**A repeated interrupt forgets the pending one**~~ — **fixed.** Recording
    an id that is already pending is a no-op, so a resent interrupt keeps it.

35. ~~**NOTIFY_REPLY for an unknown worker still gets a reply**~~ — **fixed.**
    Both paths now log and drop it.

## Consistency

36. **Device numbers should be exposed unpacked, and consistently**
    - `MakeNod`/`MakeNodeReq` expose `rdev()` as a packed `u32`, `InodeAttrs` has
      no setter at all, and `StatX` takes `major, minor` pairs, because
      `fuse_statx` stores them split.
    - The wire `rdev` is an encoded `dev_t` whose packing is platform-specific:
      Linux's 32-bit form is 12 bits of major and 20 of minor (split, with the
      low 8 minor bits first), while macOS uses 8 and 24. nix's `major`/`minor`
      operate on the host's 64-bit `dev_t`, which is a different layout again, so
      they can't be used directly on this field.
    - Plan: a `DeviceNumber` type with per-platform pack/unpack, exposed as
      `MakeNodeReq::device_number() -> Option<DeviceNumber>`,
      `InodeAttrs::device_number(DeviceNumber)` and the same for `StatX`, with
      `TODO(e2e)` notes on the packing until end-to-end tests can confirm it.

## Checked and correct

- **Request layouts:** every request struct matches the libfuse 7.45 header, and
  the macOS (7.19) variants of `fuse_rename_in`, `fuse_getxattr_in` (also used by
  listxattr) and `fuse_setattr_in` are right. SETVOLNAME, GETXTIMES and EXCHANGE
  use opcodes 61–63 on both macFUSE 4 and 5.
- **Reply layouts:** entry, attr (including the 104-byte macOS `fuse_attr`), open,
  create, statfs, lock, poll, lseek, bmap, write, xattr-size, xtimes and ioctl.
- **Notification layouts:** inval_inode, store, retrieve (with notify_unique
  routing by worker), poll wakeup and inc_epoch are right, apart from item 4.
- **Handshake:**
  - version negotiation and the reply sizes for minors below 5 and below 23;
  - `max_background`/`congestion_threshold` (7.13+) and `time_gran` (7.23+);
  - `max_pages` and `max_stack_depth`;
  - the minor it reports back to the kernel.
- **macFUSE 5 compatibility:** the kext side is still 7.19 with Darwin flags at
  bits 23–30. macFUSE 5 drops XTIMES (31) and EXCHANGE_DATA (28), which the
  crate's negotiation handles because it's driven by the kernel's offer.
- **Server behaviour:**
  - the rename and fallocate gating;
  - the getxattr/listxattr size-probe handling and ERANGE;
  - DESTROY;
  - FORGET and BATCH_FORGET getting no reply.
