//! The cancel handle.
//!
//! A flag plus a notification, rather than a flag alone. The flag is what the
//! stage machine polls between and inside stages; the notification is what lets
//! the translation stage abandon a request that is waiting on a provider
//! instead of waiting for it to time out first. A run cancelled while four
//! requests are in flight should stop in milliseconds, not in ninety seconds.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Notify;

use crate::khata::KhataTilqai;
use crate::taqaddum::MarhalaTilqai;

/// A handle a caller keeps and can cancel a run with, from any thread.
///
/// Cloning shares the same signal, so the interface can hold one clone, the run
/// another, and a shutdown hook a third.
#[derive(Clone, Default)]
pub struct MiqbadIlgha {
    /// Set once, never cleared: a cancelled run stays cancelled.
    alam: Arc<AtomicBool>,
    /// Woken on cancellation, so an awaiting stage does not poll.
    nabh: Arc<Notify>,
}

impl MiqbadIlgha {
    /// A handle that has not been cancelled.
    #[must_use]
    pub fn jadeed() -> Self {
        Self::default()
    }

    /// Cancels the run.
    ///
    /// Idempotent, and safe from any thread. `notify_waiters` rather than
    /// `notify_one`, because more than one stage may be awaiting the signal and
    /// waking only the first would leave the others parked.
    pub fn alghi(&self) {
        self.alam.store(true, Ordering::SeqCst);
        self.nabh.notify_waiters();
    }

    /// Whether the run has been cancelled.
    #[must_use]
    pub fn mulgha(&self) -> bool {
        self.alam.load(Ordering::SeqCst)
    }

    /// Resolves as soon as the run is cancelled, immediately if it already is.
    ///
    /// The registration is taken *before* the flag is re-read, which closes the
    /// window in which a cancel between the read and the await would be missed.
    ///
    /// `notified()` alone does not take it: it builds the future, and tokio
    /// enrols the waiter on the *first poll*. Without the `enable` below, the
    /// ordering this function is named for did not exist — a cancel landing
    /// between `mulgha()` and the first poll of the `select!` that awaits this
    /// was dropped, and the run carried on until the next progress tick noticed
    /// the flag. That is the difference between stopping in milliseconds and
    /// stopping at the next event, on the path that stops paid work.
    pub async fn intazir(&self) {
        let mustami = self.nabh.notified();
        tokio::pin!(mustami);
        let _ = mustami.as_mut().enable();
        if self.mulgha() {
            return;
        }
        mustami.await;
    }

    /// The cancellation failure for a stage, when the flag is set.
    ///
    /// Returned as an error so that a `?` at the head of every stage is the
    /// whole of the cancellation check; [`crate::tanfidh::arrib`] turns it back
    /// into a status before it returns.
    ///
    /// # Errors
    ///
    /// [`KhataTilqai::Mulgha`] when the run has been cancelled.
    pub fn tahaqquq(&self, marhala: MarhalaTilqai) -> Result<(), KhataTilqai> {
        if self.mulgha() {
            Err(KhataTilqai::Mulgha { marhala })
        } else {
            Ok(())
        }
    }
}

impl fmt::Debug for MiqbadIlgha {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MiqbadIlgha")
            .field("mulgha", &self.mulgha())
            .finish()
    }
}
