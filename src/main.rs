use anyhow::Result;
use app_core::support::app::run_application;

fn main() -> Result<()> {
    let app = app_core::TriangleApp::default();
    run_application(app)?;
    Ok(())
}
