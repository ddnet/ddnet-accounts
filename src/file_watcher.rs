use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use notify::{
    event::RenameMode, recommended_watcher, Event, EventHandler, RecommendedWatcher, RecursiveMode,
    Watcher,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug)]
pub struct FileWatcher {
    rx: UnboundedReceiver<notify::Result<Event>>,
    _watcher: Option<RecommendedWatcher>,
    file: PathBuf,
}

impl FileWatcher {
    pub fn new(path: &Path, file: &Path) -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        struct TokioSender(UnboundedSender<notify::Result<Event>>);
        impl EventHandler for TokioSender {
            fn handle_event(&mut self, event: notify::Result<Event>) {
                let _ = self.0.send(event);
            }
        }
        let mut watcher = recommended_watcher(TokioSender(tx)).unwrap();

        if let Err(err) = watcher.watch(path, RecursiveMode::Recursive) {
            log::info!(target: "fs-watch", "could not watch directory/file: {err}");
        }

        let file = std::path::absolute(path.join(file)).unwrap();

        Self {
            rx,
            _watcher: Some(watcher),
            file,
        }
    }

    pub async fn wait_for_change(&mut self) -> anyhow::Result<()> {
        loop {
            match self.rx.recv().await {
                Some(Ok(ev)) => {
                    let handle_ev = matches!(
                        ev.kind,
                        notify::EventKind::Access(notify::event::AccessKind::Close(
                            notify::event::AccessMode::Write
                        )) | notify::EventKind::Modify(notify::event::ModifyKind::Name(
                            RenameMode::Both | RenameMode::To | RenameMode::From,
                        )) | notify::EventKind::Remove(
                            notify::event::RemoveKind::File | notify::event::RemoveKind::Folder
                        )
                    );
                    // check if the file exists
                    let file_exists = ev.paths.iter().any(|path| self.file.eq(path));
                    if file_exists && handle_ev {
                        // if the file exist, make sure the file is not modified for at least 1 second
                        let mut last_modified = None;

                        while let Ok(file) = tokio::fs::File::open(&self.file).await {
                            if let Some(modified) = file
                                .metadata()
                                .await
                                .ok()
                                .and_then(|metadata| metadata.modified().ok())
                            {
                                if let Some(file_last_modified) = last_modified {
                                    if modified == file_last_modified {
                                        break;
                                    } else {
                                        // else try again
                                        last_modified = Some(modified);
                                    }
                                } else {
                                    last_modified = Some(modified);
                                }
                            } else {
                                break;
                            }
                            drop(file);
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                        return Ok(());
                    }
                }
                Some(Err(err)) => {
                    log::error!(target: "file-watcher", "event err: {err}");
                }
                None => {
                    return Err(anyhow::anyhow!("Channel closed"));
                }
            }
        }
    }
}
