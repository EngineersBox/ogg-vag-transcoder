use std::io::{BufWriter, Write, Result};

use super::vag::{VAGChunk, VAGFlag, VAG_SAMPLE_BYTES, VAG_SAMPLE_NIBBLE};

#[derive(Default)]
pub struct VAGEncoder {
    last_predict_and_shift: u8
}

const VAG_LUT_ENCODINGS: [[f64; 2]; 5] = [
    [0.0, 0.0],
    [-64.0 / 64.0, 0.0],
    [-115.0 / 64.0, 52.0 / 64.0],
    [-98.0 / 64.0, 55.0 / 64.0],
    [-122.0 / 64.0, 60.0 / 64.0]
];

impl VAGEncoder {
    pub fn encode_chunk<T: Write>(&mut self, samples: &Vec<i16>, loop_flag: bool, loop_start: u32, loop_end: u32, writer: &mut BufWriter<T>) -> Result<()> {
        let mut _hist_1: f64 = 0.0;
        let mut hist_1: f64 = 0.0;
        let mut _hist_2: f64 = 0.0;
        let mut hist_2: f64 = 0.0;
        let full_chunks: usize = samples.len() / VAG_SAMPLE_NIBBLE;
        let mut exit_next: bool = false;
        for (iter, pos) in (0..samples.len()).step_by(VAG_SAMPLE_NIBBLE).enumerate() {
            if exit_next {
                break;
            }
            let mut buf: [i16; VAG_SAMPLE_NIBBLE] = [0; VAG_SAMPLE_NIBBLE];
            if iter < full_chunks {
                buf.copy_from_slice(&samples[pos..pos + VAG_SAMPLE_NIBBLE]);
            } else {
                let remaining = samples.len() - pos;
                buf.copy_from_slice(&samples[pos..pos + remaining]);
            }
            let mut chunk: VAGChunk = VAGChunk::default();
            let mut predict = 0;
            let mut shift = 0;
            let mut min: f64 = 1e10;
            let mut sample_1: f64 = 0.0;
            let mut sample_2: f64 = 0.0;
            let mut predict_buf: [[f64; 28]; 5] = [[0.0; 28]; 5];
            for (j, encoding) in VAG_LUT_ENCODINGS.iter().enumerate() {
                let mut max: f64 = 0.0;
                sample_1 = _hist_1;
                sample_2 = _hist_2;
                for k in 0..VAG_SAMPLE_NIBBLE {
                    let mut sample: f64 = buf[k] as f64;
                    sample = f64::min(f64::max(sample, -30720.0), 30719.0);
                    let ds: f64 = sample + (sample_1 * encoding[0]) + (sample_2 * encoding[1]);
                    predict_buf[k][j] = ds;
                    max = f64::max(max, f64::abs(ds));
                    sample_2 = sample_1;
                    sample_1 = sample;
                }
                if max < min {
                    min = max;
                    predict = j;
                }
                if min <= 7.0 {
                    predict = 0;
                    break;
                }
            }
            _hist_1 = sample_1;
            _hist_2 = sample_2;
            let mut d_samples: [f64; 28] = [0f64; 28];
            for i in 0..28usize {
                d_samples[i] = predict_buf[i][predict];
            }
            let min2: i32 = min as i32;
            let mut shift_mask = 0x4000;
            shift = 0;
            while shift < 12 {
                if shift_mask & (min2 + (shift_mask >> 3)) != 0 {
                    break;
                }
                shift += 1;
                shift_mask >>= 1;
            }
            chunk.predict = predict as i8;
            chunk.shift = shift;
            // flags
            if samples.len() - pos > 28 {
                chunk.flags = VAGFlag::Nothing as u8;
                if loop_flag {
                    chunk.flags = VAGFlag::LoopRegion as u8;
                    if iter as u32 == loop_start {
                        chunk.flags = VAGFlag::LoopStart as u8;
                    } else if iter as u32 == loop_end {
                        chunk.flags = VAGFlag::LoopEnd as u8;
                        exit_next = true;
                    }
                }
                
            } else {
                chunk.flags = VAGFlag::LoopLastBlock as u8;
                if loop_flag {
                    chunk.flags = VAGFlag::LoopEnd as u8;
                }
            }
            let mut out_buf: [i16; VAG_SAMPLE_NIBBLE] = [0; VAG_SAMPLE_NIBBLE];
            let encoding: &[f64; 2] = &VAG_LUT_ENCODINGS[predict];
            for k in 0..VAG_SAMPLE_NIBBLE {
                let sample_double_trans: f64 = d_samples[k] + (hist_1 * encoding[0]) + (hist_2 * encoding[1]);
                let sample_double: f64 = sample_double_trans * (1 << shift) as f64;
                let mut current_sample = ((sample_double as i64 + 0x800) & 0xFFFFF000) as i32;
                current_sample = i32::min(i32::max(current_sample, i16::MIN as i32), i16::MAX as i32);
                out_buf[k] = current_sample as i16;
                current_sample >>= shift;
                hist_2 = hist_1;
                hist_1 = current_sample as f64 - sample_double_trans;
            }
            for k in 0..VAG_SAMPLE_BYTES {
                chunk.sample[k] = (((out_buf[(k * 2) + 1] >> 8) & 0xF0) | ((out_buf[k * 2] >> 12) & 0xF)) as u8;
            }
            self.last_predict_and_shift = ((((chunk.predict as i16) << 4) & 0xF0) | (chunk.shift as i16 & 0x0F)) as u8;
            writer.write_all(&self.last_predict_and_shift.to_ne_bytes())?;
            writer.write_all(&chunk.flags.to_ne_bytes())?;
            writer.write_all(&chunk.sample)?;
        }
        Ok(())
    }

    pub fn encode_ending<T: Write>(&self, loop_flag: bool, writer: &mut BufWriter<T>) -> Result<()> {
        if !loop_flag {
            writer.write_all(&self.last_predict_and_shift.to_ne_bytes())?;
            writer.write_all(&(VAGFlag::PlaybackEnd as u8).to_ne_bytes())?;
            writer.write_all(&[0; VAG_SAMPLE_BYTES])?;
        }
        Ok(())
    }

}
