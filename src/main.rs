#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use chrono::{DateTime, Utc};
use format::OutputFormat;
use futures::channel::mpsc::{channel, Sender};
use futures_util::{future, pin_mut, StreamExt};
use mcai_worker_sdk::{
  default_rust_mcai_worker_description, job::JobResult, prelude::*, MessageError,
};
use providers::speechmatics::websocket_response::WebsocketResponse;

use std::{
  convert::TryFrom,
  str::FromStr,
  sync::{
    atomic::{
      AtomicUsize,
      Ordering::{Acquire, Release},
    },
    mpsc::Sender as StdSender,
    Arc, Mutex,
  },
  thread,
  thread::JoinHandle,
  time::Duration,
};
use tokio::runtime::Runtime;
use tokio_tungstenite::tungstenite::protocol::Message;

mod format;
mod providers;
use providers::speechmatics::websocket_response;

default_rust_mcai_worker_description!();

#[derive(Debug, Default)]
struct McaiRustWorker {}

#[derive(Debug, Default)]
#[allow(dead_code)]
struct TranscriptEvent {
  sequence_number: u64,
  start_time: Option<f32>,
  audio_source_sender: Option<Sender<Message>>,
  sender: Option<Arc<Mutex<StdSender<ProcessResult>>>>,
  ws_thread: Option<JoinHandle<()>>,
  clock_vec: Arc<Mutex<Vec<DateTime<Utc>>>>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct WorkerParameters {
  /// # Custom vocabulary
  /// Extend the knowledge of the provider by adding some specific words.
  custom_vocabulary: Option<String>,
  /// # Provider
  /// Name of the provider used for the transcription
  provider: String,
  /// # Service Instance IP
  /// IP address of the service instance
  service_instance_ip: Option<String>,
  /// # Transcript Interval
  /// Interval between two transcripts arrival
  transcript_interval: Option<String>,
  /// # Diarisation balance
  /// Balance between accuracy and recall for diarisation (0.0 to 1.0, standard is 0.4)
  diarisation_balance: Option<String>,
  /// # Output Format
  /// Output Format for transcription between EBU-TT-D and json
  output_format: Option<String>,
  destination_path: String,
  source_path: String,
}

impl McaiWorker<WorkerParameters, RustMcaiWorkerDescription> for TranscriptEvent {
  fn init_process(
    &mut self,
    parameters: WorkerParameters,
    format_context: Arc<Mutex<FormatContext>>,
    response_sender: Arc<Mutex<StdSender<ProcessResult>>>,
  ) -> Result<Vec<StreamDescriptor>> {
    let format_context = format_context.lock().unwrap();
    self.start_time = format_context.get_start_time();
    let start_offset = self.start_time.unwrap();

    let selected_streams = get_first_audio_stream_id(&format_context)?;

    let cloned_parameters = parameters;
    let param_output_format = cloned_parameters.output_format.clone();

    let output_format = OutputFormat::from_str(
      &(param_output_format.unwrap_or_else(|| OutputFormat::EbuTtD.to_string())),
    )
    .expect("Cannot get output format");

    let (audio_source_sender, audio_source_receiver) = channel(10000);

    self.audio_source_sender = Some(audio_source_sender);

    let cloned_sender = response_sender.clone();
    let cloned_clock_vec = self.clock_vec.clone();
    let start_time = self.start_time;

    self.sender = Some(response_sender);

    self.ws_thread = Some(thread::spawn(move || {
      let sequence_number = Arc::new(AtomicUsize::new(0));

      let future = async {
        let ws_stream = providers::speechmatics::new(&cloned_parameters).await;
        let (ws_sender, ws_receiver) = ws_stream.split();

        let send_to_ws = audio_source_receiver.map(Ok).forward(ws_sender);

        let receive_from_ws = {
          ws_receiver.for_each(|event| async {
            let event = event.unwrap();
            debug!("{}", event);
            let event: Result<WebsocketResponse> = WebsocketResponse::try_from(event);

            if let Ok(event) = event {
              if event.message == "AudioAdded" {}
              if event.message == "EndOfTranscript" {
                info!("End of transcript from provider");
                let result = ProcessResult::end_of_process();
                cloned_sender.lock().unwrap().send(result).unwrap();
              }
              if event.message == "AddTranscript" {
                match output_format {
                  OutputFormat::EbuTtD => {
                    if let Some(mut metadata) = event.metadata {
                      metadata.start_time += start_offset as f64;
                      metadata.end_time += start_offset as f64;
                      let sequence_index = sequence_number.load(Acquire);
                      cloned_clock_vec.lock().unwrap().clear();

                      let result =
                        ProcessResult::new_xml(metadata.generate_ttml(start_time, sequence_index));
                      cloned_sender.lock().unwrap().send(result).unwrap();

                      sequence_number.store(sequence_index + 1, Release);
                    }
                  }
                  OutputFormat::Json => {
                    let sequence_index = sequence_number.load(Acquire);
                    let updated_metadata = if let Some(metadata) = event.metadata {
                      let clock: DateTime<Utc> = cloned_clock_vec.lock().unwrap()[0];
                      cloned_clock_vec.lock().unwrap().clear();
                      info!("Clock {}", clock);
                      Some(websocket_response::Metadata {
                        start_time: metadata.start_time,
                        end_time: metadata.end_time,
                        transcript: metadata.transcript,
                        clock: Some(clock),
                      })
                    } else {
                      None
                    };
                    let updated_event = WebsocketResponse {
                      message: event.message,
                      id: event.id,
                      kind: event.kind,
                      quality: event.quality,
                      reason: event.reason,
                      metadata: updated_metadata,
                      results: event.results,
                    };

                    let result = ProcessResult::new_json(&updated_event);
                    cloned_sender.lock().unwrap().send(result).unwrap();

                    sequence_number.store(sequence_index + 1, Release);
                  }
                }
              }
            } else {
              debug!("receive raw message: {:?}", event);
            }
          })
        };

        pin_mut!(send_to_ws, receive_from_ws);
        future::select(send_to_ws, receive_from_ws).await;
        info!("Ending Authot Live server.");
      };

      let mut runtime = Runtime::new().unwrap();

      runtime.block_on(future);
    }));

    Ok(selected_streams)
  }

  fn process_frames(
    &mut self,
    job_result: JobResult,
    _stream_index: usize,
    process_frames: &[mcai_worker_sdk::prelude::ProcessFrame],
  ) -> Result<ProcessResult> {
    let process_frame: &ProcessFrame = &process_frames[0];
    match &process_frame {
      ProcessFrame::AudioVideo(frame) => unsafe {
        trace!(
          "Frame {} samples, {} channels, {} bytes",
          (*frame.frame).nb_samples,
          (*frame.frame).channels,
          (*frame.frame).linesize[0],
        );

        let size = ((*frame.frame).channels * (*frame.frame).nb_samples * 2) as usize;
        let data = Vec::from_raw_parts((*frame.frame).data[0], size, size);
        let message = Message::binary(data.clone());
        std::mem::forget(data);

        if let Some(audio_source_sender) = &mut self.audio_source_sender {
          let mut sended = false;
          let clock: DateTime<Utc> = Utc::now();
          self.clock_vec.lock().unwrap().push(clock);
          while !sended {
            match audio_source_sender.try_send(message.clone()) {
              Ok(_) => {
                sended = true;
              }
              Err(error) => {
                if error.is_full() {
                  warn!("Buffer is full!");
                  thread::sleep(Duration::from_millis(50));
                }
                if error.is_disconnected() {
                  return Err(MessageError::ProcessingError(job_result));
                }
              }
            }
          }
        }
      },
      _ => {
        return Err(MessageError::RuntimeError(format!(
          "Could not open frame as it was no AudioVideo frame in job {:?}",
          job_result.get_str_job_id()
        )))
      }
    };

    Ok(ProcessResult::empty())
  }

  fn ending_process(&mut self) -> Result<()> {
    if let Some(audio_source_sender) = &mut self.audio_source_sender {
      let data = json!({
        "message": "EndOfStream",
        "last_seq_no": 0
      });

      let message = Message::Text(data.to_string());

      audio_source_sender.try_send(message).unwrap()
    }

    self.ws_thread.take().map(JoinHandle::join);
    Ok(())
  }
}

fn get_first_audio_stream_id(format_context: &FormatContext) -> Result<Vec<StreamDescriptor>> {
  // select first audio stream index
  for stream_index in 0..format_context.get_nb_streams() {
    info!(
      "Stream {:?}, type {:?}",
      stream_index,
      format_context.get_stream_type(stream_index as isize)
    );
    if format_context.get_stream_type(stream_index as isize) == AVMediaType::AVMEDIA_TYPE_AUDIO {
      let channel_layouts = vec!["mono".to_string()];
      let sample_formats = vec!["s16".to_string()];
      let sample_rates = vec![16000];
      let filters = vec![AudioFilter::Format(AudioFormat {
        sample_rates,
        channel_layouts,
        sample_formats,
      })];

      let stream_descriptor = StreamDescriptor::new_audio(stream_index as usize, filters);

      return Ok(vec![stream_descriptor]);
    }
  }

  Err(MessageError::RuntimeError(
    "No such audio stream in the source".to_string(),
  ))
}

fn main() {
  let worker = TranscriptEvent::default();
  start_worker(worker);
}
