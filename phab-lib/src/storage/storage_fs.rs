use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Error;
use slugify::slugify;
use thiserror::Error;

use crate::dto::Task;
use crate::dto::Watchlist;
use crate::storage::storage::PhabStorage;
use crate::types::ResultAnyError;

type Table = HashMap<String, Watchlist>;
type FileDB = HashMap<String, Table>;

pub struct PhabStorageFilesystem {
  pub filepath: PathBuf,
  db_content: FileDB,
}

impl PhabStorageFilesystem {
  pub fn new(filepath: impl AsRef<Path>) -> ResultAnyError<PhabStorageFilesystem> {
    let mut storage = PhabStorageFilesystem {
      db_content: HashMap::new(),
      filepath: PathBuf::from(filepath.as_ref()),
    };

    storage.reload()?;

    return Ok(storage);
  }
}

impl PhabStorageFilesystem {
  fn watchlist_table(&mut self) -> &mut Table {
    return self
      .db_content
      .entry("watchlists".to_owned())
      .or_insert(HashMap::new());
  }

  fn reload(&mut self) -> ResultAnyError<()> {
    if !self.filepath.exists() {
      let _ = self.watchlist_table();

      self.persist()?;
    }

    let str = fs::read_to_string(&self.filepath)?;
    self.db_content = serde_json::from_str(&str)?;

    return Ok(());
  }

  fn persist(&self) -> ResultAnyError<()> {
    // Create db file if not exist
    if !self.filepath.exists() {
      let mut cloned_filepath = self.filepath.clone();
      cloned_filepath.pop();

      fs::create_dir_all(&cloned_filepath)?;
    }

    // Write all db content to db file
    let content = serde_json::to_string(&self.db_content)?;
    fs::write(&self.filepath, content)?;

    return Ok(());
  }
}

#[derive(Debug, Error)]
enum PhabStorageFilesystemError {
  #[error("PhabStorageFilesystemError err: {message:?}")]
  QueryError { message: String },
}

impl PhabStorageFilesystemError {
  fn query_error(message: &str) -> Error {
    return PhabStorageFilesystemError::QueryError {
      message: message.to_owned(),
    }
    .into();
  }
}

impl PhabStorage for PhabStorageFilesystem {
  fn add_to_watchlist(&mut self, watchlist_id: &str, task: &Task) -> ResultAnyError<()> {
    self
      .watchlist_table()
      .get_mut(watchlist_id)
      .unwrap()
      .tasks
      .push(task.clone());

    self.persist()?;

    return Ok(());
  }

  fn create_watchlist(&mut self, watchlist: &Watchlist) -> ResultAnyError<Watchlist> {
    let watchlists = self.db_content.get_mut("watchlists").unwrap();
    let watchlist_id = slugify!(&watchlist.name);
    let mut watchlist = watchlist.clone();

    watchlist.id = Some(watchlist_id.clone());
    watchlists.insert(watchlist_id, watchlist.to_owned());

    self.persist()?;

    return Ok(watchlist);
  }

  fn get_watchlists(&mut self) -> ResultAnyError<Vec<Watchlist>> {
    let watchlists: Vec<Watchlist> = self.watchlist_table().values().cloned().collect();

    return Ok(watchlists);
  }

  fn get_watchlist_by_id(&mut self, watchlist_id: &str) -> ResultAnyError<Option<Watchlist>> {
    let watchlist = self.watchlist_table().get(watchlist_id).map(Clone::clone);

    return Ok(watchlist);
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
    use fake::Fake;
    use fake::Faker;

    fn create_new(db_dir_path: PathBuf) -> ResultAnyError<PhabStorageFilesystem> {
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

    fn test_db_dir(fn_name: &str) -> PathBuf {
      return PathBuf::from(format!("/tmp/__phab_for_testing/db_{}", fn_name));
    }

    #[test]
    fn it_should_create_dir_and_load_data_for_first_time() -> ResultAnyError<()> {
      let db_dir_path = test_db_dir(function_name!());
      let _dir_cleaner = DirCleaner {
        dir: db_dir_path.clone(),
      };
      let mut storage = test::reload::create_new(db_dir_path)?;

      let watchlists = storage.get_watchlists()?;

      assert_eq!(watchlists.len(), 0);

      return Ok(());
    }

    #[test]
    fn it_should_insert_data() -> ResultAnyError<()> {
      let db_dir_path = test_db_dir(function_name!());
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

    #[test]
    fn it_should_add_to_watchlist() -> ResultAnyError<()> {
      let db_dir_path = test_db_dir(function_name!());
      let _dir_cleaner = DirCleaner {
        dir: db_dir_path.clone(),
      };
      let mut storage = test::reload::create_new(db_dir_path)?;
      let watchlist = Watchlist {
        id: None,
        name: String::from("hey ho test watchlist"),
        tasks: vec![],
      };

      let mut task_1: Task = Faker.fake();
      task_1.id = "foo".to_owned();

      let mut task_2: Task = Faker.fake();
      task_2.id = "Bar".to_owned();

      let watchlist = storage.create_watchlist(&watchlist)?;
      let watchlist_id = watchlist.id.unwrap();
      storage.add_to_watchlist(&watchlist_id, &task_1)?;
      storage.add_to_watchlist(&watchlist_id, &task_2)?;

      let watchlist = storage.get_watchlist_by_id(&watchlist_id)?;

      assert!(watchlist.is_some());

      let tasks: Vec<Task> = watchlist.unwrap().tasks;

      assert_eq!(tasks.len(), 2);
      assert_eq!(tasks.get(0).unwrap().id, "foo");
      assert_eq!(tasks.get(1).unwrap().id, "Bar");

      return Ok(());
    }
  }
}
