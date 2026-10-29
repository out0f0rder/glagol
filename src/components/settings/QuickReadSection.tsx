import { useEffect, useState } from "react";
import { toast } from "sonner";
import { useI18n } from "@/contexts/PreferencesContext";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { HotkeyEditor } from "@/components/dictation/HotkeyEditor";
import { getQuickReadSettings, setQuickReadOption, setQuickReadShortcut, quickReadAction, type QuickReadSettings } from "@/lib/tauri";
const actions = ["selection", "clipboard", "pointer", "ocr", "pause", "stop"] as const;
const labels = { selection: "qrSelection", clipboard: "qrClipboard", pointer: "qrPointer", ocr: "qrOcr", pause: "qrPause", stop: "qrStop" } as const;
export function QuickReadSection() {
  const { t } = useI18n();
  const [settings, setSettings] = useState<QuickReadSettings | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const reload = () => getQuickReadSettings().then(value => { setSettings(value); setError(null); }).catch(e => setError(String(e)));
  useEffect(() => { void reload(); }, []);
  async function option(key: "player" | "ocr_language" | "phonetic_english", value: string) {
    setBusy(true);
    try { await setQuickReadOption(key, value); await reload(); } catch (e) { toast.error(String(e)); }
    finally { setBusy(false); }
  }
  return <Card id="quick-read">
    <CardHeader><CardTitle>{t("qrTitle")}</CardTitle><CardDescription>{t("qrDescription")}</CardDescription></CardHeader>
    <CardContent className="space-y-4">
      <p className="text-sm text-muted-foreground">{t("qrSetup")}</p>
      <p className="text-sm text-muted-foreground">{t("qrClipboardSafety")}</p>
      {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
      {settings && <>
        {settings.registration_errors.length > 0 && <p role="alert" className="text-sm text-destructive">{t("qrConflicts")} {settings.registration_errors.join(", ")}</p>}
        <div className="space-y-3">{actions.map(action => <div className="flex flex-wrap items-center justify-between gap-3" key={action}>
          <span className="text-sm">{t(labels[action])}</span>
          <HotkeyEditor value={settings.shortcuts[action]} disabled={busy}
            onSave={async shortcut => { await setQuickReadShortcut(action, shortcut); await reload(); }} onError={message => toast.error(message)} />
        </div>)}</div>
        <label className="flex items-center justify-between gap-3 text-sm"><span>{t("qrShowPlayer")}</span><Switch checked={settings.show_player} disabled={busy} onCheckedChange={checked => void option("player", checked ? "1" : "0")} /></label>
        <label className="flex items-center justify-between gap-3 text-sm"><span>{t("qrPhoneticEnglish")}</span><Switch checked={settings.phonetic_english} disabled={busy} onCheckedChange={checked => void option("phonetic_english", checked ? "1" : "0")} /></label>
        <p className="text-xs text-muted-foreground">{t("qrPhoneticNote")}</p>
        <label className="flex flex-wrap items-center justify-between gap-3 text-sm"><span>{t("qrOcrLanguage")}</span>
          <select className="rounded border bg-background p-2" value={settings.ocr_language} disabled={busy} onChange={event => void option("ocr_language", event.target.value)}><option value="ru-RU">Русский</option><option value="en-US">English</option></select>
        </label>
        <p className="text-xs text-muted-foreground">{t("qrOcrNote")}</p>
        <Button variant="secondary" onClick={() => void quickReadAction("clipboard").catch(e => toast.error(String(e)))}>{t("qrClipboard")}</Button>
      </>}
    </CardContent>
  </Card>;
}
