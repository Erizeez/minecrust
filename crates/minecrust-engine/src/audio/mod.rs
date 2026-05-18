use kira::{AudioManager as KiraAudioManager, AudioManagerSettings, DefaultBackend, Tween};
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings, StaticSoundHandle};
use std::time::Duration;

pub struct AudioManager {
    manager: KiraAudioManager<DefaultBackend>,
    current_music: Option<StaticSoundHandle>,
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            manager: KiraAudioManager::new(AudioManagerSettings::default()).unwrap(),
            current_music: None,
        }
    }

    pub fn play_music(&mut self, path: &str) {
        if let Some(ref mut music) = self.current_music {
            // Fade out the current music over 2 seconds
            music.stop(Tween {
                duration: Duration::from_secs(2),
                ..Default::default()
            });
        }

        // Load the new sound data
        let mut sound_data = StaticSoundData::from_file(path).unwrap();
        
        // Ensure the track loops indefinitely and has a fade-in
        sound_data.settings = sound_data.settings
            .loop_region(..)
            .fade_in_tween(Some(Tween {
                duration: Duration::from_secs(2),
                ..Default::default()
            }));

        // Play the new track with a 2-second fade-in
        let handle = self.manager.play(sound_data).unwrap();
        
        self.current_music = Some(handle);
    }
}
