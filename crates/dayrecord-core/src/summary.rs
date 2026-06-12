use crate::CoreError;

pub const REQUIRED_SECTIONS: [&str; 4] = [
    "## 今日概览",
    "## 主要工作内容",
    "## 重要粘贴片段摘要",
    "## 明日待办",
];

pub fn normalize_summary_markdown(content: &str) -> Result<String, CoreError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(CoreError::SummaryParse("empty summary".into()));
    }

    for section in REQUIRED_SECTIONS {
        if !trimmed.contains(section) {
            return Err(CoreError::SummaryParse(format!("missing section: {section}")));
        }
    }

    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_summary() -> String {
        "## 今日概览（含大致时间分配）\n概览\n\
         ## 主要工作内容（按应用/项目/场景分组，结合时长说明投入）\n工作\n\
         ## 重要粘贴片段摘要\n粘贴\n\
         ## 明日待办（能推断则列出，否则写「暂无」）\n暂无"
            .to_string()
    }

    #[test]
    fn accepts_valid_sections() {
        assert!(normalize_summary_markdown(&valid_summary()).is_ok());
    }

    #[test]
    fn rejects_missing_section() {
        let bad = "## 今日概览\nonly one";
        assert!(normalize_summary_markdown(bad).is_err());
    }
}
