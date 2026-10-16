//! Handy recorder implementation of the capture executor's physical driver contract.
use super::device::Session;
use std::sync::Arc;
use super::{
    admission::Admission,
    owner::{Device, Driver, Preferences},
};
pub struct RecorderDriver;
impl Driver for RecorderDriver {
    type Device = Session;
    fn resolve(&self, preferences: &Preferences) -> Result<String, &'static str> {
        let inputs = super::device::inputs()?;
        inputs.iter().find(|input| !preferences.preferred.is_empty() && input.id == preferences.preferred)
            .or_else(|| inputs.iter().find(|input| input.is_default))
            .map(|input| input.id.clone()).ok_or("microphone_unavailable")
    }
    fn open(
        &mut self,
        preferences: &Preferences,
        _generation: u64,
        admission: Arc<Admission>,
    ) -> Result<Session, &'static str> {
        Session::open(
            &preferences.preferred,
            preferences.channel,
            admission,
        )
    }
}
impl Device for Session {
    fn id(&self) -> &str {
        &self.device_id
    }
    fn begin(&self, admission: Arc<Admission>) -> Result<(), &'static str> {
        self.begin(admission)
    }
    fn drain(&self) -> Result<Vec<f32>, &'static str> {
        self.drain()
    }
    fn drained(&self) -> Result<bool, &'static str> {
        self.drained()
    }
    fn cancel(&self) {
        self.cancel();
    }
    fn idle(&self) -> Result<(), &'static str> {
        self.idle()
    }
}
