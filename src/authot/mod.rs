mod authot_live_information;
pub mod start_recognition_information;
pub mod websocket_response;

pub use start_recognition_information::{
  StartRecognitionInformation, StartRecognitionInformationNew,
};

use crate::WorkerParameters;
use authot_live_information::AuthotLiveInformation;
use futures_util::{sink::SinkExt, stream::StreamExt};
use mcai_worker_sdk::prelude::*;
use reqwest::Client;
use std::{
  convert::{TryFrom, TryInto},
  thread, time,
};
use tokio::net::TcpStream;
use tokio_tls::TlsStream;
use tokio_tungstenite::{connect_async, stream::Stream, WebSocketStream};
use websocket_response::WebsocketResponse;

type McaiWebSocketStream = WebSocketStream<Stream<TcpStream, TlsStream<TcpStream>>>;

pub struct Authot {
  token: String,
}

impl Authot {
  pub async fn new(parameters: &WorkerParameters) -> (Self, McaiWebSocketStream) {
    let authot = Authot {
      token: parameters
        .authot_token
        .clone()
        .unwrap_or_else(|| "".to_string()),
    };

    let service_ip: String = parameters
      .service_instance_ip
      .as_ref()
      .unwrap_or(&"localhost".to_string())
      .to_string();

    let websocket_url = match &parameters.provider[..] {
      "authot" => {
        let authot = Authot {
          token: parameters
            .authot_token
            .clone()
            .unwrap_or_else(|| "".to_string()),
        };
        if let Some(authot_live_id) = parameters.authot_live_id {
          authot
            .get_websocket_url_from_live_id(authot_live_id)
            .await
            .unwrap()
        } else {
          let authot_live_information = authot.new_live().await.unwrap();
          authot.get_websocket_url(&authot_live_information).await
        }
      }
      "speechmatics" | "speechmatics_standard" | "speechmatics_enhanced" => {
        format!("ws://{}:9000/v2", service_ip)
      }
      _ => {
        info!(
          "Provider {} not found, fallback to speechmatics",
          parameters.provider
        );
        format!("ws://{}:9000/v2", service_ip)
      }
    };

    let (mut ws_stream, _) = connect_async(websocket_url)
      .await
      .expect("Failed to connect");

    match &parameters.provider[..] {
      "speechmatics_standard" | "speechmatics_enhanced" => {
        let mode: String = if &parameters.provider[..] == "speechmatics_standard" {
          "standard".to_string()
        } else {
          "enhanced".to_string()
        };
        let mut start_recognition_information = self::StartRecognitionInformationNew::new(mode);
        if let Some(custom_vocabulary) = &parameters.custom_vocabulary {
          start_recognition_information.set_custom_vocabulary(custom_vocabulary.to_string());
        }

        if let Some(max_delay) = &parameters.transcript_interval {
          if let Ok(max_delay_float) = max_delay.parse::<f64>() {
            start_recognition_information.set_max_delay(max_delay_float);
          }
        }

        if let Some(diarisation_balance) = &parameters.diarisation_balance {
          if let Ok(diarisation_balance_float) = diarisation_balance.parse::<f64>() {
            start_recognition_information.set_diarisation(diarisation_balance_float);
          }
        }

        ws_stream
          .send(start_recognition_information.try_into().unwrap())
          .await
          .expect("unable to send start recognition information");

        while let Some(Ok(event)) = ws_stream.next().await {
          let event: Result<WebsocketResponse> =
            self::websocket_response::WebsocketResponse::try_from(event);
          if let Ok(event) = event {
            if event.message == "RecognitionStarted" {
              break;
            }
          }
        }

        (authot, ws_stream)
      }
      _ => {
        let mut start_recognition_information = self::StartRecognitionInformation::new();
        if let Some(custom_vocabulary) = &parameters.custom_vocabulary {
          start_recognition_information.set_custom_vocabulary(custom_vocabulary.to_string());
        }

        if let Some(max_delay) = &parameters.transcript_interval {
          if let Ok(max_delay_float) = max_delay.parse::<f64>() {
            start_recognition_information.set_max_delay(max_delay_float);
          }
        }

        if let Some(diarisation_balance) = &parameters.diarisation_balance {
          if let Ok(diarisation_balance_float) = diarisation_balance.parse::<f64>() {
            start_recognition_information.set_diarisation(diarisation_balance_float);
          }
        }

        ws_stream
          .send(start_recognition_information.try_into().unwrap())
          .await
          .expect("unable to send start recognition information");

        while let Some(Ok(event)) = ws_stream.next().await {
          let event: Result<WebsocketResponse> =
            self::websocket_response::WebsocketResponse::try_from(event);
          if let Ok(event) = event {
            if event.message == "RecognitionStarted" {
              break;
            }
          }
        }

        (authot, ws_stream)
      }
    }
  }

  pub async fn new_live(&self) -> Result<AuthotLiveInformation> {
    let url = format!(
      "https:///authot.live/api/streams/new?lang=fr&translation=false&access_token={}",
      self.token
    );

    let client = Client::builder().build().unwrap();

    let initial_response = client.post(&url).send().await.unwrap();

    let text = initial_response.text().await.unwrap();

    debug!("{:?}", text);
    let initial_response: AuthotLiveInformation = serde_json::from_str(&text).unwrap();

    debug!("{:?}", initial_response);

    loop {
      let one_second = time::Duration::from_millis(2000);
      thread::sleep(one_second);

      let response = self
        .get_live_information(initial_response.id.unwrap())
        .await;

      info!(
        "authot job id {} - {}",
        response.id.unwrap_or(0),
        response.message.clone().unwrap_or_else(|| "".to_string())
      );
      if response.stream_state == 0 {
        return Ok(response);
      }
    }
  }

  async fn get_live_information(&self, live_id: u32) -> AuthotLiveInformation {
    let url = format!("https://authot.live/api/streams/{}/info_stream", live_id);
    let client = Client::builder().build().unwrap();

    client
      .get(&url)
      .header("access-token", self.token.to_owned())
      .send()
      .await
      .unwrap()
      .json::<AuthotLiveInformation>()
      .await
      .unwrap()
  }

  pub async fn get_websocket_url(&self, live_information: &AuthotLiveInformation) -> String {
    format!(
      "{}?access_token={}",
      live_information
        .url
        .clone()
        .expect("Missing URL in live response message"),
      self.token
    )
  }

  pub async fn get_websocket_url_from_live_id(&self, live_id: u32) -> Result<String> {
    let live_information = self.get_live_information(live_id).await;
    Ok(self.get_websocket_url(&live_information).await)
  }
}
