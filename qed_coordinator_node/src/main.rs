use anyhow::Ok;

mod processor;
mod worker;

fn main() -> anyhow::Result<()> {
    processor::run()?;
    Ok(())
}
