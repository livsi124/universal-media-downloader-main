import { reactive } from 'vue'
import en from '../locales/en.json'
import ru from '../locales/ru.json'
import uk from '../locales/uk.json'

const translations = { en, ru, uk }

const state = reactive({
    locale: 'ru',
    messages: ru,
})

export function useI18n() {
    function t(key, fallback = '') {
        return state.messages[key] || fallback || key
    }

    function setLocale(lang) {
        state.locale = lang
        state.messages = translations[lang] || en
    }

    function getLocale() {
        return state.locale
    }

    return { t, setLocale, getLocale, locale: state }
}
