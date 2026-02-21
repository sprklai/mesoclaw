/**
 * IdentityEditor — markdown editor with live preview for each identity file.
 *
 * Lists all identity files; selecting one loads its content into a split
 * editor/preview pane.  Save calls `updateIdentityFile` via identityStore.
 */

import { useEffect, useState } from "react";

import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useIdentityStore } from "@/stores/identityStore";

// ─── FileList ─────────────────────────────────────────────────────────────────

interface FileListProps {
  selectedFile: string | null;
  onSelect: (fileName: string) => void;
}

function FileList({ selectedFile, onSelect }: FileListProps) {
  const files = useIdentityStore((s) => s.files);
  const isLoading = useIdentityStore((s) => s.isLoading);

  if (isLoading) {
    return (
      <p className="text-xs text-muted-foreground animate-pulse px-1">
        Loading files…
      </p>
    );
  }

  if (files.length === 0) {
    return (
      <p className="text-xs text-muted-foreground px-1">
        No identity files found.
      </p>
    );
  }

  return (
    <ul className="flex flex-col gap-0.5">
      {files.map((f) => (
        <li key={f.name}>
          <button
            type="button"
            onClick={() => onSelect(f.name)}
            className={cn(
              "w-full rounded-md px-3 py-1.5 text-left text-sm transition-colors hover:bg-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              selectedFile === f.name && "bg-accent border border-primary"
            )}
          >
            <span className="font-mono text-xs">{f.name}</span>
          </button>
        </li>
      ))}
    </ul>
  );
}

// ─── EditorPane ───────────────────────────────────────────────────────────────

interface EditorPaneProps {
  fileName: string;
}

function EditorPane({ fileName }: EditorPaneProps) {
  const getFileContent = useIdentityStore((s) => s.getFileContent);
  const saveFile = useIdentityStore((s) => s.saveFile);
  const error = useIdentityStore((s) => s.error);

  const [content, setContent] = useState("");
  const [original, setOriginal] = useState("");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [preview, setPreview] = useState(false);

  useEffect(() => {
    setLoading(true);
    getFileContent(fileName)
      .then((c) => {
        setContent(c);
        setOriginal(c);
      })
      .finally(() => setLoading(false));
  }, [fileName, getFileContent]);

  const isDirty = content !== original;

  const handleSave = async () => {
    setSaving(true);
    try {
      await saveFile(fileName, content);
      setOriginal(content);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <p className="text-xs text-muted-foreground animate-pulse p-2">
        Loading {fileName}…
      </p>
    );
  }

  return (
    <div className="flex flex-1 flex-col gap-2 overflow-hidden">
      {/* Toolbar */}
      <div className="flex items-center gap-2 shrink-0">
        <span className="font-mono text-xs font-semibold flex-1">{fileName}</span>
        {isDirty && (
          <Badge variant="secondary" className="text-xs">
            unsaved
          </Badge>
        )}
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setPreview((v) => !v)}
        >
          {preview ? "Edit" : "Preview"}
        </Button>
        <Button
          variant="default"
          size="sm"
          disabled={!isDirty || saving}
          onClick={handleSave}
        >
          {saving ? "Saving…" : saved ? "Saved ✓" : "Save"}
        </Button>
      </div>

      {error && (
        <p className="text-xs text-destructive">{error}</p>
      )}

      {/* Editor / Preview */}
      {preview ? (
        <div className="flex-1 overflow-y-auto rounded-md border bg-muted/20 p-3">
          <pre className="whitespace-pre-wrap break-words text-[12px] text-foreground/90 font-sans leading-relaxed">
            {content}
          </pre>
        </div>
      ) : (
        <textarea
          className="flex-1 resize-none rounded-md border bg-background p-3 font-mono text-xs text-foreground focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          value={content}
          onChange={(e) => setContent(e.target.value)}
          spellCheck={false}
          aria-label={`Edit ${fileName}`}
        />
      )}
    </div>
  );
}

// ─── IdentityEditor ───────────────────────────────────────────────────────────

interface IdentityEditorProps {
  className?: string;
}

export function IdentityEditor({ className }: IdentityEditorProps) {
  const loadFiles = useIdentityStore((s) => s.loadFiles);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);

  useEffect(() => {
    loadFiles();
  }, [loadFiles]);

  return (
    <div className={cn("flex gap-4 h-[480px]", className)}>
      {/* File list sidebar */}
      <div className="w-44 shrink-0 overflow-y-auto">
        <p className="mb-2 text-xs font-semibold text-muted-foreground uppercase tracking-wide px-1">
          Identity Files
        </p>
        <FileList selectedFile={selectedFile} onSelect={setSelectedFile} />
      </div>

      {/* Editor area */}
      <div className="flex flex-1 flex-col overflow-hidden">
        {selectedFile ? (
          <EditorPane key={selectedFile} fileName={selectedFile} />
        ) : (
          <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
            Select a file to edit
          </div>
        )}
      </div>
    </div>
  );
}
