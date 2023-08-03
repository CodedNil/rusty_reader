mod display;
mod feed;

fn main() {
    dioxus_web::launch(display::App);
}
