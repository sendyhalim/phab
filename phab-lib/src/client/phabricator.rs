use std::fs;

use anyhow::Error;
use futures::future;
use futures::future::BoxFuture;
use futures::future::FutureExt;
use reqwest::Client as HttpClient;
use reqwest::ClientBuilder as HttpClientBuilder;
use reqwest::Identity;
use serde_json::Value;

use crate::client::config::PhabricatorClientConfig;
use crate::dto::Task;
use crate::dto::TaskFamily;
use crate::dto::User;
use crate::types::ResultAnyError;

pub struct PhabricatorClient {
  http: HttpClient,
  host: String,
  api_token: String,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ErrorType {
  #[error("Certificate identity path: {pkcs12_path:?}, error: {message:?}")]
  CertificateIdentityError {
    pkcs12_path: String,
    message: String,
  },

  #[error("Fail to configure http client, error: {message}")]
  FailToConfigureHttpClient { message: String },

  #[error("Validation error: {message}")]
  ValidationError { message: String },

  #[error("Fetch sub tasks error: {message}")]
  FetchSubTasksError { message: String },

  #[error("Fetch task error: {message}")]
  FetchTaskError { message: String },

  #[error("Parse error: {message}")]
  ParseError { message: String },
}

impl PhabricatorClient {
  /// This function will trim 'T' at the start of phabricator id.
  /// This is to cover case when you copy-paste the phabricator id from url,
  /// e.g. yourphabhost.com/T1234
  /// ```
  /// # use phab_lib::client::phabricator::PhabricatorClient;
  ///
  /// let phabricator_id  = PhabricatorClient::clean_id("T1234");
  /// assert_eq!(phabricator_id, "1234");
  /// ```
  pub fn clean_id(id: &str) -> &str {
    return id.trim_start_matches('T');
  }

  pub fn new(config: PhabricatorClientConfig) -> ResultAnyError<PhabricatorClient> {
    let mut http_client_builder = Ok(HttpClientBuilder::new());
    let PhabricatorClientConfig {
      host,
      api_token,
      cert_identity_config,
    } = config;

    let cert_identity: Option<Result<_, _>> = cert_identity_config.map(|config| {
      return fs::read(&config.pkcs12_path)
        .map_err(|err| ErrorType::FailToConfigureHttpClient {
          message: format!("Failed to read pkcs12 from {}, {}", config.pkcs12_path, err),
        })
        .and_then(|bytes| {
          return Identity::from_pkcs12_der(&bytes, &config.pkcs12_password).map_err(|err| {
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
      .map_err(Error::new)
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
  pub async fn get_user_by_phid(&self, user_phid: &str) -> ResultAnyError<Option<User>> {
    return self
      .get_users_by_phids(vec![user_phid])
      .await
      .map(|users| users.get(0).map(ToOwned::to_owned));
  }

  pub async fn get_task_by_id(&self, task_id: &str) -> ResultAnyError<Option<Task>> {
    return self
      .get_tasks_by_ids(vec![task_id])
      .await
      .map(|tasks| tasks.get(0).map(ToOwned::to_owned));
  }

  pub async fn get_users_by_phids(&self, user_phids: Vec<&str>) -> ResultAnyError<Vec<User>> {
    let mut form: Vec<(String, &str)> = vec![("api.token".to_owned(), self.api_token.as_str())];

    for i in 0..user_phids.len() {
      let key = format!("constraints[phids][{}]", i);
      let user_phid = user_phids.get(i).unwrap();

      form.push((key, user_phid));
    }

    let url = format!("{}/api/user.search", self.host);

    log::debug!("Getting user by id {} {:?}", url, form);

    let result = self
      .http
      .post(&url)
      .form(&form)
      .send()
      .await
      .map_err(Error::new)?;

    let response_text = result.text().await.map_err(Error::new)?;

    log::debug!("Response {}", response_text);

    let body: Value = serde_json::from_str(response_text.as_str()).map_err(Error::new)?;

    if let Value::Array(users_json) = &body["result"]["data"] {
      if users_json.is_empty() {
        return Ok(vec![]);
      }

      log::debug!("Parsing {:?}", users_json);

      // We only have 1 possible assignment
      let users: Vec<User> = users_json.iter().map(User::from_json).collect();

      log::debug!("Parsed {:?}", users);

      return Ok(users);
    } else {
      return Err(
        ErrorType::ParseError {
          message: format!("Cannot parse {}", &body),
        }
        .into(),
      );
    }
  }

  pub async fn get_tasks_by_ids(&self, task_ids: Vec<&str>) -> ResultAnyError<Vec<Task>> {
    let mut form: Vec<(String, &str)> = vec![
      ("api.token".to_owned(), self.api_token.as_str()),
      ("order".to_owned(), "oldest"),
      ("attachments[columns]".to_owned(), "true"),
      ("attachments[projects]".to_owned(), "true"),
    ];

    for i in 0..task_ids.len() {
      let key = format!("constraints[ids][{}]", i);
      let task_id = PhabricatorClient::clean_id(task_ids.get(i).unwrap());

      form.push((key, task_id));
    }

    let url = format!("{}/api/maniphest.search", self.host);

    log::debug!("Getting task by id {} {:?}", url, form);

    let result = self
      .http
      .post(&url)
      .form(&form)
      .send()
      .await
      .map_err(Error::new)?;

    let response_text = result.text().await.map_err(Error::new)?;

    log::debug!("Response {}", response_text);

    let body: Value = serde_json::from_str(response_text.as_str()).map_err(Error::new)?;

    if let Value::Array(tasks_json) = &body["result"]["data"] {
      if tasks_json.is_empty() {
        return Ok(vec![]);
      }

      log::debug!("Parsing {:?}", tasks_json);

      // We only have 1 possible assignment
      let tasks: Vec<Task> = tasks_json.iter().map(Task::from_json).collect();

      log::debug!("Parsed {:?}", tasks);

      return Ok(tasks);
    } else {
      return Err(
        ErrorType::ParseError {
          message: format!("Cannot parse {}", &body),
        }
        .into(),
      );
    }
  }

  pub async fn get_task_family(&self, root_task_id: &str) -> ResultAnyError<Option<TaskFamily>> {
    let parent_task = self.get_task_by_id(root_task_id).await?;

    if parent_task.is_none() {
      return Ok(None);
    }

    let parent_task = parent_task.unwrap();

    let child_tasks = self.get_child_tasks(vec![root_task_id]).await?;
    let task_family = TaskFamily {
      parent_task,
      children: child_tasks,
    };

    return Ok(Some(task_family));
  }

  pub fn get_child_tasks<'a>(
    &'a self,
    parent_task_ids: Vec<&'a str>,
  ) -> BoxFuture<'a, ResultAnyError<Vec<TaskFamily>>> {
    return async move {
      if parent_task_ids.is_empty() {
        return Err(
          ErrorType::ValidationError {
            message: String::from("Parent ids cannot be empty"),
          }
          .into(),
        );
      }

      let mut form: Vec<(String, &str)> = vec![("api.token".to_owned(), self.api_token.as_str())];

      for i in 0..parent_task_ids.len() {
        let task_id = PhabricatorClient::clean_id(parent_task_ids.get(i).unwrap());
        let key = format!("constraints[parentIDs][{}]", i);

        form.push((key, task_id));
      }

      form.push(("order".to_owned(), "oldest"));
      form.push(("attachments[columns]".to_owned(), "true"));
      form.push(("attachments[projects]".to_owned(), "true"));

      let url = format!("{}/api/maniphest.search", self.host);

      log::debug!("Getting tasks {} {:?}", url, form);

      let result = self
        .http
        .post(&url)
        .form(&form)
        .send()
        .await
        .map_err(Error::new)?;

      let response_text = result.text().await.map_err(Error::new)?;

      log::debug!("Response {}", response_text);

      let body: Value = serde_json::from_str(response_text.as_str()).map_err(Error::new)?;

      if let Value::Array(tasks_json) = &body["result"]["data"] {
        let tasks: Vec<BoxFuture<ResultAnyError<TaskFamily>>> = tasks_json
          .iter()
          .map(|v: &Value| -> BoxFuture<ResultAnyError<TaskFamily>> {
            return async move {
              let parent_task = Task::from_json(&v);

              let children = self
                .get_child_tasks(vec![parent_task.id.as_str()])
                .await
                .map_err(|err| {
                  return ErrorType::FetchSubTasksError {
                    message: format!(
                      "Could not fetch sub tasks with parent id {}, err: {}",
                      parent_task.id, err
                    ),
                  };
                })?;

              return Ok(TaskFamily {
                parent_task,
                children,
              });
            }
            .boxed();
          })
          .collect();

        let (tasks, failed_tasks): (Vec<_>, Vec<_>) = future::join_all(tasks)
          .await
          .into_iter()
          .partition(Result::is_ok);

        if !failed_tasks.is_empty() {
          let error = ErrorType::FetchSubTasksError {
            message: failed_tasks
              .into_iter()
              .fold(String::new(), |acc, task_result| {
                return format!("{}\n{}", acc, task_result.err().unwrap());
              }),
          };

          return Err(error.into());
        }

        let task_families: Vec<TaskFamily> = tasks.into_iter().map(Result::unwrap).collect();

        return Ok(task_families);
      } else {
        return Err(
          ErrorType::ParseError {
            message: format!("Cannot parse {}", &body),
          }
          .into(),
        );
      }
    }
    .boxed();
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::client::config::CertIdentityConfig;

  fn dummy_config() -> PhabricatorClientConfig {
    return PhabricatorClientConfig {
      host: "http://localhost".into(),
      api_token: "foo".into(),
      cert_identity_config: None,
    };
  }

  #[test]
  fn test_create_new_client_with_invalid_pkcs12_path() {
    let mut config = dummy_config();
    config.cert_identity_config = Some(CertIdentityConfig {
      pkcs12_path: "/path/to/invalid/config".into(),
      pkcs12_password: "testpassword".into(),
    });

    let maybe_client = PhabricatorClient::new(config);

    assert!(maybe_client.is_err());

    let err = maybe_client.err().unwrap();

    assert!(err
      .to_string()
      .contains("Failed to read pkcs12 from /path/to/invalid/config"));
  }
}
