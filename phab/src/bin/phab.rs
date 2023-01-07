use clap::App as Cli;
use clap::Arg;
use clap::ArgMatches;
use clap::SubCommand;
use env_logger;

use lib::types::ResultAnyError;
use phab_lib::client::phabricator::PhabricatorClient;
use phab_lib::dto::TaskFamily;

pub mod built_info {
  include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[tokio::main]
pub async fn main() -> ResultAnyError<()> {
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

  let print_json = Arg::with_name("print_json")
    .takes_value(false)
    .long("print-json")
    .help("Set if you want to print json");

  return SubCommand::with_name("task")
    .setting(clap::AppSettings::ArgRequiredElseHelp)
    .about("task cli")
    .subcommand(
      SubCommand::with_name("detail")
        .about("View task detail")
        .arg(task_id_arg)
        .arg(&print_json),
    );
}

async fn handle_task_cli(cli: &ArgMatches<'_>) -> ResultAnyError<()> {
  let home_dir = std::env::var("HOME").unwrap();
  let config = lib::config::parse_from_setting_path(format!("{}/.phab", home_dir))?;

  if let Some(task_detail_cli) = cli.subcommand_matches("detail") {
    let parent_task_id = task_detail_cli.value_of("task_id").unwrap();
    let print_json = task_detail_cli.is_present("print_json");

    let phabricator = PhabricatorClient::new(config)?;

    let task_family = phabricator.get_task_family(parent_task_id).await?;

    if task_family.is_none() {
      println!("Could not find task {}", parent_task_id);
    }

    // Just for printing purposes
    let task_families = vec![task_family.unwrap()];

    if print_json {
      println!("{}", TaskFamily::json_string(&task_families)?);
    } else {
      print_tasks(&task_families, 0);
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
      .or(Some(String::from("NoBoard")))
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
