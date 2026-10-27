import { loadFormulaEngine } from '@office/formula-engine'
import { onMounted, ref } from 'vue'

/** Compiles the WebAssembly calculation engine once; the editor mounts after `ready`. */
export function useEngineReady() {
  const ready = ref(false)
  const failed = ref(false)

  onMounted(() => {
    loadFormulaEngine().then(
      () => {
        ready.value = true
      },
      () => {
        failed.value = true
      },
    )
  })

  return { ready, failed }
}
