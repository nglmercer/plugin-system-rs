use obws::requests::inputs::{InputId, Volume};
use obws::requests::scene_items::SetEnabled;
use obws::requests::scenes::SceneId;
use obws::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObsConnectionInfo {
    pub host: String,
    pub port: u16,
    pub connected: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObsStreamStatus {
    pub active: bool,
    pub reconnecting: bool,
    pub timecode: String,
    pub duration: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObsRecordStatus {
    pub active: bool,
    pub paused: bool,
    pub timecode: String,
    pub duration: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObsScene {
    pub name: String,
    pub index: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObsInput {
    pub name: String,
    pub kind: String,
    pub uuid: String,
    pub muted: bool,
    pub volume: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObsTransition {
    pub name: String,
    pub kind: String,
    pub duration: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObsSceneItem {
    pub id: i32,
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObsStats {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub fps: f64,
    pub render_skipped: u32,
    pub render_total: u32,
    pub output_skipped: u32,
    pub output_total: u32,
}

#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct ObsTestState {
    pub connected: bool,
    pub host: String,
    pub port: u16,
    pub stream_active: bool,
    pub record_active: bool,
    pub record_paused: bool,
    pub virtual_cam_active: bool,
    pub replay_buffer_active: bool,
    pub current_scene: String,
    pub studio_mode: bool,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub fps: f64,
    pub scenes: Vec<ObsScene>,
    pub inputs: Vec<ObsInput>,
    pub transitions: Vec<ObsTransition>,
    pub scene_items: Vec<ObsSceneItem>,
}

#[cfg(test)]
impl Default for ObsTestState {
    fn default() -> Self {
        Self {
            connected: false,
            host: "127.0.0.1".to_string(),
            port: 4455,
            stream_active: false,
            record_active: false,
            record_paused: false,
            virtual_cam_active: false,
            replay_buffer_active: false,
            current_scene: "Escena".to_string(),
            studio_mode: false,
            cpu_usage: 12.5,
            memory_usage: 34.0,
            fps: 60.0,
            scenes: vec![
                ObsScene {
                    name: "Escena".to_string(),
                    index: 0,
                },
                ObsScene {
                    name: "Escena 2".to_string(),
                    index: 1,
                },
            ],
            inputs: vec![ObsInput {
                name: "Mic".to_string(),
                kind: "audio_input_capture".to_string(),
                uuid: "input-uuid".to_string(),
                muted: false,
                volume: 0.5,
            }],
            transitions: vec![
                ObsTransition {
                    name: "Cut".to_string(),
                    kind: "cut".to_string(),
                    duration: 0,
                },
                ObsTransition {
                    name: "Fade".to_string(),
                    kind: "fade".to_string(),
                    duration: 0,
                },
            ],
            scene_items: vec![ObsSceneItem {
                id: 1,
                name: "Camera".to_string(),
                enabled: true,
            }],
        }
    }
}

pub struct ObsController {
    client: Arc<RwLock<Option<Client>>>,
    #[cfg(test)]
    state: Arc<RwLock<ObsTestState>>,
    connection_info: ObsConnectionInfo,
}

#[allow(unreachable_code)]
impl ObsController {
    pub fn new() -> Self {
        #[cfg(not(test))]
        let state = {
            Self {
                client: Arc::new(RwLock::new(None)),
                connection_info: ObsConnectionInfo::default(),
            }
        };

        #[cfg(test)]
        let state = {
            Self {
                client: Arc::new(RwLock::new(None)),
                state: Arc::new(RwLock::new(ObsTestState::default())),
                connection_info: ObsConnectionInfo::default(),
            }
        };

        state
    }

    #[cfg(test)]
    pub(crate) fn with_test_state(state: ObsTestState) -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(state)),
            connection_info: ObsConnectionInfo::default(),
        }
    }

    pub async fn connect(
        &mut self,
        host: &str,
        port: u16,
        password: Option<&str>,
    ) -> Result<(), String> {
        #[cfg(test)]
        {
            let _ = password;
            let mut state = self.state.write().await;
            state.connected = true;
            state.host = host.to_string();
            state.port = port;
            self.connection_info = ObsConnectionInfo {
                host: host.to_string(),
                port,
                connected: true,
            };
            return Ok(());
        }

        let client = Client::connect(host, port, password)
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;

        *self.client.write().await = Some(client);
        self.connection_info = ObsConnectionInfo {
            host: host.to_string(),
            port,
            connected: true,
        };
        Ok(())
    }

    pub async fn disconnect(&mut self) {
        #[cfg(test)]
        {
            let mut state = self.state.write().await;
            state.connected = false;
            self.connection_info.connected = false;
            return;
        }

        *self.client.write().await = None;
        self.connection_info.connected = false;
    }

    pub fn is_connected(&self) -> bool {
        self.connection_info.connected
    }

    pub fn connection_info(&self) -> &ObsConnectionInfo {
        &self.connection_info
    }

    pub async fn get_stream_status(&self) -> Result<ObsStreamStatus, String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            let state = self.state.read().await;
            return Ok(ObsStreamStatus {
                active: state.stream_active,
                reconnecting: false,
                timecode: "00:00:00:00".to_string(),
                duration: 0,
                bytes: 0,
            });
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        let status = client
            .streaming()
            .status()
            .await
            .map_err(|e| format!("{}", e))?;
        Ok(ObsStreamStatus {
            active: status.active,
            reconnecting: status.reconnecting,
            timecode: format!("{}", status.timecode),
            duration: status.duration.whole_milliseconds() as u64,
            bytes: status.bytes,
        })
    }

    pub async fn start_stream(&self) -> Result<(), String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            self.state.write().await.stream_active = true;
            return Ok(());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .streaming()
            .start()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn stop_stream(&self) -> Result<(), String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            self.state.write().await.stream_active = false;
            return Ok(());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .streaming()
            .stop()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_record_status(&self) -> Result<ObsRecordStatus, String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            let state = self.state.read().await;
            return Ok(ObsRecordStatus {
                active: state.record_active,
                paused: state.record_paused,
                timecode: "00:00:00:00".to_string(),
                duration: 0,
                bytes: 0,
            });
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        let status = client
            .recording()
            .status()
            .await
            .map_err(|e| format!("{}", e))?;
        Ok(ObsRecordStatus {
            active: status.active,
            paused: status.paused,
            timecode: format!("{}", status.timecode),
            duration: status.duration.whole_milliseconds() as u64,
            bytes: status.bytes,
        })
    }

    pub async fn start_record(&self) -> Result<(), String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            let mut state = self.state.write().await;
            state.record_active = true;
            state.record_paused = false;
            return Ok(());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .recording()
            .start()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn stop_record(&self) -> Result<String, String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            self.state.write().await.record_active = false;
            return Ok("stopped".to_string());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .recording()
            .stop()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn toggle_record_pause(&self) -> Result<bool, String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            let mut state = self.state.write().await;
            state.record_paused = !state.record_paused;
            return Ok(state.record_paused);
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .recording()
            .toggle_pause()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_scene_list(&self) -> Result<(String, Vec<ObsScene>), String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            let state = self.state.read().await;
            return Ok((state.current_scene.clone(), state.scenes.clone()));
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        let list = client.scenes().list().await.map_err(|e| format!("{}", e))?;
        let current = list
            .current_program_scene
            .map(|s| s.name)
            .unwrap_or_default();
        let scenes = list
            .scenes
            .into_iter()
            .map(|s| ObsScene {
                name: s.id.name,
                index: s.index as i32,
            })
            .collect();
        Ok((current, scenes))
    }

    pub async fn set_current_scene(&self, name: &str) -> Result<(), String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            self.state.write().await.current_scene = name.to_string();
            return Ok(());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .scenes()
            .set_current_program_scene(name)
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_input_list(&self) -> Result<Vec<ObsInput>, String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            return Ok(self.state.read().await.inputs.clone());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        let inputs = client
            .inputs()
            .list(None)
            .await
            .map_err(|e| format!("{}", e))?;
        let mut result = Vec::new();
        for input in inputs {
            let name = &input.id.name;
            let input_id: InputId = name.as_str().into();
            let muted = client.inputs().muted(input_id).await.unwrap_or(false);
            let volume = client
                .inputs()
                .volume(input_id)
                .await
                .map(|v| v.mul as f64)
                .unwrap_or(0.0);
            result.push(ObsInput {
                name: input.id.name,
                kind: input.kind,
                uuid: input.id.uuid.to_string(),
                muted,
                volume,
            });
        }
        Ok(result)
    }

    pub async fn set_input_volume(&self, name: &str, volume: f64) -> Result<(), String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            let mut state = self.state.write().await;
            if let Some(input) = state.inputs.iter_mut().find(|input| input.name == name) {
                input.volume = volume;
            }
            return Ok(());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        let input_id: InputId = name.into();
        client
            .inputs()
            .set_volume(input_id, Volume::Mul(volume as f32))
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn set_input_mute(&self, name: &str, muted: bool) -> Result<(), String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            let mut state = self.state.write().await;
            if let Some(input) = state.inputs.iter_mut().find(|input| input.name == name) {
                input.muted = muted;
            }
            return Ok(());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        let input_id: InputId = name.into();
        client
            .inputs()
            .set_muted(input_id, muted)
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_virtual_cam_status(&self) -> Result<bool, String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            return Ok(self.state.read().await.virtual_cam_active);
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .virtual_cam()
            .status()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn toggle_virtual_cam(&self) -> Result<bool, String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            let mut state = self.state.write().await;
            state.virtual_cam_active = !state.virtual_cam_active;
            return Ok(state.virtual_cam_active);
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .virtual_cam()
            .toggle()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_replay_buffer_status(&self) -> Result<bool, String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            return Ok(self.state.read().await.replay_buffer_active);
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .replay_buffer()
            .status()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn save_replay_buffer(&self) -> Result<(), String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            self.state.write().await.replay_buffer_active = true;
            return Ok(());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .replay_buffer()
            .save()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_transitions(&self) -> Result<Vec<ObsTransition>, String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            return Ok(self.state.read().await.transitions.clone());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        let list = client
            .transitions()
            .list()
            .await
            .map_err(|e| format!("{}", e))?;
        let transitions = list
            .transitions
            .into_iter()
            .map(|t| ObsTransition {
                name: t.id.name,
                kind: t.kind,
                duration: 0,
            })
            .collect();
        Ok(transitions)
    }

    pub async fn set_transition(&self, name: &str) -> Result<(), String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            let mut state = self.state.write().await;
            if let Some(transition) = state.transitions.iter_mut().find(|t| t.name == name) {
                transition.kind = "custom".to_string();
            } else {
                state.transitions.push(ObsTransition {
                    name: name.to_string(),
                    kind: "custom".to_string(),
                    duration: 0,
                });
            }
            return Ok(());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .transitions()
            .set_current(name)
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_scene_item_list(&self, scene_name: &str) -> Result<Vec<ObsSceneItem>, String> {
        #[cfg(test)]
        {
            let _ = scene_name;
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            return Ok(self.state.read().await.scene_items.clone());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        let scene_id: SceneId = scene_name.into();
        let items = client
            .scene_items()
            .list(scene_id)
            .await
            .map_err(|e| format!("{}", e))?;
        let mut scene_items = Vec::new();
        for item in items {
            let enabled = client
                .scene_items()
                .enabled(scene_id, item.id)
                .await
                .unwrap_or(false);
            scene_items.push(ObsSceneItem {
                id: item.id as i32,
                name: item.source_name,
                enabled,
            });
        }
        Ok(scene_items)
    }

    pub async fn set_scene_item_enabled(
        &self,
        scene_name: &str,
        item_id: i32,
        enabled: bool,
    ) -> Result<(), String> {
        #[cfg(test)]
        {
            let _ = scene_name;
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            let mut state = self.state.write().await;
            if let Some(item) = state.scene_items.iter_mut().find(|item| item.id == item_id) {
                item.enabled = enabled;
            }
            return Ok(());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        let scene_id: SceneId = scene_name.into();
        client
            .scene_items()
            .set_enabled(SetEnabled {
                scene: scene_id,
                item_id: item_id as i64,
                enabled,
            })
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_studio_mode(&self) -> Result<bool, String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            return Ok(self.state.read().await.studio_mode);
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .ui()
            .studio_mode_enabled()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn set_studio_mode(&self, enabled: bool) -> Result<(), String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            self.state.write().await.studio_mode = enabled;
            return Ok(());
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .ui()
            .set_studio_mode_enabled(enabled)
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_stats(&self) -> Result<ObsStats, String> {
        #[cfg(test)]
        {
            if !self.connection_info.connected {
                return Err("Not connected".to_string());
            }
            let state = self.state.read().await;
            return Ok(ObsStats {
                cpu_usage: state.cpu_usage,
                memory_usage: state.memory_usage,
                fps: state.fps,
                render_skipped: 0,
                render_total: 0,
                output_skipped: 0,
                output_total: 0,
            });
        }

        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        let stats = client
            .general()
            .stats()
            .await
            .map_err(|e| format!("{}", e))?;
        Ok(ObsStats {
            cpu_usage: stats.cpu_usage,
            memory_usage: stats.memory_usage,
            fps: stats.active_fps,
            render_skipped: stats.render_skipped_frames,
            render_total: stats.render_total_frames,
            output_skipped: stats.output_skipped_frames,
            output_total: stats.output_total_frames,
        })
    }
}
