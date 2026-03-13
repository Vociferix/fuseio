use crate::mount::Mount;
use crate::{Builder, Filesystem};

use compio::runtime::JoinHandle;
use compio::runtime::event::EventHandle;

use std::io::Result;
use std::mem::ManuallyDrop;

pub struct MountHandle {
    task: ManuallyDrop<JoinHandle<Result<()>>>,
    unmount_notifier: Option<EventHandle>,
}

impl MountHandle {
    pub(crate) fn new(task: JoinHandle<Result<()>>, unmount_notifier: EventHandle) -> Self {
        Self {
            task: ManuallyDrop::new(task),
            unmount_notifier: Some(unmount_notifier),
        }
    }
}

impl MountHandle {
    pub fn is_mounted(&self) -> bool {
        !self.task.is_finished()
    }

    pub fn is_unmounting(&self) -> bool {
        self.unmount_notifier.is_none()
    }

    pub fn request_unmount(&mut self) {
        if let Some(notifier) = self.unmount_notifier.take() {
            notifier.notify();
        }
    }

    pub async fn unmount(mut self) -> Result<()> {
        self.request_unmount();
        self.wait().await
    }

    pub async fn wait(self) -> Result<()> {
        let mut this = ManuallyDrop::new(self);
        let _ = this.unmount_notifier.take();

        // SAFETY: Since `this` is wrapped in `ManuallyDrop`, `Drop::drop`
        //         won't be called. And since we don't access `this` again
        //         here, `this.task` is never accessed again.
        let task = unsafe { ManuallyDrop::take(&mut this.task) };

        match task.await {
            Ok(res) => res,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
}

impl std::fmt::Debug for MountHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MountHandle { .. }")
    }
}

impl Drop for MountHandle {
    fn drop(&mut self) {
        // SAFETY: Since this is in `Drop::drop`, we know that `self.task`
        //         will never be accessed again.
        let task = unsafe { ManuallyDrop::take(&mut self.task) };

        // If the handle is ignored or otherwise dropped without calling
        // `.wait()` or `.unmount()`, we want the filesystem loop to keep
        // running in the background. Don't stop the filesystem loop
        // unless explicitly asked to do so.
        task.detach();
    }
}
