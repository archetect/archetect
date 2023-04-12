use rhai::Engine;

pub (crate) fn register(engine: &mut Engine) {
    engine.register_fn("display", | message: &str| {
        eprintln!("{}", message);
    });

    engine.register_fn("display", || {
        eprintln!();
    });

    engine.register_fn("eprint", | message: &str| {
        eprintln!("{}", message);
    });

    engine.register_fn("eprint", || {
        eprintln!();
    });
}