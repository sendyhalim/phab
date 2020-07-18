use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CertIdentityConfig {
  pub pkcs12_path: String,
  pub pkcs12_password: String,
}

#[derive(Debug, Deserialize)]
pub struct PhabricatorClientConfig {
  pub host: String,
  pub api_token: String,
  pub cert_identity_config: Option<CertIdentityConfig>,
}
