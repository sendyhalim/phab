use config::Config as BaseConfig;
use config::File;
use config::FileFormat;
use std::path::Path;

use crate::types::ResultDynError;
use phab_lib::client::config::PhabricatorClientConfig;

pub fn parse_from_setting_path(
  setting_path: impl AsRef<Path>,
) -> ResultDynError<PhabricatorClientConfig> {
  let mut c = BaseConfig::new();

  let file_config = File::new(setting_path.as_ref().to_str().unwrap(), FileFormat::Hjson);

  c.merge(file_config)?;

  return c.try_into().map_err(|err| {
    return failure::err_msg(err.to_string());
  });
}
