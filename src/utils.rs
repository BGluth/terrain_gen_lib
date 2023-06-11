pub type GenerationError = Result<(), String>;

#[derive(Debug, Default)]
pub(crate) struct IdGenerator {}

impl IdGenerator {
    pub(crate) fn gen_id(&mut self) -> usize {
        todo!()
    }
}
