import { useEffect, useRef, useState } from "react";
import WaveSurfer from "wavesurfer.js";
import { convertFileSrc } from "@tauri-apps/api/core";
import { addAudioMark, deleteAudioMark, setAttachmentDuration } from "../api";
import type { Attachment, AudioMark } from "../types";
import { formatTime } from "../format";
import { useI18n } from "../i18n";
import { cssColor, useTheme } from "../theme";

// 波形画在 canvas 上，用不了 Tailwind 类，只能从当前主题的 CSS 变量里取色
const waveColors = () => ({
  waveColor: cssColor("neutral-300", "#d8cfba"),
  progressColor: cssColor("navy", "#22356f"),
  cursorColor: cssColor("ochre", "#c97b63"),
});

interface Props {
  attachment: Attachment;
  marks: AudioMark[];
  onMarksChanged: (next: AudioMark[]) => void;
  interviewId: number;
}

export default function AudioPlayer({ attachment, marks, onMarksChanged, interviewId }: Props) {
  const { t } = useI18n();
  const { theme } = useTheme();
  const containerRef = useRef<HTMLDivElement>(null);
  const wsRef = useRef<WaveSurfer | null>(null);
  const [ready, setReady] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [current, setCurrent] = useState(0);
  const [duration, setDuration] = useState(attachment.duration ?? 0);
  const [label, setLabel] = useState("");
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const attachmentId = attachment.id;
  const knownDuration = attachment.duration;

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    let disposed = false;

    // 走 asset: 协议让 WebView 直接流式读取本地文件。
    // 之前是整段音频 base64 过 IPC 再 atob 逐字节还原，几十 MB 的录音会把界面卡死。
    const ws = WaveSurfer.create({
      container,
      url: convertFileSrc(attachment.filepath),
      ...waveColors(),
      height: 72,
      normalize: true,
    });
    wsRef.current = ws;

    ws.on("ready", () => {
      if (disposed) return;
      const total = ws.getDuration();
      setDuration(total);
      setReady(true);
      setPlaying(false);
      // 附件表里的 duration 一直是空的，解码出来后回填一次。
      if (knownDuration == null && total > 0) {
        setAttachmentDuration(attachmentId, total).catch(() => {});
      }
    });
    ws.on("timeupdate", (time) => {
      if (!disposed) setCurrent(time);
    });
    ws.on("play", () => setPlaying(true));
    ws.on("pause", () => setPlaying(false));
    ws.on("finish", () => setPlaying(false));
    ws.on("error", () => {
      if (!disposed) setError("audio-load-failed");
    });

    return () => {
      disposed = true;
      wsRef.current = null;
      ws.destroy();
    };
    // 依赖里不能放 t：否则切换语言会把所有音频重新加载一遍。
  }, [attachment.filepath, attachmentId, knownDuration]);

  // 切换主题时只换颜色，不重新加载音频
  useEffect(() => {
    wsRef.current?.setOptions(waveColors());
  }, [theme.id]);

  const togglePlay = () => {
    const ws = wsRef.current;
    if (!ws) return;
    if (ws.isPlaying()) ws.pause();
    else ws.play();
  };

  const seekTo = (time: number) => {
    const ws = wsRef.current;
    if (!ws) return;
    ws.setTime(time);
    setCurrent(time);
  };

  const handleAddMark = async () => {
    if (!wsRef.current) return;
    if (!label.trim()) {
      setError(t("请填写打点标签"));
      return;
    }
    setBusy(true);
    setError("");
    try {
      const created = await addAudioMark({
        interviewId,
        attachmentId,
        position: Math.round(current * 100) / 100,
        label: label.trim(),
        note: note.trim() || null,
      });
      onMarksChanged([...marks, created].sort((a, b) => a.position - b.position));
      setLabel("");
      setNote("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleDeleteMark = async (id: number) => {
    setError("");
    try {
      await deleteAudioMark(id);
      onMarksChanged(marks.filter((m) => m.id !== id));
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="border border-neutral-200 rounded-md p-3 bg-neutral-50">
      <div className="flex items-center gap-2 mb-2">
        <span className="text-xs font-medium text-neutral-600">{t("音频复盘 · {filename}", { filename: attachment.filename })}</span>
      </div>

      <div ref={containerRef} className={ready ? "" : "opacity-40"} />

      {!ready && !error && (
        <div className="text-xs text-neutral-400 py-2">{t("音频解码中…")}</div>
      )}

      <div className="flex items-center gap-3 mt-2">
        <button
          onClick={togglePlay}
          disabled={!ready}
          className="px-3 py-1.5 text-xs rounded-md bg-navy text-on-navy hover:bg-navy-deep disabled:opacity-40"
        >
          {playing ? t("暂停") : t("播放")}
        </button>
        <span className="text-xs text-neutral-600 tabular-nums">
          {formatTime(current)} / {formatTime(duration)}
        </span>
      </div>

      <div className="mt-3 grid grid-cols-1 md:grid-cols-[1fr_2fr_auto] gap-2 items-start">
        <input
          className="rounded-md border border-neutral-300 px-2 py-1 text-xs focus:outline-none focus:ring-2 focus:ring-navy bg-white"
          placeholder={t("打点标签")}
          value={label}
          onChange={(e) => setLabel(e.target.value)}
        />
        <input
          className="rounded-md border border-neutral-300 px-2 py-1 text-xs focus:outline-none focus:ring-2 focus:ring-navy bg-white"
          placeholder={t("备注（可选）")}
          value={note}
          onChange={(e) => setNote(e.target.value)}
        />
        <button
          onClick={handleAddMark}
          disabled={busy || !ready}
          className="px-3 py-1.5 text-xs rounded-md border border-neutral-500 text-neutral-700 hover:bg-neutral-100 disabled:opacity-40"
        >
          {busy ? t("保存中…") : t("打点 @ {time}", { time: formatTime(current) })}
        </button>
      </div>

      {marks.length > 0 && (
        <ul className="mt-2 space-y-1">
          {marks.map((m) => (
            <li key={m.id} className="flex items-center gap-2 text-xs bg-white rounded px-2 py-1 border border-neutral-200">
              <button
                onClick={() => seekTo(m.position)}
                className="text-neutral-700 font-medium tabular-nums hover:underline whitespace-nowrap"
              >
                {formatTime(m.position)}
              </button>
              <span className="font-medium text-neutral-700">{m.label}</span>
              {m.note && <span className="text-neutral-400 truncate">{m.note}</span>}
              <span className="flex-1" />
              <button onClick={() => handleDeleteMark(m.id)} className="text-red-500 hover:text-red-600 whitespace-nowrap">
                {t("删除")}
              </button>
            </li>
          ))}
        </ul>
      )}

      {error && (
        <div className="mt-2 text-xs text-red-600">
          {error === "audio-load-failed" ? t("音频加载失败") : error}
        </div>
      )}
    </div>
  );
}
