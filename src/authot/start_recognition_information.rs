use std::{
  convert::TryInto,
  fs::File,
  io::{BufReader, Error, ErrorKind},
};
use tokio_tungstenite::tungstenite::protocol::Message;

#[derive(Debug, Serialize)]
pub struct StartRecognitionInformation {
  pub message: TranscriptionMode,
  pub transcription_config: TranscriptionConfig,
  pub audio_format: AudioFormat,
}

impl StartRecognitionInformation {
  pub fn new() -> Self {
    StartRecognitionInformation {
      message: TranscriptionMode::StartRecognition,
      transcription_config: TranscriptionConfig {
        language: Language::Fr,
        enable_partials: false,
        max_delay: 5.0,
        diarization: "speaker_change".to_string(),
        additional_vocab: vec![],
      },
      audio_format: AudioFormat {
        audio_type: AudioType::Raw,
        encoding: AudioEncoding::PcmS16le,
        sample_rate: 16000,
      },
    }
  }

  pub fn set_custom_vocabulary(&mut self, custom_vocabulary_path: String) {
    let custom_voc_file = File::open(custom_vocabulary_path).expect("File does not exist");
    let reader = BufReader::new(custom_voc_file);
    let custom_vocabulary: Vec<CustomVocabulary> =
      serde_json::from_reader(reader).expect("JSON was not well-formatted");
    self.transcription_config.additional_vocab = custom_vocabulary;
  }

  pub fn set_max_delay(&mut self, max_delay: f64) {
    self.transcription_config.max_delay = max_delay;
  }
}

impl TryInto<Message> for StartRecognitionInformation {
  type Error = Error;
  fn try_into(self) -> Result<Message, Self::Error> {
    let serialized = serde_json::to_string(&self)
      .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
    Ok(Message::text(serialized))
  }
}

#[derive(Debug, Serialize)]
pub struct StartRecognitionInformationNew {
  pub message: TranscriptionMode,
  pub transcription_config: TranscriptionConfigNew,
  pub audio_format: AudioFormat,
}

impl StartRecognitionInformationNew {
  pub fn new() -> Self {
    StartRecognitionInformationNew {
      message: TranscriptionMode::StartRecognition,
      transcription_config: TranscriptionConfigNew {
        language: Language::Fr,
        enable_partials: false,
        max_delay: 5.0,
        diarization: "speaker_change".to_string(),
        additional_vocab: vec![],
        operating_point: "standard".to_string(),
      },
      audio_format: AudioFormat {
        audio_type: AudioType::Raw,
        encoding: AudioEncoding::PcmS16le,
        sample_rate: 16000,
      },
    }
  }

  pub fn set_custom_vocabulary(&mut self, custom_vocabulary_path: String) {
    let custom_voc_file = File::open(custom_vocabulary_path).expect("File does not exist");
    let reader = BufReader::new(custom_voc_file);
    let custom_vocabulary: Vec<CustomVocabulary> =
      serde_json::from_reader(reader).expect("JSON was not well-formatted");
    self.transcription_config.additional_vocab = custom_vocabulary;
  }

  pub fn set_max_delay(&mut self, max_delay: f64) {
    self.transcription_config.max_delay = max_delay;
  }
}

impl TryInto<Message> for StartRecognitionInformationNew {
  type Error = Error;
  fn try_into(self) -> Result<Message, Self::Error> {
    let serialized = serde_json::to_string(&self)
      .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
    Ok(Message::text(serialized))
  }
}

#[derive(Debug, Serialize)]
pub struct TranscriptionConfig {
  pub language: Language,
  pub enable_partials: bool,
  pub max_delay: f64,
  pub diarization: String,
  pub additional_vocab: Vec<CustomVocabulary>,
}

#[derive(Debug, Serialize)]
pub struct TranscriptionConfigNew {
  pub language: Language,
  pub enable_partials: bool,
  pub max_delay: f64,
  pub diarization: String,
  pub additional_vocab: Vec<CustomVocabulary>,
  pub operating_point: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CustomVocabulary {
  pub content: String,
  pub sounds_like: Vec<String>,
}

#[derive(Debug, Serialize)]
pub enum TranscriptionMode {
  StartRecognition,
}

#[derive(Debug, Serialize)]
pub struct AudioFormat {
  #[serde(rename = "type")]
  pub audio_type: AudioType,
  pub encoding: AudioEncoding,
  pub sample_rate: u32,
}

#[derive(Debug, Serialize)]
pub enum Language {
  #[serde(rename = "fr")]
  Fr,
}

#[derive(Debug, Serialize)]
pub enum AudioType {
  #[serde(rename = "raw")]
  Raw,
}

#[derive(Debug, Serialize)]
pub enum AudioEncoding {
  #[serde(rename = "pcm_s16le")]
  PcmS16le,
}
