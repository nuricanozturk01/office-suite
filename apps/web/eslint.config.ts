import { defineConfigWithVueTs, vueTsConfigs } from '@vue/eslint-config-typescript'
import type { Linter } from 'eslint'
import prettier from 'eslint-config-prettier'
import vue from 'eslint-plugin-vue'
import vueA11y from 'eslint-plugin-vuejs-accessibility'

// Layers from top to bottom; a layer may only import from layers below it.
const layers = ['app', 'pages', 'widgets', 'features', 'editors', 'entities', 'shared'] as const

const sliceInternals = ['pages', 'widgets', 'features', 'editors', 'entities'].map(
  (layer) => `@/${layer}/*/*`,
)

function forbiddenFor(layer: (typeof layers)[number], index: number): string[] {
  return layers.slice(0, index + 1).flatMap((upper) => {
    if (upper !== layer) {
      return [`@/${upper}`, `@/${upper}/**`]
    }
    // Within a sliced layer, other slices are off-limits; one's own slice uses relative paths.
    return layer === 'app' || layer === 'shared' ? [] : [`@/${upper}/*`]
  })
}

const layerBoundaries: Linter.Config[] = layers.map((layer, index) => {
  const forbidden = forbiddenFor(layer, index)
  const patterns = [
    { group: sliceInternals, message: "Import a slice through its public API ('index.ts') only." },
    ...(forbidden.length > 0
      ? [
          {
            group: forbidden,
            message: 'Layer violation: import only from lower layers (CLAUDE.md §6.1).',
          },
        ]
      : []),
  ]
  return {
    files: [`src/${layer}/**/*.{ts,vue}`],
    rules: { 'no-restricted-imports': ['error', { patterns }] },
  }
})

export default defineConfigWithVueTs(
  { ignores: ['dist', 'coverage', 'node_modules', 'eslint.config.ts', 'public/apps'] },
  vue.configs['flat/recommended'],
  vueA11y.configs['flat/recommended'],
  vueTsConfigs.strictTypeChecked,
  vueTsConfigs.stylisticTypeChecked,
  {
    rules: {
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-floating-promises': 'error',
      '@typescript-eslint/no-misused-promises': 'error',
      '@typescript-eslint/consistent-type-imports': 'error',
      'vue/block-order': ['error', { order: ['script', 'template', 'style'] }],
      'vue/define-macros-order': 'error',
      'vue/no-v-html': 'error',
      // Type-based optional props are `undefined` when absent; a runtime default adds nothing.
      'vue/require-default-prop': 'off',
      'vuejs-accessibility/label-has-for': ['error', { required: { some: ['nesting', 'id'] } }],
      'no-console': ['error', { allow: ['warn', 'error'] }],
    },
  },
  ...layerBoundaries,
  prettier,
)
