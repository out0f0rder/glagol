import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Pause, Play, Square, SkipBack, SkipForward, X } from "lucide-react";
import { useI18n } from "@/contexts/PreferencesContext";
import { getQuickReadState, hideQuickReadPlayer, quickReadAction, setQuickReadSpeed, type QuickReadAction, type QuickReadState } from "@/lib/tauri";
export function MiniPlayer() {
  const { t, locale } = useI18n();
  const [state, setState] = useState<QuickReadState | null>(null);
  useEffect(() => {
    let disposed = false;
    const off = listen<QuickReadState>("quick-read-state", event => { if (!disposed) setState(event.payload); });
    void off.then(() => getQuickReadState()).then(value => { if (!disposed) setState(value); }).catch(() => undefined);
    return () => { disposed = true; void off.then(stop => stop()); };
  }, []);
  function action(action: QuickReadAction) { void quickReadAction(action).catch(() => undefined); }
  const button = "flex h-8 w-8 items-center justify-center rounded hover:bg-white/20 disabled:opacity-40";
  return <div className="m-1 rounded-xl bg-zinc-900/95 p-3 text-white shadow-lg" role="region" aria-label={t("qrTitle")}>
    <div className="flex items-center justify-between gap-2 text-xs">
      <span>{t("qrTitle")}{state && state.total > 0 ? ` · ${state.current + 1}/${state.total}` : ""}{state?.phase === "preparing" || state?.phase === "extracting" ? ` · ${t("qrPreparing")}` : ""}</span>
      <button type="button" className={button} aria-label={t("qrHidePlayer")} title={t("qrHidePlayer")} onClick={() => void hideQuickReadPlayer().catch(() => undefined)}><X size={14} /></button>
    </div>
    {state?.error ? <p role="alert" className="line-clamp-3 text-xs text-red-200" title={state.error}>{state.error}</p> : <div className="flex items-center justify-between gap-1">
      <button type="button" className={button} aria-label={t("qrPrevious")} onClick={() => action("previous")}><SkipBack size={18} /></button>
      <button type="button" className={button} aria-label={t("qrPause")} onClick={() => action("pause")}>{state?.paused ? <Play size={18} /> : <Pause size={18} />}</button>
      <button type="button" className={button} aria-label={t("qrStop")} onClick={() => action("stop")}><Square size={18} /></button>
      <button type="button" className={button} aria-label={t("qrNext")} onClick={() => action("next")}><SkipForward size={18} /></button>
      <button type="button" className={button} aria-label={t("qrSlower")} onClick={() => void setQuickReadSpeed(Math.max(0.5, (state?.speed ?? 1) - 0.25))}>−</button>
      <span className="w-12 text-center text-xs tabular-nums">{(state?.speed ?? 1).toLocaleString(locale)}×</span>
      <button type="button" className={button} aria-label={t("qrFaster")} onClick={() => void setQuickReadSpeed(Math.min(2, (state?.speed ?? 1) + 0.25))}>+</button>
    </div>}
  </div>;
}
