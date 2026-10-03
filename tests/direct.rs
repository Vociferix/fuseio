//! Mounting for real through [`DirectMount`], which needs the privilege to
//! mount in the current namespace.
//!
//! Arranging that privilege is the caller's job rather than this file's: the
//! platforms share no mechanism for it. Linux wants a user namespace or root;
//! FreeBSD wants root, or `vfs.usermount=1` together with a mountpoint this
//! user owns. Putting either here would fork the test on the thing the test is
//! least about, so these are `#[ignore]`d and whoever runs them brings the
//! privilege with them:
//!
//! ```text
//! # Linux, without becoming root:
//! unshare --user --map-root-user --mount -- cargo test --test direct -- --ignored
//!
//! # FreeBSD, already root:
//! cargo test --test direct -- --ignored
//! ```
//!
//! Nothing may touch a mountpoint while one of these tests holds it. No server
//! is answering `FUSE_INIT`, so any access -- including a `stat` -- blocks
//! until the unmount. That is why the mount is looked up in the kernel's mount
//! table rather than by asking the filesystem anything.

// macFUSE mounts through its own helper, so there is no `DirectMount` there and
// nothing in here to compile.
#![cfg(not(target_os = "macos"))]

use fuseio::MountOpt;
use fuseio::conn::{Conn, SharedConnection};
use fuseio::mount::{DirectMount, Mount, Unmount};

/// `FUSE_INIT`, which the kernel queues when it mounts. Reading it back proves
/// the descriptor passed in `fd=` really became this filesystem's device.
const FUSE_INIT: u32 = 26;

/// Comfortably over `FUSE_MIN_READ_BUFFER`, below which the device refuses a
/// read outright.
const BUFFER: usize = 128 * 1024;

/// A mountpoint of this test's own, since these run in parallel.
fn mountpoint(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fuseio-{name}-{}", std::process::id()));

    std::fs::create_dir_all(&dir).expect("mountpoint");

    dir
}

/// The opcode of a request, from the `opcode` field of its `fuse_in_header`.
fn opcode(request: &[u8]) -> u32 {
    u32::from_ne_bytes(request[4..8].try_into().expect("a header"))
}

#[compio::test]
#[ignore = "mounts for real, which needs privilege"]
async fn the_device_arrives_and_the_mount_goes_away() {
    let dir = mountpoint("direct");
    let mounter = DirectMount::new();

    let outcome = async {
        let (conn, pending) = mounter.mount(&dir, &[MountOpt::Rw], 1).await?;
        let unmounter = pending.await?;

        let compio::buf::BufResult(res, request) = conn.recv_request(vec![0u8; BUFFER]).await;
        let first = res.map(|(len, ())| (len, opcode(&request)));

        // Unmounted whatever the read did, so a failure cannot leave the mount
        // behind for the rest of the suite to trip over.
        unmounter
            .unmount(Conn::Shared(conn), &dir, &[])
            .await
            .and(first)
    }
    .await;

    let _ = std::fs::remove_dir(&dir);

    let (len, opcode) = outcome.expect("mount, read and unmount");

    assert_eq!(opcode, FUSE_INIT);
    assert!(len >= 56, "an INIT request is a header and a body: {len}");
}

/// A clone shares the connection, whether it got there through
/// `FUSE_DEV_IOC_CLONE` with a queue of its own or through `dup`, so the
/// pending `FUSE_INIT` is readable from it either way.
#[compio::test]
#[ignore = "mounts for real, which needs privilege"]
async fn a_clone_reaches_the_same_connection() {
    let dir = mountpoint("direct-clone");
    let mounter = DirectMount::new();

    let outcome = async {
        let (conn, pending) = mounter.mount(&dir, &[MountOpt::Rw], 1).await?;
        let unmounter = pending.await?;

        // Kept apart from the unmount below so that a failure in here still
        // takes the mount down before it is reported.
        let first = async {
            let clone = conn.try_clone().await?;
            let compio::buf::BufResult(res, request) = clone.recv_request(vec![0u8; BUFFER]).await;

            res.map(|(_, ())| opcode(&request))
        }
        .await;

        unmounter
            .unmount(Conn::Shared(conn), &dir, &[])
            .await
            .and(first)
    }
    .await;

    let _ = std::fs::remove_dir(&dir);

    assert_eq!(outcome.expect("mount, clone, read and unmount"), FUSE_INIT);
}

/// What the mount looks like from outside is Linux's own rendering, so this is
/// where the test stops being portable.
// TODO: Worth a FreeBSD counterpart reading `getmntinfo(3)` once
//       `mount/direct/bsd.rs` exists.
#[cfg(target_os = "linux")]
#[compio::test]
#[ignore = "mounts for real, which needs privilege"]
async fn the_mount_table_describes_the_mount() {
    let dir = mountpoint("direct-table");
    let needle = dir.display().to_string();
    let mounter = DirectMount::new();

    let options = [
        MountOpt::Rw,
        MountOpt::FsName("demofs".into()),
        MountOpt::SubType("demo".into()),
        MountOpt::MaxRead(65536),
    ];

    let outcome = async {
        let (conn, pending) = mounter.mount(&dir, &options, 1).await?;
        let unmounter = pending.await?;

        // Read while mounted, asserted after unmounting.
        let line = std::fs::read_to_string("/proc/self/mounts")
            .unwrap_or_default()
            .lines()
            .find(|line| line.contains(&needle))
            .map(str::to_owned);

        unmounter
            .unmount(Conn::Shared(conn), &dir, &[])
            .await
            .map(|()| line)
    }
    .await;

    let after = std::fs::read_to_string("/proc/self/mounts").unwrap_or_default();
    let _ = std::fs::remove_dir(&dir);

    let line = outcome
        .expect("mount and unmount")
        .expect("not in /proc/self/mounts while mounted");

    let mut fields = line.split_whitespace();

    // `fsname` goes in the source, which is why it needs no escaping, and the
    // subtype goes in the type.
    assert_eq!(fields.next(), Some("demofs"), "{line}");
    assert_eq!(fields.next(), Some(needle.as_str()), "{line}");
    assert_eq!(fields.next(), Some("fuse.demo"), "{line}");

    let rendered = fields.next().unwrap_or_default();

    for expected in ["rw", "nosuid", "nodev", "max_read=65536"] {
        assert!(rendered.contains(expected), "{expected} missing: {line}");
    }

    assert!(!after.contains(&needle), "still mounted afterwards");
}
