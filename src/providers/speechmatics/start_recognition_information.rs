use serde::{Deserialize, Serialize};
use std::{
  convert::TryInto,
  fs::File,
  io::{BufReader, Error, ErrorKind},
};
use tokio_tungstenite::tungstenite::protocol::Message;

#[derive(Debug, Serialize)]
/// Transcription session information
pub struct StartRecognitionInformation {
  pub message: TranscriptionMode,
  pub transcription_config: TranscriptionConfig,
  pub audio_format: AudioFormat,
}

impl StartRecognitionInformation {
  /// Creating a structure used by the websocket to create a Session
  ///
  /// # Arguments
  ///
  /// * `mode` - Corresponding to the operating point (see Speechmatics Documentation)
  /// # Examples
  ///
  /// ```
  /// let start_recognition_information = StartRecognitionInformation::new("enhanced");
  /// ```
  pub fn new(mode: String) -> Self {
    StartRecognitionInformation {
      message: TranscriptionMode::StartRecognition,
      transcription_config: TranscriptionConfig {
        language: Language::Fr,
        enable_partials: false,
        max_delay: 5.0,
        diarization: "speaker_change".to_string(),
        speaker_change_sensitivity: 0.4,
        additional_vocab: vec![],
        operating_point: mode,
      },
      audio_format: AudioFormat {
        audio_type: AudioType::Raw,
        encoding: AudioEncoding::PcmS16le,
        sample_rate: 16000,
      },
    }
  }

  pub fn set_custom_vocabulary(&mut self, custom_vocabulary_path: &str) {
    let custom_voc_file = File::open(custom_vocabulary_path).expect("File does not exist");
    let reader = BufReader::new(custom_voc_file);
    let custom_vocabulary = serde_json::from_reader(reader).expect("JSON was not well-formatted");
    self.transcription_config.additional_vocab = custom_vocabulary;
  }

  pub fn set_max_delay(&mut self, max_delay: f64) {
    self.transcription_config.max_delay = max_delay;
  }

  pub fn set_diarisation(&mut self, diarisation: f32) {
    self.transcription_config.speaker_diarization_config = Some(SpeakerDiarizationConfig {
      max_speakers: Some(50),
      prefer_current_speaker: Some(false),
      speaker_sensitivity: Some(diarisation),
    });
  }
}

impl TryInto<Message> for StartRecognitionInformation {
  type Error = Error;

  fn try_into(self) -> Result<Message, Self::Error> {
    let serialized =
      serde_json::to_string(&self).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

    Ok(Message::text(serialized))
  }
}

#[derive(Debug, Serialize)]
/// Transcription session configuration
/// See https://docs.speechmatics.com/features
pub struct TranscriptionConfig {
  pub language: Language,
  pub enable_partials: bool,
  pub max_delay: f64,
  pub max_delay_mode: String,
  pub diarization: String,
  pub speaker_diarization_config: Option<SpeakerDiarizationConfig>,
  pub additional_vocab: Vec<CustomVocabulary>,
  pub operating_point: String,
}

#[derive(Debug, Serialize, Deserialize)]
/// Custom vocabulary entry
/// See https://docs.speechmatics.com/features/custom-dictionary
pub struct CustomVocabulary {
  pub content: String,
  pub sounds_like: Vec<String>,
}

#[derive(Debug, Serialize)]
/// Transcription mode
/// See https://docs.speechmatics.com/features/accuracy-language-packs
pub enum TranscriptionMode {
  StartRecognition,
}

#[derive(Debug, Serialize)]
/// Audio format
pub struct AudioFormat {
  #[serde(rename = "type")]
  pub audio_type: AudioType,
  pub encoding: AudioEncoding,
  pub sample_rate: u32,
}

#[derive(Debug, Serialize, Default)]
/// Diarization Config
/// See https://docs.speechmatics.com/speech-to-text/realtime/realtime_diarization#configuration
pub struct SpeakerDiarizationConfig {
  pub max_speakers: Option<i32>,
  pub prefer_current_speaker: Option<bool>,
  pub speaker_sensitivity: Option<f32>,
}

#[derive(Debug, Serialize)]
/// Transcription language
/// See https://docs.speechmatics.com/features/accuracy-language-packs
pub enum Language {
  #[serde(rename = "fr")]
  Fr,
}

#[derive(Debug, Serialize)]
/// Audio type
pub enum AudioType {
  #[serde(rename = "raw")]
  Raw,
}

#[derive(Debug, Serialize)]
/// Audio encoding
pub enum AudioEncoding {
  #[serde(rename = "pcm_s16le")]
  PcmS16le,
}
