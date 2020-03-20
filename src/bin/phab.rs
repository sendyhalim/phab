use std::error::Error;

use clap::App as Cli;
use clap::Arg;
use clap::ArgMatches;
use clap::SubCommand;

use lib::client::phabricator::PhabricatorClient;

type ResultDynError<T> = Result<T, Box<dyn Error>>;

#[tokio::main]
pub async fn main() -> ResultDynError<()> {
  let cli = Cli::new("phab").subcommand(task_cmd()).get_matches();

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

  return SubCommand::with_name("task")
    .setting(clap::AppSettings::ArgRequiredElseHelp)
    .about("task cli")
    .subcommand(
      SubCommand::with_name("detail")
        .about("View task detail")
        .arg(task_id_arg)
        .arg(&api_token_arg),
    );
}

async fn handle_task_cli(cli: &ArgMatches<'_>) -> ResultDynError<()> {
  if let Some(task_detail_cli) = cli.subcommand_matches("detail") {
    let parent_task_id = task_detail_cli.value_of("task_id").unwrap();
    let api_token = task_detail_cli.value_of("api_token").unwrap();
    let phabricator = PhabricatorClient::new(api_token);

    let tasks = phabricator.get_tasks(vec![parent_task_id]).await?;

    for task in &tasks {
      println!(
        "[{} - P: {}] {}",
        task.board.name,
        task.point.or(Some(0)).unwrap(),
        task.name,
      );
    }
  }

  return Ok(());
}
