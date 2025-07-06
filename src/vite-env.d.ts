/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

// Tauri 全局变量类型声明
declare global {
  interface Window {
    __TAURI__?: any;
  }
}
