mod vag;
mod logging;

extern crate lewton;
#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate slog_async;
extern crate slog_json;
extern crate lazy_static;

use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use lewton::inside_ogg::OggStreamReader;

pub(crate) use lazy_static::lazy_static;
use lewton::VorbisError;
use slog::Logger;

use crate::logging::logging::initialize_logging;
use crate::vag::encoder::VAGEncoder;

lazy_static! {
    static ref LOGGER: Logger = initialize_logging(String::from("ogg-vag-transcoder"));
}

fn main() {
    info!(&crate::LOGGER, "Configured Logging");
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        crit!(&crate::LOGGER, "Usage: ogg-vag-transcoder <input ogg path> <output vag path>");
        return;
    }
    let ogg_file: File = match File::open(&args[0]) {
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
    if ogg_stream.ident_hdr.audio_channels > 2 {
        crit!(&crate::LOGGER, "Cannot process more than 2 channels");
        return;
    }
    info!(&crate::LOGGER, "Input sample rate: {sample_rate}");
    let vag_file: File = match File::open(&args[1]) {
        Ok(f) => f,
        Err(e) => {
            crit!(&crate::LOGGER, "Unable to create vag file: {e}");
            return;
        }
    };
    let mut vag_writer: BufWriter<File> = BufWriter::new(vag_file);
    info!(&crate::LOGGER, "Created output writer for VAG stream");
    let mut vag_encoder: VAGEncoder = VAGEncoder::default();
    let mut chunk_count = 0;
    while let Some(samples) = ogg_stream.read_dec_packet_itl()
        .unwrap_or_else(|e: VorbisError| -> Option<Vec<i16>> {
            error!(&crate::LOGGER, "Failed to read + decode interleaved packet {e}");
            None
        }) {
        match vag_encoder.encode_chunk(&samples, false, 0, 0, &mut vag_writer) {
            Ok(_) => {},
            Err(e) => {
                error!(&crate::LOGGER, "Failed to encode chunk {chunk_count}: {e}");
                return;
            }
        };
        chunk_count += 1;
    }
    match vag_encoder.encode_ending(false, &mut vag_writer) {
        Ok(_) => {},
        Err(e) => {
            error!(&crate::LOGGER, "Failed to encoding ending of {chunk_count} chunks: {e}");
        }
    }
    info!(&crate::LOGGER, "Finished encoding {chunk_count} chunks, flushing writer");
    match vag_writer.flush() {
        Ok(_) => {},
        Err(e) => crit!(&crate::LOGGER, "Failed to flush VAG buffer: {e}")
    }
}

