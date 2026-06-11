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

pub struct ObsController {
    client: Arc<RwLock<Option<Client>>>,
    connection_info: ObsConnectionInfo,
}

impl ObsController {
    pub fn new() -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            connection_info: ObsConnectionInfo::default(),
        }
    }

    pub async fn connect(
        &mut self,
        host: &str,
        port: u16,
        password: Option<&str>,
    ) -> Result<(), String> {
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
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .streaming()
            .start()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn stop_stream(&self) -> Result<(), String> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .streaming()
            .stop()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_record_status(&self) -> Result<ObsRecordStatus, String> {
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
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .recording()
            .start()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn stop_record(&self) -> Result<String, String> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .recording()
            .stop()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn toggle_record_pause(&self) -> Result<bool, String> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .recording()
            .toggle_pause()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_scene_list(&self) -> Result<(String, Vec<ObsScene>), String> {
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
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .scenes()
            .set_current_program_scene(name)
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_input_list(&self) -> Result<Vec<ObsInput>, String> {
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
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .virtual_cam()
            .status()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn toggle_virtual_cam(&self) -> Result<bool, String> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .virtual_cam()
            .toggle()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_replay_buffer_status(&self) -> Result<bool, String> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .replay_buffer()
            .status()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn save_replay_buffer(&self) -> Result<(), String> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .replay_buffer()
            .save()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_transitions(&self) -> Result<Vec<ObsTransition>, String> {
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
            })
            .collect();
        Ok(transitions)
    }

    pub async fn set_transition(&self, name: &str) -> Result<(), String> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .transitions()
            .set_current(name)
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_scene_item_list(&self, scene_name: &str) -> Result<Vec<ObsSceneItem>, String> {
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
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .ui()
            .studio_mode_enabled()
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn set_studio_mode(&self, enabled: bool) -> Result<(), String> {
        let guard = self.client.read().await;
        let client = guard.as_ref().ok_or("Not connected")?;
        client
            .ui()
            .set_studio_mode_enabled(enabled)
            .await
            .map_err(|e| format!("{}", e))
    }

    pub async fn get_stats(&self) -> Result<ObsStats, String> {
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
