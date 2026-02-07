import { useEffect, useState } from "react";

interface Props {
  open: boolean;
  title: string;
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  confirmLabel?: string;
  requireText?: string;
}

export function ConfirmDialog({
  open,
  title,
  message,
  onConfirm,
  onCancel,
  confirmLabel = "Confirm",
  requireText,
}: Props) {
  const [typedValue, setTypedValue] = useState("");

  useEffect(() => {
    if (open) {
      setTypedValue("");
    }
  }, [open, title]);

  if (!open) {
    return null;
  }

  const canConfirm = requireText ? typedValue === requireText : true;

  return (
    <div className="fixed inset-0 z-20 flex items-center justify-center bg-black/70 p-4">
      <div className="w-full max-w-md rounded border border-slate-700 bg-slate-900 p-4">
        <h3 className="text-lg font-semibold">{title}</h3>
        <p className="mt-2 text-sm text-slate-300">{message}</p>
        {requireText && (
          <div className="mt-3 space-y-2">
            <label className="block text-xs text-slate-300">
              Type <span className="font-semibold text-rose-300">{requireText}</span> to continue
            </label>
            <input
              value={typedValue}
              onChange={(event) => setTypedValue(event.target.value)}
              placeholder={requireText}
              className="w-full rounded border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-slate-100 placeholder:text-slate-500 focus:border-blue-500 focus:outline-none"
            />
          </div>
        )}
        <div className="mt-4 flex justify-end gap-2">
          <button type="button" className="rounded bg-slate-700 px-3 py-2 text-sm" onClick={onCancel}>
            Cancel
          </button>
          <button
            type="button"
            className="rounded bg-rose-700 px-3 py-2 text-sm disabled:cursor-not-allowed disabled:opacity-40"
            onClick={onConfirm}
            disabled={!canConfirm}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
