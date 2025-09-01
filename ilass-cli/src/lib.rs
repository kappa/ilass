use encoding_rs::Encoding;
use failure::ResultExt;
use ilass::{TimeDelta as AlgTimeDelta, TimePoint as AlgTimePoint, TimeSpan as AlgTimeSpan};
use pbr::ProgressBar;
use std::cmp::{max, min};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::result::Result;

use errors::*;

pub mod errors;
pub mod video_decoder;

use subparse::timetypes::*;
use subparse::{SubtitleFile, get_subtitle_format_err, parse_bytes};

pub struct NoProgressInfo {}

impl ilass::ProgressHandler for NoProgressInfo {
    fn init(&mut self, _steps: i64) {}
    fn inc(&mut self) {}
    fn finish(&mut self) {}
}

impl video_decoder::ProgressHandler for NoProgressInfo {
    fn init(&mut self, _steps: i64) {}
    fn inc(&mut self) {}
    fn finish(&mut self) {}
}

pub struct ProgressInfo {
    init_msg: Option<String>,
    prescaler: i64,
    counter: i64,
    progress_bar: Option<ProgressBar<std::io::Stdout>>,
}

impl ProgressInfo {
    pub fn new(prescaler: i64, init_msg: Option<String>) -> ProgressInfo {
        ProgressInfo {
            init_msg,
            prescaler,
            counter: 0,
            progress_bar: None,
        }
    }
}

impl ProgressInfo {
    fn init(&mut self, steps: i64) {
        self.progress_bar = Some(ProgressBar::new((steps / self.prescaler) as u64));
        if let Some(init_msg) = &self.init_msg {
            println!("{}", init_msg);
        }
    }

    fn inc(&mut self) {
        self.counter += 1;
        if self.counter == self.prescaler {
            self.progress_bar.as_mut().unwrap().inc();
            self.counter = 0;
        }
    }

    fn finish(&mut self) {
        self.progress_bar.as_mut().unwrap().finish_println("\n");
    }
}

impl ilass::ProgressHandler for ProgressInfo {
    fn init(&mut self, steps: i64) {
        self.init(steps)
    }
    fn inc(&mut self) {
        self.inc()
    }
    fn finish(&mut self) {
        self.finish()
    }
}

impl video_decoder::ProgressHandler for ProgressInfo {
    fn init(&mut self, steps: i64) {
        self.init(steps)
    }
    fn inc(&mut self) {
        self.inc()
    }
    fn finish(&mut self) {
        self.finish()
    }
}

pub fn read_file_to_bytes(path: &Path) -> Result<Vec<u8>, FileOperationError> {
    let mut file = File::open(path).with_context(|_| FileOperationErrorKind::FileOpen {
        path: path.to_path_buf(),
    })?;
    let mut v = Vec::new();
    file.read_to_end(&mut v)
        .with_context(|_| FileOperationErrorKind::FileRead {
            path: path.to_path_buf(),
        })?;
    Ok(v)
}

pub fn write_data_to_file(path: &Path, d: Vec<u8>) -> Result<(), FileOperationError> {
    let mut file = File::create(path).with_context(|_| FileOperationErrorKind::FileOpen {
        path: path.to_path_buf(),
    })?;
    file.write_all(&d).with_context(|_| FileOperationErrorKind::FileWrite {
        path: path.to_path_buf(),
    })?;
    Ok(())
}

pub fn timing_to_alg_timepoint(t: TimePoint, interval: i64) -> AlgTimePoint {
    assert!(interval > 0);
    AlgTimePoint::from(t.msecs() / interval)
}

pub fn alg_delta_to_delta(t: AlgTimeDelta, interval: i64) -> TimeDelta {
    assert!(interval > 0);
    let time_int: i64 = t.into();
    TimeDelta::from_msecs(time_int * interval)
}

pub fn timings_to_alg_timespans(v: &[TimeSpan], interval: i64) -> Vec<AlgTimeSpan> {
    v.iter()
        .cloned()
        .map(|timespan| {
            AlgTimeSpan::new_safe(
                timing_to_alg_timepoint(timespan.start, interval),
                timing_to_alg_timepoint(timespan.end, interval),
            )
        })
        .collect()
}

pub fn alg_deltas_to_timing_deltas(v: &[AlgTimeDelta], interval: i64) -> Vec<TimeDelta> {
    v.iter().cloned().map(|x| alg_delta_to_delta(x, interval)).collect()
}

/// Groups consecutive timespans with the same delta together.
pub fn get_subtitle_delta_groups(mut v: Vec<(AlgTimeDelta, TimeSpan)>) -> Vec<(AlgTimeDelta, Vec<TimeSpan>)> {
    v.sort_by_key(|t| min((t.1).start, (t.1).end));

    let mut result: Vec<(AlgTimeDelta, Vec<TimeSpan>)> = Vec::new();

    for (delta, original_timespan) in v {
        let mut new_block = false;

        if let Some(last_tuple_ref) = result.last_mut() {
            if delta == last_tuple_ref.0 {
                last_tuple_ref.1.push(original_timespan);
            } else {
                new_block = true;
            }
        } else {
            new_block = true;
        }

        if new_block {
            result.push((delta, vec![original_timespan]));
        }
    }

    result
}

pub enum InputFileHandler {
    Subtitle(SubtitleFileHandler),
    Video(VideoFileHandler),
}

pub struct SubtitleFileHandler {
    file_format: subparse::SubtitleFormat,
    subtitle_file: SubtitleFile,
    subparse_timespans: Vec<TimeSpan>,
}

impl SubtitleFileHandler {
    pub fn open_sub_file(
        file_path: &Path,
        sub_encoding: Option<&'static Encoding>,
        sub_fps: f64,
    ) -> Result<SubtitleFileHandler, InputSubtitleError> {
        let sub_data = read_file_to_bytes(file_path)
            .with_context(|_| InputSubtitleErrorKind::ReadingSubtitleFileFailed(file_path.to_path_buf()))?;

        let file_format = get_subtitle_format_err(file_path.extension(), &sub_data)
            .with_context(|_| InputSubtitleErrorKind::UnknownSubtitleFormat(file_path.to_path_buf()))?;

        let parsed_subtitle_data: SubtitleFile = parse_bytes(file_format, &sub_data, sub_encoding, sub_fps)
            .with_context(|_| InputSubtitleErrorKind::ParsingSubtitleFailed(file_path.to_path_buf()))?;

        let subparse_timespans: Vec<TimeSpan> = parsed_subtitle_data
            .get_subtitle_entries()
            .with_context(|_| InputSubtitleErrorKind::RetrievingSubtitleLinesFailed(file_path.to_path_buf()))?
            .into_iter()
            .map(|subentry| subentry.timespan)
            .map(|timespan: TimeSpan| {
                TimeSpan::new(min(timespan.start, timespan.end), max(timespan.start, timespan.end))
            })
            .collect();

        Ok(SubtitleFileHandler {
            file_format,
            subparse_timespans,
            subtitle_file: parsed_subtitle_data,
        })
    }

    pub fn file_format(&self) -> subparse::SubtitleFormat {
        self.file_format
    }

    pub fn timespans(&self) -> &[TimeSpan] {
        self.subparse_timespans.as_slice()
    }

    pub fn into_subtitle_file(self) -> subparse::SubtitleFile {
        self.subtitle_file
    }
}

pub struct VideoFileHandler {
    //video_file_format: VideoFileFormat,
    subparse_timespans: Vec<TimeSpan>,
    //aligner_timespans: Vec<ilass::TimeSpan>,
}

impl VideoFileHandler {
    pub fn from_cache(timespans: Vec<TimeSpan>) -> VideoFileHandler {
        VideoFileHandler {
            subparse_timespans: timespans,
        }
    }

    pub fn open_video_file(
        file_path: &Path,
        audio_index: Option<usize>,
        video_decode_progress: impl video_decoder::ProgressHandler,
    ) -> Result<VideoFileHandler, InputVideoError> {
        //video_decoder::VideoDecoder::decode(file_path, );
        use voice_activity_detector::VoiceActivityDetector;

        struct SileroVad {
            detector: VoiceActivityDetector,
            vad_buffer: Vec<bool>,
            sample_buffer: Vec<i16>,
        }

        impl video_decoder::AudioReceiver for SileroVad {
            type Output = Vec<bool>;
            type Error = InputVideoError;

            fn push_samples(&mut self, samples: &[i16]) -> Result<(), InputVideoError> {
                // Accumulate i16 samples
                self.sample_buffer.extend_from_slice(samples);

                // Silero expects 512 samples (32ms at 16kHz), process when we have enough
                while self.sample_buffer.len() >= 512 {
                    let chunk: Vec<i16> = self.sample_buffer.drain(..512).collect();

                    let speech_prob = self.detector.predict(chunk);

                    // Use 0.5 threshold for speech detection
                    let is_voice = speech_prob > 0.5;
                    self.vad_buffer.push(is_voice);
                }

                Ok(())
            }

            fn finish(mut self) -> Result<Vec<bool>, InputVideoError> {
                // Process any remaining samples
                if !self.sample_buffer.is_empty() {
                    // Pad to 512 samples if needed
                    self.sample_buffer.resize(512, 0);
                    let speech_prob = self.detector.predict(self.sample_buffer);
                    self.vad_buffer.push(speech_prob > 0.5);
                }
                Ok(self.vad_buffer)
            }
        }

        let vad_processor = SileroVad {
            detector: VoiceActivityDetector::builder()
                .sample_rate(16000)
                .chunk_size(512usize)
                .build()
                .map_err(|_| InputVideoErrorKind::VadAnalysisFailed)?,
            vad_buffer: Vec::new(),
            sample_buffer: Vec::new(),
        };

        let chunk_processor = video_decoder::ChunkedAudioReceiver::new(160, vad_processor);

        let vad_buffer =
            video_decoder::VideoDecoder::decode(file_path, audio_index, chunk_processor, video_decode_progress)
                .with_context(|_| InputVideoErrorKind::FailedToDecode {
                    path: PathBuf::from(file_path),
                })?;

        let mut voice_segments: Vec<(i64, i64)> = Vec::new();
        let mut voice_segment_start = 0;

        let combine_with_distance_lower_than = 0;

        let mut last_segment_end = 0;
        let mut already_saved_span = true;

        for (i, is_voice_segment) in vad_buffer.into_iter().chain(std::iter::once(false)).enumerate() {
            let i = i as i64;

            if is_voice_segment {
                last_segment_end = i;
                if already_saved_span {
                    voice_segment_start = i;
                    already_saved_span = false;
                }
            } else {
                // not a voice segment
                if i - last_segment_end >= combine_with_distance_lower_than && !already_saved_span {
                    voice_segments.push((voice_segment_start, last_segment_end));
                    already_saved_span = true;
                }
            }
        }

        let subparse_timespans: Vec<TimeSpan> = voice_segments
            .into_iter()
            .map(|(start, end)| TimeSpan::new(TimePoint::from_msecs(start * 32), TimePoint::from_msecs(end * 32)))
            .collect();

        Ok(VideoFileHandler {
            //video_file_format: VideoFileFormat::NotImplemented,
            subparse_timespans,
        })
    }

    pub fn filter_with_min_span_length_ms(&mut self, min_vad_span_length_ms: i64) {
        self.subparse_timespans = self
            .subparse_timespans
            .iter()
            .filter(|ts| ts.len() >= TimeDelta::from_msecs(min_vad_span_length_ms))
            .cloned()
            .collect();
    }

    pub fn timespans(&self) -> &[TimeSpan] {
        self.subparse_timespans.as_slice()
    }
}

impl InputFileHandler {
    pub fn open(
        file_path: &Path,
        audio_index: Option<usize>,
        sub_encoding: Option<&'static Encoding>,
        sub_fps: f64,
        video_decode_progress: impl video_decoder::ProgressHandler,
    ) -> Result<InputFileHandler, InputFileError> {
        if let Some(extension) = file_path.extension().map(|os_str| os_str.to_string_lossy()) {
            let known_extensions = ["srt", "vob", "idx", "ass", "ssa", "sub"];
            if known_extensions.contains(&extension.as_ref()) {
                return Ok(SubtitleFileHandler::open_sub_file(file_path, sub_encoding, sub_fps)
                    .map(InputFileHandler::Subtitle)
                    .with_context(|_| InputFileErrorKind::SubtitleFile(file_path.to_path_buf()))?);
            }
        }

        // Did not match any subtitle extensions we support, assume it's a video file.
        Ok(
            VideoFileHandler::open_video_file(file_path, audio_index, video_decode_progress)
                .map(InputFileHandler::Video)
                .with_context(|_| InputFileErrorKind::VideoFile(file_path.to_path_buf()))?,
        )
    }

    pub fn into_subtitle_file(self) -> Option<SubtitleFile> {
        match self {
            InputFileHandler::Video(_) => None,
            InputFileHandler::Subtitle(sub_handler) => Some(sub_handler.subtitle_file),
        }
    }

    pub fn timespans(&self) -> &[TimeSpan] {
        match self {
            InputFileHandler::Video(video_handler) => video_handler.timespans(),
            InputFileHandler::Subtitle(sub_handler) => sub_handler.timespans(),
        }
    }

    pub fn filter_video_with_min_span_length_ms(&mut self, min_vad_span_length_ms: i64) {
        if let InputFileHandler::Video(video_handler) = self {
            video_handler.filter_with_min_span_length_ms(min_vad_span_length_ms);
        }
    }
}

pub fn guess_fps_ratio(
    ref_spans: &[ilass::TimeSpan],
    in_spans: &[ilass::TimeSpan],
    ratios: &[f64],
    mut progress_handler: impl ilass::ProgressHandler,
) -> (Option<usize>, ilass::TimeDelta) {
    progress_handler.init(ratios.len() as i64);
    let (delta, score) = ilass::align_nosplit(ref_spans, in_spans, ilass::overlap_scoring, ilass::NoProgressHandler);
    progress_handler.inc();

    //let desc = ["25/24", "25/23.976", "24/25", "24/23.976", "23.976/25", "23.976/24"];
    //println!("score 1: {}", score);

    let (mut opt_idx, mut opt_delta, mut opt_score) = (None, delta, score);

    for (scale_factor_idx, scaling_factor) in ratios.iter().cloned().enumerate() {
        let stretched_in_spans: Vec<ilass::TimeSpan> = in_spans.iter().map(|ts| ts.scaled(scaling_factor)).collect();

        let (delta, score) = ilass::align_nosplit(
            ref_spans,
            &stretched_in_spans,
            ilass::overlap_scoring,
            ilass::NoProgressHandler,
        );
        progress_handler.inc();

        //println!("score {}: {}", desc[scale_factor_idx], score);

        if score > opt_score {
            opt_score = score;
            opt_idx = Some(scale_factor_idx);
            opt_delta = delta;
        }
    }

    progress_handler.finish();

    (opt_idx, opt_delta)
}

pub fn print_error_chain(error: failure::Error) {
    let show_bt_opt = std::env::vars()
        .find(|(key, _)| key == "RUST_BACKTRACE")
        .map(|(_, value)| value);
    let show_bt = show_bt_opt.is_some() && show_bt_opt != Some("0".to_string());

    println!("error: {}", error);
    if show_bt {
        println!("stack trace: {}", error.backtrace());
    }

    for cause in error.as_fail().iter_causes() {
        println!("caused by: {}", cause);
        if show_bt && let Some(backtrace) = cause.backtrace() {
            println!("stack trace: {}", backtrace);
        }
    }

    if !show_bt {
        println!();
        println!("not: run with environment variable 'RUST_BACKTRACE=1' for detailed stack traces");
    }
}
