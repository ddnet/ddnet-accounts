use std::{
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
};

use ddnet_account_client::errors::FsLikeError;

#[derive(Debug, Clone)]
pub struct Fs {
    pub secure_path: PathBuf,
}

impl Fs {
    async fn create_dirs_impl(path: impl AsRef<Path>) -> anyhow::Result<(), FsLikeError> {
        Ok(tokio::fs::create_dir_all(path).await?)
    }

    pub async fn new(secure_path: PathBuf) -> anyhow::Result<Self, FsLikeError> {
        Self::create_dirs_impl(&secure_path).await?;
        Ok(Self { secure_path })
    }

    pub async fn delete(&self) -> anyhow::Result<(), FsLikeError> {
        tokio::fs::remove_dir(&self.secure_path).await?;
        Ok(())
    }

    pub async fn create_dirs(&self, path: &Path) -> anyhow::Result<(), FsLikeError> {
        Self::create_dirs_impl(self.secure_path.join(path)).await
    }

    pub async fn write(
        &self,
        path: &Path,
        name: &OsStr,
        file: Vec<u8>,
    ) -> anyhow::Result<(), FsLikeError> {
        let path = self.secure_path.join(path);
        let path_thread = path.clone();
        let tmp_file = tokio::task::spawn_blocking(move || {
            let mut tmp_file = tempfile::NamedTempFile::new_in(&path_thread)?;
            tmp_file.write_all(&file)?;
            tmp_file.flush()?;
            Ok::<_, std::io::Error>(tmp_file)
        })
        .await
        .map_err(|err| FsLikeError::Other(err.into()))??;
        let (_, tmp_path) = tmp_file.keep().map_err(|err| FsLikeError::Fs(err.error))?;
        tokio::fs::rename(tmp_path, path.join(name)).await?;
        Ok(())
    }

    pub async fn read(&self, path: &Path) -> anyhow::Result<Vec<u8>, FsLikeError> {
        Ok(tokio::fs::read(self.secure_path.join(path)).await?)
    }

    pub async fn remove(&self, path: &Path) -> anyhow::Result<(), FsLikeError> {
        Ok(tokio::fs::remove_file(self.secure_path.join(path)).await?)
    }
}
