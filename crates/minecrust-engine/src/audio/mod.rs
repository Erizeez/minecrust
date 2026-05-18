use kira::AudioManager as KiraAudioManager;
use kira::AudioManagerSettings;
use kira::DefaultBackend;

pub struct AudioManager {
    manager: KiraAudioManager<DefaultBackend>,
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            manager: KiraAudioManager::new(AudioManagerSettings::default()).unwrap(),
        }
    }

    // Future: 3D spatial audio
    // pub fn play_at_location(...)
}
