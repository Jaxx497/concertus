fn main() -> anyhow::Result<()> {
    unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
    concertus::app_core::Concertus::new().run()?;
    Ok(())
}
