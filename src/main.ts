// src/main.ts
import { createApp } from 'vue'
import './style.css'
import App from './App.vue'
import i18n from './i18n' // 引入我们刚刚写好的多语言配置

const app = createApp(App)

app.use(i18n) // 挂载 i18n
app.mount('#app')