use rusty_js_macros::prototype;

pub struct A{

}

#[prototype]
impl A{
    #[setter]
    #[getter]
    pub fn get(&self, a:usize) -> usize{
        0
    }
}