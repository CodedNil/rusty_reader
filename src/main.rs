#![allow(non_snake_case)]
mod display;
mod feed;

use dioxus_fullstack::launch::LaunchBuilder;
#[cfg(feature = "ssr")]
use tokio::runtime::Runtime;

fn main() {
    #[cfg(feature = "ssr")]
    {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1200)); // 20 minutes
        let res = rt.block_on(async {
            loop {
                feed::pull_articles().await;
                interval.tick().await;
            }
        });
    }

    LaunchBuilder::new(display::App).launch();
}
