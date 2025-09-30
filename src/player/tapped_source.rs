use super::PlayerState;
use anyhow::Result;
use rodio::{ChannelCount, Source};
use std::{
    num::NonZero,
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct TappedSource<I> {
    input: I,
    player_state: Arc<Mutex<PlayerState>>,
}

impl<I> TappedSource<I> {
    pub fn new(input: I, player_state: Arc<Mutex<PlayerState>>) -> Self {
        TappedSource {
            input,
            player_state,
        }
    }
}

impl<I> Iterator for TappedSource<I>
where
    I: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.input.next() {
            if let Ok(mut state) = self.player_state.try_lock() {
                if state.oscilloscope_buffer.len() >= 1024 {
                    state.oscilloscope_buffer.pop_front();
                }
                state.oscilloscope_buffer.push_back(sample);
            }
            Some(sample)
        } else {
            None
        }
    }
}

impl<I> Source for TappedSource<I>
where
    I: Source<Item = f32>,
{
    fn channels(&self) -> ChannelCount {
        self.input.channels()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.input.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.input.total_duration()
    }

    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }

    fn bits_per_sample(&self) -> Option<rodio::BitDepth> {
        self.input.bits_per_sample()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.input.try_seek(pos)
    }
}
