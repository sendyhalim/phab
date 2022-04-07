use anyhow::Error;
use fake::Dummy;
use fake::Fake;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::types::ResultAnyError;

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskFamily {
  pub parent_task: Task,
  pub children: Vec<TaskFamily>,
}

impl TaskFamily {
  pub fn json_string(task_families: &[TaskFamily]) -> ResultAnyError<String> {
    return serde_json::to_string(task_families).map_err(Error::new);
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, Dummy)]
pub struct Task {
  pub id: String,
  pub task_type: String,
  pub phid: String,
  pub name: String,
  pub description: String,
  pub author_phid: String,
  pub assigned_phid: Option<String>,
  pub status: String,
  pub priority: String,
  pub point: Option<u64>,
  pub project_phids: Vec<String>,
  pub board: Option<Board>,
  pub created_at: u64,
  pub updated_at: u64,
}

impl Task {
  pub fn from_json(v: &Value) -> Task {
    let project_phids: &Vec<Value> = match &v["attachments"]["projects"]["projectPHIDs"] {
      Value::Array(arr) => arr,
      _ => panic!(
        "Project phids is not an array {:?}",
        v["attachments"]["projects"]["projectPHIDs"]
      ),
    };

    let project_phids: Vec<String> = project_phids.iter().map(json_to_string).collect();

    let board: Option<&Value> =
      Task::guess_board_from_projects(&v["attachments"]["columns"]["boards"], &project_phids);
    let fields: &Value = &v["fields"];

    let task = Task {
      id: format!("{}", v["id"].as_u64().unwrap()),
      task_type: json_to_string(&v["type"]),
      phid: json_to_string(&v["phid"]),
      name: json_to_string(&fields["name"]),
      description: json_to_string(&fields["description"]["raw"]),
      author_phid: json_to_string(&fields["authorPHID"]),
      assigned_phid: fields["ownerPHID"].as_str().map(Into::into),
      status: json_to_string(&fields["status"]["value"]),
      priority: json_to_string(&fields["priority"]["name"]),
      point: fields["points"].as_u64(),
      project_phids,
      board: board.map(|board: &Value| {
        return Board {
          id: board["id"].as_u64().unwrap(),
          phid: board["phid"].as_str().unwrap().into(),
          name: board["name"].as_str().unwrap().into(),
        };
      }),
      created_at: fields["dateCreated"].as_u64().unwrap(),
      updated_at: fields["dateModified"].as_u64().unwrap(),
    };

    return task;
  }

  pub fn guess_board_from_projects<'a>(
    boards: &'a Value,
    project_phids: &[String],
  ) -> Option<&'a Value> {
    return project_phids
      .iter()
      .find(|phid| {
        return boards[phid] != Value::Null;
      })
      .map(|phid| &boards[&phid]["columns"][0]);
  }
}

fn json_to_string(v: &Value) -> String {
  return v.as_str().unwrap().into();
}

#[derive(Clone, Debug, Serialize, Deserialize, Dummy)]
pub struct Board {
  pub id: u64,
  pub phid: String,
  pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Dummy)]
pub struct Watchlist {
  pub id: Option<String>,
  pub name: String,
  pub tasks: Vec<Task>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Dummy)]
pub struct User {
  pub id: String,
  pub phid: String,
  pub username: String,
  pub name: String,
  pub created_at: u64,
  pub updated_at: u64,
}

impl User {
  pub fn from_json(v: &Value) -> User {
    let fields: &Value = &v["fields"];

    return User {
      id: format!("{}", v["id"].as_u64().unwrap()),
      phid: json_to_string(&v["phid"]),
      username: json_to_string(&fields["username"]),
      name: json_to_string(&fields["realName"]),
      created_at: fields["dateCreated"].as_u64().unwrap(),
      updated_at: fields["dateModified"].as_u64().unwrap(),
    };
  }
}
