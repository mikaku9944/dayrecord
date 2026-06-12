export function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) {
    return `${h}小时${m}分`;
  }
  return `${m}分钟`;
}

export function formatCount(n: number): string {
  return n.toLocaleString("zh-CN");
}

export function recordingLabel(recording: boolean): string {
  return recording ? "录制中" : "已暂停";
}
