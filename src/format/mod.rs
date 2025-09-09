use mcai_worker_sdk::prelude::*;
use serde::Deserialize;
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Deserialize)]
pub enum OutputFormat {
  EbuTtD,
  Json,
}

impl FromStr for OutputFormat {
  type Err = MessageError;

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

impl fmt::Display for OutputFormat {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let str = match &self {
      OutputFormat::EbuTtD => "EBU-TT-D",
      OutputFormat::Json => "JSON",
    };

    write!(f, "{str}")
  }
}
