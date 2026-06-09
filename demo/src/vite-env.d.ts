/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_BASE_URL?: string;
  readonly VITE_MAINNET_RPC_URL?: string;
  readonly VITE_DEFAULT_RPC?: string;
  readonly [key: `VITE_RPC_URL_${number}`]: string | undefined;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
