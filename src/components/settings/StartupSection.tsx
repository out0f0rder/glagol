import { useEffect, useState } from "react";
import { toast } from "sonner";
import { useI18n } from "@/contexts/PreferencesContext";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Button } from "@/components/ui/button";
import { getStartupSettings, setLaunchAtLogin, setStartInTray, type StartupSettings } from "@/lib/tauri";

export function StartupSection() {
  const { t } = useI18n();
  const [settings, setSettings] = useState<StartupSettings | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  async function reload() {
    try { setSettings(await getStartupSettings()); setError(null); }
    catch (e) { setError(String(e)); }
  }
  useEffect(() => { void reload(); }, []);
  async function save(key: "login" | "tray", enabled: boolean) {
    setBusy(true);
    try {
      await (key === "login" ? setLaunchAtLogin(enabled) : setStartInTray(enabled));
      await reload();
    } catch (e) { toast.error(String(e)); }
    finally { setBusy(false); }
  }
  return <Card id="startup">
    <CardHeader><CardTitle>{t("startupTitle")}</CardTitle><CardDescription>{t("startupDescription")}</CardDescription></CardHeader>
    <CardContent className="space-y-4">
      {error && <div role="alert" className="space-y-2 text-sm text-destructive"><p>{error}</p><Button variant="outline" onClick={() => void reload()}>{t("retry")}</Button></div>}
      {settings && <>
        <div className="flex items-center justify-between gap-3">
          <div className="space-y-1"><label htmlFor="launch-at-login" className="text-sm font-medium">{t("startupLogin")}</label><p className="text-xs text-muted-foreground">{t("startupLoginNote")}</p></div>
          <Switch id="launch-at-login" checked={settings.autostart_enabled} disabled={busy || !settings.autostart_supported} onCheckedChange={enabled => void save("login", enabled)} />
        </div>
        {settings.autostart_error && <p role="alert" className="text-sm text-destructive">{settings.autostart_error}</p>}
        {settings.autostart_other_copy && <p className="text-xs text-muted-foreground">{t("startupOtherCopy")}</p>}
        {!settings.autostart_supported && <p className="text-xs text-muted-foreground">{t("startupWindowsOnly")}</p>}
        <div className="flex items-center justify-between gap-3">
          <div className="space-y-1"><label htmlFor="start-in-tray" className="text-sm font-medium">{t("startupTray")}</label><p className="text-xs text-muted-foreground">{t("startupTrayNote")}</p></div>
          <Switch id="start-in-tray" checked={settings.start_in_tray} disabled={busy} onCheckedChange={enabled => void save("tray", enabled)} />
        </div>
        <p className="text-xs text-muted-foreground">{t("startupOpenNote")}</p>
      </>}
    </CardContent>
  </Card>;
}
