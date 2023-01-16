use std::f32::consts::PI;
use std::io::Cursor;

use gloo::file::File;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
struct ControlBoardProps {
    on_loaded: Callback<(Vec<i16>, Vec<i16>)>,
}

#[function_component(ControlBoard)]
fn control_board(ControlBoardProps { on_loaded }: &ControlBoardProps) -> Html {
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
                on_loaded.emit((left_channel, right_channel));
            });
            file_reader.set(Some(reader));
        })
    };

    html! {
        <div class="control-board">
            <label for="load-sample-file">{"Load sample file"}</label>
            <input id="load-sample-file" type="file" onchange={on_change} />
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct SignalViewProps {
    left_samples: Vec<i16>,
    right_samples: Vec<i16>,
}

#[function_component(SignalView)]
fn signal_view(
    SignalViewProps {
        left_samples,
        right_samples,
    }: &SignalViewProps,
) -> Html {
    let len = left_samples.len();
    let scale = ((u16::MAX as u32 + 99) / 100) as f32;
    let lines = left_samples
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            format!(
                "L {:.2} {:.2} ",
                i as f32 * 100.0 / len as f32,
                (sample as i32 - i16::MIN as i32) as f32 / scale,
            )
        })
        .collect::<String>();

    html! {
        <svg
            class="signal-view"
            viewBox="0 0 100 100"
            xmlns="http://www.w3.org/2000/svg"
            preserveAspectRatio="none">
            <path vector-effect="non-scaling-stroke" d={format!("M 0 50 {lines}")} />
        </svg>
    }
}

#[function_component]
fn MainLayout() -> Html {
    let samples = use_state(|| {
        let max = 1024;
        let wave = (0..max)
            .map(|i| ((2.0 * PI * i as f32 / max as f32).sin() * i16::MAX as f32) as i16)
            .collect::<Vec<_>>();
        (wave.clone(), wave)
    });
    let on_loaded = {
        let samples = samples.clone();
        Callback::from(move |samples_pair| {
            samples.set(samples_pair);
        })
    };

    html! {
        <div class="main-layout">
            <ControlBoard on_loaded={on_loaded} />
            <SignalView left_samples={samples.0.clone()} right_samples={samples.1.clone()} />
        </div>
    }
}

fn main() {
    yew::Renderer::<MainLayout>::new().render();
}
