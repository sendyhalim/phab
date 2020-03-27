use futures::future::BoxFuture;
use futures::future::FutureExt;
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

    print_tasks(&phabricator, parent_task_id, 0).await?;
  }

  return Ok(());
}

fn print_tasks<'a>(
  phabricator: &'a PhabricatorClient,
  parent_task_id: &'a str,
  indentation_level: usize,
) -> BoxFuture<'a, ResultDynError<()>> {
  return async move {
    let tasks = phabricator.get_tasks(vec![parent_task_id]).await?;
    let indentation = std::iter::repeat(" ")
      .take(indentation_level * 2)
      .collect::<String>();

    for task in &tasks {
      let board_name = task
        .board
        .as_ref()
        .map(|b| b.name.clone())
        .or({ Some(String::from("NoBoard")) })
        .unwrap();

      println!(
        "{}[T{} - {} point: {}] {}",
        indentation,
        task.id,
        board_name,
        task.point.or(Some(0)).unwrap(),
        task.name,
      );

      // TODO: Do async recursion-blocking within phabricator client.
      let sub_tasks = phabricator.get_tasks(vec![&task.id]).await?;

      if sub_tasks.len() > 0 {
        print_tasks(phabricator, &task.id, indentation_level + 1).await?;
      }
    }

    return Ok(());
  }
  .boxed();
}
