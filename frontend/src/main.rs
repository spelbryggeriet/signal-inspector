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

fn decibel(amplitude: f32, reference: f32) -> f32 {
    20.0 * (amplitude.abs() / reference.abs()).log10()
}

fn map_range(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    to_min + (value - from_min) / (from_max - from_min) * (to_max - to_min)
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
        sample_rate,
        show_fft,
        ..
    }: &SignalViewProps,
) -> Html {
    const X_SCALE: f32 = 1.025;
    const Y_SCALE: f32 = 1.0125;

    let samples = left_samples;

    let num_samples = samples.len();

    if *show_fft {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(num_samples);

        let mut transform: Vec<_> = samples
            .iter()
            .map(|sample| Complex::from(*sample as f32))
            .collect();

        fft.process(&mut transform);
        transform.truncate(transform.len() / 2);
        let num_usable_samples = transform.len();

        let half_sample_rate_log = (*sample_rate as f32 / 2.0).log10();

        let rms = (samples
            .iter()
            .map(|&sample| sample as f32 * sample as f32)
            .sum::<f32>()
            / num_usable_samples as f32)
            .sqrt();
        let max_volume = transform
            .iter()
            .map(|c| decibel(c.norm(), rms))
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
        let min_volume = 0.0;
        let lines = transform
            .iter()
            .enumerate()
            .skip(1)
            .map(|(frequency, &amplitude)| {
                let frequency_log =
                    (frequency as f32 * *sample_rate as f32 / num_samples as f32).log10();
                let volume = decibel(amplitude.norm(), rms).max(min_volume);
                format!("{frequency_log:.4} {:.4} ", -volume)
            })
            .collect::<String>();

        let order_of_magnitude = (*sample_rate as f32).log10().floor() as u32;
        let x_ticks = (0..=order_of_magnitude)
            .flat_map(|o| {
                (1..10).map(move |i| {
                    let frequency_log = ((i * 10_u32.pow(o)) as f32).log10();
                    let scaling = if i == 1 { 0.025 } else { 0.0 };

                    format!(
                        "M {frequency_log} {} L {frequency_log} {:.4} ",
                        -max_volume,
                        -(min_volume - scaling * (max_volume - min_volume)),
                    )
                })
            })
            .collect::<String>();

        let x_tick_labels = (0..=order_of_magnitude)
            .map(|order| {
                let frequency = 10_u32.pow(order);
                let mut left = map_range(
                    (frequency as f32).log10(),
                    0.0,
                    half_sample_rate_log,
                    0.0,
                    100.0 / Y_SCALE,
                );
                if left.is_infinite() {
                    left = 0.0;
                }

                let unit = if order < 3 { "hertz" } else { "kilohertz" };

                html! {
                    <p
                        class={format!("unit {unit}")}
                        style={format!("left: {left:.4}%")}>
                        {format!("{}", 10_u32.pow(order % 3))}
                    </p>
                }
            })
            .collect::<Html>();

        let min_volume_tick = 3 * (min_volume / 3.0).ceil() as i32;
        let max_volume_tick = 3 * (max_volume / 3.0).floor() as i32;
        let volume_step =
            3 * (1 + ((max_volume_tick - min_volume_tick) as f32).log10().floor() as usize);

        let y_ticks = (min_volume_tick..=max_volume_tick)
            .step_by(volume_step)
            .map(|volume| {
                format!(
                    "M 0 {0:.4} L {1:.4} {0:.4} ",
                    -volume,
                    Y_SCALE * half_sample_rate_log,
                )
            })
            .collect::<String>();

        let y_tick_labels = (min_volume_tick..=max_volume_tick)
            .step_by(volume_step)
            .map(|volume| {
                let top = map_range(volume as f32, max_volume, min_volume, 0.0, 100.0 / X_SCALE);

                html! {
                    <p
                        class="unit decibel"
                        style={format!("top: {top:.4}%")}>
                        {format!("{volume}")}
                    </p>
                }
            })
            .collect::<Html>();

        html! {
            <>
                <div class="signal-view">
                    <svg xmlns="http://www.w3.org/2000/svg">
                        <svg
                            viewBox={format!("0 {:.4} {:.4} {:.4}",
                                -max_volume,
                                Y_SCALE * half_sample_rate_log,
                                X_SCALE * (max_volume - min_volume),
                            )}
                            preserveAspectRatio="none">
                            <path vector-effect="non-scaling-stroke" d={x_ticks} />
                            <path vector-effect="non-scaling-stroke" d={y_ticks} />
                            <path vector-effect="non-scaling-stroke"
                                d={format!("M 0 0 L {lines} {half_sample_rate_log:.4} 0")} />
                            <rect vector-effect="non-scaling-stroke"
                                y={format!("{:.4}", -max_volume)}
                                width={format!("{half_sample_rate_log:.4}")}
                                height={format!("{:.4}", max_volume - min_volume)} />
                        </svg>
                    </svg>
                </div>
                <div class="x-labels">
                    {x_tick_labels}
                </div>
                <div class="y-labels">
                    {y_tick_labels}
                </div>
                <div class="empty-box" />
            </>
        }
    } else {
        let max_amplitude = samples.iter().cloned().max().unwrap_or(i16::MAX) as i32;
        let min_amplitude = samples.iter().cloned().min().unwrap_or(i16::MIN) as i32;

        let lines = samples
            .iter()
            .enumerate()
            .map(|(i, &amplitude)| format!("{i} {:} ", -(amplitude as i32)))
            .collect::<String>();

        let x_ticks = (0..=num_samples)
            .step_by(*sample_rate as usize)
            .map(|sample| {
                format!(
                    "M {sample} {} L {sample} {:.2} ",
                    -max_amplitude,
                    1.05 * (max_amplitude - min_amplitude) as f32,
                )
            })
            .collect::<String>();

        let x_tick_labels = (0..=num_samples)
            .step_by(*sample_rate as usize)
            .map(|sample| {
                let left = map_range(
                    sample as f32,
                    0.0,
                    (num_samples) as f32,
                    0.0,
                    100.0 / Y_SCALE,
                );

                html! {
                    <p
                        class="unit second"
                        style={format!("left: {left:.4}%")}>
                        {format!("{}", sample / *sample_rate as usize)}
                    </p>
                }
            })
            .collect::<Html>();

        let y_ticks = [min_amplitude, 0, max_amplitude]
            .into_iter()
            .map(|amplitude| {
                format!(
                    "M 0 {0} L {1} {0} ",
                    -amplitude,
                    X_SCALE * num_samples as f32
                )
            })
            .collect::<String>();

        let y_tick_labels = [min_amplitude, 0, max_amplitude]
            .into_iter()
            .map(|amplitude| {
                let top = map_range(
                    amplitude as f32,
                    max_amplitude as f32,
                    min_amplitude as f32,
                    0.0,
                    100.0 / X_SCALE,
                );
                let display = if amplitude == 0 {
                    0.0
                } else {
                    map_range(
                        amplitude as f32,
                        i16::MIN as f32,
                        i16::MAX as f32,
                        -100.0,
                        100.0,
                    )
                };

                html! {
                    <p
                        class="unit percentage"
                        style={format!("top: {top:.4}%")}>
                        {format!("{display:.0}")}
                    </p>
                }
            })
            .collect::<Html>();

        html! {
            <>
                <div class="signal-view">
                    <svg xmlns="http://www.w3.org/2000/svg">
                        <svg
                            viewBox={format!("0 {} {:.4} {:.4}",
                                -max_amplitude,
                                Y_SCALE * num_samples as f32,
                                X_SCALE * (max_amplitude - min_amplitude) as f32,
                            )}
                            preserveAspectRatio="none">
                            <path vector-effect="non-scaling-stroke" d={x_ticks} />
                            <path vector-effect="non-scaling-stroke" d={y_ticks} />
                            <path vector-effect="non-scaling-stroke"
                                d={format!("M 0 0 L {lines} {num_samples} 0")} />
                            <rect vector-effect="non-scaling-stroke"
                                y={format!("{}", -max_amplitude)}
                                width={num_samples.to_string()}
                                height={format!("{}", (max_amplitude - min_amplitude))} />
                        </svg>
                    </svg>
                </div>
                <div class="x-labels">
                    {x_tick_labels}
                </div>
                <div class="y-labels">
                    {y_tick_labels}
                </div>
                <div class="empty-box" />
            </>
        }
    }
}

#[function_component(App)]
fn app() -> Html {
    let samples = use_state(|| {
        let frequency = 5;
        let sample_rate = 44100;
        let wave = (0..sample_rate)
            .map(|i| {
                map_range(
                    (2.0 * PI * frequency as f32 * i as f32 / sample_rate as f32).sin(),
                    -1.0,
                    1.0,
                    i16::MIN as f32,
                    i16::MAX as f32,
                ) as i16
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
        <div class="app">
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
    yew::Renderer::<App>::new().render();
}
