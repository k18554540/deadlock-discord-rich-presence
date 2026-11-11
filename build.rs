
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winres::WindowsResource::new();

        resource.set_icon("icon.ico");

        resource
            .compile()
            .expect("Failed to embed application icon");
    }
}
