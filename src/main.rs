mod vag;
mod logging;

extern crate lewton;
#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate slog_async;
extern crate slog_json;
extern crate lazy_static;
extern crate indicatif;

use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use lewton::inside_ogg::OggStreamReader;

pub(crate) use lazy_static::lazy_static;
use lewton::VorbisError;
use slog::Logger;
use indicatif::ProgressBar;

use crate::logging::logging::initialize_logging;
use crate::vag::encoder::VAGEncoder;

lazy_static! {
    static ref LOGGER: Logger = initialize_logging(String::from("ogg-vag-transcoder"));
}

fn run() {
    info!(&crate::LOGGER, "Configured Logging");
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        crit!(&crate::LOGGER, "Usage: ogg-vag-transcoder <input ogg path> <output vag path>");
        return;
    }
    let ogg_file: File = match File::open(&args[1]) {
        Ok(f) => f,
        Err(e) => {
            crit!(&crate::LOGGER, "Unable to read ogg file: {e}");
            return;
        }
    };
    let ogg_file_reader: BufReader<File> = BufReader::new(ogg_file);
    let mut ogg_stream: OggStreamReader<BufReader<File>> = match OggStreamReader::new(ogg_file_reader) {
        Ok(s) => s,
        Err(e) => {
            crit!(&crate::LOGGER, "Unable to create ogg stream from file: {e}");
            return;
        }
    };
    info!(&crate::LOGGER, "Opened OGG input file as stream");
    let sample_rate: i32 = ogg_stream.ident_hdr.audio_sample_rate as i32;
    let channels = ogg_stream.ident_hdr.audio_channels;
    if channels > 2 {
        crit!(&crate::LOGGER, "Cannot process more than 2 channels");
        return;
    }
    debug!(&crate::LOGGER, "[OGG IDENT] Sample rate: {sample_rate}Hz");
    debug!(&crate::LOGGER, "[OGG IDENT] Audio channels: {channels}");
    debug!(&crate::LOGGER, "[OGG IDENT] Blocksizes [0: {}] [1: {}]", ogg_stream.ident_hdr.blocksize_0, ogg_stream.ident_hdr.blocksize_1);
    ogg_stream.comment_hdr.comment_list.iter().for_each(|elem: &(String, String)|
        debug!(&crate::LOGGER, "[OGG COMMENT] {}: {}", elem.0, elem.1)
    );
    let vag_file: File = match OpenOptions::new()
                                            .write(true)
                                            .create(true)
                                            .open(&args[2]) {
        Ok(f) => f,
        Err(e) => {
            crit!(&crate::LOGGER, "Unable to create vag file: {e}");
            return;
        }
    };
    let mut vag_writer: BufWriter<File> = BufWriter::new(vag_file);
    info!(&crate::LOGGER, "Created output writer for VAG stream");
    let mut vag_encoder: VAGEncoder = VAGEncoder::default();
    let mut chunk_count: usize = 0;
    let mut source_sample_count: usize = 0;
    let mut dest_sample_bytes: usize = 0;
    let bar: ProgressBar = ProgressBar::new_spinner();
    while let Some(samples) = ogg_stream.read_dec_packet_itl()
        .unwrap_or_else(|e: VorbisError| -> Option<Vec<i16>> {
            error!(&crate::LOGGER, "Failed to read + decode interleaved packet [Chunk: {chunk_count}] {e}");
            None
        }) {
        match vag_encoder.encode_chunk(&samples, false, 0, 0, &mut vag_writer) {
            Ok(bytes) => {
                dest_sample_bytes += bytes;
                bar.set_message(format!("Chunks: {chunk_count} OGG Samples: {source_sample_count} VAG Bytes: {dest_sample_bytes}"));
                bar.tick();
            },
            Err(e) => {
                error!(&crate::LOGGER, "Failed to encode chunk {chunk_count}: {e}");
                bar.abandon();
                return;
            }
        };
        source_sample_count += samples.len();
        chunk_count += 1;
    }
    match vag_encoder.encode_ending(false, &mut vag_writer) {
        Ok(bytes) => {
            dest_sample_bytes += bytes;
            bar.set_message(format!("Chunks: {chunk_count} OGG Samples: {source_sample_count} VAG Bytes: {dest_sample_bytes}"));
            bar.tick();
        },
        Err(e) => {
            error!(&crate::LOGGER, "Failed to encoding ending of {chunk_count} chunks: {e}");
        }
    }
    bar.finish();
    info!(&crate::LOGGER, "Finished encoding {chunk_count} chunks, flushing writer");
    match vag_writer.flush() {
        Ok(_) => {},
        Err(e) => crit!(&crate::LOGGER, "Failed to flush VAG buffer: {e}")
    }
}

fn main() {
    run();
    // Hacky stuff cause I can't be bothered making the logger
    // flush all at exit.
    std::thread::sleep(std::time::Duration::from_millis(1000));
}
