mod format;
mod providers;

use format::OutputFormat;
use providers::speechmatics::{websocket_response, websocket_response::WebsocketResponse};

use chrono::{DateTime, Utc};
use futures::channel::mpsc::{Sender, channel};
use futures_util::{StreamExt, future, pin_mut};
use mcai_worker_sdk::{
  MessageError, default_rust_mcai_worker_description,
  job::JobResult,
  prelude::{
    ffmpeg_next::sys::{AV_NOPTS_VALUE, AV_TIME_BASE},
    *,
  },
};
use serde::Deserialize;
use serde_json::json;
use std::{
  convert::TryFrom,
  str::FromStr,
  sync::{
    Arc, Mutex,
    atomic::{
      AtomicUsize,
      Ordering::{Acquire, Release},
    },
  },
  thread::{self, JoinHandle},
  time::Duration,
};
use tokio::runtime::Runtime;
use tokio_tungstenite::tungstenite::protocol::Message;

default_rust_mcai_worker_description!();

#[derive(Debug, Default)]
#[allow(dead_code)]
struct TranscriptWorker {
  sequence_number: u64,
  start_time: Option<f32>,
  audio_source_sender: Option<Sender<Message>>,
  output: Option<Arc<Mutex<DataOutput>>>,
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

impl McaiWorker<WorkerParameters, RustMcaiWorkerDescription> for TranscriptWorker {
  fn init_process(&mut self, process_builder: ProcessBuilder) -> Result<InitProcessReturn> {
    let parameters: WorkerParameters = process_builder.get_job_parameters()?;

    let output = Arc::new(Mutex::new(process_builder.try_new_data_output()?));
    self.output = Some(output.clone());

    let media_source_builder = process_builder.try_new_media_source_builder()?;

    // Store the start time
    self.start_time = {
      let start_time =
        unsafe { (*media_source_builder.input_format_context().as_ptr()).start_time };

      if start_time == AV_NOPTS_VALUE {
        None
      } else {
        Some(start_time as f32 / AV_TIME_BASE as f32)
      }
    };

    let start_offset = self.start_time.unwrap();

    let selected_streams_descriptors = {
      let stream = media_source_builder
        .input_format_context()
        .streams()
        .find(|stream| stream.parameters().medium() == MediaType::Audio)
        .ok_or(MessageError::RuntimeError(
          "No such audio stream in the source".into(),
        ))?;

      info!("Stream {:?}", stream.index());

      let channel_layouts = vec!["mono".into()];
      let sample_formats = vec!["s16".into()];
      let sample_rates = vec![16000];

      let filters = vec![AudioFilter::Format(AudioFormat {
        sample_rates,
        channel_layouts,
        sample_formats,
      })];

      vec![MediaStreamDescriptor::new_audio(stream.index(), filters)]
    };

    let media_source = media_source_builder.try_build(selected_streams_descriptors)?;
    let source = Source::Media(media_source);

    // Specify output format
    let output_format =
      parameters
        .output_format
        .as_ref()
        .map_or(OutputFormat::EbuTtD, |param_output_format| {
          OutputFormat::from_str(param_output_format).expect("Cannot get output format")
        });

    let (audio_source_sender, audio_source_receiver) = channel(10000);
    self.audio_source_sender = Some(audio_source_sender);

    // Spawn a thread listening to the websocket
    self.ws_thread = {
      let output = output.clone();
      let clock_vec = self.clock_vec.clone();
      let start_time = self.start_time;

      Some(thread::spawn(move || {
        let sequence_number = Arc::new(AtomicUsize::new(0));

        let future = async {
          match providers::speechmatics::new(&parameters).await {
            Err(e) => {
              panic!("{}", MessageError::RuntimeError(e.to_string()));
            }
            Ok(ws_stream) => {
              let (ws_sender, ws_receiver) = ws_stream.split();

              let send_to_ws = audio_source_receiver.map(Ok).forward(ws_sender);

              let receive_from_ws = ws_receiver.for_each(|event| async {
                if let Ok(event) = event {
                  debug!("{event}");
                  let event: Result<WebsocketResponse> = WebsocketResponse::try_from(event);

                  match event {
                    Ok(event) => match event.message.as_str() {
                      "AudioAdded" => {
                        debug!("Audio added to websocket");
                      }
                      "EndOfTranscript" => {
                        info!("End of transcript from provider");
                        let _ = output.lock().unwrap().complete();
                      }
                      "AddTranscript" => match output_format {
                        OutputFormat::EbuTtD => {
                          if let Some(mut metadata) = event.metadata {
                            metadata.start_time += start_offset as f64;
                            metadata.end_time += start_offset as f64;
                            let sequence_index = sequence_number.load(Acquire);
                            clock_vec.lock().unwrap().clear();

                            let data_output_frame = DataOutputFrame::new_xml(
                              metadata.generate_ttml(start_time, sequence_index),
                            );
                            output.lock().unwrap().push(data_output_frame);

                            sequence_number.store(sequence_index + 1, Release);
                          }
                        }
                        OutputFormat::Json => {
                          debug!("Received event: {event:?}");
                          let sequence_index = sequence_number.load(Acquire);
                          let updated_metadata = if let Some(metadata) = event.metadata {
                            // Somehow the clock vec can be surprisingly empty !
                            let local_clock_vec = clock_vec.lock().unwrap();
                            let clock: DateTime<Utc> = if local_clock_vec.is_empty() {
                              Utc::now()
                            } else {
                              local_clock_vec[0]
                            };
                            clock_vec.lock().unwrap().clear();
                            info!("Clock {clock}");
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
                            format: event.format,
                            id: event.id,
                            kind: event.kind,
                            quality: event.quality,
                            reason: event.reason,
                            metadata: updated_metadata,
                            results: event.results,
                          };

                          let data_output_frame = DataOutputFrame::new_json(updated_event);
                          output.lock().unwrap().push(data_output_frame);

                          sequence_number.store(sequence_index + 1, Release);
                        }
                      },
                      _ => {}
                    },
                    _ => {
                      debug!("receive raw message: {event:?}");
                    }
                  }
                }
              });

              pin_mut!(send_to_ws, receive_from_ws);
              future::select(send_to_ws, receive_from_ws).await;
              info!("Ending transcription.");
            }
          }
        };

        let mut runtime = Runtime::new().unwrap();

        runtime.block_on(future);
      }))
    };

    Ok(InitProcessReturn { source, output })
  }

  fn process_media_frames(
    &mut self,
    job_result: JobResult,
    _stream_index: usize,
    frames: Vec<MediaProcessFrame>,
  ) -> Result<ProcessResult> {
    for frame in frames {
      if let MediaProcessFrame::Audio(audio_frame) = frame {
        trace!(
          "Frame {} samples, {} channels",
          audio_frame.samples(),
          audio_frame.channels(),
        );

        if let Some(audio_source_sender) = &mut self.audio_source_sender {
          let mut message = Some(Message::binary(audio_frame.data(0)));

          let clock = Utc::now();
          self.clock_vec.lock().unwrap().push(clock);

          while let Err(error) = audio_source_sender.try_send(message.take().unwrap()) {
            if error.is_disconnected() {
              error!("Websocket is disconnected.");
              return Err(MessageError::ProcessingError(Box::new(
                job_result
                  .with_status(JobStatus::Error)
                  .with_message("Websocket is disconnected."),
              )));
            }
            if error.is_full() {
              warn!("Buffer is full!");
              message = Some(error.into_inner());
              thread::sleep(Duration::from_millis(50));
            }
          }
        }
      }
    }

    Ok(ProcessResult::Nothing)
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

fn main() {
  let worker = TranscriptWorker::default();
  start_worker(worker);
}
