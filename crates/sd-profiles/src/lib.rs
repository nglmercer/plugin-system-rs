use sd_types::{Profile, ProfileId};
use sd_events::{EventBus, StreamEvent};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ProfileManager {
    profiles: Arc<RwLock<HashMap<ProfileId, Profile>>>,
    active_profile: Arc<RwLock<Option<ProfileId>>>,
    events: Arc<EventBus>,
}

impl ProfileManager {
    pub fn new(events: Arc<EventBus>) -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
            active_profile: Arc::new(RwLock::new(None)),
            events,
        }
    }

    pub async fn create_profile(&self, name: impl Into<String>) -> ProfileId {
        let profile = Profile::new(name);
        let id = profile.id.clone();
        let mut profiles = self.profiles.write().await;
        profiles.insert(id.clone(), profile);
        id
    }

    pub async fn get_profile(&self, id: &ProfileId) -> Option<Profile> {
        let profiles = self.profiles.read().await;
        profiles.get(id).cloned()
    }

    pub async fn list_profiles(&self) -> Vec<Profile> {
        let profiles = self.profiles.read().await;
        profiles.values().cloned().collect()
    }

    pub async fn update_profile(&self, profile: Profile) -> bool {
        let mut profiles = self.profiles.write().await;
        if profiles.contains_key(&profile.id) {
            profiles.insert(profile.id.clone(), profile);
            true
        } else {
            false
        }
    }

    pub async fn delete_profile(&self, id: &ProfileId) -> bool {
        let mut profiles = self.profiles.write().await;
        profiles.remove(id).is_some()
    }

    pub async fn set_active_profile(&self, id: ProfileId) -> bool {
        let profiles = self.profiles.read().await;
        if profiles.contains_key(&id) {
            drop(profiles);
            let mut active = self.active_profile.write().await;
            *active = Some(id.clone());
            self.events.emit(StreamEvent::ProfileChanged { profile: id });
            true
        } else {
            false
        }
    }

    pub async fn get_active_profile(&self) -> Option<Profile> {
        let active = self.active_profile.read().await;
        if let Some(id) = active.as_ref() {
            let profiles = self.profiles.read().await;
            profiles.get(id).cloned()
        } else {
            None
        }
    }
}
