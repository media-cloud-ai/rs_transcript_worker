#[derive(Debug, Deserialize)]
pub struct Token {
  pub access_token: String,
  pub scope: Option<String>,
  pub expires_in: Option<u32>,
  pub message: Option<String>,
  pub token_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Authentication {
  pub client_id: String,
  pub username: String,
  pub password: String,
  pub connection: String,
  pub grant_type: String,
  pub scope: String,
}
