use crate::dto::Task;
use crate::dto::Watchlist;
use crate::types::ResultDynError;

pub trait PhabStorage {
  fn add_to_watchlist(&mut self, watchlist_id: &str, task: &Task) -> ResultDynError<()>;
  fn create_watchlist(&mut self, watchlist: &Watchlist) -> ResultDynError<Watchlist>;
  fn get_watchlists(&mut self) -> ResultDynError<Vec<Watchlist>>;
  fn get_watchlist_by_id(&mut self, watchlist_id: &str) -> ResultDynError<Option<Watchlist>>;
}
