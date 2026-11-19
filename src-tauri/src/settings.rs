use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub model: String,
    pub compute_type: String,
    pub language: String,
    pub microphone: String,
    pub shortcut: String,
    pub mode: String,
    pub remove_fillers: bool,
    pub auto_insert: bool,
    pub vad: bool,
    pub live_preview: bool,
    pub terminology: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: "base".into(),
            compute_type: "int8".into(),
            language: "auto".into(),
            microphone: String::new(),
            shortcut: "Ctrl+Alt+Space".into(),
            mode: "verbatim".into(),
            remove_fillers: false,
            auto_insert: false,
            vad: true,
            live_preview: false,
            terminology: String::new(),
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !["tiny", "base", "small"].contains(&self.model.as_str())
            || !["int8", "float32"].contains(&self.compute_type.as_str())
            || !["auto", "en", "hi"].contains(&self.language.as_str())
            || !["verbatim", "clean", "paragraphs"].contains(&self.mode.as_str())
        {
            return Err("Invalid model, precision, language or writing mode.".into());
        }
        if self.microphone.len() > 512 || self.shortcut.len() > 80 || self.terminology.len() > 2000
        {
            return Err("Setting exceeds its size limit.".into());
        }
        if self.shortcut.split('+').count() < 2 || self.shortcut.contains("Escape") {
            return Err("Use a shortcut with a modifier, such as Ctrl+Alt+Space.".into());
        }
        Ok(())
    }

    pub fn read(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let settings: Self = serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| {
                format!(
                    "Settings are invalid. Restore or remove {}: {e}",
                    path.display()
                )
            })?;
        settings.validate()?;
        Ok(settings)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        use std::io::Write;
        self.validate()?;
        let directory = path.parent().ok_or("Settings directory missing")?;
        fs::create_dir_all(directory).map_err(|e| e.to_string())?;
        let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.as_file().sync_all().map_err(|e| e.to_string())?;
        file.persist(path).map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_settings_replace_and_validation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        let mut settings = Settings::default();
        settings.save(&path).unwrap();
        settings.model = "tiny".into();
        settings.save(&path).unwrap();
        assert_eq!(Settings::read(&path).unwrap().model, "tiny");
        settings.model = "unknown".into();
        assert!(settings.save(&path).is_err());
        assert_eq!(Settings::read(&path).unwrap().model, "tiny");
    }
    #[test]
    fn corrupt_settings_are_reported() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        fs::write(&path, "{").unwrap();
        assert!(Settings::read(&path).is_err());
    }
}
