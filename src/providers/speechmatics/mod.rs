pub mod start_recognition_information;
pub mod websocket_response;

pub use start_recognition_information::StartRecognitionInformation;
use websocket_response::WebsocketResponse;

use crate::WorkerParameters;
use futures_util::{sink::SinkExt, stream::StreamExt};
use mcai_worker_sdk::prelude::*;
use std::convert::{TryFrom, TryInto};
use tokio::net::TcpStream;
use tokio_tls::TlsStream;
use tokio_tungstenite::{WebSocketStream, connect_async, stream::Stream};

type McaiWebSocketStream = WebSocketStream<Stream<TcpStream, TlsStream<TcpStream>>>;

/// Creates a new connection to Speechmatics Real-Time container/instance
/// See Speechmatics documentation : https://docs.speechmatics.com/introduction/rt-guide
pub async fn new(parameters: &WorkerParameters) -> Result<McaiWebSocketStream> {
  let service_ip = parameters
    .service_instance_ip
    .as_ref()
    .map_or("localhost", String::as_str);

  match parameters.provider.as_str() {
    "speechmatics" | "speechmatics_standard" | "speechmatics_enhanced" => {}
    _ => {
      info!(
        "Provider {} not found, fallback to speechmatics",
        parameters.provider
      );
    }
  }

  let websocket_port = match service_ip {
    "speechmatics-2" => 9002,
    _ => 9000,
  };
  let websocket_url = format!("ws://{service_ip}:{websocket_port}/v2");

  let mut i = 0;
  let (mut ws_stream, _) = loop {
    let result = connect_async(websocket_url.clone()).await;
    if let Err(e) = result {
      error!("{e}");
      if i == 3 {
        return Err(MessageError::RuntimeError(e.to_string()));
      }
      i += 1;
    } else {
      break result.unwrap();
    }
  };

  let mode = if &parameters.provider == "speechmatics_standard" {
    "standard"
  } else {
    "enhanced"
  };

  // Information for the websocket
  let mut start_recognition_information = StartRecognitionInformation::new(mode.into());

  // Custom Vocabulary
  if let Some(custom_vocabulary) = &parameters.custom_vocabulary {
    start_recognition_information.set_custom_vocabulary(custom_vocabulary);
  }

  // Transcript interval (length of each transcript)
  if let Some(max_delay) = &parameters.transcript_interval
    && let Ok(max_delay_float) = max_delay.parse::<f64>()
  {
    start_recognition_information.set_max_delay(max_delay_float);
  }

  // Diarisation balance
  if let Some(diarisation_balance) = &parameters.diarisation_balance
    && let Ok(diarisation_balance_float) = diarisation_balance.parse::<f32>()
  {
    start_recognition_information.set_diarisation(diarisation_balance_float);
  }

  ws_stream
    .send(start_recognition_information.try_into().unwrap())
    .await
    .map_err(|e| MessageError::RuntimeError(e.to_string()))?;

  while let Some(Ok(event)) = ws_stream.next().await {
    let event: Result<WebsocketResponse> =
      self::websocket_response::WebsocketResponse::try_from(event);
    if let Ok(event) = event
      && event.message == "RecognitionStarted"
    {
      break;
    }
  }

  Ok(ws_stream)
}
