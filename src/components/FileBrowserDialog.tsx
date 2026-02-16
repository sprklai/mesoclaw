import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { createPortal } from "react-dom";

import { Button } from "@/components/ui/button";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  ChevronRight,
  Database,
  File,
  Folder,
  FolderOpen,
  HardDrive,
  Home,
  Loader2,
} from "@/lib/icons";
import { cn } from "@/lib/utils";

interface FileEntry {
  name: string;
  path: string;
  is_directory: boolean;
  size: number | null;
  modified: string | null;
}

interface DirectoryContents {
  path: string;
  parent: string | null;
  entries: FileEntry[];
}

interface FileBrowserDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSelect: (path: string) => void;
  title?: string;
  description?: string;
  extensions?: string[];
}

export function FileBrowserDialog({
  open,
  onOpenChange,
  onSelect,
  title = "Select Database File",
  description = "Browse to select a SQLite database file",
  extensions = ["db", "sqlite", "sqlite3"],
}: FileBrowserDialogProps) {
  const [currentPath, setCurrentPath] = useState<string>("");
  const [contents, setContents] = useState<DirectoryContents | null>(null);
  const [commonDirs, setCommonDirs] = useState<[string, string][]>([]);
  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pathInput, setPathInput] = useState<string>("");

  const loadDirectory = useCallback(
    async (path?: string) => {
      setIsLoading(true);
      setError(null);
      setSelectedFile(null);

      try {
        const result = await invoke<DirectoryContents>("list_directory", {
          path: path || null,
          extensions,
        });
        setContents(result);
        setCurrentPath(result.path);
        setPathInput(result.path);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setIsLoading(false);
      }
    },
    [extensions]
  );

  const loadCommonDirs = useCallback(async () => {
    try {
      const dirs = await invoke<[string, string][]>("get_common_directories");
      setCommonDirs(dirs);
    } catch (err) {
      console.error("Failed to load common directories:", err);
    }
  }, []);

  useEffect(() => {
    if (open) {
      loadDirectory();
      loadCommonDirs();
    }
  }, [open, loadDirectory, loadCommonDirs]);

  const handleNavigate = (path: string) => {
    loadDirectory(path);
  };

  const handleGoUp = () => {
    if (contents?.parent) {
      loadDirectory(contents.parent);
    }
  };

  const handleEntryClick = (entry: FileEntry) => {
    if (entry.is_directory) {
      loadDirectory(entry.path);
    } else {
      setSelectedFile(entry.path);
    }
  };

  const handleEntryDoubleClick = (entry: FileEntry) => {
    if (entry.is_directory) {
      loadDirectory(entry.path);
    } else {
      onSelect(entry.path);
      onOpenChange(false);
    }
  };

  const handleSelect = () => {
    if (selectedFile) {
      onSelect(selectedFile);
      onOpenChange(false);
    }
  };

  const handlePathInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      loadDirectory(pathInput);
    }
  };

  const formatSize = (bytes: number | null) => {
    if (bytes === null) return "";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const getBreadcrumbs = () => {
    if (!currentPath) return [];
    const parts = currentPath.split("/").filter(Boolean);
    const breadcrumbs: { name: string; path: string }[] = [];
    let path = "";
    for (const part of parts) {
      path += `/${part}`;
      breadcrumbs.push({ name: part, path });
    }
    return breadcrumbs;
  };

  const getFileIcon = (entry: FileEntry) => {
    if (entry.is_directory) {
      return <Folder className="h-4 w-4 text-amber-500" />;
    }
    const name = entry.name.toLowerCase();
    if (
      name.endsWith(".db") ||
      name.endsWith(".sqlite") ||
      name.endsWith(".sqlite3")
    ) {
      return <Database className="h-4 w-4 text-blue-500" />;
    }
    return <File className="h-4 w-4 text-gray-500" />;
  };

  return createPortal(
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="z-60 w-[95vw] max-w-[1200px] max-h-[90vh] p-6 overflow-hidden flex flex-col text-foreground">
        <div className="flex flex-col gap-4 min-h-0">
          <div>
            <h2 className="text-lg font-semibold leading-none tracking-tight text-foreground">
              {title}
            </h2>
            <p className="text-sm text-muted-foreground mt-1.5">
              {description}
            </p>
          </div>

          <div className="flex flex-col gap-3 flex-1 min-h-0">
            {/* Path input */}
            <div className="flex gap-2">
              <Input
                className="font-mono text-sm text-foreground flex-1 min-w-0"
                placeholder="/path/to/directory"
                value={pathInput}
                onChange={(e) => setPathInput(e.target.value)}
                onKeyDown={handlePathInputKeyDown}
              />
              <Button
                size="icon"
                variant="outline"
                onClick={() => loadDirectory(pathInput)}
                className="shrink-0"
              >
                <ChevronRight className="h-4 w-4" />
              </Button>
            </div>

            {/* Breadcrumb navigation */}
            <div className="flex items-center gap-1 text-sm overflow-x-auto overflow-y-hidden pb-1 min-h-0">
              <button
                className="flex items-center gap-1 px-2 py-1 rounded hover:bg-accent text-muted-foreground shrink-0"
                type="button"
                onClick={() => loadDirectory("/")}
                title="Root"
              >
                <HardDrive className="h-3 w-3" />
              </button>
              {getBreadcrumbs().map((crumb, i) => (
                <div className="flex items-center shrink-0" key={crumb.path}>
                  <ChevronRight className="h-3 w-3 text-muted-foreground shrink-0" />
                  <button
                    className={cn(
                      "px-2 py-1 rounded hover:bg-accent truncate max-w-[120px] sm:max-w-[150px]",
                      i === getBreadcrumbs().length - 1
                        ? "font-medium text-foreground"
                        : "text-muted-foreground"
                    )}
                    title={crumb.path}
                    type="button"
                    onClick={() => handleNavigate(crumb.path)}
                  >
                    {crumb.name}
                  </button>
                </div>
              ))}
            </div>

            <div className="flex gap-3 flex-1 min-h-0">
              {/* Sidebar with common directories */}
              <div className="w-44 shrink-0 border rounded-md p-2 space-y-1 overflow-y-auto overflow-x-hidden bg-card">
                <p className="text-xs font-medium text-muted-foreground mb-2 px-2 truncate">
                  Quick Access
                </p>
                {commonDirs.map(([name, path]) => (
                  <button
                    className={cn(
                      "w-full flex items-center gap-2 px-2 py-1.5 rounded-sm text-sm hover:bg-accent transition-colors text-left text-foreground min-w-0",
                      currentPath === path && "bg-accent"
                    )}
                    key={path}
                    type="button"
                    onClick={() => handleNavigate(path)}
                  >
                    {name === "Home" ? (
                      <Home className="h-4 w-4 text-blue-500 shrink-0" />
                    ) : (
                      <FolderOpen className="h-4 w-4 text-amber-500 shrink-0" />
                    )}
                    <span className="truncate flex-1 min-w-0">{name}</span>
                  </button>
                ))}
              </div>

              {/* File list */}
              <div className="flex-1 min-w-[400px] border rounded-md overflow-hidden flex flex-col min-h-0 bg-card">
                {isLoading ? (
                  <div className="flex-1 flex items-center justify-center">
                    <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                  </div>
                ) : error ? (
                  <div className="flex-1 flex items-center justify-center p-4">
                    <p className="text-sm text-destructive text-center wrap-break-word">
                      {error}
                    </p>
                  </div>
                ) : contents?.entries.length === 0 ? (
                  <div className="flex-1 flex items-center justify-center p-4">
                    <p className="text-sm text-muted-foreground text-center wrap-break-word">
                      No files or folders found
                    </p>
                  </div>
                ) : (
                  <div className="flex-1 overflow-y-auto overflow-x-hidden">
                    {/* Go up button */}
                    {contents?.parent && (
                      <button
                        className="w-full flex items-center gap-3 px-3 py-2 hover:bg-accent transition-colors text-left border-b text-foreground min-w-0"
                        type="button"
                        onClick={handleGoUp}
                      >
                        <Folder className="h-4 w-4 text-amber-500 shrink-0" />
                        <span className="text-sm truncate">..</span>
                      </button>
                    )}
                    {contents?.entries.map((entry) => (
                      <button
                        className={cn(
                          "w-full flex items-center gap-3 px-3 py-2.5 hover:bg-accent transition-colors text-left group text-foreground min-w-0",
                          selectedFile === entry.path && "bg-accent"
                        )}
                        key={entry.path}
                        title={entry.path}
                        type="button"
                        onClick={() => handleEntryClick(entry)}
                        onDoubleClick={() => handleEntryDoubleClick(entry)}
                      >
                        {getFileIcon(entry)}
                        <span className="flex-1 text-sm min-w-0 truncate">
                          {entry.name}
                        </span>
                        <div className="flex items-center gap-2 shrink-0 ml-2">
                          {!entry.is_directory && entry.size !== null && (
                            <span className="text-xs text-muted-foreground whitespace-nowrap tabular-nums">
                              {formatSize(entry.size)}
                            </span>
                          )}
                          {entry.modified && (
                            <span className="text-xs text-muted-foreground hidden xl:block whitespace-nowrap tabular-nums">
                              {entry.modified}
                            </span>
                          )}
                        </div>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </div>

            {/* Selected file display */}
            {selectedFile && (
              <div className="flex items-center gap-2 p-2 bg-accent rounded-md min-w-0 shrink-0">
                <Database className="h-4 w-4 text-blue-500 shrink-0" />
                <span
                  className="text-sm font-medium truncate text-foreground flex-1 min-w-0"
                  title={selectedFile}
                >
                  {selectedFile}
                </span>
              </div>
            )}

            {/* Footer buttons */}
            <div className="flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2 gap-2 shrink-0">
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
              >
                Cancel
              </Button>
              <Button
                disabled={!selectedFile}
                type="button"
                onClick={handleSelect}
              >
                Select
              </Button>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>,
    document.body
  );
}
