use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use ignore::{WalkBuilder, WalkState};

use crate::filter::Filter;
use crate::{DiscoverError, SourceFile};

/// Every file under `root` that `.gitignore` and `.ignore` files do not
/// exclude, walked in parallel. Used outside git.
pub(crate) fn all(root: &Path, filter: &Arc<Filter>) -> Result<Vec<SourceFile>, DiscoverError> {
    let found = Mutex::new(Vec::new());
    let failure = Mutex::new(None);
    let prune = Arc::clone(filter);
    let base = root.to_path_buf();

    WalkBuilder::new(root)
        .hidden(false)
        .parents(false)
        .require_git(false)
        .follow_links(false)
        .filter_entry(move |entry| {
            entry.depth() == 0 || !entry.file_type().is_some_and(|kind| kind.is_dir()) || relative(&base, entry.path()).is_some_and(|rel| prune.enters(rel))
        })
        .build_parallel()
        .run(|| {
            let mut sink = Sink { files: Vec::new(), found: &found };
            let failure = &failure;

            Box::new(move |result| {
                let entry = match result {
                    Ok(entry) => entry,
                    Err(err) if err.io_error().is_some_and(|err| err.kind() == io::ErrorKind::NotFound) => return WalkState::Continue,
                    Err(err) => {
                        failure.lock().unwrap_or_else(PoisonError::into_inner).get_or_insert(err);

                        return WalkState::Quit;
                    }
                };

                if entry.file_type().is_some_and(|kind| kind.is_file())
                    && let Some(file) = relative(root, entry.path()).and_then(|rel| filter.source_file(rel))
                {
                    sink.files.push(file);
                }

                WalkState::Continue
            })
        });

    if let Some(err) = failure.into_inner().unwrap_or_else(PoisonError::into_inner) {
        return Err(DiscoverError::Walk { path: root.to_path_buf(), message: err.to_string() });
    }

    Ok(found.into_inner().unwrap_or_else(PoisonError::into_inner))
}

fn relative<'a>(root: &Path, path: &'a Path) -> Option<&'a str> {
    path.strip_prefix(root).ok()?.to_str()
}

/// One walker thread's finds, handed over when the thread's visitor is dropped.
struct Sink<'a> {
    files: Vec<SourceFile>,
    found: &'a Mutex<Vec<SourceFile>>,
}

impl Drop for Sink<'_> {
    fn drop(&mut self) {
        self.found.lock().unwrap_or_else(PoisonError::into_inner).append(&mut self.files);
    }
}
