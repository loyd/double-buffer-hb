//! A single-writer, single-collector double buffer.
//!
//! Uses [`membarrier::light`] on the writer and [`membarrier::heavy`] on the
//! collector, shifting the memory barrier cost away from the write path.
//! The crate provides platform-specific barriers and a `SeqCst` fence fallback.
//!
//! ```
//! use double_buffer_hb::DoubleBuffer;
//!
//! let mut buffer = DoubleBuffer::<Vec<u64>>::default();
//! let (mut writer, mut collector) = buffer.split();
//! writer.write(|values| values.push(42));
//! collector.advance();
//! assert_eq!(collector.try_collect(Vec::new()), Ok(vec![42]));
//! ```

use std::cell::{Cell, UnsafeCell};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};

const IDLE: u64 = u64::MAX;

/// Two reusable buffers shared by one writer and one collector.
///
/// The writer modifies the current epoch's buffer. The collector advances the
/// epoch, waits for the writer to leave the previous buffer, and replaces it.
/// Use [`Self::split`] to borrow the two roles, including across scoped threads.
pub struct DoubleBuffer<T> {
    shared: Shared<T>,
    pending: Option<u64>,
}

struct Shared<T> {
    writing: AtomicU64,
    epoch: AtomicU64,
    buffers: [UnsafeCell<T>; 2],
}

// SAFETY: The split handles enforce one user per role. The epoch handshake
// gives exclusive access to each buffer and transfers it with release/acquire
// synchronization. Values cross threads, but are never concurrently borrowed.
unsafe impl<T: Send> Sync for Shared<T> {}

impl<T> DoubleBuffer<T> {
    /// Creates a double buffer from two initial values.
    pub fn new(buffers: [T; 2]) -> Self {
        Self {
            shared: Shared {
                writing: AtomicU64::new(IDLE),
                epoch: AtomicU64::new(0),
                buffers: buffers.map(UnsafeCell::new),
            },
            pending: None,
        }
    }

    /// Borrows the unique writer and collector.
    ///
    /// Dropping a collector with a pending epoch leaves that epoch's data in
    /// place. A subsequent split resumes collecting it before advancing again.
    pub fn split(&mut self) -> (Writer<'_, T>, Collector<'_, T>) {
        (
            Writer {
                shared: &self.shared,
                not_sync: PhantomData,
            },
            Collector {
                shared: &self.shared,
                pending: &mut self.pending,
                not_sync: PhantomData,
            },
        )
    }

    /// Returns both buffers in index order.
    pub fn into_inner(self) -> [T; 2] {
        self.shared.buffers.map(UnsafeCell::into_inner)
    }
}

impl<T: Default> Default for DoubleBuffer<T> {
    fn default() -> Self {
        Self::new([T::default(), T::default()])
    }
}

/// The unique writer, movable between threads when `T: Send`, but not `Sync`.
pub struct Writer<'a, T> {
    shared: &'a Shared<T>,
    not_sync: PhantomData<Cell<()>>,
}

impl<T> Writer<'_, T> {
    /// Mutates the active buffer and returns the closure's result.
    ///
    /// References into the buffer cannot escape the closure. If it panics,
    /// the writer still announces that it is idle; partial changes remain.
    pub fn write<R>(&mut self, write: impl FnOnce(&mut T) -> R) -> R {
        let mut epoch = self.shared.epoch.load(Ordering::Relaxed);
        loop {
            self.shared.writing.store(epoch, Ordering::Release);
            // Paired with the collector's heavy barrier: both sides cannot
            // miss the other's announcement. The crate selects a compiler
            // fence when process-wide barriers are supported, or a full fence.
            membarrier::light();
            let current = self.shared.epoch.load(Ordering::Acquire);
            if current == epoch {
                break;
            }
            epoch = current;
        }

        struct IdleOnDrop<'a>(&'a AtomicU64);
        impl Drop for IdleOnDrop<'_> {
            fn drop(&mut self) {
                self.0.store(IDLE, Ordering::Release);
            }
        }
        let _idle = IdleOnDrop(&self.shared.writing);
        // SAFETY: The announcement and validation reserve this buffer until
        // _idle drops. Only one Writer exists, and write requires &mut self.
        write(unsafe { &mut *self.shared.buffers[(epoch & 1) as usize].get() })
    }
}

/// The unique collector, movable between threads when `T: Send`, but not `Sync`.
pub struct Collector<'a, T> {
    shared: &'a Shared<T>,
    pending: &'a mut Option<u64>,
    not_sync: PhantomData<Cell<()>>,
}

impl<T> Collector<'_, T> {
    /// Switches the writer to the other buffer and marks the old one pending.
    /// Returns `false` if a previous collection is still pending.
    pub fn advance(&mut self) -> bool {
        if self.pending.is_some() {
            return false;
        }
        let epoch = self.shared.epoch.load(Ordering::Relaxed);
        // Reserve IDLE and wrap over an even number of epochs, preserving the
        // alternation of buffer indices even at rollover.
        let next = if epoch == IDLE - 2 { 0 } else { epoch + 1 };
        self.shared.epoch.store(next, Ordering::Release);
        membarrier::heavy();
        *self.pending = Some(epoch);
        true
    }

    /// Takes the pending buffer, leaving `replacement` in its place.
    ///
    /// Returns `Err(replacement)` if no epoch is pending or the writer is
    /// still using it. The caller can check other shards before retrying.
    pub fn try_collect(&mut self, replacement: T) -> Result<T, T> {
        let Some(epoch) = *self.pending else {
            return Err(replacement);
        };
        if self.shared.writing.load(Ordering::Acquire) == epoch {
            return Err(replacement);
        }
        // SAFETY: The light/heavy barrier pair ensures a writer observes the new
        // epoch or is observed here. An acquire load of IDLE/a newer epoch
        // synchronizes with the writer leaving the old buffer. This role is
        // unique, and we cannot advance again until replacement is complete.
        let old = unsafe {
            std::mem::replace(
                &mut *self.shared.buffers[(epoch & 1) as usize].get(),
                replacement,
            )
        };
        *self.pending = None;
        Ok(old)
    }

    /// Advances (unless already pending) and waits to collect a buffer.
    ///
    /// Yields while the writer is busy. This can wait indefinitely if the
    /// writer's closure does not return.
    pub fn collect(&mut self, mut replacement: T) -> T {
        self.advance();
        loop {
            match self.try_collect(replacement) {
                Ok(buffer) => return buffer,
                Err(value) => replacement = value,
            }
            std::thread::yield_now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::mpsc;
    use std::thread;

    #[test]
    fn alternates_and_reuses_replacements() {
        let mut buffer = DoubleBuffer::new([vec![1], vec![2]]);
        let (mut writer, mut collector) = buffer.split();
        assert_eq!(collector.try_collect(vec![9]), Err(vec![9]));
        writer.write(|values| values.push(3));
        assert!(collector.advance());
        assert!(!collector.advance());
        writer.write(|values| values.push(4));
        assert_eq!(collector.try_collect(vec![5]), Ok(vec![1, 3]));
        assert_eq!(collector.collect(vec![6]), vec![2, 4]);
        writer.write(|values| values.push(7));
        assert_eq!(collector.collect(vec![]), vec![5, 7]);
        assert_eq!(buffer.into_inner(), [vec![], vec![6]]);
    }

    #[test]
    fn pending_collection_survives_resplitting() {
        let mut buffer = DoubleBuffer::new([10, 20]);
        {
            let (_, mut collector) = buffer.split();
            assert!(collector.advance());
        }
        let (mut writer, mut collector) = buffer.split();
        assert!(!collector.advance());
        writer.write(|value| *value += 1);
        assert_eq!(collector.collect(0), 10);
        assert_eq!(collector.collect(0), 21);
    }

    #[test]
    fn waits_for_active_writer() {
        let mut buffer = DoubleBuffer::<Vec<usize>>::default();
        let (mut writer, mut collector) = buffer.split();
        let (started_tx, started_rx) = mpsc::channel();
        let (finish_tx, finish_rx) = mpsc::channel();
        thread::scope(|scope| {
            let writer_thread = scope.spawn(move || {
                writer.write(|values| {
                    values.push(1);
                    started_tx.send(()).unwrap();
                    finish_rx.recv().unwrap();
                    values.push(2);
                });
            });
            started_rx.recv().unwrap();
            assert!(collector.advance());
            assert_eq!(collector.try_collect(vec![3]), Err(vec![3]));
            finish_tx.send(()).unwrap();
            writer_thread.join().unwrap();
            assert_eq!(collector.try_collect(vec![3]), Ok(vec![1, 2]));
        });
    }

    #[test]
    fn panic_releases_writer() {
        let mut buffer = DoubleBuffer::<Vec<usize>>::default();
        let (mut writer, mut collector) = buffer.split();
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                writer.write(|values| {
                    values.push(1);
                    panic!("interrupted write");
                });
            }))
            .is_err()
        );
        assert!(collector.advance());
        assert_eq!(collector.try_collect(vec![]), Ok(vec![1]));
        writer.write(|values| values.push(2));
        assert_eq!(collector.collect(vec![]), vec![2]);
    }

    #[test]
    fn rollover_preserves_buffer_alternation() {
        let mut buffer = DoubleBuffer::new([0, 0]);
        *buffer.shared.epoch.get_mut() = IDLE - 2;
        let (mut writer, mut collector) = buffer.split();
        writer.write(|value| *value = 1);
        assert!(collector.advance());
        writer.write(|value| *value = 2);
        assert_eq!(collector.try_collect(0), Ok(1));
        assert_eq!(collector.collect(0), 2);
    }

    #[test]
    fn concurrent_collection_preserves_every_write() {
        const COUNT: usize = 100_000;
        let mut buffer = DoubleBuffer::<Vec<usize>>::default();
        let (mut writer, mut collector) = buffer.split();
        let done = std::sync::atomic::AtomicBool::new(false);
        thread::scope(|scope| {
            scope.spawn(|| {
                for value in 0..COUNT {
                    writer.write(|values| values.push(value));
                }
                done.store(true, Ordering::Release);
            });
            let mut collected = Vec::new();
            while !done.load(Ordering::Acquire) {
                collected.extend(collector.collect(Vec::new()));
            }
            // Drain both buffers, including any final write after an advance.
            collected.extend(collector.collect(Vec::new()));
            collected.extend(collector.collect(Vec::new()));
            assert_eq!(collected, (0..COUNT).collect::<Vec<_>>());
        });
    }
}
