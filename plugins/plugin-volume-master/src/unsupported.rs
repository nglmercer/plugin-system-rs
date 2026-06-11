use crate::{AppVolume, VolumeControl, VolumeState};

pub struct UnsupportedController;

pub fn create_controller() -> Box<dyn VolumeControl> {
    Box::new(UnsupportedController)
}

pub fn per_app_supported() -> bool {
    false
}

impl VolumeControl for UnsupportedController {
    fn get_master_volume(&self) -> Result<VolumeState, String> {
        Err("Volume control not supported on this platform".to_string())
    }

    fn set_master_volume(&mut self, _volume: f32) -> Result<(), String> {
        Err("Volume control not supported on this platform".to_string())
    }

    fn set_muted(&mut self, _muted: bool) -> Result<(), String> {
        Err("Volume control not supported on this platform".to_string())
    }

    fn get_app_volumes(&self) -> Result<Vec<AppVolume>, String> {
        Ok(Vec::new())
    }

    fn set_app_volume(&mut self, _app_name: &str, _volume: f32) -> Result<(), String> {
        Err("Volume control not supported on this platform".to_string())
    }

    fn set_app_muted(&mut self, _app_name: &str, _muted: bool) -> Result<(), String> {
        Err("Volume control not supported on this platform".to_string())
    }
}
