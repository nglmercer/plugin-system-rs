use windows::core::{Interface, Result as WinResult};
use windows::Win32::Foundation::{BOOL, TRUE};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eMultimedia, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
};

use crate::{AppVolume, VolumeControl, VolumeState};

pub struct WindowsController {
    _private: (),
}

pub fn create_controller() -> Box<dyn VolumeControl> {
    Box::new(WindowsController { _private: () })
}

pub fn per_app_supported() -> bool {
    true
}

fn ensure_com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

fn get_endpoint_volume() -> WinResult<IAudioEndpointVolume> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
        let volume = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)?;
        Ok(volume)
    }
}

fn get_device() -> WinResult<IMMDevice> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
        Ok(device)
    }
}

impl VolumeControl for WindowsController {
    fn get_master_volume(&self) -> Result<VolumeState, String> {
        ensure_com();

        unsafe {
            let volume = get_endpoint_volume()
                .map_err(|e| format!("Failed to get endpoint volume: {}", e))?;

            let level = volume
                .GetMasterVolumeLevelScalar()
                .map_err(|e| format!("GetMasterVolumeLevelScalar: {}", e))?;

            let muted = volume.GetMute().map_err(|e| format!("GetMute: {}", e))?;

            let device = get_device().map_err(|e| format!("Get device: {}", e))?;

            let device_name = device
                .GetId()
                .map(|id| id.to_string().unwrap_or_default())
                .unwrap_or_default();

            Ok(VolumeState {
                master_volume: level * 100.0,
                muted: muted == TRUE,
                default_device_name: device_name,
            })
        }
    }

    fn set_master_volume(&mut self, volume: f32) -> Result<(), String> {
        ensure_com();

        unsafe {
            let ep = get_endpoint_volume()
                .map_err(|e| format!("Failed to get endpoint volume: {}", e))?;

            let scalar = (volume / 100.0).clamp(0.0, 1.0);
            ep.SetMasterVolumeLevelScalar(scalar, std::ptr::null())
                .map_err(|e| format!("SetMasterVolumeLevelScalar: {}", e))?;
        }

        Ok(())
    }

    fn set_muted(&mut self, muted: bool) -> Result<(), String> {
        ensure_com();

        unsafe {
            let ep = get_endpoint_volume()
                .map_err(|e| format!("Failed to get endpoint volume: {}", e))?;

            let mute_val = BOOL::from(muted);
            ep.SetMute(mute_val, std::ptr::null())
                .map_err(|e| format!("SetMute: {}", e))?;
        }

        Ok(())
    }

    fn get_app_volumes(&self) -> Result<Vec<AppVolume>, String> {
        ensure_com();

        let mut apps = Vec::new();

        unsafe {
            use windows::Win32::Media::Audio::{
                IAudioSessionControl2, IAudioSessionManager2, ISimpleAudioVolume,
            };

            let device = get_device().map_err(|e| format!("Get device: {}", e))?;

            let session_manager: IAudioSessionManager2 = device
                .Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate IAudioSessionManager2: {}", e))?;

            let session_list = session_manager
                .GetSessionEnumerator()
                .map_err(|e| format!("GetSessionEnumerator: {}", e))?;

            let count = session_list
                .GetCount()
                .map_err(|e| format!("GetCount: {}", e))?;

            for i in 0..count {
                if let Ok(session_control) = session_list.GetSession(i) {
                    if let Ok(session2) = session_control.cast::<IAudioSessionControl2>() {
                        if let Ok(display_name) = session2.GetDisplayName() {
                            let name = display_name.to_string().unwrap_or_default();
                            if name.is_empty() {
                                if let Ok(proc_name) = session2.GetProcessId() {
                                    apps.push(AppVolume {
                                        name: format!("PID:{}", proc_name),
                                        volume: 0.0,
                                        muted: false,
                                        pid: Some(proc_name),
                                    });
                                }
                                continue;
                            }

                            let mut vol = 0.0f32;
                            let mut muted = false;

                            if let Ok(simple_vol) = session_control.cast::<ISimpleAudioVolume>() {
                                if let Ok(v) = simple_vol.GetMasterVolume() {
                                    vol = v;
                                }
                                if let Ok(m) = simple_vol.GetMute() {
                                    muted = m == TRUE;
                                }
                            }

                            let pid = session2.GetProcessId().ok();

                            apps.push(AppVolume {
                                name,
                                volume: vol * 100.0,
                                muted,
                                pid,
                            });
                        }
                    }
                }
            }
        }

        Ok(apps)
    }

    fn set_app_volume(&mut self, app_name: &str, volume: f32) -> Result<(), String> {
        ensure_com();

        unsafe {
            use windows::Win32::Media::Audio::{IAudioSessionManager2, ISimpleAudioVolume};

            let device = get_device().map_err(|e| format!("Get device: {}", e))?;
            let session_manager: IAudioSessionManager2 = device
                .Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate: {}", e))?;

            let session_list = session_manager
                .GetSessionEnumerator()
                .map_err(|e| format!("GetSessionEnumerator: {}", e))?;

            let count = session_list
                .GetCount()
                .map_err(|e| format!("GetCount: {}", e))?;

            let scalar = (volume / 100.0).clamp(0.0, 1.0);

            for i in 0..count {
                if let Ok(session_control) = session_list.GetSession(i) {
                    if let Ok(session2) = session_control
                        .cast::<windows::Win32::Media::Audio::IAudioSessionControl2>(
                    ) {
                        if let Ok(display_name) = session2.GetDisplayName() {
                            let name = display_name.to_string().unwrap_or_default();
                            if name == app_name {
                                if let Ok(simple_vol) = session_control.cast::<ISimpleAudioVolume>()
                                {
                                    simple_vol
                                        .SetMasterVolume(scalar, std::ptr::null())
                                        .map_err(|e| format!("SetMasterVolume: {}", e))?;
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(format!("App '{}' not found", app_name))
    }

    fn set_app_muted(&mut self, app_name: &str, muted: bool) -> Result<(), String> {
        ensure_com();

        unsafe {
            use windows::Win32::Media::Audio::{IAudioSessionManager2, ISimpleAudioVolume};

            let device = get_device().map_err(|e| format!("Get device: {}", e))?;
            let session_manager: IAudioSessionManager2 = device
                .Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate: {}", e))?;

            let session_list = session_manager
                .GetSessionEnumerator()
                .map_err(|e| format!("GetSessionEnumerator: {}", e))?;

            let count = session_list
                .GetCount()
                .map_err(|e| format!("GetCount: {}", e))?;

            let mute_val = BOOL::from(muted);

            for i in 0..count {
                if let Ok(session_control) = session_list.GetSession(i) {
                    if let Ok(session2) = session_control
                        .cast::<windows::Win32::Media::Audio::IAudioSessionControl2>(
                    ) {
                        if let Ok(display_name) = session2.GetDisplayName() {
                            let name = display_name.to_string().unwrap_or_default();
                            if name == app_name {
                                if let Ok(simple_vol) = session_control.cast::<ISimpleAudioVolume>()
                                {
                                    simple_vol
                                        .SetMute(mute_val, std::ptr::null())
                                        .map_err(|e| format!("SetMute: {}", e))?;
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(format!("App '{}' not found", app_name))
    }
}
