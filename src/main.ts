import { createApp } from "vue";
import { createPinia } from "pinia";
import ElementPlus from "element-plus";
import "element-plus/dist/index.css";
import "element-plus/theme-chalk/dark/css-vars.css";
import * as ElementPlusIconsVue from "@element-plus/icons-vue";
import App from "./App.vue";
import dayjs from "dayjs";
import "dayjs/locale/zh-cn";

// 配置dayjs
dayjs.locale("zh-cn");

async function initApp() {
  const app = createApp(App);
  const pinia = createPinia();

  // 注册所有Element Plus图标
  for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
    app.component(key, component);
  }

  app.use(pinia);
  app.use(ElementPlus);
  
  // 设置HTML为深色主题
  document.documentElement.className = 'dark';
  
  // 在 Tauri 环境中等待初始化完成
  if (window.__TAURI__) {
    // 等待 Tauri 初始化完成
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  
  app.mount("#app");
}

initApp().catch(console.error);
