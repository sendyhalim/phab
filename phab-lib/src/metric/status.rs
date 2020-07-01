use std::collections::HashSet;

use crate::dto::Task;

pub struct StatusMetric;

impl StatusMetric {
  /// ```
  /// use std::collections::HashSet;
  /// use fake::Faker;
  /// use fake::Fake;
  /// use fake::Dummy;
  /// use phab_lib::dto::Task;
  /// use phab_lib::metric::status::StatusMetric;
  ///
  /// let mut done_statuses = HashSet::new();
  /// done_statuses.insert("done".to_owned());
  /// done_statuses.insert("foo".to_owned());
  ///
  /// let mut task_1: Task = Faker.fake();
  /// task_1.status = "done".to_owned();
  ///
  /// let mut task_2: Task = Faker.fake();
  /// task_2.status = "done".to_owned();
  ///
  /// let tasks: Vec<Task> = vec![
  ///   task_1,
  ///   task_2,
  ///   Faker.fake(),
  /// ];
  ///
  /// assert_eq!(StatusMetric::count_done_tasks(vec![], &done_statuses), 0);
  /// assert_eq!(StatusMetric::count_done_tasks(tasks, &done_statuses), 2);
  /// ```
  pub fn count_done_tasks(tasks: Vec<Task>, done_statuses: &HashSet<String>) -> usize {
    return tasks
      .iter()
      .filter(|task| done_statuses.contains(&task.status))
      .count();
  }
}
