interface Window {
  pywebview?: {
    api?: {
      save_file_dialog: () => Promise<string | null>;
    };
  };
  __TAURI__?: any;
  __TAURI_INTERNALS__?: any;
}

