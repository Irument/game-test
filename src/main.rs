fn main() {
    let _ = simplelog::SimpleLogger::init(
        log::LevelFilter::Info,
        simplelog::ConfigBuilder::new().build(),
    );

    let _ = game_test::start();
}
