//! Bounded, seek-based SEG-Y preview. Original samples stay in the immutable file.
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub const MAX_TRACES: usize = 250_000;
pub const MAX_SLICE_EDGE: usize = 512;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trace {
    pub inline: i32,
    pub crossline: i32,
    pub offset: u64,
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Index {
    pub schema: String,
    pub bytes: u64,
    pub sample_format: u16,
    pub samples: usize,
    pub interval_us: u16,
    pub delay_ms: i16,
    pub inlines: Vec<i32>,
    pub crosslines: Vec<i32>,
    pub traces: Vec<Trace>,
    pub warnings: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Slice {
    pub axis: String,
    pub position: i32,
    pub width: usize,
    pub height: usize,
    pub horizontal: Vec<i32>,
    pub vertical: Vec<i32>,
    pub amplitudes: Vec<Option<f32>>,
    pub min: f32,
    pub max: f32,
    pub samples_read: usize,
    pub sample_stride: usize,
    pub domain: String,
    pub interval_us: u16,
    pub delay_ms: i16,
}
fn u16be(b: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([b[o], b[o + 1]])
}
fn i16be(b: &[u8], o: usize) -> i16 {
    i16::from_be_bytes([b[o], b[o + 1]])
}
fn i32be(b: &[u8], o: usize) -> i32 {
    i32::from_be_bytes(b[o..o + 4].try_into().unwrap())
}
fn width(format: u16) -> Result<usize> {
    match format{1|2|5=>Ok(4),3=>Ok(2),8=>Ok(1),_=>bail!("Unsupported SEG-Y sample format {format}; supported: IBM32, IEEE32, int32, int16, int8 (big-endian)")}
}
fn amplitude(bytes: &[u8], format: u16) -> f32 {
    match format {
        1 => {
            let v = u32::from_be_bytes(bytes.try_into().unwrap());
            if v & 0x7fffffff == 0 {
                return 0.;
            }
            let sign = if v >> 31 == 0 { 1. } else { -1. };
            let exponent = ((v >> 24) & 127) as i32 - 64;
            (sign * (v & 0xffffff) as f64 / 16777216. * 16f64.powi(exponent)) as f32
        }
        2 => i32::from_be_bytes(bytes.try_into().unwrap()) as f32,
        3 => i16::from_be_bytes(bytes.try_into().unwrap()) as f32,
        5 => f32::from_be_bytes(bytes.try_into().unwrap()),
        8 => bytes[0] as i8 as f32,
        _ => unreachable!(),
    }
}

pub fn index_file(path: &Path) -> Result<Index> {
    let mut file = File::open(path)?;
    let bytes = file.metadata()?.len();
    ensure!(
        bytes >= 3600,
        "SEG-Y is shorter than its textual and binary headers"
    );
    file.seek(SeekFrom::Start(3200))?;
    let mut header = [0u8; 400];
    file.read_exact(&mut header)?;
    let sample_format = u16be(&header, 24);
    let size = width(sample_format)?;
    let samples = u16be(&header, 20) as usize;
    let interval_us = u16be(&header, 16);
    ensure!(
        samples > 0 && interval_us > 0,
        "SEG-Y requires positive fixed sample count and interval"
    );
    ensure!(
        header[300] <= 1,
        "SEG-Y revision 2+ is not yet supported; convert explicitly to revision 0/1"
    );
    let extended = i16be(&header, 304);
    ensure!(
        extended >= 0,
        "Variable-count extended textual headers are unsupported"
    );
    let mut offset = 3600 + (extended as u64) * 3200;
    ensure!(offset <= bytes, "Truncated extended textual headers");
    let mut traces = Vec::new();
    let mut keys = BTreeSet::new();
    let mut inlines = BTreeSet::new();
    let mut crosslines = BTreeSet::new();
    let mut delay_ms = None;
    while offset < bytes {
        ensure!(
            traces.len() < MAX_TRACES,
            "SEG-Y trace index exceeds the bounded 250,000-trace limit"
        );
        ensure!(offset + 240 <= bytes, "Truncated SEG-Y trace header");
        file.seek(SeekFrom::Start(offset))?;
        let mut h = [0u8; 240];
        file.read_exact(&mut h)?;
        let ns = u16be(&h, 114) as usize;
        let dt = u16be(&h, 116);
        ensure!(
            (ns == 0 || ns == samples) && (dt == 0 || dt == interval_us),
            "Variable sample count or interval is unsupported"
        );
        let delay = i16be(&h, 108);
        if let Some(first) = delay_ms {
            ensure!(first == delay, "Variable recording delay is unsupported");
        } else {
            delay_ms = Some(delay);
        }
        let inline = i32be(&h, 188);
        let crossline = i32be(&h, 192);
        ensure!(keys.insert((inline,crossline)),"Duplicate inline/crossline traces: prestack/offset gathers require explicit stacking and are unsupported");
        let scalar = i16be(&h, 70);
        let factor = if scalar < 0 {
            1. / (-(scalar as f64))
        } else if scalar > 0 {
            scalar as f64
        } else {
            1.
        };
        let x = i32be(&h, 180) as f64 * factor;
        let y = i32be(&h, 184) as f64 * factor;
        let data = offset + 240;
        let end = data + samples as u64 * size as u64;
        ensure!(end <= bytes, "Truncated SEG-Y trace samples");
        traces.push(Trace {
            inline,
            crossline,
            offset: data,
            x,
            y,
        });
        inlines.insert(inline);
        crosslines.insert(crossline);
        offset = end;
    }
    ensure!(!traces.is_empty(), "SEG-Y has no traces");
    Ok(Index{schema:"3dmk.segy-index/1".into(),bytes,sample_format,samples,interval_us,delay_ms:delay_ms.unwrap_or(0),inlines:inlines.into_iter().collect(),crosslines:crosslines.into_iter().collect(),traces,warnings:vec!["CRS, horizontal coordinate units and vertical domain are not inferred from headers.".into(),"Time-domain seismic cannot be depth-aligned with wells without an explicit velocity/time-depth model.".into(),"SEG-D, VOXL, prestack gathers, variable-length traces and SEG-Y revision 2+ are unsupported.".into()]})
}

pub fn read_slice(
    path: &Path,
    index: &Index,
    axis: &str,
    position: i32,
    lod: usize,
) -> Result<Slice> {
    ensure!(
        ["inline", "crossline", "sample"].contains(&axis),
        "Slice axis must be inline, crossline or sample"
    );
    let lod = lod.clamp(1, 128);
    let is_sample = axis == "sample";
    if is_sample {
        ensure!(
            position >= 0 && (position as usize) < index.samples,
            "Sample position is outside the volume"
        );
    } else if axis == "inline" {
        ensure!(index.inlines.contains(&position), "Inline is not indexed");
    } else {
        ensure!(
            index.crosslines.contains(&position),
            "Crossline is not indexed"
        );
    }
    let horizontal_source = if axis == "crossline" {
        &index.inlines
    } else {
        &index.crosslines
    };
    let hstep = lod.max(horizontal_source.len().div_ceil(MAX_SLICE_EDGE));
    let horizontal: Vec<i32> = horizontal_source.iter().step_by(hstep).copied().collect();
    let vlen = if is_sample {
        index.inlines.len()
    } else {
        index.samples
    };
    let vstep = lod.max(vlen.div_ceil(MAX_SLICE_EDGE));
    let vertical: Vec<i32> = if is_sample {
        index.inlines.iter().step_by(vstep).copied().collect()
    } else {
        (0..index.samples)
            .step_by(vstep)
            .map(|v| v as i32)
            .collect()
    };
    let (w, h) = (horizontal.len(), vertical.len());
    ensure!(
        w * h <= MAX_SLICE_EDGE * MAX_SLICE_EDGE,
        "Slice exceeds display budget"
    );
    let lookup = index
        .traces
        .iter()
        .map(|t| ((t.inline, t.crossline), t))
        .collect::<BTreeMap<_, _>>();
    let mut file = File::open(path)?;
    ensure!(
        file.metadata()?.len() == index.bytes,
        "SEG-Y changed after indexing"
    );
    let bytes_per = width(index.sample_format)?;
    let mut values = vec![None; w * h];
    let mut samples_read = 0;
    let mut minimum = f32::INFINITY;
    let mut maximum = f32::NEG_INFINITY;
    let mut record = vec![
        0u8;
        if is_sample {
            bytes_per
        } else {
            index.samples * bytes_per
        }
    ];
    for (column, &horizontal_value) in horizontal.iter().enumerate() {
        let rows: Vec<(usize, i32)> = if is_sample {
            vertical.iter().copied().enumerate().collect()
        } else {
            vec![(0, position)]
        };
        for (row, other) in rows {
            let key = if axis == "crossline" {
                (horizontal_value, position)
            } else {
                (other, horizontal_value)
            };
            let Some(trace) = lookup.get(&key) else {
                continue;
            };
            file.seek(SeekFrom::Start(
                trace.offset
                    + if is_sample {
                        position as u64 * bytes_per as u64
                    } else {
                        0
                    },
            ))?;
            file.read_exact(&mut record)
                .context("Cannot read indexed SEG-Y samples")?;
            samples_read += record.len() / bytes_per;
            let targets: Vec<(usize, usize)> = if is_sample {
                vec![(row, 0)]
            } else {
                vertical
                    .iter()
                    .enumerate()
                    .map(|(r, &sample)| (r, sample as usize))
                    .collect()
            };
            for (r, sample) in targets {
                let value = amplitude(
                    &record[sample * bytes_per..(sample + 1) * bytes_per],
                    index.sample_format,
                );
                if value.is_finite() {
                    values[r * w + column] = Some(value);
                    minimum = minimum.min(value);
                    maximum = maximum.max(value);
                }
            }
        }
    }
    if !minimum.is_finite() {
        minimum = 0.;
        maximum = 0.;
    }
    Ok(Slice {
        axis: axis.into(),
        position,
        width: w,
        height: h,
        horizontal,
        vertical,
        amplitudes: values,
        min: minimum,
        max: maximum,
        samples_read,
        sample_stride: vstep,
        domain: "sample/time; depth unresolved".into(),
        interval_us: index.interval_us,
        delay_ms: index.delay_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_fixture_index_and_random_slices() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/seismic/segyio-small.sgy");
        let index = index_file(&path).unwrap();
        assert_eq!(index.traces.len(), 25);
        assert_eq!(index.samples, 50);
        assert_eq!(index.inlines, vec![1, 2, 3, 4, 5]);
        assert_eq!(index.crosslines, vec![20, 21, 22, 23, 24]);
        let slice = read_slice(&path, &index, "inline", 1, 1).unwrap();
        assert_eq!((slice.width, slice.height), (5, 50));
        assert_eq!(slice.samples_read, 250);
        let sample = read_slice(&path, &index, "sample", 10, 1).unwrap();
        assert_eq!(sample.samples_read, 25);
        assert_eq!(slice.amplitudes[10 * 5], sample.amplitudes[0]);
        assert!(read_slice(&path, &index, "sample", 50, 1).is_err());
        let lod = read_slice(&path, &index, "inline", 1, 4).unwrap();
        assert!(lod.amplitudes.len() < slice.amplitudes.len());
    }
    #[test]
    fn ibm_known_answers() {
        assert_eq!(amplitude(&[0x41, 0x10, 0, 0], 1), 1.);
        assert_eq!(amplitude(&[0xc1, 0x20, 0, 0], 1), -2.);
        assert_eq!(amplitude(&[0, 0, 0, 0], 1), 0.);
    }
}
