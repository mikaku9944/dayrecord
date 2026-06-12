# Hermes Agent 接入

## 文件导出

```bash
dayrecord export --target hermes
# 或指定目录
dayrecord export --target hermes --out ~/.hermes/memories/dayrecord
```

生成文件：

- `USER.md` — 习惯画像（≤1375 字符）
- `MEMORY.md` — 活跃事实（≤2200 字符）
- `memories/YYYY-MM-DD.md` — 每日复盘
- `facts.md` — 全量双时态事实

## 安装到 Hermes

```bash
cp -r hermes-export/* ~/.hermes/memories/
```

Windows WSL2 用户可将导出目录软链到 `~/.hermes/memories/dayrecord/`。
