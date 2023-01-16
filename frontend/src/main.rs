use std::cmp::Ordering;
use std::f32::consts::PI;
use std::io::Cursor;

use gloo::file::File;
use rustfft::{num_complex::Complex, FftPlanner};
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
struct ControlBoardProps {
    on_loaded: Callback<(Vec<i16>, Vec<i16>, u32)>,
    on_fft: Callback<()>,
    show_fft: bool,
}

#[function_component(ControlBoard)]
fn control_board(
    ControlBoardProps {
        on_loaded,
        on_fft,
        show_fft,
    }: &ControlBoardProps,
) -> Html {
    let file_reader = use_state(|| None);
    let on_change = {
        let on_loaded = on_loaded.clone();
        Callback::from(move |event: Event| {
            let file: web_sys::File = event
                .target_unchecked_into::<HtmlInputElement>()
                .files()
                .unwrap()
                .get(0)
                .unwrap();
            let file = File::from(file);
            let on_loaded = on_loaded.clone();
            let reader = gloo::file::callbacks::read_as_bytes(&file, move |res| {
                let data = res.unwrap();
                let reader = hound::WavReader::new(Cursor::new(data)).unwrap();
                let spec = reader.spec();

                let mut is_left = true;
                let (left_channel, right_channel) = reader
                    .into_samples::<i16>()
                    .try_fold((Vec::new(), Vec::new()), |mut acc, sample| {
                        let sample = sample?;
                        if is_left {
                            acc.0.push(sample)
                        } else {
                            acc.1.push(sample)
                        }
                        is_left = !is_left;
                        Result::<_, hound::Error>::Ok(acc)
                    })
                    .unwrap();
                on_loaded.emit((left_channel, right_channel, spec.sample_rate));
            });
            file_reader.set(Some(reader));
        })
    };
    let on_click = {
        let on_fft = on_fft.clone();
        Callback::from(move |_| on_fft.emit(()))
    };

    html! {
        <div class="control-board">
            <div>
                <label for="load-sample-file">{"Load sample file"}</label>
                <input id="load-sample-file" type="file" accept=".wav" onchange={on_change} />
            </div>
            <div>
                <button style="width: 250px" onclick={on_click}>{
                    if *show_fft {
                        "Show sample"
                    } else {
                        "Show frequency spectrum"
                    }
                }</button>
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct SignalViewProps {
    left_samples: Vec<i16>,
    right_samples: Vec<i16>,
    sample_rate: u32,
    show_fft: bool,
}

#[function_component(SignalView)]
fn signal_view(
    SignalViewProps {
        left_samples,
        right_samples,
        sample_rate,
        show_fft,
    }: &SignalViewProps,
) -> Html {
    let samples = left_samples;

    let len = samples.len();

    if *show_fft {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(len);

        let mut transform: Vec<_> = samples
            .iter()
            .map(|sample| Complex::from(*sample as f32))
            .collect();

        fft.process(&mut transform);
        transform.truncate(transform.len() / 2);
        let len = transform.len();

        let sample_rate_log = (*sample_rate as f32).log10();

        let rms = (transform
            .iter()
            .map(|&sample| sample.re * sample.re)
            .sum::<f32>()
            / len as f32)
            .sqrt();
        let max_decibel = transform
            .iter()
            .map(|c| 20.0 * (c.re.abs() / rms).log10())
            .max_by(|x, y| {
                x.partial_cmp(y).unwrap_or_else(|| {
                    if !x.is_nan() {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                })
            })
            .unwrap_or(0.0);
        let min_decibel = -72.0;
        let lines = transform
            .iter()
            .enumerate()
            .map(|(frequency, &amplitude)| {
                let frequency = frequency as f32 * *sample_rate as f32 / len as f32;
                let mut frequency_log = frequency.log10() / sample_rate_log * *sample_rate as f32;
                if frequency_log.is_infinite() {
                    frequency_log = 0.0;
                }
                let decibel = 20.0 * (amplitude.re.abs() / rms).log10();

                format!(
                    "{frequency_log:.4} {:.4} ",
                    (1.0 - (decibel - min_decibel) / (max_decibel - min_decibel)).min(1.0),
                )
            })
            .collect::<String>();

        let order_of_magnitude = (*sample_rate as f32).log10().floor() as u32;
        let ticks = (0..=order_of_magnitude)
            .flat_map(|o| {
                (1..10).map(move |i| {
                    let f = i * 10_u32.pow(o);
                    let mut frequency_log =
                        (f as f32).log10() / sample_rate_log * *sample_rate as f32;
                    if frequency_log.is_infinite() {
                        frequency_log = 0.0;
                    }

                    let y_pos = if i == 1 { 1.025 } else { 1.0125 };
                    format!("M {0} {y_pos} {0} 1 ", frequency_log)
                })
            })
            .collect::<String>();

        let tick_labels = (0..=order_of_magnitude)
            .map(|o| {
                let f = 10_u32.pow(o);
                let mut left = (f as f32).log10() / sample_rate_log * 100.0;
                if left.is_infinite() {
                    left = 0.0;
                }

                let text = format!("{} {}", 10_u32.pow(o % 3), if o < 3 { "Hz" } else { "kHz" });

                html! {
                    <p
                        style={format!("left: calc({left:.2}% - 5px)")}>
                        {text}
                    </p>
                }
            })
            .collect::<Html>();

        html! {
            <div class="signal-view">
                <svg
                    viewBox={format!("0 0 {sample_rate} 1")}
                    xmlns="http://www.w3.org/2000/svg"
                    preserveAspectRatio="none">
                    <path vector-effect="non-scaling-stroke" d={format!("M 0 1 L {lines} {sample_rate} 1")} />
                    <path vector-effect="non-scaling-stroke" d={ticks} />
                    <rect vector-effect="non-scaling-stroke" width={sample_rate.to_string()} height="1" />
                </svg>
                <div class="labels">
                    {tick_labels}
                </div>
            </div>
        }
    } else {
        let max_amplitude = samples
            .iter()
            .map(|&sample| sample.unsigned_abs())
            .max()
            .unwrap_or(u16::MAX);
        let lines = samples
            .iter()
            .enumerate()
            .map(|(i, &amplitude)| format!("{i} {:.4} ", -amplitude as f32 / max_amplitude as f32))
            .collect::<String>();

        let ticks = (0..=len)
            .step_by(*sample_rate as usize)
            .map(|second| format!("M {0} 2 L {0} 1 ", second))
            .collect::<String>();

        let tick_labels = (0..=len / *sample_rate as usize)
            .map(|second| {
                html! {
                    <p
                        style={format!("left: calc({:.2}% - 5px)",
                            second as f32 / len as f32 * *sample_rate as f32 * 100.0,
                        )}>
                        {format!("{second} s")}
                    </p>
                }
            })
            .collect::<Html>();

        html! {
            <>
                <div class="signal-view">
                    <svg
                        viewBox={format!("0 -1 {len} 2")}
                        xmlns="http://www.w3.org/2000/svg"
                        preserveAspectRatio="none">
                        <path vector-effect="non-scaling-stroke" d={format!("M 0 0 L {lines} {len} 0")} />
                        <path vector-effect="non-scaling-stroke" d={ticks} />
                        <rect vector-effect="non-scaling-stroke" y="-1" width={len.to_string()} height="2" />
                    </svg>
                </div>
                <div class="labels">
                    {tick_labels}
                </div>
            </>
        }
    }
}

#[function_component]
fn MainLayout() -> Html {
    let samples = use_state(|| {
        let frequency = 5;
        let sample_rate = 44100;
        let wave = (0..9 * sample_rate / 8)
            .map(|i| {
                ((2.0 * PI * frequency as f32 * i as f32 / sample_rate as f32).sin()
                    * i16::MAX as f32) as i16
            })
            .collect::<Vec<_>>();
        (wave.clone(), wave, sample_rate)
    });
    let show_fft = use_state(|| false);
    let on_loaded = {
        let samples = samples.clone();
        Callback::from(move |samples_data| {
            samples.set(samples_data);
        })
    };
    let on_fft = {
        let show_fft = show_fft.clone();
        Callback::from(move |_| {
            show_fft.set(!*show_fft);
        })
    };

    html! {
        <div class="main-layout">
            <ControlBoard on_loaded={on_loaded} on_fft={on_fft} show_fft={*show_fft} />
            <SignalView
                left_samples={samples.0.clone()}
                right_samples={samples.1.clone()}
                sample_rate={samples.2}
                show_fft={*show_fft} />
        </div>
    }
}

fn main() {
    yew::Renderer::<MainLayout>::new().render();
}
