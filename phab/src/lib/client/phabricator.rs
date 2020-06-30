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
    let form: Vec<(String, &str)> = vec![
      ("api.token".to_owned(), self.api_token.as_str()),
      ("constraints[phids][0]".to_owned(), user_phid),
    ];

    let url = format!("{}/api/user.search", self.host);

    log::debug!("Getting users {} {:?}", url, form);

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

    if let Value::Array(users) = &body["result"]["data"] {
      if users.is_empty() {
        return Ok(None);
      }

      let user_json = users.get(0);

      log::debug!("Parsing {:?}", user_json);

      // We only have 1 possible assignment
      let user = User::from_json(user_json.unwrap());

      return Ok(Some(user));
    } else {
      return Err(
        ErrorType::ParseError {
          message: format!("Cannot parse {}", &body),
        }
        .into(),
      );
    }
  }

  pub fn get_tasks<'a>(
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
                .get_tasks(vec![parent_task.id.as_str()])
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
