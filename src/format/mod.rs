use mcai_worker_sdk::prelude::*;
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize)]
pub enum OutputFormat {
  EbuTtD,
  Json,
}

impl FromStr for OutputFormat {
  type Err = mcai_worker_sdk::MessageError;

  fn from_str(input: &str) -> Result<OutputFormat> {
    match input {
      "EBU_TT_D" => Ok(OutputFormat::EbuTtD),
      "JSON" => Ok(OutputFormat::Json),
      _ => {
        warn!("Unknwon output format, falling back to EBU-TT-D");
        Ok(OutputFormat::EbuTtD)
      }
    }
  }
}

impl ToString for OutputFormat {
  fn to_string(&self) -> String {
    match &self {
      OutputFormat::EbuTtD => "EBU-TT-D".to_string(),
      OutputFormat::Json => "JSON".to_string(),
    }
  }
}
