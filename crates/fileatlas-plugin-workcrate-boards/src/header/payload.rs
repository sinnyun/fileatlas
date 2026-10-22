use super::*;

/// Structured values in the native aggregate node's note; display text is never parsed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Details {
    version: u32,
    material: String,
    size: String,
    quantity: String,
    extra: BTreeMap<String, String>,
}
impl Details {
    pub(super) fn from_info(info: &PageInfo) -> Self {
        Self {
            version: 1,
            material: info.material.clone(),
            size: info.size.clone(),
            quantity: info.quantity.clone(),
            extra: info.extra.clone(),
        }
    }
    pub(super) fn text(&self) -> String {
        [&self.material, &self.size, &self.quantity]
            .into_iter()
            .chain(self.extra.values())
            .map(|value| single_line(value))
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" | ")
    }
    pub(crate) fn read(note: &Value, info: &mut PageInfo) -> Result<(), BoardsError> {
        let value: Self = serde_json::from_value(note["details"].clone())
            .map_err(|_| BoardsError::Invalid("物料栏字段记录损坏，请从备份恢复原生工程".into()))?;
        if value.version != 1 {
            return Err(BoardsError::Invalid(
                "物料栏字段版本暂不支持，请升级程序".into(),
            ));
        }
        info.material = value.material;
        info.size = value.size;
        info.quantity = value.quantity;
        info.extra = value.extra;
        Ok(())
    }
}
pub(super) fn single_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregate_is_one_line_and_metadata_round_trips_separators_and_empty_fields()
    -> Result<(), BoardsError> {
        let info = PageInfo {
            material: "背景 | 展架".into(),
            size: "20 × 30 cm".into(),
            quantity: "1 套".into(),
            extra: BTreeMap::from([
                ("工艺".into(), "UV\n覆膜".into()),
                ("空字段".into(), "".into()),
            ]),
            ..Default::default()
        };
        let details = Details::from_info(&info);
        assert_eq!(details.text(), "背景 | 展架 | 20 × 30 cm | 1 套 | UV 覆膜");
        let mut restored = PageInfo::default();
        Details::read(&json!({"details": details}), &mut restored)?;
        assert_eq!(restored.extra, info.extra);
        assert_eq!(restored.material, info.material);
        assert_eq!(Details::from_info(&PageInfo::default()).text(), "");
        assert!(Details::read(&json!({"details":{"version":99}}), &mut restored).is_err());
        Ok(())
    }
}
