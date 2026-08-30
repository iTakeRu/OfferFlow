import { useEffect, useRef, useState } from "react";
import WaveSurfer from "wavesurfer.js";
import {
  readFileBase64, listAudioMarks, addAudioMark, deleteAudioMark,
} from "../api";
import type { Attachment, AudioMark } from "../types";
import { useI18n } from "../i18n";

interface Props {
  attachment: Attachment;
  interviewId: number;
}

function mimeOf(name: string): string {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  const map: Record<string, string> = {
    mp3: "audio/mpeg",
    wav: "audio/wav",
    m4a: "audio/mp4",
    ogg: "audio/ogg",
    flac: "audio/flac",
    aac: "audio/aac",
    webm: "audio/webm",
  };
  return map[ext] ?? "audio/mpeg";
}

function formatTime(s: number): string {
  if (!isFinite(s) || s < 0) return "0:00";
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${sec.toString().padStart(2, "0")}`;
}

export default function AudioPlayer({ attachment, interviewId }: Props) {
  const { t } = useI18n();
  const containerRef = useRef<HTMLDivElement>(null);
  const wsRef = useRef<WaveSurfer | null>(null);
  const [ready, setReady] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [current, setCurrent] = useState(0);
  const [duration, setDuration] = useState(0);
  const [marks, setMarks] = useState<AudioMark[]>([]);
  const [label, setLabel] = useState("");
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    let ws: WaveSurfer | null = null;
    let objectUrl: string | null = null;
    let disposed = false;

    (async () => {
      try {
        const b64 = await readFileBase64(attachment.filepath);
        const binary = atob(b64);
        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
        const blob = new Blob([bytes], { type: mimeOf(attachment.filename) });
        objectUrl = URL.createObjectURL(blob);

        ws = WaveSurfer.create({
          container: containerRef.current!,
          url: objectUrl,
          waveColor: "#d4d4d4",
          progressColor: "#404040",
          cursorColor: "#f59e0b",
          height: 72,
          normalize: true,
        });

        ws.on("ready", () => {
          if (disposed) return;
          setDuration(ws!.getDuration());
          setReady(true);
          setPlaying(false);
        });
        ws.on("timeupdate", (t) => {
          if (!disposed) setCurrent(t);
        });
        ws.on("play", () => setPlaying(true));
        ws.on("pause", () => setPlaying(false));
        ws.on("finish", () => setPlaying(false));
        ws.on("error", () => {
          if (!disposed) setError(t("音频加载失败"));
        });

        wsRef.current = ws;
      } catch (e) {
        if (!disposed) setError(String(e));
      }
    })();

    return () => {
      disposed = true;
      if (wsRef.current) {
        wsRef.current.destroy();
        wsRef.current = null;
      }
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [attachment.filepath, t]);

  useEffect(() => {
    listAudioMarks(interviewId)
      .then((all) =>
        setMarks(
          all
            .filter((m) => m.attachment_id === attachment.id)
            .sort((a, b) => a.position - b.position),
        ),
      )
      .catch((e) => setError(String(e)));
  }, [interviewId, attachment.id]);

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
    const ws = wsRef.current;
    if (!ws) return;
    if (!label.trim()) {
      setError(t("请填写打点标签"));
      return;
    }
    setBusy(true);
    setError("");
    try {
      const created = await addAudioMark({
        interviewId,
        attachmentId: attachment.id,
        position: Math.round(current * 100) / 100,
        label: label.trim(),
        note: note.trim() || null,
      });
      setMarks((prev) => [...prev, created].sort((a, b) => a.position - b.position));
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
      setMarks((prev) => prev.filter((m) => m.id !== id));
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
          className="px-3 py-1.5 text-xs rounded-md bg-neutral-900 text-white hover:bg-neutral-800 disabled:opacity-40"
        >
          {playing ? t("暂停") : t("播放")}
        </button>
        <span className="text-xs text-neutral-600 tabular-nums">
          {formatTime(current)} / {formatTime(duration)}
        </span>
      </div>

      <div className="mt-3 grid grid-cols-1 md:grid-cols-[1fr_2fr_auto] gap-2 items-start">
        <input
          className="rounded-md border border-neutral-300 px-2 py-1 text-xs focus:outline-none focus:ring-2 focus:ring-neutral-400 bg-white"
          placeholder={t("打点标签")}
          value={label}
          onChange={(e) => setLabel(e.target.value)}
        />
        <input
          className="rounded-md border border-neutral-300 px-2 py-1 text-xs focus:outline-none focus:ring-2 focus:ring-neutral-400 bg-white"
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

      {error && <div className="mt-2 text-xs text-red-600">{error}</div>}
    </div>
  );
}