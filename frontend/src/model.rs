use std::{cmp::Ordering, io::Cursor};

use hound::{SampleFormat, WavReader, WavSpec};
use im::{vector::Iter, Vector};

pub struct Signal {
    data: SignalData,
    sample_rate: u32,
}

enum SignalData {
    Mono(Channel),
    Stereo(Channel, Channel),
}

impl Signal {
    pub fn from_mono(channel: Channel, sample_rate: u32) -> Self {
        Self {
            data: SignalData::Mono(channel),
            sample_rate,
        }
    }

    pub fn from_wav(data: Vec<u8>) -> Result<Self, hound::Error> {
        let reader = WavReader::new(Cursor::new(data))?;
        let spec = reader.spec();

        if spec.channels == 1 {
            Self::read_into_mono(reader, spec)
        } else if spec.channels == 2 {
            Self::read_into_stereo(reader, spec)
        } else {
            panic!("unsupported number of channels: {}", spec.channels);
        }
    }

    pub fn channel(&self, n: usize) -> &Channel {
        match (n, &self.data) {
            (0, SignalData::Mono(channel) | SignalData::Stereo(channel, _)) => channel,
            (1, SignalData::Stereo(_, channel)) => channel,
            _ => panic!("channel {n} does not exist"),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn read_into_mono(
        reader: WavReader<Cursor<Vec<u8>>>,
        spec: WavSpec,
    ) -> Result<Self, hound::Error> {
        macro_rules! collect_samples {
            ($type:ty, $fn:ident) => {{
                let mut data = Vec::new();
                for result in reader.into_samples::<$type>() {
                    let sample = result?;
                    data.push(sample);
                }
                Channel::$fn(data, spec.bits_per_sample)
            }};
        }

        let channel = match (spec.sample_format, spec.bits_per_sample) {
            (SampleFormat::Int, 1..=8) => collect_samples!(i8, from_samples_i8),
            (SampleFormat::Int, 9..=16) => collect_samples!(i16, from_samples_i16),
            (SampleFormat::Int, 17..=32) => collect_samples!(i32, from_samples_i32),
            (SampleFormat::Float, 1..=32) => collect_samples!(f32, from_samples_f32),
            _ => panic!("unsupported format"),
        };

        Ok(Self {
            data: SignalData::Mono(channel),
            sample_rate: spec.sample_rate,
        })
    }

    fn read_into_stereo(
        reader: WavReader<Cursor<Vec<u8>>>,
        spec: WavSpec,
    ) -> Result<Self, hound::Error> {
        macro_rules! collect_samples {
            ($type:ty, $fn:ident) => {{
                let mut left = Vec::new();
                let mut right = Vec::new();

                let mut is_left = true;
                for result in reader.into_samples::<$type>() {
                    let sample = result?;
                    if is_left {
                        left.push(sample);
                    } else {
                        right.push(sample);
                    }
                    is_left = !is_left;
                }

                (
                    Channel::$fn(left, spec.bits_per_sample),
                    Channel::$fn(right, spec.bits_per_sample),
                )
            }};
        }

        let (left_channel, right_channel) = match (spec.sample_format, spec.bits_per_sample) {
            (SampleFormat::Int, 1..=8) => collect_samples!(i8, from_samples_i8),
            (SampleFormat::Int, 9..=16) => collect_samples!(i16, from_samples_i16),
            (SampleFormat::Int, 17..=32) => collect_samples!(i32, from_samples_i32),
            (SampleFormat::Float, 1..=32) => collect_samples!(f32, from_samples_f32),
            _ => panic!("unsupported format"),
        };

        Ok(Self {
            data: SignalData::Stereo(left_channel, right_channel),
            sample_rate: spec.sample_rate,
        })
    }
}

#[derive(Clone, PartialEq)]
pub struct Channel {
    data: Vector<u8>,
    bits_per_sample: u16,
    sample_format: SampleFormat,
}

impl Channel {
    pub fn from_samples_i8(samples: impl IntoIterator<Item = i8>, bits_per_sample: u16) -> Self {
        assert!(
            (1..=8).contains(&bits_per_sample),
            "unsupported number of bits per sample: {bits_per_sample}",
        );

        Self {
            data: samples.into_iter().flat_map(i8::to_ne_bytes).collect(),
            bits_per_sample,
            sample_format: SampleFormat::Int,
        }
    }

    pub fn from_samples_i16(samples: impl IntoIterator<Item = i16>, bits_per_sample: u16) -> Self {
        assert!(
            (1..=16).contains(&bits_per_sample),
            "unsupported number of bits per sample: {bits_per_sample}",
        );

        Self {
            data: samples.into_iter().flat_map(i16::to_ne_bytes).collect(),
            bits_per_sample,
            sample_format: SampleFormat::Int,
        }
    }

    pub fn from_samples_i32(samples: impl IntoIterator<Item = i32>, bits_per_sample: u16) -> Self {
        assert!(
            (1..=32).contains(&bits_per_sample),
            "unsupported number of bits per sample: {bits_per_sample}",
        );

        Self {
            data: samples.into_iter().flat_map(i32::to_ne_bytes).collect(),
            bits_per_sample,
            sample_format: SampleFormat::Int,
        }
    }

    pub fn from_samples_f32(samples: impl IntoIterator<Item = f32>, bits_per_sample: u16) -> Self {
        assert!(
            (1..=32).contains(&bits_per_sample),
            "unsupported number of bits per sample: {bits_per_sample}",
        );

        Self {
            data: samples.into_iter().flat_map(f32::to_ne_bytes).collect(),
            bits_per_sample,
            sample_format: SampleFormat::Float,
        }
    }

    pub fn lower_bound(&self) -> Sample {
        match (self.sample_format, self.bytes_per_sample()) {
            (SampleFormat::Int, 1) => Sample::Int8(i8::MIN),
            (SampleFormat::Int, 2) => Sample::Int16(i16::MIN),
            (SampleFormat::Int, 3..=4) => Sample::Int32(i32::MIN),
            (SampleFormat::Float, 1..=4) => Sample::Float32(f32::MIN),
            _ => unreachable!(),
        }
    }

    pub fn upper_bound(&self) -> Sample {
        match (self.sample_format, self.bytes_per_sample()) {
            (SampleFormat::Int, 1) => Sample::Int8(i8::MAX),
            (SampleFormat::Int, 2) => Sample::Int16(i16::MAX),
            (SampleFormat::Int, 3..=4) => Sample::Int32(i32::MAX),
            (SampleFormat::Float, 1..=4) => Sample::Float32(f32::MAX),
            _ => unreachable!(),
        }
    }

    pub fn count(&self) -> usize {
        self.data.len() / self.bytes_per_sample() as usize
    }

    pub fn iter(&self) -> ChannelIter {
        ChannelIter {
            inner: self.data.iter(),
            chunk_len: self.bytes_per_sample(),
            sample_format: self.sample_format,
        }
    }

    fn bytes_per_sample(&self) -> u16 {
        (self.bits_per_sample + 7) / 8
    }
}

pub struct ChannelIter<'a> {
    inner: Iter<'a, u8>,
    sample_format: SampleFormat,
    chunk_len: u16,
}

impl Iterator for ChannelIter<'_> {
    type Item = Sample;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.sample_format, self.chunk_len) {
            (SampleFormat::Int, 1) => {
                let bytes = [self.inner.next().copied()?];
                Some(Sample::Int8(i8::from_ne_bytes(bytes)))
            }
            (SampleFormat::Int, 2) => {
                let bytes = [self.inner.next().copied()?, self.inner.next().copied()?];
                Some(Sample::Int16(i16::from_ne_bytes(bytes)))
            }
            (SampleFormat::Int, 3..=4) => {
                let bytes = [
                    self.inner.next().copied()?,
                    self.inner.next().copied()?,
                    self.inner.next().copied()?,
                    self.inner.next().copied()?,
                ];
                Some(Sample::Int32(i32::from_ne_bytes(bytes)))
            }
            (SampleFormat::Float, 1..=4) => {
                let bytes = [
                    self.inner.next().copied()?,
                    self.inner.next().copied()?,
                    self.inner.next().copied()?,
                    self.inner.next().copied()?,
                ];
                Some(Sample::Float32(f32::from_ne_bytes(bytes)))
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Sample {
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Float32(f32),
}

impl Sample {
    pub fn is_zero(&self) -> bool {
        matches!(self, Self::Int8(0) | Self::Int16(0) | Self::Int32(0),)
            || matches!(self, Self::Float32(n) if *n == 0.0)
    }

    pub fn into_zero(self) -> Self {
        match self {
            Self::Int8(_) => Self::Int8(0),
            Self::Int16(_) => Self::Int16(0),
            Self::Int32(_) => Self::Int32(0),
            Self::Float32(_) => Self::Float32(0.0),
        }
    }
}

impl Eq for Sample {}

impl PartialOrd for Sample {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Sample {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Int8(left), Self::Int8(right)) => left.cmp(right),
            (Self::Int16(left), Self::Int16(right)) => left.cmp(right),
            (Self::Int32(left), Self::Int32(right)) => left.cmp(right),
            (Self::Float32(left), Self::Float32(right)) => left
                .partial_cmp(right)
                .unwrap_or_else(|| panic!("undefined comparison: {left} <> {right}")),
            (left, right) => panic!("undefined comparison: {left:?} <> {right:?}"),
        }
    }
}

impl From<Sample> for f64 {
    fn from(value: Sample) -> Self {
        match value {
            Sample::Int8(n) => n as f64,
            Sample::Int16(n) => n as f64,
            Sample::Int32(n) => n as f64,
            Sample::Float32(n) => n as f64,
        }
    }
}
