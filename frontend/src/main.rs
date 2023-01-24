use std::{cmp::Ordering, f64::consts::PI};

use gloo::file::File;
use wasm_bindgen::prelude::*;
use web_sys::HtmlInputElement;
use yew::prelude::*;

use model::{Channel, Signal, Spectrum};

#[macro_use]
mod bench;

mod model;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

fn map_range<T: Into<f64>>(value: T, from_min: T, from_max: T, to_min: f64, to_max: f64) -> f64 {
    let from_min = from_min.into();
    to_min + (value.into() - from_min) / (from_max.into() - from_min) * (to_max - to_min)
}

#[derive(Properties, PartialEq)]
struct ControlBoardProps {
    on_loaded: Callback<Signal>,
    on_spectrum: Callback<()>,
    show_spectrum: bool,
}

#[function_component(ControlBoard)]
fn control_board(
    ControlBoardProps {
        on_loaded,
        on_spectrum,
        show_spectrum,
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
        let on_spectrum = on_spectrum.clone();
        Callback::from(move |_| on_spectrum.emit(()))
    };

    html! {
        <div class="control-board">
            <div>
                <label for="load-sample-file">{"Load sample file"}</label>
                <input id="load-sample-file" type="file" accept=".wav" onchange={on_change} />
            </div>
            <div>
                <button style="width: 250px" onclick={on_click}>{
                    if *show_spectrum {
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
    mini: bool,
}

#[function_component(SignalView)]
fn signal_view(SignalViewProps { channel, mini }: &SignalViewProps) -> Html {
    const X_SCALE: f64 = 1.025;
    const Y_SCALE: f64 = 1.0125;

    let num_samples = channel.count();

    bench_start!("Preparing sample view");

    let sample_lower_bound = channel.lower_bound();
    let sample_upper_bound = channel.upper_bound();

    let min_amplitude = *use_memo(
        |_| bench!(["Calculating min amplitude"] => channel.min()),
        channel.clone(),
    );
    let max_amplitude = *use_memo(
        |_| bench!(["Calculating max amplitude"] => channel.max()),
        channel.clone(),
    );
    let lines = use_memo(
        |_| {
            bench!(["Formatting sample lines"] => channel
                .iter()
                .enumerate()
                .map(|(i, amplitude)| {
                    let percentage = map_range(amplitude, max_amplitude, min_amplitude, -100.0, 100.0);
                    format!("{i} {percentage:.4} ")
                })
                .collect::<String>())
        },
        channel.clone(),
    );

    let tick_paths = if !*mini {
        let x_ticks = bench!(["Formatting X ticks"] => (0..=num_samples)
            .step_by(channel.sample_rate() as usize)
            .map(|sample| {
                format!(
                    "M {sample} -100 L {sample} {:.4} ",
                    X_SCALE * 200.0,
                )
            })
            .collect::<String>());

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

        Some(html! {
            <>
                <path vector-effect="non-scaling-stroke" d={x_ticks} />
                <path vector-effect="non-scaling-stroke" d={y_ticks} />
            </>
        })
    } else {
        None
    };

    let tick_labels = if !*mini {
        let x_tick_labels = bench!(["Rendering X tick labels"] => (0..=num_samples)
            .step_by(channel.sample_rate() as usize)
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
                        {format!("{}", sample / channel.sample_rate() as usize)}
                    </p>
                }
            })
            .collect::<Html>());

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

        Some(html! {
            <>
                <div class="x-labels">
                    {x_tick_labels}
                </div>
                <div class="y-labels">
                    {y_tick_labels}
                </div>
            </>
        })
    } else {
        None
    };

    bench_end!();

    html! {
        <>
            <div class={classes!("plot", mini.then_some("mini"), "signal-view")}>
                <svg xmlns="http://www.w3.org/2000/svg">
                    <svg
                        viewBox={format!("0 -100 {:.4} {:.4}",
                            Y_SCALE * num_samples as f64,
                            X_SCALE * 200.0,
                        )}
                        preserveAspectRatio="none">
                        {tick_paths}
                        <path vector-effect="non-scaling-stroke"
                            d={format!("M 0 0 L {lines} {num_samples} 0")} />
                        <rect vector-effect="non-scaling-stroke"
                            y="-100"
                            width={num_samples.to_string()}
                            height="200" />
                    </svg>
                </svg>
            </div>
            {tick_labels}
            <div class="empty-box" />
        </>
    }
}

#[derive(Properties, PartialEq)]
struct SpectrumViewProps {
    spectrum: Spectrum,
    show: bool,
}

#[function_component(SpectrumView)]
fn spectrum_view(SpectrumViewProps { spectrum, show }: &SpectrumViewProps) -> Html {
    const X_SCALE: f64 = 1.025;
    const Y_SCALE: f64 = 1.0125;

    bench_start!("Preparing frequency view");

    let num_usable_samples = spectrum.len();
    let half_sample_rate_log = (spectrum.sample_rate() as f64 / 2.0).log10();

    let rms = *use_memo(
        |_| {
            bench!(["Calculating RMS"] => {
                let square_sum = spectrum
                    .iter()
                    .map(|c| c.norm())
                    .map(|f| f * f)
                    .sum::<f64>();

                (square_sum / num_usable_samples as f64).sqrt()
            })
        },
        spectrum.clone(),
    );

    let centroid = *use_memo(
        |_| {
            bench!(["Calculating centroid"] => {
                let numerator: f64 = spectrum
                    .iter()
                    .enumerate()
                    .map(|(n, c)| {
                        let frequency = spectrum.bin_to_frequency(n);
                        let magnitude = c.norm();
                        frequency * magnitude
                    })
                    .sum();
                let denominator: f64 = spectrum
                    .iter()
                    .map(|c| c.norm())
                    .sum();
                numerator / denominator
            })
        },
        spectrum.clone(),
    );
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

    let max_volume = *use_memo(
        |_| {
            bench!(["Calculating max volume"] => spectrum
            .iter()
            .map(|c| Spectrum::decibel(c.norm(), rms))
            .max_by(|x, y| {
                x.partial_cmp(y).unwrap_or_else(|| {
                    if !x.is_nan() {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                })
            })
            .unwrap_or(0.0))
        },
        spectrum.clone(),
    );
    let min_volume = 0.0;
    let lines = use_memo(
        |_| {
            bench!(["Formatting frequency lines"] => spectrum
            .iter()
            .enumerate()
            .skip(1)
            .map(|(n, &amplitude)| {
                let frequency_log = spectrum.bin_to_frequency(n).log10();
                let volume = Spectrum::decibel(amplitude.norm(), rms).max(min_volume);
                format!("{frequency_log:.4} {:.4} ", -volume)
            })
            .collect::<String>())
        },
        spectrum.clone(),
    );

    if !*show {
        return html!();
    }

    let order_of_magnitude = (spectrum.sample_rate() as f32).log10().floor() as u32;
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
            <div class="plot spectrum-view">
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
            Signal::Mono(Channel::from_samples_f32(wave, 32, sample_rate))
        })
    });
    let channel = signal.channel(0);
    let spectrum = use_memo(|_| channel.spectrum(), channel.clone());

    let show_spectrum = use_state(|| false);

    let on_loaded = {
        let signal = signal.clone();
        Callback::from(move |new_signal| {
            signal.set(new_signal);
        })
    };
    let on_spectrum = {
        let show_spectrum = show_spectrum.clone();
        Callback::from(move |_| {
            show_spectrum.set(!*show_spectrum);
        })
    };

    bench_end!();

    html! {
        <div class={classes!("app", show_spectrum.then_some("split"))}>
            <ControlBoard
                on_loaded={on_loaded}
                on_spectrum={on_spectrum}
                show_spectrum={*show_spectrum} />
            <SignalView
                channel={channel.clone()}
                mini={*show_spectrum} />
            <SpectrumView spectrum={(*spectrum).clone()} show={*show_spectrum} />
        </div>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
