#![allow(non_snake_case)]
mod display;
mod feed;

use dioxus_fullstack::launch::LaunchBuilder;

fn main() {
    #[cfg(feature = "ssr")]
    {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(20 * 60));
                loop {
                    interval.tick().await;
                    println!("Pulling articles");
                    feed::pull_articles().await;
                }
            });
    }

    LaunchBuilder::new(display::App).launch();
}
