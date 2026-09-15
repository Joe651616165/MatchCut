<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { useI18n } from "vue-i18n";

// 国际化 Hook
const { locale, t } = useI18n();

interface Asset {
  name: string;
  path: string;
  kind: string;
  timestamp_ms: number;
}

interface MatchPair {
  pair_id: number;
  video: Asset | null;
  audio: Asset | null;
  status: string;
}

// 核心状态
const pairs = ref<MatchPair[]>([]);
const folderPath = ref("");
const isProcessing = ref(false);
const isDragging = ref(false);

// 新增 UI 状态：免责声明 & 成功动画
const showDisclaimer = ref(localStorage.getItem('agreed') !== 'true');
const isSuccess = ref(false);

// 新增 UI 状态：多机位排序开关
const enableMultiCamSort = ref(false);

function agreeDisclaimer() {
  localStorage.setItem('agreed', 'true');
  showDisclaimer.value = false;
}

// 软件配置及精美图标库
const softwareOptions = [
  { id: 'jianying', name: '剪映 / CapCut', icon: 'capcut' },
  { id: 'davinci', name: 'DaVinci Resolve', icon: 'davinci' },
  { id: 'fcpxml', name: 'Final Cut Pro', icon: 'fcp' },
  { id: 'premiere', name: 'Premiere Pro', icon: 'pr' },
];

const targetSoftware = ref(localStorage.getItem("targetSoftware") || "jianying");
const isDropdownOpen = ref(false);
const currentSoftware = ref(softwareOptions.find(opt => opt.id === targetSoftware.value) || softwareOptions[0]);

// 下拉菜单的 DOM 引用，用于判断点击区域
const dropdownRef = ref<HTMLElement | null>(null);

// 语言切换下拉状态
const isLangDropdownOpen = ref(false);
const langDropdownRef = ref<HTMLElement | null>(null);

const languages = [
  { code: 'zh', label: '🇨🇳 中文' },
  { code: 'en', label: '🇬🇧 English' },
  { code: 'es', label: '🇪🇸 Español' },
  { code: 'de', label: '🇩🇪 Deutsch' },
];

function changeLanguage(lang: string) {
  locale.value = lang;
  localStorage.setItem("matchcut_app_lang", lang);
  isLangDropdownOpen.value = false;
}

watch(targetSoftware, (newVal) => {
  localStorage.setItem("targetSoftware", newVal);
  currentSoftware.value = softwareOptions.find(opt => opt.id === newVal) || softwareOptions[0];
});

function selectSoftware(id: string) {
  targetSoftware.value = id;
  isDropdownOpen.value = false;
}

// 切换多机位排序，并自动重新解析
async function toggleMultiCamSort() {
  enableMultiCamSort.value = !enableMultiCamSort.value;
  if (folderPath.value && !isProcessing.value) {
    await processFolder(folderPath.value);
  }
}

// 核心处理函数（向后端发送 multiCamSort 参数）
async function processFolder(path: string) {
  folderPath.value = path;
  isProcessing.value = true;
  isSuccess.value = false;
  try {
    const result: MatchPair[] = await invoke("smart_sniff_folder", { 
      path,
      multiCamSort: enableMultiCamSort.value 
    });
    pairs.value = result.filter((p) => p.status !== "empty");
  } catch (error) {
    console.error(error);
    alert(t('common.readError'));
  } finally {
    isProcessing.value = false;
  }
}

async function handleSelectFolder() {
  if (isProcessing.value) return;
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t('common.selectTitle'),
    });
    if (selected) {
      await processFolder(selected as string);
    }
  } catch (error) {
    console.error(error);
  }
}

// 全局点击事件监听（用于完美关闭下拉菜单）
const handleGlobalClick = (event: MouseEvent) => {
  if (isDropdownOpen.value && dropdownRef.value && !dropdownRef.value.contains(event.target as Node)) {
    isDropdownOpen.value = false;
  }
  if (isLangDropdownOpen.value && langDropdownRef.value && !langDropdownRef.value.contains(event.target as Node)) {
    isLangDropdownOpen.value = false;
  }
};

let unlistenDrop: UnlistenFn | null = null;
let unlistenDragEnter: UnlistenFn | null = null;
let unlistenDragLeave: UnlistenFn | null = null;

onMounted(async () => {
  document.addEventListener('mousedown', handleGlobalClick);

  try {
    unlistenDragEnter = await listen("tauri://drag-enter", () => {
      isDragging.value = true;
    });

    unlistenDragLeave = await listen("tauri://drag-leave", () => {
      isDragging.value = false;
    });

    unlistenDrop = await listen("tauri://drag-drop", (event: any) => {
      isDragging.value = false;
      if (isProcessing.value) return;
      const payload = event.payload;
      let paths: string[] = [];
      if (Array.isArray(payload)) {
        paths = payload; 
      } else if (payload && Array.isArray(payload.paths)) {
        paths = payload.paths; 
      }
      if (paths.length > 0) {
        processFolder(paths[0]); 
      }
    });
  } catch (e) {
    console.warn("系统拖拽事件注册失败", e);
  }
});

onUnmounted(() => {
  document.removeEventListener('mousedown', handleGlobalClick);
  if (unlistenDrop) unlistenDrop();
  if (unlistenDragEnter) unlistenDragEnter();
  if (unlistenDragLeave) unlistenDragLeave();
});

// 生成草稿函数
async function handleGenerateDraft() {
  if (pairs.value.length === 0) return;
  isProcessing.value = true;
  isSuccess.value = false;
  try {
    const draftPath: string = await invoke("generate_draft", {
      pairs: pairs.value,
      workspace: folderPath.value,
      software: targetSoftware.value,
      multiCamSort: enableMultiCamSort.value 
    });
    
    isSuccess.value = true;
    setTimeout(() => {
      alert(`${t('common.successAlert')}\n路径: ${draftPath}`);
      isSuccess.value = false;
    }, 600);
    
  } catch (error) {
    console.error(error);
    alert(t('common.genError') + error);
  } finally {
    isProcessing.value = false;
  }
}
</script>

<template>
  <div class="min-h-screen bg-slate-950 bg-[radial-gradient(ellipse_at_top_right,_var(--tw-gradient-stops))] from-indigo-900/40 via-slate-950 to-slate-950 text-slate-200 font-sans overflow-hidden flex flex-col relative">
    
    <!-- 全屏免责声明弹窗 -->
    <transition enter-active-class="transition duration-500 ease-out" enter-from-class="opacity-0 scale-95" enter-to-class="opacity-100 scale-100" leave-active-class="transition duration-300 ease-in" leave-from-class="opacity-100 scale-100" leave-to-class="opacity-0 scale-95">
      <div v-if="showDisclaimer" class="fixed inset-0 z-[9999] bg-slate-950/70 backdrop-blur-xl flex items-center justify-center p-6">
        <div class="max-w-md w-full bg-[#0f1115] border border-white/10 rounded-[2rem] p-8 shadow-[0_20px_60px_rgba(138,43,226,0.15)] flex flex-col items-center text-center">
          <div class="w-16 h-16 rounded-2xl bg-gradient-to-br from-indigo-500/20 to-purple-600/20 border border-indigo-500/30 text-indigo-400 flex items-center justify-center mb-6 shadow-inner">
            <svg class="w-8 h-8" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" /></svg>
          </div>
          <h2 class="text-2xl font-black text-white mb-3 tracking-wide">{{ t('disclaimer.title') }}</h2>
          <p class="text-sm text-slate-400 leading-relaxed mb-8" v-html="t('disclaimer.content')"></p>
          <button @click="agreeDisclaimer" class="w-full py-4 rounded-2xl bg-indigo-600 hover:bg-indigo-500 text-white font-bold tracking-widest transition-all hover:shadow-[0_0_20px_rgba(99,102,241,0.4)] hover:-translate-y-0.5">
            {{ t('disclaimer.button') }}
          </button>
        </div>
      </div>
    </transition>

    <!-- 顶部导航 -->
    <header class="sticky top-0 z-50 backdrop-blur-2xl bg-slate-950/40 border-b border-white/5 shadow-2xl">
      <div class="max-w-5xl mx-auto px-6 py-4 flex items-center justify-between">
        <div class="flex items-center gap-4">
          <div class="w-10 h-10 rounded-xl flex items-center justify-center shadow-lg transition-all duration-500" 
               :class="[
                 isProcessing ? 'processing-glow bg-indigo-600/90' : (isSuccess ? 'bg-emerald-500 shadow-emerald-500/50 scale-110 rotate-y-180' : 'bg-gradient-to-br from-indigo-500 to-purple-600 shadow-indigo-500/30')
               ]">
            <svg v-if="!isSuccess" class="w-6 h-6 text-white transition-all duration-300" :class="isProcessing ? 'animate-pulse' : ''" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
            <svg v-else class="w-6 h-6 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" /></svg>
          </div>
          <div>
            <h1 class="text-xl font-black tracking-wide text-transparent bg-clip-text bg-gradient-to-r from-indigo-400 via-purple-400 to-pink-400">>MatchCut-速对齐</h1>
            <p class="text-[11px] text-slate-400 font-bold tracking-widest mt-0.5">ZERO-RENDER ENGINE</p>
          </div>
        </div>

        <div class="flex items-center gap-3">
          <!-- 语言切换按钮 (顶部导航右侧) -->
          <div class="relative" ref="langDropdownRef">
            <button @click="isLangDropdownOpen = !isLangDropdownOpen" class="flex items-center gap-2 px-3.5 py-2.5 rounded-full bg-white/5 hover:bg-white/10 border border-white/10 text-xs font-semibold text-slate-300 transition-all">
              <svg class="w-4 h-4 text-indigo-400" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129" /></svg>
              <span>{{ languages.find(l => l.code === locale)?.label || '语言' }}</span>
            </button>
            <transition enter-active-class="transition duration-200 ease-out" enter-from-class="transform scale-95 opacity-0" enter-to-class="transform scale-100 opacity-100" leave-active-class="transition duration-75 ease-in" leave-from-class="transform scale-100 opacity-100" leave-to-class="transform scale-95 opacity-0">
              <ul v-if="isLangDropdownOpen" class="absolute z-50 top-[calc(100%+8px)] right-0 w-36 bg-[#161921] border border-white/10 rounded-2xl p-1.5 shadow-[0_10px_40px_rgba(0,0,0,0.8)]">
                <li v-for="lang in languages" :key="lang.code" @click="changeLanguage(lang.code)" class="flex items-center px-3 py-2 rounded-xl cursor-pointer hover:bg-indigo-500/20 hover:text-indigo-200 text-xs transition-colors" :class="locale === lang.code ? 'bg-white/5 text-white font-bold' : 'text-slate-300'">
                  {{ lang.label }}
                </li>
              </ul>
            </transition>
          </div>

          <button @click="handleSelectFolder" :disabled="isProcessing || showDisclaimer" class="group relative px-6 py-2.5 rounded-full bg-white/5 hover:bg-white/10 border border-white/10 backdrop-blur-md transition-all duration-300 disabled:opacity-50 flex items-center gap-2 overflow-hidden shadow-lg hover:shadow-indigo-500/20">
            <svg class="w-4 h-4 text-indigo-400 group-hover:scale-110 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z" /></svg>
            <span class="text-sm font-semibold text-slate-200">{{ isProcessing ? t('common.processing') : t('common.selectFolder') }}</span>
          </button>
        </div>
      </div>
    </header>

    <main class="flex-1 max-w-5xl w-full mx-auto px-6 py-8 pb-32">
      <!-- 拖拽区 -->
      <button v-if="pairs.length === 0" @click="handleSelectFolder" :disabled="showDisclaimer" class="mt-20 flex w-full flex-col items-center justify-center p-16 rounded-[2rem] border-2 border-dashed transition-all duration-300 cursor-pointer group focus:outline-none" :class="isDragging ? 'border-indigo-500 bg-indigo-500/10 scale-[1.02]' : 'border-white/10 bg-white/[0.02] hover:bg-white/[0.05] hover:border-white/30'">
        <div class="w-20 h-20 mb-6 rounded-3xl bg-gradient-to-br from-indigo-500/20 to-purple-500/20 border transition-all duration-300 flex items-center justify-center text-indigo-400" :class="isDragging ? 'border-indigo-400 scale-110' : 'border-white/5 group-hover:scale-105'">
          <svg class="w-10 h-10" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" /></svg>
        </div>
        <h3 class="text-xl font-bold text-white mb-3 tracking-wide pointer-events-none">{{ isDragging ? t('dropzone.dropping') : (isProcessing ? t('dropzone.scanning') : t('dropzone.idle')) }}</h3>
      </button>

      <!-- 列表区 -->
      <div v-else class="grid gap-4">
        <div v-for="pair in pairs" :key="pair.pair_id" class="group relative flex items-center p-5 rounded-2xl border backdrop-blur-xl transition-all duration-300 hover:-translate-y-1 hover:shadow-2xl" :class="[pair.status === 'perfect' ? 'bg-emerald-500/5 border-emerald-500/20 hover:bg-emerald-500/10' : 'bg-rose-500/5 border-rose-500/20 hover:bg-rose-500/10']">
          <div class="w-24 shrink-0 border-r border-white/10">
            <div class="text-[10px] font-bold tracking-widest text-slate-500 mb-1">SCENE</div>
            <div class="text-3xl font-black font-mono tracking-tighter" :class="pair.status === 'perfect' ? 'text-emerald-400' : 'text-rose-400'">
              #{{ String(pair.pair_id).padStart(2, '0') }}
            </div>
          </div>
          <div class="flex-1 px-8 flex flex-col gap-3 min-w-0">
            <div class="flex items-center gap-4">
              <span class="shrink-0 px-2.5 py-1 rounded-md text-[10px] font-bold tracking-widest uppercase border" :class="pair.video ? 'bg-indigo-500/20 text-indigo-300 border-indigo-500/30' : 'bg-rose-500/20 text-rose-300 border-rose-500/30'">
                {{ pair.video ? (pair.video.kind === 'video' ? 'VIDEO' : 'IMAGE') : 'MISSING' }}
              </span>
              <span class="text-sm font-mono truncate" :class="pair.video ? 'text-slate-200' : 'text-slate-600'">{{ pair.video?.name || '----' }}</span>
            </div>
            <div class="flex items-center gap-4">
              <span class="shrink-0 px-2.5 py-1 rounded-md text-[10px] font-bold tracking-widest uppercase border" :class="pair.audio ? 'bg-purple-500/20 text-purple-300 border-purple-500/30' : 'bg-rose-500/20 text-rose-300 border-rose-500/30'">
                {{ pair.audio ? 'AUDIO' : 'MISSING' }}
              </span>
              <span class="text-sm font-mono truncate" :class="pair.audio ? 'text-slate-300' : 'text-slate-600'">{{ pair.audio?.name || '----' }}</span>
            </div>
          </div>
        </div>
      </div>
    </main>

    <!-- 底部操作栏 -->
    <div v-if="pairs.length > 0" class="fixed bottom-8 left-1/2 -translate-x-1/2 z-50 w-full max-w-4xl px-6 pointer-events-none">
      <div class="p-3 rounded-3xl bg-[#0f1115]/95 border border-white/10 backdrop-blur-3xl shadow-[0_20px_50px_rgba(0,0,0,0.8)] flex items-center justify-between pointer-events-auto">
        
        <!-- 左侧：统计信息 -->
        <div class="pl-4 pr-2 text-sm font-medium text-slate-400 flex items-center gap-3">
          <div class="w-2 h-2 rounded-full bg-emerald-500"></div>
          <span v-html="t('common.captured', { count: pairs.length })"></span>
        </div>
        
        <!-- 右侧：交互组件 -->
        <div class="flex items-center gap-3">
          
          <!-- 多机位排序切换按钮 -->
          <button @click="toggleMultiCamSort" :disabled="isProcessing" class="group relative flex items-center gap-2.5 px-4 py-3.5 rounded-2xl border transition-all duration-300 focus:outline-none disabled:opacity-50" :class="enableMultiCamSort ? 'bg-indigo-500/10 border-indigo-500/30 hover:bg-indigo-500/20' : 'bg-white/5 border-white/10 hover:bg-white/10'">
            <div class="relative flex items-center justify-center w-5 h-5 rounded-[6px] border transition-all duration-300" :class="enableMultiCamSort ? 'bg-indigo-500 border-indigo-500 shadow-[0_0_10px_rgba(99,102,241,0.5)]' : 'border-slate-500 bg-slate-800 group-hover:border-slate-400'">
              <svg v-if="enableMultiCamSort" class="w-3.5 h-3.5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" /></svg>
            </div>
            <span class="text-sm font-semibold tracking-wide transition-colors" :class="enableMultiCamSort ? 'text-indigo-300' : 'text-slate-400 group-hover:text-slate-300'">{{ t('common.multicamSort') }}</span>
          </button>

          <!-- 下拉菜单 -->
          <div class="relative" ref="dropdownRef">
            <button @click="isDropdownOpen = !isDropdownOpen" :disabled="isProcessing" class="relative z-50 flex items-center gap-3 bg-white/5 hover:bg-white/10 border border-white/10 text-slate-200 text-sm font-medium rounded-2xl pl-4 pr-10 py-3.5 transition-all shadow-inner focus:ring-2 focus:ring-indigo-500/50 disabled:opacity-50">
              <div class="w-5 h-5 shrink-0 flex items-center justify-center">
                <div v-if="currentSoftware.icon === 'capcut'" class="w-full h-full bg-black rounded flex items-center justify-center relative overflow-hidden">
                  <svg class="w-4 h-4 text-white" viewBox="0 0 24 24" fill="currentColor"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm3.5 13.5l-7-7v7h7zm-7-14v7l7-7h-7z" opacity="0.9"/></svg>
                </div>
                <div v-if="currentSoftware.icon === 'davinci'" class="w-full h-full bg-gradient-to-br from-[#1cd8d2] to-[#93edc7] rounded-full flex items-center justify-center p-[2px] shadow-sm">
                   <div class="w-full h-full bg-[#1e2025] rounded-full flex items-center justify-center">
                     <div class="w-2 h-2 rounded-full bg-gradient-to-r from-orange-400 to-rose-500"></div>
                   </div>
                </div>
                <div v-if="currentSoftware.icon === 'fcp'" class="w-full h-full rounded shadow-sm flex flex-col overflow-hidden bg-[#2d2d2d] border border-white/20">
                  <div class="h-[8px] flex bg-[repeating-linear-gradient(45deg,transparent,transparent_2px,#fff_2px,#fff_4px)] opacity-80"></div>
                  <div class="flex-1 bg-gradient-to-br from-green-400 via-blue-500 to-purple-500"></div>
                </div>
                <div v-if="currentSoftware.icon === 'pr'" class="w-full h-full bg-[#00005C] rounded flex items-center justify-center border border-[#E978FF]/30">
                  <span class="text-[#E978FF] font-black text-[10px] tracking-tighter">Pr</span>
                </div>
              </div>
              <span class="tracking-wide">{{ currentSoftware.name }}</span>
              <div class="absolute right-3 text-slate-400">
                <svg class="w-4 h-4 transition-transform duration-200" :class="isDropdownOpen ? 'rotate-180' : ''" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" /></svg>
              </div>
            </button>

            <transition enter-active-class="transition duration-200 ease-out" enter-from-class="transform scale-95 opacity-0" enter-to-class="transform scale-100 opacity-100" leave-active-class="transition duration-75 ease-in" leave-from-class="transform scale-100 opacity-100" leave-to-class="transform scale-95 opacity-0">
              <ul v-if="isDropdownOpen" class="absolute z-50 bottom-[calc(100%+12px)] right-0 w-56 bg-[#161921] border border-white/10 rounded-2xl p-1.5 shadow-[0_10px_40px_rgba(0,0,0,0.8)]">
                <li v-for="opt in softwareOptions" :key="opt.id" @click="selectSoftware(opt.id)" class="flex items-center gap-3 px-3 py-2.5 rounded-xl cursor-pointer hover:bg-indigo-500/20 hover:text-indigo-200 transition-colors" :class="targetSoftware === opt.id ? 'bg-white/5 text-white font-bold' : 'text-slate-300'">
                  <div class="w-5 h-5 shrink-0 flex items-center justify-center">
                    <div v-if="opt.icon === 'capcut'" class="w-full h-full bg-black rounded flex items-center justify-center">
                      <svg class="w-4 h-4 text-white" viewBox="0 0 24 24" fill="currentColor"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm3.5 13.5l-7-7v7h7zm-7-14v7l7-7h-7z" opacity="0.9"/></svg>
                    </div>
                    <div v-if="opt.icon === 'davinci'" class="w-full h-full bg-gradient-to-br from-[#1cd8d2] to-[#93edc7] rounded-full flex items-center justify-center p-[2px]">
                       <div class="w-full h-full bg-[#1e2025] rounded-full flex items-center justify-center"><div class="w-2 h-2 rounded-full bg-gradient-to-r from-orange-400 to-rose-500"></div></div>
                    </div>
                    <div v-if="opt.icon === 'fcp'" class="w-full h-full rounded flex flex-col overflow-hidden bg-[#2d2d2d] border border-white/20">
                      <div class="h-[8px] flex bg-[repeating-linear-gradient(45deg,transparent,transparent_2px,#fff_2px,#fff_4px)] opacity-80"></div>
                      <div class="flex-1 bg-gradient-to-br from-green-400 via-blue-500 to-purple-500"></div>
                    </div>
                    <div v-if="opt.icon === 'pr'" class="w-full h-full bg-[#00005C] rounded flex items-center justify-center border border-[#E978FF]/30">
                      <span class="text-[#E978FF] font-black text-[10px] tracking-tighter">Pr</span>
                    </div>
                  </div>
                  {{ opt.name }}
                </li>
              </ul>
            </transition>
          </div>

          <button @click="handleGenerateDraft" :disabled="isProcessing" class="group relative flex items-center gap-2 px-8 py-3.5 rounded-2xl bg-gradient-to-r from-indigo-600 to-purple-600 hover:from-indigo-500 hover:to-purple-500 text-white font-bold shadow-[0_0_30px_rgba(99,102,241,0.4)] transition-all hover:scale-[1.02] disabled:opacity-50 overflow-hidden min-w-[180px] justify-center">
            <div class="absolute inset-0 w-full h-full bg-gradient-to-r from-transparent via-white/20 to-transparent -translate-x-full group-hover:animate-[shimmer_1.5s_infinite]"></div>
            <svg class="w-5 h-5 relative z-10" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M13 10V3L4 14h7v7l9-11h-7z" /></svg>
            <span class="relative z-10 tracking-wide">
              {{ isProcessing ? t('common.injecting') : (targetSoftware === 'jianying' ? t('common.genDraft') : t('common.genXml')) }}
            </span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
.processing-glow {
  animation: pulse-glow 1.2s infinite alternate ease-in-out;
}
@keyframes pulse-glow {
  0% { 
    transform: scale(1); 
    box-shadow: 0 0 15px rgba(138,43,226,0.4); 
  }
  100% { 
    transform: scale(1.08); 
    box-shadow: 0 0 35px rgba(138,43,226,0.9); 
  }
}

@keyframes shimmer { 100% { transform: translateX(100%); } }
::-webkit-scrollbar { width: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.1); border-radius: 10px; }
.rotate-y-180 { transform: rotateY(180deg); }
</style>