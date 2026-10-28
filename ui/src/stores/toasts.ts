import { reactive } from 'vue'

export type ToastKind = 'info' | 'success' | 'error'

export interface Toast {
  id: number
  kind: ToastKind
  message: string
}

const DURATIONS: Record<ToastKind, number> = {
  info: 4000,
  success: 4000,
  error: 12000,
}

export const toasts = reactive<Toast[]>([])
let nextId = 1

export function showToast(kind: ToastKind, message: string): void {
  const id = nextId
  nextId += 1
  toasts.push({ id, kind, message })
  window.setTimeout(() => dismissToast(id), DURATIONS[kind])
}

export function dismissToast(id: number): void {
  const index = toasts.findIndex((toast) => toast.id === id)
  if (index !== -1) {
    toasts.splice(index, 1)
  }
}
