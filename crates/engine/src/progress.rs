use std::sync::atomic::{AtomicUsize, Ordering};

/// Counts processed files and notifies an optional observer. The observer is
/// called from worker threads; keep it cheap.
#[derive(Default)]
pub struct Progress {
    total: AtomicUsize,
    done: AtomicUsize,
    observer: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
}

impl Progress {
    pub fn new(observer: impl Fn(usize, usize) + Send + Sync + 'static) -> Self {
        Self { observer: Some(Box::new(observer)), ..Self::default() }
    }

    pub(crate) fn start(&self, total: usize) {
        self.total.store(total, Ordering::Relaxed);
        self.notify(0);
    }

    pub(crate) fn tick(&self) {
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;

        self.notify(done);
    }

    pub(crate) fn finish(&self) {
        self.notify(usize::MAX);
    }

    fn notify(&self, done: usize) {
        if let Some(observer) = &self.observer {
            observer(done, self.total.load(Ordering::Relaxed));
        }
    }
}
