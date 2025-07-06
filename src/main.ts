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

const app = createApp(App);
const pinia = createPinia();

// 注册所有Element Plus图标
for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component);
}

app.use(pinia);
app.use(ElementPlus);
app.mount("#app");

// 设置HTML为深色主题
document.documentElement.className = 'dark';
