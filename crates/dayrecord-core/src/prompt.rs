use crate::models::{ActivityAgg, Fact, Session};
use crate::patterns::TaskSegment;
use crate::redact::sanitize;

pub const MAX_ACTIVITIES: usize = 50;
pub const MAX_SESSIONS: usize = 30;
pub const MAX_SESSION_CHARS: usize = 80_000;

pub const SUMMARY_SYSTEM: &str = r#"你是 DayRecord 工作复盘助手。根据用户的应用使用时间轴、界面可见文本与输入会话，生成简洁专业的中文 Markdown 复盘。
要求：
- 以时间轴为主线判断做了什么
- 用「界面可见文本」（UIA 快照）补充阅读/浏览内容，用会话补充键入细节
- 中文上下文优先依据界面可见文本、窗口标题与粘贴片段
- 不编造未出现的信息；忽略拼音碎片与无意义输入
输出必须包含以下四级标题（按顺序）：
## 今日概览（含大致时间分配）
## 主要工作内容（按应用/项目/场景分组，结合时长说明投入）
## 重要粘贴片段摘要
## 明日待办（能推断则列出，否则写「暂无」）"#;

pub const EXTRACTION_SYSTEM: &str = r#"你是用户事实抽取助手。根据应用使用时间轴、界面可见文本与输入会话，提取关于用户的稳定事实。

规则：
1. 只提取稳定事实：正在做的项目、常用工具、长期偏好、关注主题、作息规律。
2. 忽略一次性碎片、拼音噪声、无意义输入、明显错字。
3. 中文上下文优先依据界面可见文本、窗口标题与粘贴片段；IME 上屏中文可能不完整。
4. 不要编造未出现的信息；不要输出敏感信息（密码、token、手机号等）。
5. 输出必须是纯 JSON 数组，无 markdown 包裹：
   [{"subject":"用户","predicate":"正在做项目","object":"DayRecord","category":"project","confidence":0.9}]
6. category 只能是：project, tool, preference, topic, schedule, routine
7. confidence 为 0.0-1.0 浮点数。"#;

pub const TASK_NAMING_SYSTEM: &str = r#"你是任务单元命名助手。根据预分段的工作块（应用链、界面可见文本、粘贴片段），为每个块生成简短中文名称与低置信度意图猜测。

规则：
1. name：2-8 字概括该时段主要工作，如「写 PRD」「查文档」「改 bug」。
2. goal_guess：一句低置信度意图推断，可为空字符串。
3. 不要编造未出现的信息；忽略拼音噪声。
4. 输出必须是纯 JSON 数组，无 markdown 包裹：
   [{"name":"写 PRD","goal_guess":"整理产品需求文档","confidence":0.7}]
5. confidence 为 0.0-1.0 浮点数。"#;

fn fmt_duration(seconds: i64) -> String {
    let m = seconds / 60;
    if m >= 60 {
        format!("{}小时{}分", m / 60, m % 60)
    } else if m >= 1 {
        format!("{m}分钟")
    } else {
        format!("{seconds}秒")
    }
}

/// Build the daily-recap user prompt from aggregated activities (with UIA
/// snapshots) and input sessions. All outbound content is locally redacted.
pub fn build_summary_user_prompt(
    day: &str,
    activities: &[ActivityAgg],
    sessions: &[Session],
    facts: &[Fact],
) -> String {
    let mut activities_sorted: Vec<&ActivityAgg> = activities.iter().collect();
    activities_sorted.sort_by(|a, b| b.seconds.cmp(&a.seconds));
    activities_sorted.truncate(MAX_ACTIVITIES);

    let mut out = String::new();
    out.push_str(&format!("日期：{day}\n\n"));

    if !facts.is_empty() {
        out.push_str("## 已知长期事实（供参考）\n");
        for f in facts {
            out.push_str(&format!(
                "- [{}] {} (confidence={:.2})\n",
                f.category.as_str(),
                f.statement(),
                f.confidence
            ));
        }
        out.push('\n');
    }

    out.push_str("## A. 应用使用时间轴（按时长降序）\n");
    if activities_sorted.is_empty() {
        out.push_str("（无）\n");
    }
    for a in &activities_sorted {
        let title = if a.window_title.trim().is_empty() {
            "(无标题)"
        } else {
            a.window_title.trim()
        };
        out.push_str(&format!(
            "- {} | {} | {}\n",
            a.app_name,
            title,
            fmt_duration(a.seconds)
        ));
        if let Some(uia) = a.uia_snapshot.as_ref().filter(|t| !t.trim().is_empty()) {
            out.push_str("  界面可见文本：\n  ");
            out.push_str(&sanitize(uia).replace('\n', "\n  "));
            out.push('\n');
        }
    }

    out.push_str(&render_sessions_block(sessions));
    out
}

fn render_sessions_block(sessions: &[Session]) -> String {
    let mut out = String::from("\n## B. 输入会话记录（按时间，已脱敏）\n");
    let mut sessions_sorted: Vec<&Session> = sessions.iter().collect();
    sessions_sorted.sort_by_key(|s| s.started_at);

    if sessions_sorted.is_empty() {
        out.push_str("（无）\n");
        return out;
    }

    let mut total_chars = 0usize;
    for s in sessions_sorted.iter().take(MAX_SESSIONS) {
        let mut line = format!(
            "[{} - {}] {} | {} | {}\n",
            s.started_at.format("%H:%M:%S"),
            s.ended_at.format("%H:%M:%S"),
            s.app_name,
            s.window_title,
            sanitize(&s.content)
        );
        if let Some(uia) = s.uia_text.as_ref().filter(|t| !t.trim().is_empty()) {
            line.push_str("  界面文本快照：\n  ");
            line.push_str(&sanitize(uia).replace('\n', "\n  "));
            line.push('\n');
        }
        if total_chars + line.chars().count() > MAX_SESSION_CHARS {
            out.push_str("\n...（会话内容已截断）\n");
            break;
        }
        total_chars += line.chars().count();
        out.push_str(&line);
    }
    out
}

pub fn build_extraction_user_prompt(
    day: &str,
    activities: &[ActivityAgg],
    sessions: &[Session],
) -> String {
    build_summary_user_prompt(day, activities, sessions, &[])
}

pub fn build_task_naming_prompt(day: &str, segments: &[TaskSegment]) -> String {
    let mut out = format!("日期：{day}\n\n## 预分段任务块\n");
    if segments.is_empty() {
        out.push_str("（无）\n");
        return out;
    }
    for (i, seg) in segments.iter().enumerate() {
        out.push_str(&format!(
            "### 块 {} | {} - {} | {} 秒\n",
            i + 1,
            seg.started_at.format("%H:%M"),
            seg.ended_at.format("%H:%M"),
            seg.total_seconds
        ));
        out.push_str(&format!("应用链：{}\n", seg.app_chain.join(" → ")));
        if let Some(uia) = seg.uia_summary.as_ref().filter(|t| !t.trim().is_empty()) {
            out.push_str("界面可见文本：\n");
            out.push_str(&sanitize(uia).replace('\n', "\n"));
            out.push('\n');
        }
        if !seg.paste_snippets.is_empty() {
            out.push_str("粘贴片段：\n");
            for p in &seg.paste_snippets {
                out.push_str(&format!("- {}\n", sanitize(p)));
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn session(content: &str) -> Session {
        Session {
            id: Some(1),
            day: "2026-06-10".into(),
            started_at: Utc::now(),
            ended_at: Utc::now(),
            app_name: "app".into(),
            window_title: "w".into(),
            content: content.into(),
            has_paste: false,
            uia_text: None,
            backspace_count: 0,
        }
    }

    #[test]
    fn truncates_sessions_at_limit() {
        let sessions = vec![session(&"x".repeat(MAX_SESSION_CHARS + 1000))];
        let prompt = build_summary_user_prompt("2026-06-10", &[], &sessions, &[]);
        assert!(prompt.contains("已截断"));
    }

    #[test]
    fn redacts_session_content() {
        let prompt = build_summary_user_prompt("2026-06-10", &[], &[session("call 13812345678")], &[]);
        assert!(prompt.contains("[PHONE]"));
    }

    #[test]
    fn includes_uia_snapshot() {
        let activities = vec![ActivityAgg {
            app_name: "chrome.exe".into(),
            window_title: "Docs".into(),
            seconds: 600,
            uia_snapshot: Some("[可见内容] DayRecord".into()),
        }];
        let prompt = build_summary_user_prompt("2026-06-10", &activities, &[], &[]);
        assert!(prompt.contains("界面可见文本"));
    }
}
