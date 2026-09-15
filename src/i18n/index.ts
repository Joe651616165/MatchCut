import { createI18n } from 'vue-i18n'

const messages = {
  zh: {
    disclaimer: {
      title: "免责声明与使用须知",
      content: "MatchCut 致力于为您提供零渲染的极速多机位同步体验。为了您的数据安全，请确保在操作前已 <strong class=\"text-indigo-300\">备份原素材</strong>。<br><br>本软件仅在本地提供基于算法的工程排版服务，不对因源文件损坏、系统崩溃等导致的直接或间接损失负责。",
      button: "我已知晓并同意"
    },
    common: {
      selectFolder: "选择素材目录",
      processing: "解析中...",
      readError: "读取失败，请查看控制台",
      selectTitle: "选择素材文件夹",
      successAlert: "🎉 魔法生效！工程已生成",
      genError: "生成失败: ",
      captured: "共捕获 <span class=\"text-white font-bold text-lg\">{count}</span> 组分镜",
      multicamSort: "多机位排序",
      injecting: "注入中...",
      genDraft: "生成草稿时间轴",
      genXml: "导出 XML 序列"
    },
    dropzone: {
      dropping: "松开鼠标立即解析",
      scanning: "正在极速扫描...",
      idle: "点击或拖拽文件夹至此处"
    }
  },
  en: {
    disclaimer: {
      title: "Disclaimer & Notice",
      content: "MatchCut is designed to provide a zero-render, lightning-fast multi-camera sync experience. For your data safety, please ensure you have <strong class=\"text-indigo-300\">backed up your source footage</strong> before proceeding.<br><br>This software provides local algorithmic timeline typesetting services only and is not responsible for direct or indirect losses caused by source file corruption or system crashes.",
      button: "I Understand & Agree"
    },
    common: {
      selectFolder: "Select Folder",
      processing: "Parsing...",
      readError: "Failed to read folder, check console",
      selectTitle: "Select Footage Directory",
      successAlert: "🎉 Magic! Project generated successfully",
      genError: "Generation failed: ",
      captured: "Captured <span class=\"text-white font-bold text-lg\">{count}</span> scenes",
      multicamSort: "Multi-Cam Sort",
      injecting: "Injecting...",
      genDraft: "Generate Timeline Draft",
      genXml: "Export XML Sequence"
    },
    dropzone: {
      dropping: "Release to parse instantly",
      scanning: "Scanning rapidly...",
      idle: "Click or drag folder here"
    }
  },
  es: {
    disclaimer: {
      title: "Aviso Legal y de Exención",
      content: "MatchCut ofrece una experiencia de sincronización multicámara ultrarrápida y sin renderizado. Por su seguridad, asegúrese de <strong class=\"text-indigo-300\">hacer una copia de seguridad</strong> antes de continuar.<br><br>Este software solo proporciona servicios locales de composición de proyectos y no se hace responsable de pérdidas derivadas de daños en archivos.",
      button: "Entendido y de acuerdo"
    },
    common: {
      selectFolder: "Seleccionar Carpeta",
      processing: "Analizando...",
      readError: "Error al leer, ver consola",
      selectTitle: "Seleccionar Directorio",
      successAlert: "🎉 ¡Magia! Proyecto generado con éxito",
      genError: "Error al generar: ",
      captured: "Capturadas <span class=\"text-white font-bold text-lg\">{count}</span> escenas",
      multicamSort: "Orden Multicámara",
      injecting: "Inyectando...",
      genDraft: "Generar Borrador",
      genXml: "Exportar Secuencia XML"
    },
    dropzone: {
      dropping: "Suelte para analizar",
      scanning: "Escaneando...",
      idle: "Haga clic o arrastre una carpeta aquí"
    }
  },
  de: {
    disclaimer: {
      title: "Haftungsausschluss",
      content: "MatchCut bietet eine blitzschnelle Multi-Kamera-Synchronisation ohne Rendering. Bitte stellen Sie sicher, dass Sie Ihre <strong class=\"text-indigo-300\">Quelldateien gesichert</strong> haben.<br><br>Diese Software bietet ausschließlich lokale Algorithmus-Dienste und haftet nicht für direkte oder indirekte Schäden.",
      button: "Verstanden und einverstanden"
    },
    common: {
      selectFolder: "Ordner auswählen",
      processing: "Analysiere...",
      readError: "Fehler beim Lesen, Konsole prüfen",
      selectTitle: "Quellverzeichnis auswählen",
      successAlert: "🎉 Magie! Projekt erfolgreich erstellt",
      genError: "Erstellung fehlgeschlagen: ",
      captured: "<span class=\"text-white font-bold text-lg\">{count}</span> Szenen erfasst",
      multicamSort: "Multi-Cam Sortierung",
      injecting: "Wird eingefügt...",
      genDraft: "Entwurf generieren",
      genXml: "XML-Sequenz exportieren"
    },
    dropzone: {
      dropping: "Loslassen zum Analysieren",
      scanning: "Scanne...",
      idle: "Ordner hierher ziehen oder klicken"
    }
  }
}

// 自动检测系统/浏览器语言逻辑
function getSystemLocale() {
  const savedLang = localStorage.getItem('matchcut_app_lang')
  if (savedLang && messages[savedLang as keyof typeof messages]) {
    return savedLang
  }

  const browserLang = (navigator.language || 'zh').toLowerCase()
  if (browserLang.startsWith('es')) return 'es'
  if (browserLang.startsWith('de')) return 'de'
  if (browserLang.startsWith('en')) return 'en'
  return 'zh'
}

const i18n = createI18n({
  legacy: false,
  locale: getSystemLocale(),
  fallbackLocale: 'zh',
  messages
})

export default i18n