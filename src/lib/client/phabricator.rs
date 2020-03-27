use std::collections::HashMap;
use std::error::Error;

use reqwest;
use serde_json::{Result as SerdeResult, Value};

pub struct PhabricatorClient {
  http: reqwest::Client,
  api_token: String,
}

impl PhabricatorClient {
  pub fn new(api_token: &str) -> PhabricatorClient {
    return PhabricatorClient {
      http: reqwest::Client::new(),
      api_token: String::from(api_token),
    };
  }
}

impl PhabricatorClient {
  pub async fn get_tasks(&self, parent_task_ids: Vec<&str>) -> Result<Vec<Task>, Box<dyn Error>> {
    let mut form: HashMap<String, &str> = HashMap::new();

    form.insert(String::from("api.token"), self.api_token.as_str());

    for i in 0..parent_task_ids.len() {
      form.insert(
        format!("constraints[parentIDs][{}]", i),
        parent_task_ids.get(i).unwrap(),
      );
    }

    form.insert(String::from("order"), "oldest");
    form.insert(String::from("attachments[columns]"), "true");
    form.insert(String::from("attachments[projects]"), "true");

    let result = self
      .http
      .post("https://p.cermati.com/api/maniphest.search")
      .form(&form)
      .send()
      .await?;

    let response_text = result.text().await?;
    let body: Value = serde_json::from_str(response_text.as_str())?;

    if let Value::Array(tasks_json) = &body["result"]["data"] {
      let tasks: Vec<Task> = tasks_json
        .iter()
        .map(|v: &Value| -> Task {
          return Task::from_json(&v);
        })
        .collect();

      return Ok(tasks);
    } else {
      panic!("Cannot parse {}", &body);
    }
  }
}

pub struct Task {
  pub id: String,
  pub task_type: String,
  pub phid: String,
  pub name: String,
  pub description: String,
  pub author_phid: String,
  pub owner_phid: Option<String>, // Assigned
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
      owner_phid: fields["ownerPHID"].as_str().map(Into::into),
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
    project_phids: &Vec<String>,
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

pub struct Board {
  pub id: u64,
  pub phid: String,
  pub name: String,
}
