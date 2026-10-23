import React, { useState } from "react";
import { Sparkles, Download, CheckCircle, X } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle, CardFooter } from "./ui/card";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { UpdateInfo, installAppUpdate } from "@/services/tauri";

interface UpdaterModalProps {
  updateInfo: UpdateInfo | null;
  onClose: () => void;
}

export const UpdaterModal: React.FC<UpdaterModalProps> = ({
  updateInfo,
  onClose,
}) => {
  const [isInstalling, setIsInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!updateInfo || !updateInfo.available) {
    return null;
  }

  const handleInstall = async () => {
    setIsInstalling(true);
    setError(null);
    try {
      await installAppUpdate();
    } catch (e: any) {
      setError(e?.message || "Failed to install update");
      setIsInstalling(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
      <Card className="w-full max-w-md border-cyan-500/30 bg-slate-900 shadow-2xl shadow-cyan-500/10">
        <CardHeader className="flex flex-row items-center justify-between pb-3">
          <div className="flex items-center gap-2">
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-cyan-500/20 text-cyan-400">
              <Sparkles className="h-4 w-4" />
            </div>
            <div>
              <CardTitle className="text-base text-white">Update Available</CardTitle>
              <Badge variant="default" className="text-[10px] mt-0.5">
                v{updateInfo.version}
              </Badge>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-white p-1 rounded-md"
          >
            <X className="h-4 w-4" />
          </button>
        </CardHeader>

        <CardContent className="space-y-3 pt-0">
          <p className="text-xs text-slate-300">
            A new version of <strong>Nord Scanner</strong> is ready to
            download and install.
          </p>

          {updateInfo.body && (
            <div className="rounded-lg bg-slate-950/80 p-3 text-xs text-slate-300 font-mono max-h-36 overflow-y-auto border border-slate-800">
              <div className="text-[11px] text-slate-500 uppercase tracking-wider mb-1">
                Release Notes
              </div>
              {updateInfo.body}
            </div>
          )}

          {error && (
            <div className="text-xs text-rose-400 bg-rose-500/10 p-2 rounded border border-rose-500/20">
              {error}
            </div>
          )}
        </CardContent>

        <CardFooter className="flex justify-end gap-2 pt-2 border-t border-slate-800/80">
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            disabled={isInstalling}
            className="text-slate-400"
          >
            Remind Me Later
          </Button>
          <Button
            variant="default"
            size="sm"
            onClick={handleInstall}
            disabled={isInstalling}
            className="bg-cyan-500 hover:bg-cyan-400 text-white shadow-lg shadow-cyan-500/20"
          >
            <Download className={`mr-1.5 h-3.5 w-3.5 ${isInstalling ? "animate-bounce" : ""}`} />
            {isInstalling ? "Downloading & Installing..." : "Update & Restart"}
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
};
