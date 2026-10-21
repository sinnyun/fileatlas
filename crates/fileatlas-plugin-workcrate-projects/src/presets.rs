use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldPreset {
    /// Builtin local/material/size/quantity, or extra:<human-readable field name>.
    pub key: String,
    pub label: String,
    pub values: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldPresets {
    pub version: u32,
    pub fields: Vec<FieldPreset>,
}

impl Default for FieldPresets {
    fn default() -> Self {
        Self {
            version: 1,
            fields: [
                ("local", "项目名称", vec![]),
                (
                    "material",
                    "物料名称",
                    vec!["主题背景", "人物立牌", "指引牌"],
                ),
                ("size", "尺寸", vec![]),
                ("quantity", "数量", vec!["1 套", "2 套", "10 套", "20 套"]),
            ]
            .into_iter()
            .map(|(key, label, values)| FieldPreset {
                key: key.into(),
                label: label.into(),
                values: values.into_iter().map(String::from).collect(),
            })
            .collect(),
        }
    }
}

pub(super) fn read(root: &Path) -> Result<FieldPresets, WorkCrateProjectsError> {
    let path = root.join("field-presets.json");
    let result = if path.exists() || path.with_extension("json.bak").exists() {
        read_json_with_backup(&path)?.0
    } else {
        FieldPresets::default()
    };
    validate(&result)?;
    Ok(result)
}

fn validate(presets: &FieldPresets) -> Result<(), WorkCrateProjectsError> {
    if presets.version != 1 {
        return Err(WorkCrateProjectsError::UnsupportedVersion(presets.version));
    }
    if presets.fields.len() > 64 {
        return Err(WorkCrateProjectsError::Invalid("最多支持 64 个字段"));
    }
    let mut keys = std::collections::BTreeSet::new();
    let mut labels = std::collections::BTreeSet::new();
    for field in &presets.fields {
        let builtin = ["local", "material", "size", "quantity"].contains(&field.key.as_str());
        if field.label.trim().is_empty()
            || field.label.chars().count() > 60
            || (!builtin && field.key != format!("extra:{}", field.label))
            || !keys.insert(&field.key)
            || !labels.insert(&field.label)
            || field.values.len() > 128
            || field
                .values
                .iter()
                .any(|v| v.trim().is_empty() || v.chars().count() > 500)
            || field
                .values
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != field.values.len()
        {
            return Err(WorkCrateProjectsError::Invalid(
                "字段名称、默认信息为空、重复或超出限制",
            ));
        }
    }
    if ["local", "material", "size", "quantity"]
        .iter()
        .any(|key| !presets.fields.iter().any(|f| f.key == *key))
    {
        return Err(WorkCrateProjectsError::Invalid("基础页面字段不能删除"));
    }
    Ok(())
}

impl WorkCrateProjectsService {
    pub fn field_presets(&self) -> Result<FieldPresets, WorkCrateProjectsError> {
        self.presets
            .read()
            .map(|p| p.clone())
            .map_err(|_| WorkCrateProjectsError::Poisoned)
    }

    pub fn save_field_presets(
        &self,
        mut presets: FieldPresets,
    ) -> Result<(), WorkCrateProjectsError> {
        for field in &mut presets.fields {
            field.label = field.label.trim().to_owned();
            field.values = field.values.iter().map(|v| v.trim().to_owned()).collect();
        }
        validate(&presets)?;
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        atomic_write_json(&self.root.join("field-presets.json"), &presets)?;
        *self
            .presets
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)? = presets;
        self.revision.fetch_add(1, Ordering::AcqRel);
        self.wake.trigger();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multiple_fields_and_choices_persist_and_duplicate_values_are_rejected()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let service = WorkCrateProjectsService::open_at(dir.path())?;
        let mut presets = service.field_presets()?;
        presets.fields.push(FieldPreset {
            key: "extra:制作工艺".into(),
            label: "制作工艺".into(),
            values: vec!["写真覆膜".into(), "UV 打印".into()],
        });
        service.save_field_presets(presets.clone())?;
        drop(service);
        let reopened = WorkCrateProjectsService::open_at(dir.path())?;
        assert_eq!(reopened.field_presets()?, presets);
        presets.fields[0].values = vec!["重复".into(), "重复".into()];
        assert!(reopened.save_field_presets(presets).is_err());
        assert_eq!(
            reopened
                .field_presets()?
                .fields
                .last()
                .ok_or("no field")?
                .values
                .len(),
            2
        );
        Ok(())
    }
}
