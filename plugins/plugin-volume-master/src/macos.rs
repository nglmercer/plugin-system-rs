use crate::{AppVolume, VolumeControl, VolumeState};

pub struct MacOSController {
    _private: (),
}

pub fn create_controller() -> Box<dyn VolumeControl> {
    Box::new(MacOSController { _private: () })
}

pub fn per_app_supported() -> bool {
    false
}

impl VolumeControl for MacOSController {
    fn get_master_volume(&self) -> Result<VolumeState, String> {
        use coreaudio_rs::sys::{
            kAudioDevicePropertyScopeOutput, kAudioHardwarePropertyDefaultOutputDevice,
            kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal, AudioDeviceID,
            AudioObjectPropertyAddress,
        };

        unsafe {
            let mut device_id: AudioDeviceID = 0;
            let mut size = std::mem::size_of::<AudioDeviceID>() as u32;
            let address = AudioObjectPropertyAddress {
                mSelector: kAudioHardwarePropertyDefaultOutputDevice,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let status = coreaudio_rs::sys::audio_object_get_property_data(
                coreaudio_rs::sys::kAudioObjectSystemObject,
                &address,
                &mut size,
                &mut device_id as *mut _ as *mut _,
            );

            if status != 0 {
                return Err(format!("Failed to get default device: {}", status));
            }

            let mut volume: f32 = 0.0;
            let mut vol_size = std::mem::size_of::<f32>() as u32;
            let vol_address = AudioObjectPropertyAddress {
                mSelector: coreaudio_rs::sys::kAudioDevicePropertyVolumeScalar,
                mScope: kAudioDevicePropertyScopeOutput,
                mElement: kAudioObjectPropertyElementMain,
            };

            let status = coreaudio_rs::sys::audio_object_get_property_data(
                device_id,
                &vol_address,
                &mut vol_size,
                &mut volume as *mut _ as *mut _,
            );

            if status != 0 {
                return Err(format!("Failed to get volume: {}", status));
            }

            let mut muted: u32 = 0;
            let mut mute_size = std::mem::size_of::<u32>() as u32;
            let mute_address = AudioObjectPropertyAddress {
                mSelector: coreaudio_rs::sys::kAudioDevicePropertyMute,
                mScope: kAudioDevicePropertyScopeOutput,
                mElement: kAudioObjectPropertyElementMain,
            };

            let _ = coreaudio_rs::sys::audio_object_get_property_data(
                device_id,
                &mute_address,
                &mut mute_size,
                &mut muted as *mut _ as *mut _,
            );

            Ok(VolumeState {
                master_volume: volume * 100.0,
                muted: muted != 0,
                default_device_name: format!("Device {}", device_id),
            })
        }
    }

    fn set_master_volume(&mut self, volume: f32) -> Result<(), String> {
        use coreaudio_rs::sys::{
            kAudioDevicePropertyScopeOutput, kAudioHardwarePropertyDefaultOutputDevice,
            kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal, AudioDeviceID,
            AudioObjectPropertyAddress,
        };

        unsafe {
            let mut device_id: AudioDeviceID = 0;
            let mut size = std::mem::size_of::<AudioDeviceID>() as u32;
            let address = AudioObjectPropertyAddress {
                mSelector: kAudioHardwarePropertyDefaultOutputDevice,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let status = coreaudio_rs::sys::audio_object_get_property_data(
                coreaudio_rs::sys::kAudioObjectSystemObject,
                &address,
                &mut size,
                &mut device_id as *mut _ as *mut _,
            );

            if status != 0 {
                return Err("Failed to get default device".to_string());
            }

            let scalar = (volume / 100.0).clamp(0.0, 1.0);
            let vol_address = AudioObjectPropertyAddress {
                mSelector: coreaudio_rs::sys::kAudioDevicePropertyVolumeScalar,
                mScope: kAudioDevicePropertyScopeOutput,
                mElement: kAudioObjectPropertyElementMain,
            };

            let status = coreaudio_rs::sys::audio_object_set_property_data(
                device_id,
                &vol_address,
                &scalar as *const _ as *const _,
                std::mem::size_of::<f32>() as u32,
            );

            if status != 0 {
                return Err(format!("Failed to set volume: {}", status));
            }
        }

        Ok(())
    }

    fn set_muted(&mut self, muted: bool) -> Result<(), String> {
        use coreaudio_rs::sys::{
            kAudioDevicePropertyScopeOutput, kAudioHardwarePropertyDefaultOutputDevice,
            kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal, AudioDeviceID,
            AudioObjectPropertyAddress,
        };

        unsafe {
            let mut device_id: AudioDeviceID = 0;
            let mut size = std::mem::size_of::<AudioDeviceID>() as u32;
            let address = AudioObjectPropertyAddress {
                mSelector: kAudioHardwarePropertyDefaultOutputDevice,
                mScope: kAudioObjectPropertyScopeGlobal,
                mElement: kAudioObjectPropertyElementMain,
            };

            let status = coreaudio_rs::sys::audio_object_get_property_data(
                coreaudio_rs::sys::kAudioObjectSystemObject,
                &address,
                &mut size,
                &mut device_id as *mut _ as *mut _,
            );

            if status != 0 {
                return Err("Failed to get default device".to_string());
            }

            let mute_val: u32 = if muted { 1 } else { 0 };
            let mute_address = AudioObjectPropertyAddress {
                mSelector: coreaudio_rs::sys::kAudioDevicePropertyMute,
                mScope: kAudioDevicePropertyScopeOutput,
                mElement: kAudioObjectPropertyElementMain,
            };

            let status = coreaudio_rs::sys::audio_object_set_property_data(
                device_id,
                &mute_address,
                &mute_val as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            );

            if status != 0 {
                return Err(format!("Failed to set mute: {}", status));
            }
        }

        Ok(())
    }

    fn get_app_volumes(&self) -> Result<Vec<AppVolume>, String> {
        Ok(Vec::new())
    }

    fn set_app_volume(&mut self, _app_name: &str, _volume: f32) -> Result<(), String> {
        Err("Per-app volume control not supported on macOS".to_string())
    }

    fn set_app_muted(&mut self, _app_name: &str, _muted: bool) -> Result<(), String> {
        Err("Per-app volume control not supported on macOS".to_string())
    }
}
