// ---------- //
// 0. Imports //
// ---------- //

mod params;
mod editor;

use fundsp::hacker::*;
use num_derive::FromPrimitive;
use params::{Parameter, Parameters};
use std::{convert::TryFrom, sync::Arc, time::Duration};
use vst::prelude::*;
use wmidi::{Note, Velocity};

use rand::Rng;

const SAMPLE_RATE: f32 = 48_000f32;
const BUFFER_SIZE: f32 = 10.0 * SAMPLE_RATE; // 10 seconds of audio

struct Orbsat {
    // audio: Box<dyn AudioUnit64 + Send>,
    note: Option<(Note, Velocity)>,
    parameters: Arc<Parameters>,
    // ------------- //
    // 1. New fields //
    // ------------- //
    enabled: bool,
    sample_rate: f32,
    time: Duration,
    captured_audio: Box<(Vec<f32>, Vec<f32>)>,
    i_sample: usize,
    o_sample: usize,
    last_output_left: f32,
    last_output_right: f32,
    virtual_audio_left: (Vec<f32>, Vec<f32>),
    virtual_audio_right: (Vec<f32>, Vec<f32>),
    editor: Option<editor::PluginEditor>,
}

fn add_attack(virtual_audio_left: &mut Vec<f32>, buffer: &Vec<f32>) {
    let mut i = 0;
    let buffer_start_amount = buffer[0];
    let virtual_audio_left_len = virtual_audio_left.len();
    while i < virtual_audio_left_len {
        virtual_audio_left[virtual_audio_left_len - 1 - i] = match i { 0 => -buffer_start_amount, _ => -virtual_audio_left[virtual_audio_left_len - i]/(i as f32) };
        i += 1;
    }
}

fn add_release(virtual_audio_right: &mut Vec<f32>, buffer: &Vec<f32>) {
    let mut i = 0;
    let buffer_start_amount = buffer[buffer.len() - 1];
    let virtual_audio_right_len = virtual_audio_right.len();
    while i < virtual_audio_right_len {
        virtual_audio_right[i] = match i { 0 => -buffer_start_amount, _ => -virtual_audio_right[i - 1]/(i as f32) };
        i += 1;
    }
}

impl Plugin for Orbsat {

    #[allow(clippy::precedence)]
    fn new(_host: HostCallback) -> Self {
        let params = Arc<crate::Parameters> = Arc::new(Default::default());
        Self {
            parameters: Default::default(),
            note: None,
            time: Duration::default(),
            sample_rate: SAMPLE_RATE,
            enabled: true,
            captured_audio: Box::new((vec![0.0; BUFFER_SIZE as usize], vec![0.0; BUFFER_SIZE as usize])),
            i_sample: 0,
            o_sample: 0,
            last_output_left: 0.0,
            last_output_right: 0.0,
            virtual_audio_left: (vec![0.0; (SAMPLE_RATE / 100.0) as usize], vec![0.0; (SAMPLE_RATE / 100.0) as usize]),
            virtual_audio_right: (vec![0.0; (SAMPLE_RATE / 100.0) as usize], vec![0.0; (SAMPLE_RATE / 100.0) as usize]),
            editor: Some(editor::PluginEditor {
                params,
                window_handle: None,
                is_open: false,
            }),
        }
    }

    fn init(&mut self) {
        let Info {
            name,
            version,
            unique_id,
            ..
        } = self.get_info();
        let home = dirs::home_dir().unwrap().join("tmp");
        let id_string = format!("{name}-{version}-{unique_id}-log.txt");
        let log_file = std::fs::File::create(home.join(id_string)).unwrap();
        let log_config = ::simplelog::ConfigBuilder::new()
            .set_time_to_local(true)
            .build();
        simplelog::WriteLogger::init(simplelog::LevelFilter::Info, log_config, log_file).ok();
        log_panics::init();
        log::info!("init");
    }

    fn get_info(&self) -> Info {
        Info {
            name: "Orbsat".into(),
            vendor: "Zinnusl".into(),
            unique_id: 128956,
            category: Category::Generator,
            inputs: 2,
            outputs: 2,
            parameters: 1,
            ..Info::default()
        }
    }
    
    fn get_editor(&mut self) -> Option<Box<dyn vst::editor::Editor>> {
        if let Some(editor) = self.editor.take() {
            Some(Box::new(editor) as Box<dyn vst::editor::Editor>)
        } else {
            None
        }
    }

    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.parameters) as Arc<dyn PluginParameters>
    }

    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        let (inputs, mut outputs) = buffer.split();
        if outputs.len() >= 2 && inputs.len() >= 2 {
            if self.note.is_some() {
                self.o_sample = 0;

                let (i_left, i_right) = (inputs.get(0), inputs.get(1));
                let (b_left, b_right): (&mut Vec<f32>, &mut Vec<f32>) = (self.captured_audio.0.as_mut(), self.captured_audio.1.as_mut());
                let (o_left, o_right) = (outputs.get_mut(0), outputs.get_mut(1));

                let to_skip = min(o_left.len(), i_left.len());
                for (output, input) in b_left.iter_mut().skip(self.i_sample).zip(i_left.iter()) {
                    *output = *input;
                }

                for (output, input) in b_right.iter_mut().skip(self.i_sample).zip(i_right.iter()) {
                    *output = *input;
                }
                self.i_sample += to_skip;

                o_left.fill(0.0);
                o_right.fill(0.0);

                // for (output, input) in o_left.iter_mut().zip(out_left_buffer.iter()) {
                //     *output = *input as f32;
                // }
                // for (output, input) in o_right.iter_mut().zip(out_right_buffer.iter()) {
                //     *output = *input as f32;
                // }

                self.last_output_left = 0.0;
                self.last_output_right = 0.0;

                if self.i_sample >= BUFFER_SIZE as usize {
                    self.i_sample = 0;
                    add_attack(&mut self.virtual_audio_left.0, &self.captured_audio.0.as_mut());
                    add_attack(&mut self.virtual_audio_left.1, &self.captured_audio.1.as_mut());
                }
            }
            else {
                // self.i_sample = 0;

                let (o_left, o_right) = (outputs.get_mut(0), outputs.get_mut(1));
                let (b_left, b_right) = (&self.captured_audio.as_ref().0, &self.captured_audio.as_ref().1);

                let to_skip = min(o_left.len(), b_left.len());
                for (output, input) in o_left.iter_mut().zip(b_left.iter().rev().cycle().skip((BUFFER_SIZE - self.i_sample as f32) as usize).skip(self.o_sample)) {
                    *output = *input;
                    self.last_output_left = *input;
                }

                for (output, input) in o_right.iter_mut().zip(b_right.iter().rev().cycle().skip((BUFFER_SIZE - self.i_sample as f32) as usize).skip(self.o_sample)) {
                    *output = *input;
                    self.last_output_right = *input;
                }
                self.o_sample += to_skip;
                if self.o_sample >= b_left.len() {
                    self.o_sample = 0;
                }
            }
        }
    }

    fn process_events(&mut self, events: &vst::api::Events) {
        for event in events.events() {
            if let vst::event::Event::Midi(midi) = event {
                if let Ok(midi) = wmidi::MidiMessage::try_from(midi.data.as_slice()) {
                    match midi {
                        wmidi::MidiMessage::NoteOn(_channel, note, velocity) => {
                            // ----------------------------------------- //
                            // 6. Set `NoteOn` time tag and enable synth //
                            // ----------------------------------------- //
                            self.set_tag(Tag::NoteOn, self.time.as_secs_f64());
                            self.note = Some((note, velocity));
                            self.enabled = true;
                        }
                        wmidi::MidiMessage::NoteOff(_channel, note, _velocity) => {
                            if let Some((current_note, ..)) = self.note {
                                if current_note == note {
                                    self.note = None;
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    // ------------------------------ //
    // 7. Implement `set_sample_rate` //
    // ------------------------------ //
    fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
        self.time = Duration::default();
    }
}

impl Orbsat {
    #[inline(always)]
    fn set_tag(&mut self, _tag: Tag, _value: f64) {
    }

    #[inline(always)]
    fn set_tag_with_param(&mut self, tag: Tag, param: Parameter) {
        self.set_tag(tag, self.parameters.get_parameter(param as i32) as f64);
    }
}

#[derive(FromPrimitive, Clone, Copy)]
pub enum Tag {
    Freq = 0,
    Modulation = 1,
    NoteOn = 2,
}
vst::plugin_main!(Orbsat);
