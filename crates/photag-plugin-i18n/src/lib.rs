//! Fluent-backed English/Chinese localization owned by Cordis.
#[cfg(test)]
mod tests;
mod view;

use cordis::{Context, Plugin, Service};
use fluent_bundle::{FluentArgs, FluentResource, concurrent::FluentBundle};
use photag_ui_bridge::i18n::Translator;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-CN")]
    Chinese,
}

impl Language {
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Chinese => "zh-CN",
        }
    }
    pub fn parse(code: &str) -> Result<Self, I18nError> {
        match code {
            "en" => Ok(Self::English),
            "zh-CN" => Ok(Self::Chinese),
            _ => Err(I18nError::Language(code.to_owned())),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Preference {
    #[serde(default)]
    language: Language,
}

pub struct I18nService {
    language: RwLock<Language>,
    path: PathBuf,
    english: FluentBundle<FluentResource>,
    chinese: FluentBundle<FluentResource>,
    keys: HashMap<String, String>,
}

impl Service for I18nService {
    const NAME: &'static str = "photag/i18n";
}

fn bundle(locale: &str, source: &str) -> Result<FluentBundle<FluentResource>, I18nError> {
    let resource = FluentResource::try_new(source.to_owned())
        .map_err(|(_, errors)| I18nError::Catalog(format!("{locale}: {errors:?}")))?;
    let mut bundle = FluentBundle::new_concurrent(vec![
        locale
            .parse()
            .map_err(|error| I18nError::Catalog(format!("{error}")))?,
    ]);
    bundle.set_use_isolating(false);
    bundle
        .add_resource(resource)
        .map_err(|errors| I18nError::Catalog(format!("{locale}: {errors:?}")))?;
    Ok(bundle)
}

impl I18nService {
    pub fn open(path: PathBuf) -> Result<Self, I18nError> {
        let preference = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Preference>(&bytes).ok())
            .unwrap_or_default();
        Ok(Self {
            language: RwLock::new(preference.language),
            path,
            english: bundle("en", include_str!("../locales/en.ftl"))?,
            chinese: bundle("zh-CN", include_str!("../locales/zh-CN.ftl"))?,
            keys: serde_json::from_str(include_str!("../locales/keys.json"))
                .map_err(|error| I18nError::Catalog(error.to_string()))?,
        })
    }

    pub fn language(&self) -> Language {
        *self.language.read().expect("language lock")
    }

    pub fn set_language(&self, language: Language) -> Result<(), I18nError> {
        let mut current = self.language.write().expect("language lock");
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_vec_pretty(&Preference { language })
            .map_err(|error| I18nError::Catalog(error.to_string()))?;
        // Persist before changing visible state, so write failures do not lie.
        std::fs::write(&self.path, json)?;
        *current = language;
        photag_ui_bridge::i18n::changed();
        Ok(())
    }

    fn format(
        &self,
        bundle: &FluentBundle<FluentResource>,
        key: &str,
        args: &FluentArgs<'_>,
    ) -> Option<String> {
        let pattern = bundle.get_message(key)?.value()?;
        let mut errors = Vec::new();
        let result = bundle
            .format_pattern(pattern, Some(args), &mut errors)
            .into_owned();
        errors.is_empty().then_some(result)
    }
}

impl Translator for I18nService {
    fn translate(&self, source: &str, values: &[(&str, String)]) -> String {
        let Some(key) = self.keys.get(source) else {
            for prefix in ["• ", "● ", "✓ ", "○ ", "⊘ ", "⇅ "] {
                if let Some(label) = source.strip_prefix(prefix) {
                    return format!("{prefix}{}", self.translate(label, values));
                }
            }
            return source.to_owned();
        };
        let mut args = FluentArgs::new();
        for (name, value) in values {
            if ["count", "photos", "videos", "total"].contains(name) {
                if let Ok(number) = value.parse::<f64>() {
                    args.set(*name, number);
                    continue;
                }
            }
            args.set(*name, value.as_str());
        }
        let localized = if self.language() == Language::Chinese {
            self.format(&self.chinese, key, &args)
        } else {
            None
        };
        localized
            .or_else(|| self.format(&self.english, key, &args))
            .unwrap_or_else(|| source.to_owned())
    }
}

pub struct I18nPlugin;
#[derive(Default)]
pub struct I18nConfig {
    pub path: Option<PathBuf>,
}

impl Plugin for I18nPlugin {
    type Config = I18nConfig;
    type Input = I18nConfig;
    type PrepareError = std::convert::Infallible;
    type ApplyError = I18nError;
    fn name(&self) -> std::borrow::Cow<'_, str> {
        "photag.i18n".into()
    }
    fn prepare(&self, config: Self::Config) -> Result<Self::Input, Self::PrepareError> {
        Ok(config)
    }
    #[tracing::instrument(skip_all, err)]
    async fn apply(&self, context: Context, config: &Self::Input) -> Result<(), Self::ApplyError> {
        let path = config.path.clone().unwrap_or_else(|| {
            dirs::data_local_dir()
                .unwrap_or_default()
                .join("PhoTag/language.json")
        });
        let service = Arc::new(I18nService::open(path)?);
        let translator: Arc<dyn Translator> = service.clone();
        let _publication = context
            .provide(service.clone())
            .map_err(|error| I18nError::Lifecycle(error.to_string()))?;
        context
            .effect_sync(|| photag_ui_bridge::i18n::install(None))
            .map_err(|error| I18nError::Lifecycle(error.to_string()))?;
        photag_ui_bridge::i18n::install(Some(Arc::downgrade(&translator)));
        photag_ui_bridge::register_views_for_plugin(
            &context,
            "photag.i18n",
            [view::language(Arc::downgrade(&service))],
        )
        .map_err(|error| I18nError::Lifecycle(error.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum I18nError {
    #[error("unsupported language: {0}")]
    Language(String),
    #[error("invalid Fluent catalog: {0}")]
    Catalog(String),
    #[error("i18n lifecycle: {0}")]
    Lifecycle(String),
    #[error("could not save language: {0}")]
    Io(#[from] std::io::Error),
}
