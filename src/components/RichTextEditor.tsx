import { useEditor, EditorContent } from "@tiptap/react";
import type { Editor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { useI18n } from "../i18n";

interface Props {
  value: string;
  onChange: (html: string) => void;
}

const btn =
  "px-2 py-1 rounded text-sm hover:bg-neutral-200 disabled:opacity-30 disabled:hover:bg-transparent disabled:cursor-not-allowed";

function Toolbar({ editor }: { editor: Editor }) {
  const { t } = useI18n();
  return (
    <div className="flex flex-wrap items-center gap-0.5 border-b border-neutral-200 px-1.5 py-1 bg-neutral-50 rounded-t-md">
      <button
        className={`${btn} ${editor.isActive("bold") ? "bg-neutral-200 font-bold" : "font-semibold"}`}
        onClick={() => editor.chain().focus().toggleBold().run()}
        title={t("加粗")}
      >
        B
      </button>
      <button
        className={`${btn} ${editor.isActive("italic") ? "bg-neutral-200 italic" : "italic"}`}
        onClick={() => editor.chain().focus().toggleItalic().run()}
        title={t("斜体")}
      >
        I
      </button>
      <button
        className={`${btn} ${editor.isActive("strike") ? "bg-neutral-200" : ""}`}
        onClick={() => editor.chain().focus().toggleStrike().run()}
        title={t("删除线")}
      >
        S
      </button>
      <span className="w-px h-4 bg-neutral-300 mx-1" />
      <button
        className={`${btn} ${editor.isActive("bulletList") ? "bg-neutral-200" : ""}`}
        onClick={() => editor.chain().focus().toggleBulletList().run()}
        title={t("无序列表")}
      >
        • {t("列表")}
      </button>
      <button
        className={`${btn} ${editor.isActive("orderedList") ? "bg-neutral-200" : ""}`}
        onClick={() => editor.chain().focus().toggleOrderedList().run()}
        title={t("有序列表")}
      >
        1. {t("列表")}
      </button>
      <span className="w-px h-4 bg-neutral-300 mx-1" />
      <button
        className={`${btn} ${editor.isActive("heading", { level: 2 }) ? "bg-neutral-200 font-bold" : ""}`}
        onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
        title={t("标题2")}
      >
        H2
      </button>
      <button
        className={`${btn} ${editor.isActive("heading", { level: 3 }) ? "bg-neutral-200 font-bold" : ""}`}
        onClick={() => editor.chain().focus().toggleHeading({ level: 3 }).run()}
        title={t("标题3")}
      >
        H3
      </button>
      <span className="w-px h-4 bg-neutral-300 mx-1" />
      <button className={btn} onClick={() => editor.chain().focus().undo().run()} title={t("撤销")} disabled={!editor.can().undo()}>
        {t("撤销")}
      </button>
      <button className={btn} onClick={() => editor.chain().focus().redo().run()} title={t("重做")} disabled={!editor.can().redo()}>
        {t("重做")}
      </button>
    </div>
  );
}

export default function RichTextEditor({ value, onChange }: Props) {
  const editor = useEditor({
    extensions: [StarterKit],
    content: value,
    onUpdate: ({ editor }) => onChange(editor.getHTML()),
    editorProps: {
      attributes: {
        class: "min-h-[120px] px-3 py-2 focus:outline-none text-sm text-neutral-800",
      },
    },
  });

  if (!editor) return null;

  return (
    <div className="border border-neutral-300 rounded-md bg-white focus-within:ring-2 focus-within:ring-neutral-400">
      <Toolbar editor={editor} />
      <EditorContent editor={editor} />
    </div>
  );
}