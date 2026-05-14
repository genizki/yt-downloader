use crate::settings::AppSettings;

pub trait SettingsInterface {
    fn settings(&self) -> &AppSettings;
    fn settings_mut(&mut self) -> &mut AppSettings;
    fn save_settings(&self) -> anyhow::Result<()>;
}
