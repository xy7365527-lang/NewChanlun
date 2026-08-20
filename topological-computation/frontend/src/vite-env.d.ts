/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_FENGLIANG_DAEMON_INSTANCES?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
