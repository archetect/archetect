use crate::vendor::tera::Tera;

pub mod filters;

pub fn create_tera() -> Tera {
    let mut tera = Tera::default();
    filters::apply_filters(&mut tera);
    tera
}
