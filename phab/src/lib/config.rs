use std::fs;
use std::path::Path;

use crate::types::ResultAnyError;
use phab_lib::client::config::PhabricatorClientConfig;

pub fn parse_from_setting_path(
  setting_path: impl AsRef<Path>,
) -> ResultAnyError<PhabricatorClientConfig> {
  let file_content = fs::read_to_string(&setting_path)?;

  let configuration: PhabricatorClientConfig = deser_hjson::from_str(&file_content)?;

  return Ok(configuration);
}
