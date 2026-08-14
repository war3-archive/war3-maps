import { useCallback, useRef, useState, type DragEvent, type KeyboardEvent } from "react";

export function DropZone({
  disabled,
  onFile,
}: {
  disabled?: boolean;
  onFile: (file: File) => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragover, setDragover] = useState(false);

  const openPicker = useCallback(() => {
    if (!disabled) inputRef.current?.click();
  }, [disabled]);

  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragover(false);
    const file = e.dataTransfer.files?.[0];
    if (file) onFile(file);
  };

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      openPicker();
    }
  };

  return (
    <section
      className={`dropzone${dragover ? " dragover" : ""}`}
      tabIndex={0}
      role="button"
      aria-label="Drop a Warcraft III map file"
      aria-disabled={disabled}
      onClick={(e) => {
        if ((e.target as HTMLElement).closest("button")) return;
        openPicker();
      }}
      onKeyDown={onKeyDown}
      onDragEnter={(e) => {
        e.preventDefault();
        setDragover(true);
      }}
      onDragOver={(e) => {
        e.preventDefault();
        setDragover(true);
      }}
      onDragLeave={(e) => {
        e.preventDefault();
        setDragover(false);
      }}
      onDrop={onDrop}
    >
      <input
        ref={inputRef}
        type="file"
        accept=".w3x,.w3m,application/octet-stream"
        hidden
        onChange={() => {
          const file = inputRef.current?.files?.[0];
          if (file) onFile(file);
          if (inputRef.current) inputRef.current.value = "";
        }}
      />
      <div className="dropzone-icon" aria-hidden="true">
        🗺
      </div>
      <h2>
        Drop a <code>.w3x</code> / <code>.w3m</code> map here
      </h2>
      <p>or click to browse · parsed in-browser · nothing is uploaded</p>
      <button type="button" className="btn btn-primary" disabled={disabled} onClick={openPicker}>
        Choose file
      </button>
    </section>
  );
}
