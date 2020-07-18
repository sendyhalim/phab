use std::fs;

use failure::Fail;
use futures::future;
use futures::future::BoxFuture;
use futures::future::FutureExt;
use reqwest::Client as HttpClient;
use reqwest::ClientBuilder as HttpClientBuilder;
use reqwest::Identity;
use serde_json::Value;

use crate::dto::Task;
use crate::dto::TaskFamily;
use crate::dto::User;
use crate::types::ResultDynError;

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

  #[fail(display = "Validation error: {}", message)]
  ValidationError { message: String },

  #[fail(display = "Fetch sub tasks error: {}", message)]
  FetchSubTasksError { message: String },

  #[fail(display = "Fetch task error: {}", message)]
  FetchTaskError { message: String },

  #[fail(display = "Parse error: {}", message)]
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
      .map_err(failure::Error::from)
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
  pub async fn get_user_by_phid(&self, user_phid: &str) -> ResultDynError<Option<User>> {
    return self
      .get_users_by_phids(vec![user_phid])
      .await
      .map(|users| users.get(0).map(ToOwned::to_owned));
  }

  pub async fn get_task_by_id(&self, task_id: &str) -> ResultDynError<Option<Task>> {
    return self
      .get_tasks_by_ids(vec![task_id])
      .await
      .map(|tasks| tasks.get(0).map(ToOwned::to_owned));
  }

  pub async fn get_users_by_phids(&self, user_phids: Vec<&str>) -> ResultDynError<Vec<User>> {
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
      .map_err(failure::Error::from)?;

    let response_text = result.text().await.map_err(failure::Error::from)?;

    log::debug!("Response {}", response_text);

    let body: Value = serde_json::from_str(response_text.as_str()).map_err(failure::Error::from)?;

    if let Value::Array(users_json) = &body["result"]["data"] {
      if users_json.is_empty() {
        return Ok(vec![]);
      }

      log::debug!("Parsing {:?}", users_json);

      // We only have 1 possible assignment
      let users: Vec<User> = users_json.iter().map(User::from_json).collect();

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

  pub async fn get_tasks_by_ids(&self, task_ids: Vec<&str>) -> ResultDynError<Vec<Task>> {
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
      .map_err(failure::Error::from)?;

    let response_text = result.text().await.map_err(failure::Error::from)?;

    log::debug!("Response {}", response_text);

    let body: Value = serde_json::from_str(response_text.as_str()).map_err(failure::Error::from)?;

    if let Value::Array(tasks_json) = &body["result"]["data"] {
      if tasks_json.is_empty() {
        return Ok(vec![]);
      }

      log::debug!("Parsing {:?}", tasks_json);

      // We only have 1 possible assignment
      let tasks: Vec<Task> = tasks_json.iter().map(Task::from_json).collect();

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

  pub async fn get_task_family(&self, root_task_id: &str) -> ResultDynError<Option<TaskFamily>> {
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
  ) -> BoxFuture<'a, ResultDynError<Vec<TaskFamily>>> {
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
        .map_err(failure::Error::from)?;

      let response_text = result.text().await.map_err(failure::Error::from)?;

      log::debug!("Response {}", response_text);

      let body: Value =
        serde_json::from_str(response_text.as_str()).map_err(failure::Error::from)?;

      if let Value::Array(tasks_json) = &body["result"]["data"] {
        let tasks: Vec<BoxFuture<ResultDynError<TaskFamily>>> = tasks_json
          .iter()
          .map(|v: &Value| -> BoxFuture<ResultDynError<TaskFamily>> {
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
