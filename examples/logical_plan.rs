use rustj::Engine;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "1 + 2 * 3".into());
    let plan = Engine::new().analyze(&source)?;
    println!("{plan:#?}");
    Ok(())
}
