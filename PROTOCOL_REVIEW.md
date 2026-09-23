# Protocol correctness review

This review covers request decoding, reply encoding, notifications, the handshake,
server dispatch and the `Fs` defaults. It compares them against libfuse
(`~/devel/libfuse`) and macFUSE (`~/devel/macfuse`, branches `master` = macFUSE 4 /
libfuse 2.9 and `support/libfuse-3.18` = macFUSE 5). DAX and CUSE are out of scope.

Each item has a matching `TODO` comment at the location given. Line numbers are as
of this review.

## Critical: wrong behaviour on current kernels

1. ~~**Interrupted requests never get a reply**~~ — **addressed.**
   `InterruptMode` (Cancel / Abort / Ignore) on `Config`, plus the cancellation
   API on `Req` (`is_cancelled`, `cancelled`, `run_until_cancelled`). Abort mode
   now replies EINTR when the handler didn't finish. Token reuse is safe: the
   interrupt path holds a clone, so `try_reset` can't recycle a token that is
   still reachable. Three smaller follow-ups remain, listed as items 30–32.

2. **`SETXATTR` value is one byte short, and an empty value panics**
   (`src/proto/request/setxattr.rs:124`)
   - The value starts after the key's NUL, so it ends at `key_end + 1 + size`.
     The length check and `truncate` stop at `key_end + size`.
   - As a result, every value loses its last byte.
   - With `size == 0` (e.g. `setfattr -n user.x`), `value()` slices
     `[key_end+1..key_end]` and panics. This affects all platforms.

3. **macOS `SETXATTR` layout is wrong** (`src/proto/request/setxattr.rs:124`)
   - On macOS, `fuse_setxattr_in` is `{size, flags, position, padding}`
     (16 bytes).
   - SETXATTR_EXT is never negotiated on macOS (wire bit 29 is
     `DARWIN_CASE_INSENSITIVE` there), so the 8-byte layout is used. The key is
     therefore read from `position`, and usually decodes as an empty string.
   - `position` (the resource fork offset) should be exposed on `SetXattrReq`,
     like `GetXattrReq::offset`.

4. **Entry and delete notifications are always rejected**
   (`src/proto/notify/entry.rs:46`, `src/proto/notify/delete.rs:52`)
   - `namelen` is taken from the buffer that already includes the trailing NUL.
   - The kernel requires `size == sizeof(out) + namelen + 1` and returns EINVAL
     otherwise. libfuse sends `namelen = strlen(name)` followed by `namelen + 1`
     bytes.
   - As a result, `invalidate_entry`, `expire_entry` and `delete_inode` all fail.

5. **`STATX` reply layout is wrong** (`src/proto/response/statx.rs:12`)
   - `fuse_statx_out.spare` is `uint64_t[2]`, but the crate uses `[u32; 2]`.
   - Every field after it is therefore 8 bytes early, and the reply is 8 bytes
     short, so the kernel rejects it.
   - Separately, `fs_device_number` writes `rdev_*` instead of `dev_*`
     (`statx.rs:168`).

6. **`InodeAttrs` can't set permission bits or `rdev`**
   (`src/proto/response/attr.rs:146`)
   - `kind()` overwrites the whole mode, and nothing else writes `mode` or `rdev`.
   - As a result, every getattr, setattr, lookup, create, mknod and readdirplus
     reply reports mode `0000`, and device nodes have no device number.

7. **`COPY_FILE_RANGE` (47) reply is encoded as `COPY_FILE_RANGE_64`'s**
   (`src/server.rs:733`)
   - Opcode 47 replies with `fuse_write_out` (`u32 size` + padding). Only 53
     replies with `fuse_copy_file_range_out` (`u64`).
   - Both share the `u64` reply. On little-endian hosts the bytes match only when
     the count fits in `u32`.
   - libfuse clamps opcode 47's length to `0xfffff000`; the crate passes it
     unclamped. A copy of 4 GiB or more is therefore reported to the kernel as a
     truncated count.

8. **Out-of-range error values hang the caller** (`src/proto/response/error.rs:12`)
   - Linux validates `oh.error` (it must be in `-511..=0`) *before* looking up the
     request. An invalid value fails the write, and the request is never
     completed.
   - `Error` accepts any non-zero `i32`, including negatives and values ≥ 512.
   - libfuse maps bad values to ERANGE and logs them. The crate should do the same
     for anything outside `1..512`.

9. **`Created::attr_ttl` sets the entry TTL** (`src/proto/response/create.rs:42`)
   - It calls `entry_ttl` instead of `attrs_ttl`.

## High: incorrect semantics or platform behaviour

10. **`setattr` kill-suid/sgid bits** (`src/proto/request/setattr.rs:21`)
    - The protocol has one flag, FATTR_KILL_SUIDGID (bit 11), meaning "clear suid,
      and sgid if group-executable". Bit 12 is never sent, so `remove_sgid()` is
      always false.
    - The commented-out bits 9 and 13–17 are kernel-internal `ATTR_*` flags, not
      protocol bits. Bit 9 is FATTR_LOCKOWNER.

11. **macOS `setattr` ctime** (`src/proto/request/setattr.rs:288`)
    - macOS (protocol 7.19) never sends FATTR_CTIME (bit 10). It sends
      CHGTIME (bit 29) instead, so `ctime()` is always `None` on macOS.
    - macFUSE 5 maps FATTR_DARWIN_CTIME onto FATTR_CTIME. Doing the same would
      make `ctime()` portable and let `chgtime()` go.

12. **Pre-1970 timestamps** (`setattr.rs:288`, `attr.rs:146`, `statx.rs:12`,
    `xtimes.rs`)
    - The time fields are signed seconds sent as `u64`, with nanoseconds in
      `[0, 1e9)`.
    - Decoding them as unsigned makes `touch -d 1969-…` fail with EINVAL.
    - Encoding clamps pre-epoch times to 0.

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

15. **macFUSE 5 `FUSE_MONITOR` (60)** (`src/server.rs:226`)
    - libfuse answers it with *no reply* when the handler is unimplemented. The
      crate replies ENOSYS: an unexpected write that the kext rejects at best.
    - It should be decoded (`fuse_monitor_in {flags, padding}`) and ignored, or
      exposed.

16. **Replies to no-reply requests on decode failure** (`src/server.rs:226`)
    - FORGET, BATCH_FORGET and NOTIFY_REPLY decode errors still send a reply.
    - A NOTIFY_REPLY that fails to decode also leaves `Context::get_cache` waiting
      forever.
    - (`NotifyReply` for an unknown worker likewise replies EINVAL in
      `handle_req`.)

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

32. **Interrupts that arrive before their request is registered are lost**
    (`src/server.rs`)
    - A request is inserted into `open_reqs` only after it is decoded. An
      INTERRUPT that arrives first is broadcast, matches nothing, and is dropped.
    - libfuse keeps unmatched interrupts in a list and flags the request when it
      arrives (`check_interrupt`).
    - With Abort gone the effect is just a missed cancellation signal, so this is
      now minor.

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
