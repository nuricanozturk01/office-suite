import { computed, ref, watch } from 'vue'

/** Which rows are ticked, by key. Keys that disappear from the list are dropped. */
export function useSelection(keys: () => string[]) {
  const selected = ref(new Set<string>())

  watch(keys, (current) => {
    const present = new Set(current)
    const kept = [...selected.value].filter((key) => present.has(key))
    if (kept.length !== selected.value.size) {
      selected.value = new Set(kept)
    }
  })

  const count = computed(() => selected.value.size)
  const allSelected = computed(
    () => keys().length > 0 && keys().every((k) => selected.value.has(k)),
  )

  function toggle(key: string): void {
    const next = new Set(selected.value)
    if (next.has(key)) {
      next.delete(key)
    } else {
      next.add(key)
    }
    selected.value = next
  }

  function toggleAll(): void {
    selected.value = allSelected.value ? new Set() : new Set(keys())
  }

  function clear(): void {
    selected.value = new Set()
  }

  return { selected, count, allSelected, toggle, toggleAll, clear }
}
