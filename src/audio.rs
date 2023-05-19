use std::{rc::Rc, cell::RefCell};

use num::ToPrimitive;
use wasm_bindgen::prelude::*;
use web_sys::{AudioContext, OscillatorType, AudioBuffer, AudioBufferSourceNode, AudioContextOptions};


/// Converts a midi note to frequency
///
/// A midi note is an integer, generally in the range of 21 to 108
pub fn midi_to_freq(note: u8) -> f32 {
    27.5 * 2f32.powf((note as f32 - 21.0) / 12.0)
}

// const AUDIO_BUFF_SIZE: u32 = 1024;
// pub const AUDIO_BUFF_SIZE: u32 = 44800;
pub const AUDIO_BUFF_SIZE: u32 = 44100;
pub const SAMPLE_RATE: u32 = 44100;

pub type AUDIO_BUFF_TYPE = Rc<RefCell<[f32;AUDIO_BUFF_SIZE as usize]>>;


#[wasm_bindgen(skip)]
pub struct WebAudioContext {
    ctx: AudioContext,
    
    pub sample: u32,

    pub t: f64,
    t_since_last_update: f64,
    pub t_readback: f64,
    pub needs_update: bool,
    write_buff_idx: usize,
    buffers: [AudioBuffer;2],
    gains: [web_sys::GainNode;2],

    prev_buff_source: AudioBufferSourceNode,
    
    #[wasm_bindgen(skip)]
    pub readback_buffer: AUDIO_BUFF_TYPE,
}


impl Drop for WebAudioContext {
    fn drop(&mut self) {
        let _ = self.ctx.close();
    }
}

#[wasm_bindgen]
impl WebAudioContext {
    pub fn update_time(&mut self){
        let curr_t = self.ctx.current_time();
        let delta_t = curr_t - self.t;
        self.t_since_last_update += delta_t;
        self.t = curr_t;

        // let sampleCount = Math.round(timeDifference * sampleRate);
        self.sample += (delta_t * SAMPLE_RATE as f64).round() as u32;

        let min_buff_step = (AUDIO_BUFF_SIZE as f64/2.0)/SAMPLE_RATE as f64;
        self.needs_update = self.t_since_last_update > min_buff_step;
    }
    pub fn try_push_audio(&mut self){
        let audio_samples = *self.readback_buffer.borrow();

        {
            let write_buff = &mut self.buffers[self.write_buff_idx];
            let mut data = [0f32; AUDIO_BUFF_SIZE as usize];
            let mut diffs = [0f32; AUDIO_BUFF_SIZE as usize];
            let mut other = [0f32; AUDIO_BUFF_SIZE as usize];
            
            for idx in 0..AUDIO_BUFF_SIZE as usize {
                data[idx] = audio_samples[idx];
                other[idx] = ((self.t_readback as f32 + idx as f32/SAMPLE_RATE as f32)*1400.0).sin();
                diffs[idx] = data[idx] - other[idx];
            }
            // log::info!("{:?}",diffs);
            log::info!("{:?}",data);
            log::info!("{:?}",other);
            log::info!("{:?}",self.t_readback);

            write_buff.copy_to_channel(&data, 0).unwrap();
            write_buff.copy_to_channel(&data, 1).unwrap();

            // Referenced https://github.com/0b5vr/domain/blob/main/src/music/Music.ts
            {

                let time_now = self.ctx.current_time(); 
                let fade_t = 0.005;

                // Connect current buffer/source
                let write_buff_source = self.ctx.create_buffer_source().unwrap();
                write_buff_source.set_buffer(Some(write_buff));
                // buff_source.connect_with_audio_node(&self.ctx.destination()).unwrap();

                {
                    let write_buff_gain = &mut self.gains[self.write_buff_idx];
                    write_buff_source.connect_with_audio_node(&write_buff_gain).unwrap();
                    write_buff_gain.gain().set_value_at_time(0., time_now).unwrap();
                    write_buff_gain.gain().linear_ramp_to_value_at_time(1., time_now + fade_t).unwrap();
                }


                // Queue stop of previous source 
                let prev_buff_gain = &mut self.gains[1 - self.write_buff_idx];
                prev_buff_gain.gain().set_value_at_time(1., time_now).unwrap();
                prev_buff_gain.gain().linear_ramp_to_value_at_time(0., time_now + fade_t).unwrap();
                self.prev_buff_source.stop_with_when(time_now + fade_t).unwrap();

                // let time_now = self.ctx.current_time(); 
                // Queue start of current source

                write_buff_source.start_with_when_and_grain_offset(time_now, time_now - self.t_readback).unwrap();
                // buff_source.start().unwrap();

                log::info!("{}", "update!");

                self.t_since_last_update = 0.0;
                self.t = time_now;
                self.t_readback = 0.0;
                
                // Ping-pong
                self.write_buff_idx = 1 - self.write_buff_idx;
                self.prev_buff_source = write_buff_source;
            }
        }
    }


    #[wasm_bindgen]
    pub fn new() -> Result<WebAudioContext, JsValue> {
        log::info!("WEBAUDIO");

        let ctx = web_sys::AudioContext::new_with_context_options(
            &AudioContextOptions::new().sample_rate(SAMPLE_RATE.to_f32().unwrap())
        )?;

        let write_buff_idx = 0;
        
        let buffers = [
            ctx.create_buffer(2, AUDIO_BUFF_SIZE, SAMPLE_RATE.to_f32().unwrap())?,
            ctx.create_buffer(2, AUDIO_BUFF_SIZE, SAMPLE_RATE.to_f32().unwrap())?
        ];
        let gains = [
            ctx.create_gain().unwrap(),
            ctx.create_gain().unwrap()
        ];
        
        gains[0].connect_with_audio_node(&ctx.destination()).unwrap();
        gains[1].connect_with_audio_node(&ctx.destination()).unwrap();
        
        let readback_buffer = Rc::new(RefCell::new([0f32;AUDIO_BUFF_SIZE as usize]));
        

        let prev_buff_source = ctx.create_buffer_source()?;
        prev_buff_source.start()?;

        //   if (audioContext.state === "suspended") {
        //     audioContext.resume();
        //   }

        // let gain_dest = ctx.create_gain()?;

        log::info!("WebAudio initialized");
        
        Ok(
            WebAudioContext {
                t: ctx.current_time(),
                sample: (ctx.current_time()* SAMPLE_RATE as f64).round() as u32,
                ctx,
                t_since_last_update: 0.0,
                t_readback: 0.0,
                needs_update: true,
                // gain_dest,
                write_buff_idx,
                buffers,
                gains,
                prev_buff_source,
                readback_buffer,
            }
        )
    }

}

