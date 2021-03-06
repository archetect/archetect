extern crate actix_web;
extern crate env_logger;

use actix_web::{
    error, http, middleware, server, App, Error, HttpResponse, State, fs
};


struct AppState {
    template: tera::Tera
}

fn index(state: State<AppState>) -> Result<HttpResponse, Error> {
    render_template(state, "index.html")
}

fn detail(state: State<AppState>) -> Result<HttpResponse, Error> {
    render_template(state, "detail.html")
}

fn p404(state: State<AppState>) -> Result<HttpResponse, Error> {
    render_template(state, "404.html")
}

fn render_template(state: State<AppState>, template: &str) -> Result<HttpResponse, Error> {
    let s = state
                .template
                .render(template, tera::Context::new())
                .map_err(|_| error::ErrorInternalServerError("Template error"))?;
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

fn main() {
   crate::vendor::heck::std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    server::new(|| {
        let tera = match tera::Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")) {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
               crate::vendor::heck::std::process::exit(1);
            }
        };

        App::with_state(AppState{template: tera})
            .middleware(middleware::Logger::default())
            .handler( "/static", fs::StaticFiles::new("static")
                .unwrap()
                .show_files_listing())
            .resource("/", |r| r.method(http::Method::GET).with(index))
            .resource("/detail", |r| r.method(http::Method::GET).with(detail))
            .default_resource(|r| r.method(http::Method::GET).with(p404))
    }).bind("127.0.0.1:8080")
        .expect("Could not bind to port 8080")
        .run();
}
