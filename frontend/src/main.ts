import "./style.css";
import { api, factStatement, type HabitProfile } from "./api";
import { formatCount, formatDuration, recordingLabel } from "./format";
import { renderMarkdownPreview } from "./markdown";

const app = document.querySelector<HTMLDivElement>("#app")!;

let state = {
  recording: true,
  consent: false,
  hasApiKey: false,
  day: "",
  stats: { active_seconds: 0, session_count: 0, char_count: 0, pending_chars: 0 },
  summary: "",
  facts: [] as Awaited<ReturnType<typeof api.listFacts>>,
  habitProfile: null as HabitProfile | null,
  hermesExportDir: "",
  autoExport: false,
  hermesExpanded: false,
  apiKeyInput: "",
  error: "",
  message: "",
  loading: false,
  extracting: false,
  exporting: false,
};

let lastRenderedSummary = "";
let scrollActivityUntil = 0;
let scrollPointerDown = false;
let pendingRenderAfterScroll = false;

function markScrollActivity(): void {
  scrollActivityUntil = Date.now() + 500;
}

function isScrollInteractionActive(): boolean {
  return scrollPointerDown || Date.now() < scrollActivityUntil;
}

function scheduleFlushPendingRender(): void {
  window.setTimeout(() => {
    if (!isScrollInteractionActive() && pendingRenderAfterScroll) {
      pendingRenderAfterScroll = false;
      render({ force: true });
    }
  }, 80);
}

function captureScrollPositions(): { preview: number; facts: number } {
  return {
    preview: document.getElementById("preview")?.scrollTop ?? 0,
    facts: document.querySelector<HTMLElement>(".facts-list")?.scrollTop ?? 0,
  };
}

function restoreScrollPositions(
  scroll: { preview: number; facts: number },
  summaryUnchanged: boolean,
): void {
  if (summaryUnchanged) {
    const preview = document.getElementById("preview");
    if (preview) preview.scrollTop = scroll.preview;
  }
  const facts = document.querySelector<HTMLElement>(".facts-list");
  if (facts) facts.scrollTop = scroll.facts;
}

function setTextContent(selector: string, text: string): void {
  const el = document.querySelector(selector);
  if (el) el.textContent = text;
}

function patchLiveFields(): void {
  setTextContent("#hero-date", state.day || "—");
  setTextContent("#recording-label", recordingLabel(state.recording));
  setTextContent("#stat-active", formatDuration(state.stats.active_seconds));
  setTextContent("#stat-sessions", formatCount(state.stats.session_count));
  setTextContent("#stat-chars", formatCount(totalChars()));
  setTextContent("#stat-pending", formatCount(state.stats.pending_chars));

  const toggleRec = document.getElementById("toggle-rec");
  if (toggleRec) {
    toggleRec.textContent = state.recording ? "暂停录制" : "继续录制";
  }

  const statusDot = document.querySelector(".status-dot");
  if (statusDot) {
    statusDot.classList.toggle("on", state.recording);
    statusDot.classList.toggle("off", !state.recording);
  }
}

function factsKey(facts: typeof state.facts): string {
  return facts.map((f) => f.id).join(",");
}

function needsFullRender(before: typeof state): boolean {
  return (
    before.consent !== state.consent ||
    before.hermesExpanded !== state.hermesExpanded ||
    before.summary !== state.summary ||
    factsKey(before.facts) !== factsKey(state.facts) ||
    before.hermesExportDir !== state.hermesExportDir ||
    before.autoExport !== state.autoExport ||
    before.hasApiKey !== state.hasApiKey ||
    before.loading !== state.loading ||
    before.extracting !== state.extracting ||
    before.exporting !== state.exporting ||
    before.message !== state.message ||
    before.error !== state.error ||
    before.apiKeyInput !== state.apiKeyInput
  );
}

function bindScrollGuards(): void {
  app.addEventListener(
    "scroll",
    (e) => {
      const target = e.target as HTMLElement;
      if (target.matches("#preview, .facts-list")) {
        markScrollActivity();
      }
    },
    true,
  );

  app.addEventListener(
    "pointerdown",
    (e) => {
      const target = e.target as HTMLElement;
      if (target.closest("#preview, .facts-list, .recap-panel, .facts-area")) {
        scrollPointerDown = true;
        markScrollActivity();
      }
    },
    true,
  );

  document.addEventListener("pointerup", () => {
    if (!scrollPointerDown) return;
    scrollPointerDown = false;
    markScrollActivity();
    scheduleFlushPendingRender();
  });

  document.addEventListener("pointercancel", () => {
    if (!scrollPointerDown) return;
    scrollPointerDown = false;
    markScrollActivity();
    scheduleFlushPendingRender();
  });
}

function totalChars(): number {
  return state.stats.char_count + state.stats.pending_chars;
}

function esc(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function renderHabitSummary(profile: HabitProfile | null): string {
  if (!profile) return "习惯画像加载中…";
  const apps = profile.top_apps
    .slice(0, 3)
    .map(([a, s]) => `${a} ${Math.round(s / 60)}m`)
    .join("、");
  return `活跃高峰 ${profile.peak_period} · 常用 ${apps || "暂无"}`;
}

function renderConsent(): string {
  if (state.consent) return "";
  return `
    <div class="consent-overlay" role="dialog" aria-labelledby="consent-title">
      <div class="consent-box sketch-card">
        <h2 id="consent-title" class="section-title">隐私说明</h2>
        <p>DayRecord 会在本机记录键盘输入、粘贴、前台窗口时间与界面可见文本（UIA，无截图），用于生成工作复盘与 Agent 记忆导出。</p>
        <p>数据默认仅存本地；仅在您点击「生成今日复盘」或「抽取用户事实」时，将脱敏后的当日摘要发送至 DeepSeek API。</p>
        <p>请仅在个人自有设备上使用，可随时暂停录制或清空全部数据。</p>
        <label class="consent-label">
          <input type="checkbox" id="consent-check" />
          <span>我已了解并同意</span>
        </label>
        <div class="action-strip">
          <button class="btn btn-primary" id="consent-accept" disabled>开始使用</button>
        </div>
      </div>
    </div>`;
}

function renderHero(): string {
  const statusClass = state.recording ? "on" : "off";
  return `
    <header class="hero sketch-card">
      <div class="hero-top">
        <div>
          <p class="hero-eyebrow">No screenshots. Just readable context.</p>
          <h1 class="hero-title">DayRecord</h1>
        </div>
        <div class="hero-meta">
          <span class="hero-date" id="hero-date">${esc(state.day || "—")}</span>
          <span class="status-pill">
            <span class="status-dot ${statusClass}" aria-hidden="true"></span>
            <span id="recording-label">${recordingLabel(state.recording)}</span>
          </span>
        </div>
      </div>
      <p class="hero-tagline">无截图读上下文 · 按需 AI · 可带走的 Agent 记忆</p>
    </header>`;
}

function renderStats(): string {
  return `
    <section class="sketch-card" aria-label="今日统计">
      <h2 class="section-title">今日速写</h2>
      <div class="metric-grid">
        <div class="metric-note">
          <span class="metric-label">活跃</span>
          <span class="metric-value" id="stat-active">${formatDuration(state.stats.active_seconds)}</span>
        </div>
        <div class="metric-note">
          <span class="metric-label">会话</span>
          <span class="metric-value" id="stat-sessions">${formatCount(state.stats.session_count)}</span>
        </div>
        <div class="metric-note">
          <span class="metric-label">字符</span>
          <span class="metric-value" id="stat-chars">${formatCount(totalChars())}</span>
        </div>
        <div class="metric-note">
          <span class="metric-label">待写入</span>
          <span class="metric-value" id="stat-pending">${formatCount(state.stats.pending_chars)}</span>
        </div>
      </div>
    </section>`;
}

function renderActionPanel(): string {
  return `
    <section class="sketch-card action-panel" aria-label="主要操作">
      <h2 class="section-title">行动</h2>
      <div class="action-strip">
        <button class="btn btn-secondary" id="toggle-rec" type="button">
          ${state.recording ? "暂停录制" : "继续录制"}
        </button>
        <button class="btn btn-primary" id="gen-summary" type="button" ${state.loading ? "disabled" : ""}>
          ${state.loading ? "生成中…" : "生成今日复盘"}
        </button>
      </div>
    </section>`;
}

function renderHermesPanel(): string {
  const expanded = state.hermesExpanded;
  const body = expanded
    ? `
      <p class="habit-summary">${esc(renderHabitSummary(state.habitProfile))}</p>
      <ol class="memory-steps" aria-label="Hermes 三步流程">
        <li>抽取用户事实</li>
        <li>导出 Hermes 记忆</li>
        <li>接入 Agent</li>
      </ol>
      <div class="action-strip">
        <button class="btn btn-secondary" id="extract-facts" type="button" ${state.extracting ? "disabled" : ""}>
          ${state.extracting ? "抽取中…" : "抽取用户事实"}
        </button>
        <button class="btn btn-secondary" id="export-hermes" type="button" ${state.exporting ? "disabled" : ""}>
          ${state.exporting ? "导出中…" : "导出 Hermes 记忆"}
        </button>
      </div>
      <div class="field-group">
        <label class="field-label" for="hermes-dir">导出目录</label>
        <input type="text" id="hermes-dir" class="field-input" value="${esc(state.hermesExportDir)}" />
        <div class="action-strip field-actions">
          <button class="btn btn-ghost" id="save-hermes-dir" type="button">保存目录</button>
          <label class="checkbox-label">
            <input type="checkbox" id="auto-export" ${state.autoExport ? "checked" : ""} />
            <span>生成复盘后自动抽取并导出</span>
          </label>
        </div>
      </div>
      <div class="facts-area">
        <h3 class="subsection-title">活跃事实</h3>
        ${
          state.facts.length === 0
            ? `<p class="empty-state">还没有抽取事实，先生成一次复盘或点击「抽取用户事实」</p>`
            : `<ul class="facts-list">
                ${state.facts
                  .map(
                    (f) => `
                  <li class="fact-chip">
                    <span class="fact-tag">${esc(f.category)}</span>
                    <span class="fact-text">${esc(factStatement(f))}</span>
                    <button class="btn btn-ghost btn-icon" type="button" data-del="${f.id}" aria-label="删除事实">删</button>
                  </li>`,
                  )
                  .join("")}
              </ul>`
        }
      </div>`
    : "";

  return `
    <section class="sketch-card memory-studio" aria-label="Hermes 记忆">
      <div class="section-header">
        <h2 class="section-title">Hermes 记忆</h2>
        <button class="btn btn-ghost" id="toggle-hermes" type="button">
          ${expanded ? "收起" : "展开"}
        </button>
      </div>
      ${body}
    </section>`;
}

function renderRecapPanel(): string {
  const content = state.summary
    ? renderMarkdownPreview(state.summary)
    : `<p class="empty-state">尚未生成复盘。完成一段时间后点击「生成今日复盘」。</p>`;
  return `
    <section class="sketch-card recap-panel" aria-label="今日复盘">
      <h2 class="section-title">今日复盘</h2>
      <div class="paper-preview" id="preview">${content}</div>
    </section>`;
}

function renderSettingsPanel(): string {
  const keyHint = state.hasApiKey
    ? "已配置（系统密钥链）"
    : "未配置时将使用占位复盘";
  return `
    <section class="sketch-card settings-panel" aria-label="设置">
      <h2 class="section-title">DeepSeek API Key</h2>
      <div class="field-group">
        <input
          type="password"
          id="api-key"
          class="field-input"
          placeholder="sk-..."
          value="${esc(state.apiKeyInput)}"
          autocomplete="off"
        />
        <div class="action-strip field-actions">
          <button class="btn btn-secondary" id="save-key" type="button">保存 Key</button>
          <span class="field-hint">${keyHint}</span>
        </div>
      </div>
    </section>
    <section class="sketch-card danger-zone" aria-label="危险操作">
      <button class="btn btn-danger" id="clear-data" type="button">清空全部数据</button>
      ${renderFeedback()}
    </section>`;
}

function renderFeedback(): string {
  const parts: string[] = [];
  if (state.message) {
    parts.push(`<p class="feedback feedback-success" role="status">${esc(state.message)}</p>`);
  }
  if (state.error) {
    parts.push(`<p class="feedback feedback-error" role="alert">${esc(state.error)}</p>`);
  }
  return parts.join("");
}

function render(options: { force?: boolean } = {}): void {
  if (!options.force && isScrollInteractionActive()) {
    pendingRenderAfterScroll = true;
    patchLiveFields();
    return;
  }

  const scroll = captureScrollPositions();
  const summaryUnchanged = state.summary === lastRenderedSummary;

  app.innerHTML = `
    <div class="app-shell">
      ${renderConsent()}
      ${renderHero()}
      ${renderStats()}
      ${renderActionPanel()}
      ${renderHermesPanel()}
      ${renderRecapPanel()}
      ${renderSettingsPanel()}
    </div>`;
  bindEvents();
  restoreScrollPositions(scroll, summaryUnchanged);
  lastRenderedSummary = state.summary;
}

function bindEvents(): void {
  const consentCheck = document.getElementById("consent-check") as HTMLInputElement | null;
  const consentAccept = document.getElementById("consent-accept");
  if (consentCheck && consentAccept) {
    consentCheck.onchange = () => {
      (consentAccept as HTMLButtonElement).disabled = !consentCheck.checked;
    };
    consentAccept.onclick = async () => {
      await api.setConsent(true);
      state.consent = true;
      render();
      await refresh();
    };
  }

  document.getElementById("toggle-rec")?.addEventListener("click", async () => {
    state.recording = !state.recording;
    await api.setRecording(state.recording);
    await refresh();
  });

  document.getElementById("gen-summary")?.addEventListener("click", async () => {
    state.loading = true;
    state.error = "";
    state.message = "";
    render();
    try {
      const s = await api.generateSummary();
      state.summary = s.content;
      state.message = "复盘已生成";
    } catch (e) {
      state.error = String(e);
    }
    state.loading = false;
    await refresh();
  });

  document.getElementById("save-key")?.addEventListener("click", async () => {
    const input = document.getElementById("api-key") as HTMLInputElement;
    const key = input.value.trim();
    if (!key) {
      state.error = "请输入有效的 API Key";
      state.message = "";
      render();
      return;
    }
    try {
      await api.setApiKey(key);
      state.hasApiKey = true;
      state.apiKeyInput = "";
      state.message = "API Key 已保存，将用于生成复盘与抽取事实";
      state.error = "";
    } catch (e) {
      state.error = String(e);
      state.message = "";
    }
    render();
  });

  document.getElementById("toggle-hermes")?.addEventListener("click", async () => {
    state.hermesExpanded = !state.hermesExpanded;
    if (state.hermesExpanded && !state.habitProfile) {
      try {
        state.habitProfile = await api.getHabitProfile();
      } catch {
        /* optional */
      }
    }
    render();
  });

  document.getElementById("extract-facts")?.addEventListener("click", async () => {
    state.extracting = true;
    state.error = "";
    state.message = "";
    render();
    try {
      const n = await api.extractFacts();
      state.facts = await api.listFacts();
      state.message = `已抽取 ${n} 条事实`;
    } catch (e) {
      state.error = String(e);
    }
    state.extracting = false;
    render();
  });

  document.getElementById("export-hermes")?.addEventListener("click", async () => {
    state.exporting = true;
    state.error = "";
    state.message = "";
    render();
    try {
      state.hermesExportDir = await api.exportHermesMemory();
      state.message = `已导出至 ${state.hermesExportDir}`;
    } catch (e) {
      state.error = String(e);
    }
    state.exporting = false;
    render();
  });

  document.getElementById("save-hermes-dir")?.addEventListener("click", async () => {
    const input = document.getElementById("hermes-dir") as HTMLInputElement;
    await api.setHermesExportDir(input.value.trim());
    state.hermesExportDir = input.value.trim();
    state.message = "导出目录已保存";
    state.error = "";
    render();
  });

  document.getElementById("auto-export")?.addEventListener("change", async (e) => {
    const checked = (e.target as HTMLInputElement).checked;
    await api.setAutoExport(checked);
    state.autoExport = checked;
  });

  document.querySelectorAll("[data-del]").forEach((el) => {
    el.addEventListener("click", async () => {
      const id = Number((el as HTMLElement).dataset.del);
      await api.deleteFact(id);
      state.facts = await api.listFacts();
      state.message = "事实已删除";
      render();
    });
  });

  document.getElementById("clear-data")?.addEventListener("click", async () => {
    if (!confirm("确定清空全部 sessions / activities / summaries / facts？consent 将保留。")) return;
    await api.clearAllData();
    state.summary = "";
    state.facts = [];
    state.message = "";
    await refresh();
  });
}

async function refresh(): Promise<void> {
  const before = { ...state, stats: { ...state.stats }, facts: [...state.facts] };

  try {
    const status = await api.getStatus();
    state.recording = status.recording;
    state.consent = status.consent;
    state.hasApiKey = status.has_api_key;
    state.day = status.day;
    state.stats = status.stats;
    const summary = await api.getSummary(status.day);
    if (summary) state.summary = summary.content;
    state.facts = await api.listFacts();
    state.hermesExportDir = await api.getHermesExportDir();
    state.autoExport = await api.getAutoExport();
    if (state.hermesExpanded) {
      state.habitProfile = await api.getHabitProfile();
    }
    if (!state.loading && !state.extracting && !state.exporting) {
      state.error = "";
    }
  } catch (e) {
    state.error = String(e);
  }

  if (isScrollInteractionActive()) {
    pendingRenderAfterScroll = true;
    patchLiveFields();
    return;
  }

  if (needsFullRender(before)) {
    render();
  } else {
    patchLiveFields();
  }
}

bindScrollGuards();
render();
refresh();
setInterval(refresh, 5000);
