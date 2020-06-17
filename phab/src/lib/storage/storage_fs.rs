use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use failure::Fail;
use slugify::slugify;

use crate::dto::Task;
use crate::dto::Watchlist;
use crate::storage::storage::PhabStorage;
use crate::types::ResultDynError;

type Table = HashMap<String, Watchlist>;
type FileDB = HashMap<String, Table>;

pub struct PhabStorageFilesystem {
  pub filepath: PathBuf,
  db_content: FileDB,
}

impl PhabStorageFilesystem {
  pub fn new(filepath: impl AsRef<Path>) -> ResultDynError<PhabStorageFilesystem> {
    let mut storage = PhabStorageFilesystem {
      db_content: HashMap::new(),
      filepath: PathBuf::from(filepath.as_ref()),
    };

    storage.reload()?;

    return Ok(storage);
  }
}

impl PhabStorageFilesystem {
  pub fn reload(&mut self) -> ResultDynError<()> {
    if !self.filepath.exists() {
      self
        .db_content
        .insert("watchlists".to_owned(), HashMap::new());
      self.persist()?;
    }

    let str = fs::read_to_string(&self.filepath)?;
    self.db_content = serde_json::from_str(&str)?;

    return Ok(());
  }

  pub fn persist(&self) -> ResultDynError<()> {
    if !self.filepath.exists() {
      let mut cloned_filepath = self.filepath.clone();
      cloned_filepath.pop();

      fs::create_dir_all(&cloned_filepath)?;
    }

    let content = serde_json::to_string(&self.db_content)?;
    fs::write(&self.filepath, content)?;

    return Ok(());
  }
}

#[derive(Debug, Fail)]
enum PhabStorageFilesystemError {
  #[fail(display = "PhabStorageFilesystemError err: {}", message)]
  QueryError { message: String },
}

impl PhabStorageFilesystemError {
  fn query_error(message: &str) -> failure::Error {
    return PhabStorageFilesystemError::QueryError {
      message: message.to_owned(),
    }
    .into();
  }
}

impl PhabStorage for PhabStorageFilesystem {
  fn add_to_watchlist(&mut self, watchlist_id: String, task: &Task) -> ResultDynError<()> {
    self
      .db_content
      .get_mut("watchlists")
      .unwrap()
      .get_mut(&watchlist_id)
      .unwrap()
      .tasks
      .push(task.clone());

    self.persist()?;

    return Ok(());
  }

  fn create_watchlist(&mut self, watchlist: &Watchlist) -> ResultDynError<Watchlist> {
    let watchlists = self.db_content.get_mut("watchlists").unwrap();
    let watchlist_id = slugify!(&watchlist.name);
    let mut watchlist = watchlist.clone();

    watchlist.id = Some(watchlist_id.clone());
    watchlists.insert(watchlist_id, watchlist.to_owned());

    self.persist()?;

    return Ok(watchlist);
  }

  fn get_watchlists(&mut self) -> ResultDynError<Vec<Watchlist>> {
    let watchlists: Vec<Watchlist> = self
      .db_content
      .get("watchlists")
      .unwrap()
      .values()
      .cloned()
      .collect();

    return Ok(watchlists);
  }

  fn get_watchlist_by_id(&mut self, watchlist_id: String) -> ResultDynError<Option<Watchlist>> {
    return Err(PhabStorageFilesystemError::query_error(
      "Not implemented yet",
    ));
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use std::fs;

  struct DirCleaner {
    dir: PathBuf,
  }

  impl Drop for DirCleaner {
    fn drop(&mut self) {
      if self.dir.exists() {
        fs::remove_dir_all(&self.dir).unwrap();
      }
    }
  }

  mod reload {
    use super::*;

    fn create_new(db_dir_path: PathBuf) -> ResultDynError<PhabStorageFilesystem> {
      let mut storage = PhabStorageFilesystem {
        db_content: HashMap::new(),
        filepath: PathBuf::from(format!(
          "{}/{}",
          db_dir_path.into_os_string().into_string().unwrap(),
          "yo.json"
        )),
      };

      storage.reload()?;

      return Ok(storage);
    }

    #[test]
    fn it_should_create_dir_and_load_data_for_first_time() -> ResultDynError<()> {
      let db_dir_path = PathBuf::from("/tmp/__phab_for_testing/db");
      let _dir_cleaner = DirCleaner {
        dir: db_dir_path.clone(),
      };
      let mut storage = test::reload::create_new(db_dir_path)?;

      let watchlists = storage.get_watchlists()?;

      assert_eq!(watchlists.len(), 0);

      return Ok(());
    }

    #[test]
    fn it_insert_data() -> ResultDynError<()> {
      let db_dir_path = PathBuf::from("/tmp/__phab_for_testing/db");
      let _dir_cleaner = DirCleaner {
        dir: db_dir_path.clone(),
      };
      let mut storage = test::reload::create_new(db_dir_path)?;
      let watchlist = Watchlist {
        id: None,
        name: String::from("hey ho test watchlist"),
        tasks: vec![],
      };

      storage.create_watchlist(&watchlist)?;
      let watchlists = storage.get_watchlists()?;

      assert_eq!(watchlists.len(), 1);
      assert_eq!(
        watchlists.get(0).unwrap().id.as_ref().unwrap(),
        "hey-ho-test-watchlist"
      );

      // Now we test reloading data, it should be the same.
      storage.reload()?;
      assert_eq!(watchlists.len(), 1);
      assert_eq!(
        watchlists.get(0).unwrap().id.as_ref().unwrap(),
        "hey-ho-test-watchlist"
      );

      return Ok(());
    }
  }
}
