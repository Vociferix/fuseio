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

13. **`Fs` defaults differ from libfuse where it matters** (`src/fs.rs:111`)
    - `open`/`opendir`: libfuse succeeds with fh 0 when they're unimplemented. The
      crate's ENOSYS default is fine on Linux and on FreeBSD with 7.23+, but on
      macOS it makes every `open(2)` fail.
    - `statfs`: libfuse replies with defaults (namelen 255, bsize 512). The crate's
      ENOSYS breaks `df` and may break mounting on macOS.

14. **Worker threads can block on poll-based backends** (`src/dev_fuse.rs:23`)
    - The device fd is never set `O_NONBLOCK`.
    - compio's poll driver (FreeBSD, macOS, and Linux without io_uring) waits for
      readability, then calls `read()`.
    - With `dup`'d fds sharing one queue (every non-Linux multi-worker setup), all
      workers wake up but only one gets the request. The rest block their thread
      in `read()`, stalling that worker's in-flight tasks.
    - Once the fd is set `O_NONBLOCK`, EAGAIN is already handled as "retry".
    - `TODO(e2e)`: whether kqueue works on `/dev/macfuseN` (and on the FSKit
      socket) is unverified.

15. ~~**macFUSE 5 `FUSE_MONITOR` (60)**~~ — **fixed.** The opcode is defined and
    answered with no reply on macOS, matching libfuse. Decoding its
    `fuse_monitor_in {flags, padding}` body and exposing it is still optional.

16. ~~**Replies to no-reply requests on decode failure**~~ — **fixed.** FORGET,
    BATCH_FORGET, macOS MONITOR and NOTIFY_REPLY no longer get an error reply,
    and a NOTIFY_REPLY that fails to decode now fails the waiting
    `Context::get_cache` instead of leaving it hung.

17. **`Read::decode` length check is inverted** (`src/proto/request/read.rs:62`)
    - The check is `>` where it should be `<`. Modern kernels send exactly
      40 bytes, so it passes today.
    - Before 7.9, `fuse_read_in` is 24 bytes, and the decoder reads past the
      request.

## Medium

18. **Default `max_write` is 16 MiB** (`src/server.rs:29`)
    - Every read buffer is `max_write + 4 KiB` and is held for the whole life of
      each in-flight request.
    - Linux caps writes at 1 MiB by default (`max_pages_limit`), and at 128 KiB
      without MAX_PAGES (macOS, FreeBSD).
    - libfuse uses 1 MiB + 4 KiB and clamps the reported `max_write` to what the
      kernel will send.

19. **Read replies aren't size-checked** (`src/server.rs:481`)
    - A reply larger than the requested size makes the kernel fail the read with
      EIO.
    - Checking and logging it would match the existing ioctl and getxattr handling.

20. **Compatibility with old protocol versions is broken in several places**
    (`src/handshake.rs:257`)
    - Every 7.x minor is accepted, but several old-version paths are wrong:
      - Before 7.9: `InodeAttrsCompat` lacks nlink/uid/gid/rdev, so the entry and
        attr reply sizes are wrong.
      - Before 7.12: CREATE uses `fuse_open_in` (name at offset 8).
      - Before 7.8: RELEASE has a 16-byte body.
      - Before 7.9: READ has a 24-byte body (see 17).
    - Every supported kernel speaks 7.12 or later (Linux ≥ 2.6.31, macOS 7.19,
      FreeBSD 12.1+ 7.28). Requiring 7.12 would let all of these paths be deleted.

21. **flock requests drop the lock owner** (`src/proto/request/getlk.rs:84`)
    - libfuse passes `owner` to flock handlers. It's what a later RELEASE with
      FLOCK_UNLOCK carries.
    - `FlockReq` has no `lock_owner()`.

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
