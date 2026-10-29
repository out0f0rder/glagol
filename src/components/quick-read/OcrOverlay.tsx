import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useI18n } from "@/contexts/PreferencesContext";
import { cancelQuickReadOcr, finishQuickReadOcr, getQuickReadRegion, type QuickReadRegion } from "@/lib/tauri";
export function OcrOverlay() {
  const { t } = useI18n();
  const [region, setRegion] = useState<QuickReadRegion | null>(null);
  const [drag, setDrag] = useState<{ x1: number; y1: number; x2: number; y2: number } | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let disposed = false;
    const accept = (value: QuickReadRegion | null) => { if (!disposed) { setRegion(value); setDrag(null); setBusy(false); } };
    const off = listen<QuickReadRegion>("quick-read-region", event => accept(event.payload));
    void off.then(() => getQuickReadRegion()).then(accept).catch(() => undefined);
    const cancel = (event: KeyboardEvent) => { if (event.key === "Escape") { event.preventDefault(); void cancelQuickReadOcr(); setDrag(null); } };
    window.addEventListener("keydown", cancel);
    return () => { disposed = true; void off.then(stop => stop()); window.removeEventListener("keydown", cancel); };
  }, []);
  return <div className="fixed inset-0 cursor-crosshair touch-none select-none bg-black/15" role="region" aria-label={t("qrOcr")}
    onContextMenu={event => event.preventDefault()}
    onPointerDown={event => { if (busy || !region || event.button !== 0) return; event.currentTarget.setPointerCapture(event.pointerId); setDrag({ x1: event.clientX, y1: event.clientY, x2: event.clientX, y2: event.clientY }); }}
    onPointerMove={event => { if (drag && !busy) setDrag({ ...drag, x2: event.clientX, y2: event.clientY }); }}
    onPointerUp={event => {
      if (!drag || !region || busy) return;
      const selection = { ...drag, x2: event.clientX, y2: event.clientY }; setBusy(true);
      event.currentTarget.releasePointerCapture(event.pointerId);
      void finishQuickReadOcr(region.generation, selection).catch(() => undefined).finally(() => { setBusy(false); setDrag(null); });
    }}>
    <div className="pointer-events-none absolute left-1/2 top-6 max-w-lg -translate-x-1/2 rounded-lg bg-zinc-900/95 px-4 py-3 text-center text-sm text-white shadow-lg">{t("qrOcrHint")}</div>
    <button type="button" className="absolute right-4 top-4 cursor-pointer rounded bg-zinc-900 px-4 py-2 text-white" onPointerDown={event => event.stopPropagation()} onClick={() => void cancelQuickReadOcr()}>{t("qrCancel")}</button>
    {drag && <div className="pointer-events-none absolute border-2 border-sky-400 bg-sky-200/10" style={{ left: Math.min(drag.x1, drag.x2), top: Math.min(drag.y1, drag.y2), width: Math.abs(drag.x2 - drag.x1), height: Math.abs(drag.y2 - drag.y1) }} />}
  </div>;
}
