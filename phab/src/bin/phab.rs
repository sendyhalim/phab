use std::error::Error;

use clap::App as Cli;
use clap::Arg;
use clap::ArgMatches;
use clap::SubCommand;
use env_logger;

use lib::client::phabricator::CertIdentityConfig;
use lib::client::phabricator::PhabricatorClient;
use lib::dto::TaskFamily;

type ResultDynError<T> = Result<T, Box<dyn Error>>;

#[macro_use]
extern crate failure;

pub mod built_info {
  include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[tokio::main]
pub async fn main() -> ResultDynError<()> {
  env_logger::init();

  let cli = Cli::new("phab")
    .version(built_info::PKG_VERSION)
    .author(built_info::PKG_AUTHORS)
    .setting(clap::AppSettings::ArgRequiredElseHelp)
    .about(built_info::PKG_DESCRIPTION)
    .subcommand(task_cmd())
    .get_matches();

  if let Some(task_cli) = cli.subcommand_matches("task") {
    handle_task_cli(task_cli).await?;
  }

  return Ok(());
}

fn task_cmd<'a, 'b>() -> Cli<'a, 'b> {
  let task_id_arg = Arg::with_name("task_id")
    .takes_value(true)
    .required(true)
    .help("task id");

  let api_token_arg = Arg::with_name("api_token")
    .takes_value(true)
    .required(true)
    .long("api-token")
    .help("api token");

  let host_arg = Arg::with_name("host")
    .takes_value(true)
    .required(true)
    .long("host")
    .help("host");

  let print_json = Arg::with_name("print_json")
    .takes_value(false)
    .long("print-json")
    .help("Set if you want to print json");

  let pkcs12_path = Arg::with_name("pkcs12_path")
    .takes_value(true)
    .long("pkcs12-path")
    .help("pkcs12 path");

  let pkcs12_password = Arg::with_name("pkcs12_password")
    .takes_value(true)
    .long("pkcs12-password")
    .help("pkcs12 password");

  return SubCommand::with_name("task")
    .setting(clap::AppSettings::ArgRequiredElseHelp)
    .about("task cli")
    .subcommand(
      SubCommand::with_name("detail")
        .about("View task detail")
        .arg(task_id_arg)
        .arg(&api_token_arg)
        .arg(&host_arg)
        .arg(&pkcs12_path)
        .arg(&pkcs12_password)
        .arg(&print_json),
    );
}

async fn handle_task_cli(cli: &ArgMatches<'_>) -> ResultDynError<()> {
  if let Some(task_detail_cli) = cli.subcommand_matches("detail") {
    let parent_task_id = task_detail_cli.value_of("task_id").unwrap();
    let api_token = task_detail_cli.value_of("api_token").unwrap();
    let host = task_detail_cli.value_of("host").unwrap();
    let pkcs12_path = task_detail_cli.value_of("pkcs12_path");
    let pkcs12_password = task_detail_cli.value_of("pkcs12_password");
    let print_json = task_detail_cli.is_present("print_json");

    if pkcs12_path.is_some() && pkcs12_password.is_none() {
      return Err(Box::new(
        format_err!("pkcs12-password must be set!").compat(),
      ));
    }

    let cert_identity_config = pkcs12_path.map(|pkcs12_path| {
      return CertIdentityConfig {
        pkcs12_path,
        pkcs12_password: pkcs12_password.unwrap(),
      };
    });

    let phabricator = PhabricatorClient::new(host, api_token, cert_identity_config)
      .map_err(|failure_error| failure_error.compat())?;

    let child_tasks = phabricator.get_tasks(vec![parent_task_id]).await?;

    if print_json {
      println!("{}", TaskFamily::json_string(&child_tasks)?);
    } else {
      print_tasks(&child_tasks, 0);
    }
  }

  return Ok(());
}

fn print_tasks(task_families: &[TaskFamily], indentation_level: usize) {
  let indentation = std::iter::repeat(" ")
    .take(indentation_level * 2)
    .collect::<String>();

  let task_families = task_families
    .iter()
    .filter(|task_family| task_family.parent_task.status != "invalid")
    .collect::<Vec<&TaskFamily>>();

  for task_family in task_families {
    let task = &task_family.parent_task;

    let board_name = task
      .board
      .as_ref()
      .map(|b| b.name.clone())
      .or({ Some(String::from("NoBoard")) })
      .unwrap();

    println!(
      "{}[T{} {} - {} point: {}] {}",
      indentation,
      task.id,
      task.status,
      board_name,
      task.point.or(Some(0)).unwrap(),
      task.name,
    );

    print_tasks(&task_family.children, indentation_level + 1);
  }
}
