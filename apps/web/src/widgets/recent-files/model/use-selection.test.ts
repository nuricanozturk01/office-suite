import { describe, expect, it } from 'vitest'
import { nextTick, ref } from 'vue'

import { useSelection } from './use-selection'

describe('useSelection', () => {
  it('toggles single rows and selects or clears all', () => {
    const keys = ref(['a', 'b', 'c'])
    const selection = useSelection(() => keys.value)

    selection.toggle('a')
    expect(selection.count.value).toBe(1)
    expect(selection.allSelected.value).toBe(false)

    selection.toggleAll()
    expect(selection.count.value).toBe(3)
    expect(selection.allSelected.value).toBe(true)

    selection.toggleAll()
    expect(selection.count.value).toBe(0)
  })

  it('drops keys that leave the list', async () => {
    const keys = ref(['a', 'b'])
    const selection = useSelection(() => keys.value)
    selection.toggleAll()

    keys.value = ['b']
    await nextTick()

    expect([...selection.selected.value]).toEqual(['b'])
  })

  it('is never "all selected" for an empty list', () => {
    const selection = useSelection(() => [])
    expect(selection.allSelected.value).toBe(false)
  })
})
