use super::*;

impl WorkCrateProjectsService {
    /// Memory-only snapshot; an empty project starts with empty builtin columns.
    pub fn project_field_presets(&self, id: &str) -> Result<FieldPresets, WorkCrateProjectsError> {
        self.ensure_project(id)?;
        Ok(self
            .project_presets
            .read()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?
            .get(id)
            .cloned()
            .unwrap_or_else(|| {
                let mut value = FieldPresets::default();
                for field in &mut value.fields {
                    field.values.clear();
                }
                value
            }))
    }

    pub fn save_project_field_presets(
        &self,
        id: &str,
        mut value: FieldPresets,
    ) -> Result<(), WorkCrateProjectsError> {
        for field in &mut value.fields {
            field.label = field.label.trim().to_owned();
            field.values = field.values.iter().map(|v| v.trim().to_owned()).collect();
        }
        presets::validate(&value)?;
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        self.ensure_project(id)?;
        let mut document = self.read_project(id)?;
        document.field_presets = Some(value);
        self.write_project(&document)?;
        self.changed();
        Ok(())
    }

    /// Union by stable field key; project options come first, duplicates appear once.
    pub fn effective_field_presets(
        &self,
        id: &str,
    ) -> Result<FieldPresets, WorkCrateProjectsError> {
        let mut project = self.project_field_presets(id)?;
        merge(&mut project, self.field_presets()?.fields);
        Ok(project)
    }

    /// Promote just one saved project column, preserving unrelated program defaults.
    pub fn promote_project_field(&self, id: &str, key: &str) -> Result<(), WorkCrateProjectsError> {
        let _gate = self
            .write_gate
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)?;
        let field = self
            .project_field_presets(id)?
            .fields
            .into_iter()
            .find(|f| f.key == key)
            .ok_or(WorkCrateProjectsError::Invalid("项目默认信息中没有此字段"))?;
        if field.values.is_empty() {
            return Err(WorkCrateProjectsError::Invalid("此列没有可提升的默认信息"));
        }
        let mut global = self.field_presets()?;
        merge(&mut global, vec![field]);
        presets::validate(&global)?;
        atomic_write_json(&self.root.join("field-presets.json"), &global)?;
        *self
            .presets
            .write()
            .map_err(|_| WorkCrateProjectsError::Poisoned)? = global;
        self.changed();
        Ok(())
    }
}

fn merge(target: &mut FieldPresets, fields: Vec<FieldPreset>) {
    for source in fields {
        if let Some(field) = target.fields.iter_mut().find(|f| f.key == source.key) {
            for value in source.values {
                if !field.values.contains(&value) {
                    field.values.push(value);
                }
            }
        } else {
            target.fields.push(source);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_isolation_promotion_reopen_and_validation() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempfile::tempdir()?;
        let service = WorkCrateProjectsService::open_at(dir.path())?;
        let a = service.create_project("A")?;
        let b = service.create_project("B")?;
        let original = service.field_presets()?;
        let mut local = service.project_field_presets(&a.id)?;
        local.fields[0].values = vec!["活动物料".into()];
        local.fields[1].values = vec!["专用展架".into(), "主题背景".into()];
        service.save_project_field_presets(&a.id, local.clone())?;
        let native = dir.path().join("a.vectorcraft");
        std::fs::write(&native, b"native document fixture")?;
        service.set_document_path(&a.id, &native)?;
        let destination = dir.path().join("copies");
        std::fs::create_dir(&destination)?;
        let copied = service.duplicate_project_document(&a.id, "Copy", &destination)?;
        assert_eq!(service.project_field_presets(&copied.id)?, local);
        assert!(
            service.project_field_presets(&b.id)?.fields[0]
                .values
                .is_empty()
        );
        assert_eq!(service.field_presets()?, original);
        let effective = service.effective_field_presets(&a.id)?;
        assert_eq!(effective.fields[1].values[0], "专用展架");
        assert_eq!(
            effective.fields[1]
                .values
                .iter()
                .filter(|v| *v == "主题背景")
                .count(),
            1
        );
        service.promote_project_field(&a.id, "material")?;
        assert_eq!(service.field_presets()?.fields[0], original.fields[0]);
        assert!(
            service.field_presets()?.fields[1]
                .values
                .contains(&"专用展架".into())
        );
        local.fields[0].values.push("活动物料".into());
        assert!(service.save_project_field_presets(&a.id, local).is_err());
        drop(service);
        let reopened = WorkCrateProjectsService::open_at(dir.path())?;
        assert_eq!(
            reopened.project_field_presets(&a.id)?.fields[0].values,
            vec!["活动物料"]
        );
        assert!(reopened.promote_project_field(&a.id, "size").is_err());
        Ok(())
    }
}
