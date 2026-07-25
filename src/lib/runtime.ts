import { invoke } from "@tauri-apps/api/core";

export interface RuntimeInfo {
  appName: string;
  version: string;
}

export function formatRuntimeInfo(info: RuntimeInfo): string {
  return `${info.appName} ${info.version}`;
}

export async function getRuntimeInfo(): Promise<RuntimeInfo> {
  return invoke<RuntimeInfo>("runtime_info");
}
