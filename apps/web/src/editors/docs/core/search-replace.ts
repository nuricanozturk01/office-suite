import { Extension } from '@tiptap/core'
import type { Node as ProseNode } from '@tiptap/pm/model'
import { Plugin, PluginKey, type Transaction } from '@tiptap/pm/state'
import { Decoration, DecorationSet } from '@tiptap/pm/view'

export interface SearchMatch {
  from: number
  to: number
}

export interface SearchState {
  term: string
  caseSensitive: boolean
  matches: SearchMatch[]
  /** Index of the current match, `-1` when there is none. */
  current: number
}

const EMPTY: SearchState = { term: '', caseSensitive: false, matches: [], current: -1 }
export const searchKey = new PluginKey<SearchState>('docs-search')

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    searchReplace: {
      setSearch: (term: string, caseSensitive?: boolean) => ReturnType
      findNext: () => ReturnType
      findPrevious: () => ReturnType
      replaceCurrent: (replacement: string) => ReturnType
      replaceAll: (replacement: string) => ReturnType
      clearSearch: () => ReturnType
    }
  }
}

/** Finds every occurrence of `term` inside single text blocks (matches never span blocks). */
export function findMatches(doc: ProseNode, term: string, caseSensitive: boolean): SearchMatch[] {
  if (term === '') {
    return []
  }
  const needle = caseSensitive ? term : term.toLowerCase()
  const matches: SearchMatch[] = []
  doc.descendants((node, pos) => {
    if (!node.isTextblock) {
      return true
    }
    let text = ''
    const offsets: number[] = []
    node.forEach((child, offset) => {
      if (child.isText) {
        for (let i = 0; i < (child.text?.length ?? 0); i += 1) {
          offsets.push(pos + 1 + offset + i)
        }
        text += child.text ?? ''
      } else {
        offsets.push(pos + 1 + offset)
        text += '￼'
      }
    })
    const haystack = caseSensitive ? text : text.toLowerCase()
    let from = haystack.indexOf(needle)
    while (from !== -1) {
      const start = offsets[from]
      const last = offsets[from + needle.length - 1]
      if (start !== undefined && last !== undefined) {
        matches.push({ from: start, to: last + 1 })
      }
      from = haystack.indexOf(needle, from + needle.length)
    }
    return false
  })
  return matches
}

function reduce(tr: Transaction, previous: SearchState, doc: ProseNode): SearchState {
  const meta: unknown = tr.getMeta(searchKey)
  const next = isSearchState(meta) ? meta : previous
  if (!tr.docChanged && next === previous) {
    return previous
  }
  if (next.term === '') {
    return EMPTY
  }
  const matches = findMatches(doc, next.term, next.caseSensitive)
  const current =
    matches.length === 0 ? -1 : Math.min(Math.max(next.current, 0), matches.length - 1)
  return { ...next, matches, current }
}

function isSearchState(value: unknown): value is SearchState {
  return typeof value === 'object' && value !== null && 'term' in value && 'matches' in value
}

export const SearchReplace = Extension.create({
  name: 'searchReplace',

  addCommands() {
    const read = (state: Parameters<typeof searchKey.getState>[0]) =>
      searchKey.getState(state) ?? EMPTY
    return {
      setSearch:
        (term, caseSensitive = false) =>
        ({ tr, dispatch }) => {
          if (dispatch) {
            tr.setMeta(searchKey, { term, caseSensitive, matches: [], current: 0 })
          }
          return true
        },
      clearSearch:
        () =>
        ({ tr, dispatch }) => {
          if (dispatch) {
            tr.setMeta(searchKey, EMPTY)
          }
          return true
        },
      findNext:
        () =>
        ({ state, tr, dispatch }) => {
          const search = read(state)
          if (search.matches.length === 0) {
            return false
          }
          if (dispatch) {
            tr.setMeta(searchKey, {
              ...search,
              current: (search.current + 1) % search.matches.length,
            })
          }
          return true
        },
      findPrevious:
        () =>
        ({ state, tr, dispatch }) => {
          const search = read(state)
          if (search.matches.length === 0) {
            return false
          }
          if (dispatch) {
            const count = search.matches.length
            tr.setMeta(searchKey, { ...search, current: (search.current - 1 + count) % count })
          }
          return true
        },
      replaceCurrent:
        (replacement) =>
        ({ state, tr, dispatch }) => {
          const search = read(state)
          const match = search.matches[search.current]
          if (match === undefined) {
            return false
          }
          if (dispatch) {
            tr.insertText(replacement, match.from, match.to)
          }
          return true
        },
      replaceAll:
        (replacement) =>
        ({ state, tr, dispatch }) => {
          const search = read(state)
          if (search.matches.length === 0) {
            return false
          }
          if (dispatch) {
            for (const match of [...search.matches].reverse()) {
              tr.insertText(replacement, match.from, match.to)
            }
          }
          return true
        },
    }
  },

  addProseMirrorPlugins() {
    return [
      new Plugin<SearchState>({
        key: searchKey,
        state: {
          init: () => EMPTY,
          apply: (tr, previous, _old, next) => reduce(tr, previous, next.doc),
        },
        props: {
          decorations: (state) => {
            const search = searchKey.getState(state)
            if (search === undefined || search.matches.length === 0) {
              return DecorationSet.empty
            }
            return DecorationSet.create(
              state.doc,
              search.matches.map((match, index) =>
                Decoration.inline(match.from, match.to, {
                  class: index === search.current ? 'search-match search-current' : 'search-match',
                }),
              ),
            )
          },
        },
      }),
    ]
  },
})
