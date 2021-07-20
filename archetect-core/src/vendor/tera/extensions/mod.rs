use crate::vendor::tera::Tera;

pub mod filters;
pub mod functions;

pub fn create_tera() -> Tera {
    let mut tera = Tera::default();
    filters::apply_filters(&mut tera);
    functions::apply_functions(&mut tera);
    tera
}
