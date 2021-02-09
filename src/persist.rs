use std::fs;
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use acme_lib::{Result, Error};
use acme_lib::persist::{Persist, PersistKey, PersistKind};

#[derive(Clone)]
pub struct FilePersist<'a> {
    dir: PathBuf,
    domain: &'a str,
}

impl<'a> FilePersist<'a> {
    pub fn new<P: AsRef<Path>>(dir: P, domain: &'a str) -> Self {
        FilePersist {
            dir: dir.as_ref().to_path_buf(),
            domain: domain,
        }
    }

    fn file_path_of(&self, key: &PersistKey) -> PathBuf {
        let mut file_name = self.domain.replace('*', "STAR");
        match key.kind {
            PersistKind::Certificate => file_name.push_str(".crt"),
            PersistKind::PrivateKey => file_name.push_str(".key"),
            PersistKind::AccountPrivateKey => file_name.push_str("_account.key"),
        }
        self.dir.join(file_name)
    }
}

impl<'a> Persist for FilePersist<'a> {
    #[cfg(not(unix))]
    fn put(&self, key: &PersistKey, value: &[u8]) -> Result<()> {
        let f_name = self.file_path_of(&key);
        fs::write(f_name, value).map_err(Error::from)
    }

    #[cfg(unix)]
    fn put(&self, key: &PersistKey, value: &[u8]) -> Result<()> {
        let f_name = self.file_path_of(&key);
        match key.kind {
            PersistKind::AccountPrivateKey | PersistKind::PrivateKey =>
                fs::OpenOptions::new()
                 .mode(0o600)
                 .write(true)
                 .truncate(true)
                 .create(true)
                 .open(f_name)?
                 .write_all(value)
                 .map_err(Error::from),
            PersistKind::Certificate => fs::write(f_name, value).map_err(Error::from),
        }
    }

    fn get(&self, key: &PersistKey) -> Result<Option<Vec<u8>>> {
        let f_name = self.file_path_of(&key);
        let ret = if let Ok(mut file) = fs::File::open(f_name) {
            let mut v = vec![];
            file.read_to_end(&mut v)?;
            Some(v)
        } else {
            None
        };
        Ok(ret)
    }
}
