use leptos::{component, create_signal, spawn_local, view, IntoView, Show, SignalGet, SignalSet, SignalUpdate};
use tauri_sys::tauri::invoke;

use crate::{filter_err, EmptyArgs};

#[component]
pub fn Home() -> impl IntoView {
    let (errors, set_errors) = create_signal(Vec::<String>::new());
    let (init_ran, set_init_ran) = create_signal(false);
    if !init_ran.get() {
        spawn_local(async move {
            let res =  invoke::<EmptyArgs, bool>("reload_thing", &EmptyArgs).await;
            match filter_err(res) {
                Ok(_) => {},
                Err(err) => {set_errors.update(|v| v.push(err.to_string()))}
            }
        });
    }
    set_init_ran.set(true);


    view! {
        <Show when=move || { errors.get().len() == 0 }
            fallback=move || view! {
                <div class="error" style="color: red;">
                    <h2>"Panic during login!"</h2>
                    <ul> {
                    move || errors.get()
                        .into_iter()
                        .map(|e| view! {
                            <li>{e.to_string()}</li>
                        })
                        .collect::<Vec<_>>()
                    }
                    </ul>
                </div>
            }>
            <div class="row">
                <div class="cell" style:height=format!("calc(126px + (2.2em * {}))", 2)>
                    <div class="celltitle">
                        Torid Viva-concinak
                    </div>
                    <hr/>
                        <p style="text-align: center;">+16.5% Heat</p>
                        <p style="text-align: center; margin-block-start: 0; margin-block-end: 0;">+16.5% Heat</p>
                    <div class="cellbuttondiv">
                        <button class="cellbutton">Edit</button>
                        <button class="cellbutton" style="background-color: #ff4444;">Delete</button>
                    </div>
                </div>
                <div class="cell" style:height=format!("calc(126px + (2.2em * {}))", 4)>
                    <div class="celltitle">
                       Skana scipha
                    </div>
                    <hr/>
                        <p style="text-align: center;">+16.5% Heat</p>
                        <p style="text-align: center;">+22.7% Slash</p>
                        <p style="text-align: center;">-3.7% Fire Rate / Attack Speed</p>
                        <p style="text-align: center;">-3.7% Fire Rate / Attack Speed</p>
                    <div class="cellbuttondiv">
                        <button class="cellbutton">Edit</button>
                        <button class="cellbutton" style="background-color: #ff4444;">Delete</button>
                    </div>
                </div>
        </div>
    </Show>
    }
}
