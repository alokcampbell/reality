#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::Route;

// landing webpage, not much to say here.

#[component]
pub fn Landing() -> Element {
    let mut join_id = use_signal(|| String::new());
    let nav = use_navigator();

    rsx! {
        div { style: "display:flex;flex-direction:column;align-items:center;justify-content:center;height:100vh;gap:1rem;",
            h1 { "Reality" }

            button {
                onclick: move |_| {
                    let id = uuid();
                    nav.push(Route::Editor { id });
                },
                "Create new document"
            }

            div { style: "display:flex;gap:0.5rem;",
                input {
                    placeholder: "Paste document ID to join...",
                    value: "{join_id}",
                    oninput: move |e| join_id.set(e.value()),
                }
                button {
                    onclick: move |_| {
                        let id = join_id.read().clone();
                        if !id.is_empty() {
                            nav.push(Route::Editor { id });
                        }
                    },
                    "Join"
                }
            }
        }
    }
}

fn uuid() -> String {
    // manages the code string
    let a = (js_sys::Math::random() * 0xffffffff_u32 as f64) as u64;
    let b = (js_sys::Math::random() * 0xffffffff_u32 as f64) as u64;
    format!("{:x}-{:x}", a, b)
}