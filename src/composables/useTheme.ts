import { ref, watch, onMounted } from 'vue'

type ThemeMode = 'light' | 'dark' | 'system'

const STORAGE_KEY = 'money-sage-theme'
const THEME_ATTR = 'data-theme'

function getSystemTheme(): 'light' | 'dark' {
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function getStoredTheme(): ThemeMode | null {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored === 'light' || stored === 'dark' || stored === 'system') {
      return stored
    }
  } catch {
    // localStorage may not be available
  }
  return null
}

function applyTheme(mode: ThemeMode) {
  const resolved = mode === 'system' ? getSystemTheme() : mode
  document.documentElement.setAttribute(THEME_ATTR, resolved)
}

function storeTheme(mode: ThemeMode) {
  try {
    localStorage.setItem(STORAGE_KEY, mode)
  } catch {
    // ignore
  }
}

export function useTheme() {
  const mode = ref<ThemeMode>(getStoredTheme() || 'system')

  const setMode = (newMode: ThemeMode) => {
    mode.value = newMode
    applyTheme(newMode)
    storeTheme(newMode)
  }

  const toggle = () => {
    const resolved = mode.value === 'system' ? getSystemTheme() : mode.value
    setMode(resolved === 'dark' ? 'light' : 'dark')
  }

  const isDark = () => {
    const resolved = mode.value === 'system' ? getSystemTheme() : mode.value
    return resolved === 'dark'
  }

  onMounted(() => {
    applyTheme(mode.value)

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    const handleChange = () => {
      if (mode.value === 'system') {
        applyTheme('system')
      }
    }

    mediaQuery.addEventListener('change', handleChange)

    watch(mode, (newMode) => {
      applyTheme(newMode)
    })

    return () => {
      mediaQuery.removeEventListener('change', handleChange)
    }
  })

  return {
    mode,
    setMode,
    toggle,
    isDark,
  }
}
