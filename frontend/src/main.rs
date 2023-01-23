use std::{cmp::Ordering, f64::consts::PI};

use gloo::file::File;
use rustfft::{num_complex::Complex, FftPlanner};
use wasm_bindgen::prelude::*;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use model::{Channel, Signal};

#[macro_use]
mod bench;

mod model;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

fn decibel(amplitude: f64, reference: f64) -> f64 {
    20.0 * (amplitude.abs() / reference.abs()).log10()
}

fn map_range<T: Into<f64>>(value: T, from_min: T, from_max: T, to_min: f64, to_max: f64) -> f64 {
    let from_min = from_min.into();
    to_min + (value.into() - from_min) / (from_max.into() - from_min) * (to_max - to_min)
}

#[derive(Properties, PartialEq)]
struct ControlBoardProps {
    on_loaded: Callback<Signal>,
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
            bench!(["Reading file"] => {
                let file: web_sys::File = event
                    .target_unchecked_into::<HtmlInputElement>()
                    .files()
                    .unwrap()
                    .get(0)
                    .unwrap();
                let file = File::from(file);
                let on_loaded = on_loaded.clone();
                let reader = gloo::file::callbacks::read_as_bytes(&file, move |res| {
                    on_loaded.emit(Signal::from_wav(res.unwrap()).unwrap());
                });
                file_reader.set(Some(reader));
            })
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
    channel: Channel,
    sample_rate: u32,
    show_fft: bool,
}

#[function_component(SignalView)]
fn signal_view(
    SignalViewProps {
        channel,
        sample_rate,
        show_fft,
    }: &SignalViewProps,
) -> Html {
    const X_SCALE: f64 = 1.025;
    const Y_SCALE: f64 = 1.0125;

    bench_start!("Preparing signal view");

    let num_samples = channel.count();

    let fft = use_memo(
        |_| FftPlanner::<f64>::new().plan_fft_forward(num_samples),
        num_samples,
    );
    let transform = use_memo(
        |_| {
            let mut transform: Vec<_> = channel
                .iter()
                .map(|sample| Complex::from(f64::from(sample)))
                .collect();

            bench!(["Calculating FFT"] => fft.process(&mut transform));

            transform.truncate(transform.len() / 2);
            transform
        },
        channel.clone(),
    );

    bench_end!();

    if *show_fft {
        bench_start!("Preparing frequency view");

        let num_usable_samples = transform.len();
        let half_sample_rate_log = (*sample_rate as f64 / 2.0).log10();

        let rms = bench!(["Calculating RMS"] => {
            let square_sum = channel
                .iter()
                .map(|sample| f64::from(sample) * f64::from(sample))
                .sum::<f64>();

            (square_sum / num_usable_samples as f64).sqrt()
        });

        let centroid = bench!(["Calculating centroid"] => {
            let numerator: f64 = transform
                .iter()
                .enumerate()
                .map(|(n, c)| {
                    let frequency = n as f64 * *sample_rate as f64 / num_samples as f64;
                    let magnitude = c.norm();
                    frequency * magnitude
                })
                .sum();
            let denominator: f64 = transform
                .iter()
                .map(|c| c.norm())
                .sum();
            numerator / denominator
        });
        let centroid_log = centroid.log10();

        let centroid_label = bench!(["Rendering centroid label"] => {
            let top = map_range(0.5, 0.0, 1.0, 0.0, 100.0 / X_SCALE);
            let mut left = map_range(
                centroid_log,
                0.0,
                half_sample_rate_log,
                0.0,
                100.0 / Y_SCALE);
            if left.is_infinite() {
                left = 0.0;
            }

            let translate_x = if left > 50.0 {
                "calc(-100% - 6px)"
            } else {
                "6px"
            };

            html! {
                <p style={format!("top: {top:.4}%;\
                                   left: {left:.4}%;\
                                   transform: translate({translate_x}, -50%)")}>
                    {format!("Centroid = {centroid:.0} Hz")}
                </p>
            }
        });

        let max_volume = bench!(["Calculating max volume"] => transform
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
            .unwrap_or(0.0));
        let min_volume = 0.0;
        let lines = bench!(["Formatting frequency lines"] => transform
            .iter()
            .enumerate()
            .skip(1)
            .map(|(n, &amplitude)| {
                let frequency_log =
                    (n as f64 * *sample_rate as f64 / num_samples as f64).log10();
                let volume = decibel(amplitude.norm(), rms).max(min_volume);
                format!("{frequency_log:.4} {:.4} ", -volume)
            })
            .collect::<String>());

        let order_of_magnitude = (*sample_rate as f32).log10().floor() as u32;
        let x_ticks = bench!(["Formatting X ticks"] => (0..=order_of_magnitude)
            .flat_map(|o| {
                (1..10).map(move |i| {
                    let frequency_log = ((i * 10_u32.pow(o)) as f64).log10();
                    let scaling = if i == 1 { 0.025 } else { 0.0 };

                    format!(
                        "M {frequency_log} {} L {frequency_log} {:.4} ",
                        -max_volume,
                        -(min_volume - scaling * (max_volume - min_volume)),
                    )
                })
            })
            .collect::<String>());

        let x_tick_labels = bench!(["Rendering X tick labels"] => (0..=order_of_magnitude)
            .map(|order| {
                let frequency = 10_u32.pow(order);
                let mut left = map_range(
                    (frequency as f64).log10(),
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
            .collect::<Html>());

        let min_volume_tick = 3 * (min_volume / 3.0).ceil() as i64;
        let max_volume_tick = 3 * (max_volume / 3.0).floor() as i64;
        let volume_step =
            3 * (1 + ((max_volume_tick - min_volume_tick) as f64).log10().floor() as usize);

        let y_ticks = bench!(["Formatting Y ticks"] => (min_volume_tick..=max_volume_tick)
            .step_by(volume_step)
            .map(|volume| {
                format!(
                    "M 0 {0:.4} L {1:.4} {0:.4} ",
                    -volume,
                    Y_SCALE * half_sample_rate_log,
                )
            })
            .collect::<String>());

        let y_tick_labels = bench!(["Rendering Y tick labels"] => (min_volume_tick..=max_volume_tick)
            .step_by(volume_step)
            .map(|volume| {
                let top = map_range(volume as f64, max_volume, min_volume, 0.0, 100.0 / X_SCALE);

                html! {
                    <p
                        class="unit decibel"
                        style={format!("top: {top:.4}%")}>
                        {format!("{volume}")}
                    </p>
                }
            })
            .collect::<Html>());

        bench_end!();

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
                            <path vector-effect="non-scaling-stroke"
                                d={format!("M {0:.4} {1:.4} L {0:.4} {2:.4}",
                                    centroid_log,
                                    -min_volume,
                                    -(max_volume - min_volume) / 2.0,
                                )} />
                            <rect vector-effect="non-scaling-stroke"
                                y={format!("{:.4}", -max_volume)}
                                width={format!("{half_sample_rate_log:.4}")}
                                height={format!("{:.4}", max_volume - min_volume)} />
                        </svg>
                    </svg>
                    {centroid_label}
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
        bench_start!("Preparing sample view");

        let sample_lower_bound = channel.lower_bound();
        let sample_upper_bound = channel.upper_bound();
        let max_amplitude = bench!(["Calculating max amplitude"] => channel
            .iter()
            .max()
            .unwrap_or(sample_upper_bound)
        );
        let min_amplitude = bench!(["Calculating min amplitude"] => channel
            .iter()
            .min()
            .unwrap_or(sample_lower_bound)
        );

        let lines = bench!(["Formatting sample lines"] => channel
            .iter()
            .enumerate()
            .map(|(i, amplitude)| {
                let percentage = map_range(amplitude, max_amplitude, min_amplitude, -100.0, 100.0);
                format!("{i} {percentage:.4} ")
            })
            .collect::<String>());

        let x_ticks = bench!(["Formatting X ticks"] => (0..=num_samples)
            .step_by(*sample_rate as usize)
            .map(|sample| {
                format!(
                    "M {sample} -100 L {sample} {:.4} ",
                    1.05 * 200.0,
                )
            })
            .collect::<String>());

        let x_tick_labels = bench!(["Rendering X tick labels"] => (0..=num_samples)
            .step_by(*sample_rate as usize)
            .map(|sample| {
                let left = map_range(
                    sample as f64,
                    0.0,
                    (num_samples) as f64,
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
            .collect::<Html>());

        let y_ticks = bench!(["Formatting Y ticks"] =>
            [
                min_amplitude,
                min_amplitude.into_zero(),
                max_amplitude,
            ]
            .into_iter()
            .map(|amplitude| {
                let percentage = map_range(amplitude, max_amplitude, min_amplitude, -100.0, 100.0);
                format!(
                    "M 0 {0:.4} L {1} {0:.4} ",
                    percentage,
                    X_SCALE * num_samples as f64
                )
            })
            .collect::<String>());

        let y_tick_labels = bench!(["Rendering Y tick labels"] =>
            [
                min_amplitude,
                min_amplitude.into_zero(),
                max_amplitude,
            ]
            .into_iter()
            .map(|amplitude| {
                let top = map_range(
                    amplitude,
                    max_amplitude,
                    min_amplitude,
                    0.0,
                    100.0 / X_SCALE,
                );
                let display = if amplitude.is_zero() {
                    0.0
                } else {
                    map_range(
                        amplitude,
                        sample_lower_bound,
                        sample_upper_bound,
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
            .collect::<Html>());

        bench_end!();

        html! {
            <>
                <div class="signal-view">
                    <svg xmlns="http://www.w3.org/2000/svg">
                        <svg
                            viewBox={format!("0 -100 {:.4} {:.4}",
                                Y_SCALE * num_samples as f64,
                                X_SCALE * 200.0,
                            )}
                            preserveAspectRatio="none">
                            <path vector-effect="non-scaling-stroke" d={x_ticks} />
                            <path vector-effect="non-scaling-stroke" d={y_ticks} />
                            <path vector-effect="non-scaling-stroke"
                                d={format!("M 0 0 L {lines} {num_samples} 0")} />
                            <rect vector-effect="non-scaling-stroke"
                                y="-100"
                                width={num_samples.to_string()}
                                height="200" />
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
    bench_start!("Preparing app");

    let signal = use_state(|| {
        bench!(["Generating default stereo signal"] => {
            let frequency = 5;
            let sample_rate = 44100;
            let wave = (0..sample_rate)
                .map(|i| {
                    map_range(
                        (2.0 * PI * frequency as f64 * i as f64 / sample_rate as f64).sin(),
                        -1.0,
                        1.0,
                        f32::MIN as f64,
                        f32::MAX as f64,
                    ) as f32
                });
            Signal::from_mono(Channel::from_samples_f32(wave, 32), sample_rate)
        })
    });
    let show_fft = use_state(|| false);
    let on_loaded = {
        let signal = signal.clone();
        Callback::from(move |new_signal| {
            signal.set(new_signal);
        })
    };
    let on_fft = {
        let show_fft = show_fft.clone();
        Callback::from(move |_| {
            show_fft.set(!*show_fft);
        })
    };

    bench_end!();

    html! {
        <div class="app">
            <ControlBoard on_loaded={on_loaded} on_fft={on_fft} show_fft={*show_fft} />
            <SignalView
                channel={signal.channel(0).clone()}
                sample_rate={signal.sample_rate()}
                show_fft={*show_fft} />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
