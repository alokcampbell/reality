#![allow(non_snake_case)]

use dioxus::prelude::*;
// this hosts the front end
mod editor;
mod landing;

use editor::Editor;
use landing::Landing;

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Landing {},
    #[route("/doc/:id")]
    Editor { id: String },
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! { Router::<Route> {} }
}