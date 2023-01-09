#![allow(dead_code)]
#![cfg_attr(feature = "unstable", feature(debugger_visualizer), debugger_visualizer(natvis_file = "../dpx_dicom_core.natvis"))]
#![cfg_attr(feature = "unstable", feature(is_sorted))]

// Module declarations
pub mod tag;
pub mod charset;
pub mod config;
pub mod settings;
pub mod vr;

// Public re-exports
pub use vr::Vr;
pub use tag::TagKey;
pub use tag::Tag;

// Crate STD lib types
pub(crate) type Arc<T> = std::sync::Arc<T>;
pub(crate) type Cow<'lifetime, T> = std::borrow::Cow<'lifetime, T>;
pub(crate) type HashMap<K, V> = std::collections::HashMap<K, V>;
pub(crate) type Map<K, V> = std::collections::BTreeMap<K, V>;
pub(crate) type RwLock<T> = std::sync::RwLock<T>;
pub(crate) type Vec<T> = std::vec::Vec<T>;

use settings::ConditionalSettings;


#[derive(Debug)]
pub struct Registry {
    settings_registry: settings::Registry,
    conditional_settings: RwLock<settings::ConditionalSettings>,
}

impl Registry {
    fn new() -> Registry {
        Registry {
            settings_registry: settings::Registry::new(),
            conditional_settings: RwLock::new(settings::ConditionalSettings::new()),
        }
    }
    fn new_empty() -> Registry {
        Registry {
            settings_registry: settings::Registry::new_empty(),
            conditional_settings: RwLock::new(settings::ConditionalSettings::new()),
        }
    }
    pub fn settings_registry(&self) -> &settings::Registry {
        &self.settings_registry
    }
    pub fn settings_registry_mut(&mut self) -> &mut settings::Registry {
        &mut self.settings_registry
    }
    pub fn conditional_settings(&self) -> &RwLock<settings::ConditionalSettings> {
        &self.conditional_settings
    }
    pub fn set_conditional_settings(&self, s: ConditionalSettings) {
        *self.conditional_settings.write().unwrap() = s;
    }
}

#[derive(Debug, Clone)]
pub struct Dicom {
    globals: Arc<Registry>,
    settings: Arc<settings::Settings>,
}

impl Dicom {
    pub fn new() -> Dicom {
        Dicom {
            globals: Arc::new(Registry::new()),
            settings: Arc::new(settings::Settings::new()),
        }
    }
    pub fn new_empty() -> Dicom {
        Dicom {
            globals: Arc::new(Registry::new_empty()),
            settings: Arc::new(settings::Settings::new()),
        }
    }

    /*pub fn override_settings(self, s: Settings) -> Self {

    }*/

    pub fn globals(&self) -> &Registry {
        &self.globals
    }
    pub fn try_globals_mut(&mut self) -> Option<&mut Registry> {
        Arc::get_mut(&mut self.globals)
    }
    pub fn settings(&self) -> &settings::Settings {
        &self.settings
    }
    pub fn settings_mut(&mut self) -> &mut settings::Settings {
        Arc::make_mut(&mut self.settings)
    }
    pub fn setting_value<'a>(&'a self, key: &'_ settings::Key) -> Option<&'a settings::Value> {
        self.settings
            .get(key)
            .or_else(|| self.globals.settings_registry.default_value_of(key))
    }
}
