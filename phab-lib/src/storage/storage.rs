use crate::dto::Task;
use crate::dto::Watchlist;
use crate::types::ResultAnyError;

pub trait PhabStorage {
  fn add_to_watchlist(&mut self, watchlist_id: &str, task: &Task) -> ResultAnyError<()>;
  fn create_watchlist(&mut self, watchlist: &Watchlist) -> ResultAnyError<Watchlist>;
  fn get_watchlists(&mut self) -> ResultAnyError<Vec<Watchlist>>;
  fn get_watchlist_by_id(&mut self, watchlist_id: &str) -> ResultAnyError<Option<Watchlist>>;
}
