use std::collections::HashMap;
use std::error::Error;
use std::fs;

use reqwest::Client as HttpClient;
use reqwest::ClientBuilder as HttpClientBuilder;
use reqwest::Identity;
use serde_json::Value;

use crate::types::ResultDynError;
use failure::Fail;

pub struct PhabricatorClient {
  http: HttpClient,
  host: String,
  api_token: String,
}

pub struct CertIdentityConfig<'a> {
  pub pkcs12_path: &'a str,
  pub pkcs12_password: &'a str,
}

#[derive(Debug, Clone, Fail)]
pub enum ErrorType {
  #[fail(
    display = "Certificate identity path: {}, error: {}",
    pkcs12_path, message
  )]
  CertificateIdentityError {
    pkcs12_path: String,
    message: String,
  },

  #[fail(display = "Fail to configure http client, error: {}", message)]
  FailToConfigureHttpClient { message: String },
}

impl PhabricatorClient {
  /// This function will trim 'T' at the start of phabricator id.
  /// This is to cover case when you copy-paste the phabricator id from url,
  /// e.g. yourphabhost.com/T1234
  /// ```
  /// # use lib::client::phabricator::PhabricatorClient;
  ///
  /// let phabricator_id  = PhabricatorClient::clean_id("T1234");
  /// assert_eq!(phabricator_id, "1234");
  /// ```
  pub fn clean_id(id: &str) -> &str {
    return id.trim_start_matches('T');
  }

  pub fn new(
    host: &str,
    api_token: &str,
    cert_identity_config: Option<CertIdentityConfig>,
  ) -> ResultDynError<PhabricatorClient> {
    let mut http_client_builder = Ok(HttpClientBuilder::new());

    let cert_identity: Option<Result<_, _>> = cert_identity_config.map(|config| {
      return fs::read(config.pkcs12_path)
        .map_err(|err| ErrorType::FailToConfigureHttpClient {
          message: err.to_string(),
        })
        .and_then(|bytes| {
          return Identity::from_pkcs12_der(&bytes, config.pkcs12_password).map_err(|err| {
            ErrorType::CertificateIdentityError {
              pkcs12_path: String::from(config.pkcs12_path),
              message: err.to_string(),
            }
          });
        });
    });

    if let Some(cert_identity) = cert_identity {
      http_client_builder =
        http_client_builder.and_then(|http_client_builder: HttpClientBuilder| {
          return cert_identity.map(|cert_identity: Identity| {
            return http_client_builder.identity(cert_identity);
          });
        });
    }

    return http_client_builder
      .and_then(|http_client_builder| {
        http_client_builder
          .build()
          .map_err(|err| ErrorType::FailToConfigureHttpClient {
            message: err.to_string(),
          })
      })
      .map_err(|err| Box::new(err.into()))
      .map(|http_client| {
        return PhabricatorClient {
          http: http_client,
          host: String::from(host),
          api_token: String::from(api_token),
        };
      });
  }
}

impl PhabricatorClient {
  pub async fn get_tasks(&self, parent_task_ids: Vec<&str>) -> Result<Vec<Task>, Box<dyn Error>> {
    let mut form: HashMap<String, &str> = HashMap::new();

    form.insert(String::from("api.token"), self.api_token.as_str());

    for i in 0..parent_task_ids.len() {
      let task_id = PhabricatorClient::clean_id(parent_task_ids.get(i).unwrap());
      form.insert(format!("constraints[parentIDs][{}]", i), task_id);
    }

    form.insert(String::from("order"), "oldest");
    form.insert(String::from("attachments[columns]"), "true");
    form.insert(String::from("attachments[projects]"), "true");

    let url = format!("{}/api/maniphest.search", self.host);
    let result = self.http.post(&url).form(&form).send().await?;

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
