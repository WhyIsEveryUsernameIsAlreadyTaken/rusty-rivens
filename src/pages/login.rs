use ev::SubmitEvent;
use http::StatusCode;
use leptos::*;
use leptos_router::{use_navigate, NavigateOptions};
use serde::{Deserialize, Serialize};
use tauri_sys::tauri::invoke;
use wasm_bindgen_futures::spawn_local;

use crate::{filter_err, EmptyArgs};

#[derive(Serialize, Deserialize)]
struct LoginArgs<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Serialize, Deserialize)]
struct WrappedStatus {
    #[serde(with = "http_serde::status_code")]
    status: StatusCode
}

#[component]
pub fn Login() -> impl IntoView {
    let (email, set_email) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (login_failed, set_login_failed) = create_signal(String::new());

    let (errors, set_errors) = create_signal(Vec::<String>::new());

    let update_email = move |ev| {
        let v = event_target_value(&ev);
        set_email.set(v);
    };
    let update_password = move |ev| {
        let v = event_target_value(&ev);
        set_password.set(v);
    };
    let (logged_in, set_logged_in) = create_signal(false);
    let (prefetch_auth, set_prefetch_auth) = create_signal(false);

    spawn_local(async move {
        let resp = invoke::<EmptyArgs, bool>("get_auth_state", &EmptyArgs).await;
        match filter_err(resp) {
            Ok(v) => {
                set_logged_in.set(v);
                if v {
                    let navigate = use_navigate();
                    let mut nav_opts = NavigateOptions::default();
                    nav_opts.replace=true;
                    navigate("/home", nav_opts);
                }
                set_prefetch_auth.set(true)
            },
            Err(err) => {set_errors.update(|v| v.push(err.to_string()))}
        }
    });

    let login = move |ev: SubmitEvent| {
        ev.prevent_default();
        spawn_local(async move {
            let email = email.get_untracked();
            let password = password.get_untracked();
            if email.is_empty() || password.is_empty() {
                return;
            }

            let args = &LoginArgs { email: &email, password: &password };
            let resp = invoke::<LoginArgs, WrappedStatus>("login", &args).await;
            let login_result = match filter_err(resp) {
                Ok(v) => {
                    v.status == StatusCode::OK
                },
                Err(err) => {set_errors.update(|v| v.push(err.to_string())); false}
            };
            if !login_result {
                set_login_failed.set(String::from("Rety Login"));
            } else {
                if !logged_in.get_untracked() {
                    set_logged_in.set(true);
                    let navigate = use_navigate();
                    let mut nav_opts = NavigateOptions::default();
                    nav_opts.replace=true;
                    navigate("/home", nav_opts);
                }
            }
        });
    };
    logging::log!("Number of errors: {}", errors.get_untracked().len());
    view! {
    <div class="container">
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
                <a href="https://tauri.app" target="_blank">
                    <img src="public/tauri.svg" class="logo tauri" alt="Tauri logo"/>
                </a>
                <a href="https://docs.rs/leptos/" target="_blank">
                    <img src="public/leptos.svg" class="logo leptos" alt="Leptos logo"/>
                </a>
            </div>

            <p style="text-align: center">"Click on the Tauri and Leptos logos to learn more."</p>

            <p style="text-align: center; color: red;"><b>{move || login_failed.get()}</b></p>

            <Show
            when=move || { prefetch_auth.get() }
                fallback=|| view! { }
            >
                    <form on:submit=login>
                        <div class="row">
                            <input
                                id="email-input"
                                type="email"
                                placeholder="Email"
                                on:input=update_email
                            />
                        </div>
                        <div class="row">
                            <input
                                id="password-input"
                                type="password"
                                placeholder="Password"
                                on:input=update_password
                            />
                        </div>
                        <div class="row">
                            <button type="submit">"Login"</button>
                        </div>
                    </form>
            </Show>
        </Show>
    </div>
    }
}
