/* Runs before the editor starts (injected by scripts/rebrand.sh). Same origin as the app. */
(function () {
  // English only: the editors pick their language from the browser; report English instead.
  Object.defineProperty(navigator, 'languages', { get: function () { return ['en-US'] } })
  Object.defineProperty(navigator, 'language', { get: function () { return 'en-US' } })

  // Theme: the editors follow `prefers-color-scheme`. Answer it from the app's own choice
  // (localStorage `office.theme`: system, light or dark) so one switch controls everything.
  var KEY = 'office.theme'
  var SCHEME = /^\(\s*prefers-color-scheme\s*:\s*(dark|light)\s*\)$/i
  var real = window.matchMedia.bind(window)
  var system = real('(prefers-color-scheme: dark)')
  var lists = []

  function preference() {
    try {
      return window.localStorage.getItem(KEY) || 'system'
    } catch (e) {
      return 'system'
    }
  }

  function dark() {
    var choice = preference()
    return choice === 'dark' || (choice === 'system' && system.matches)
  }

  function refresh() {
    lists.forEach(function (list) {
      var matches = list.scheme === 'dark' ? dark() : !dark()
      if (matches === list.matches) {
        return
      }
      list.matches = matches
      var event = { type: 'change', media: list.media, matches: matches }
      list.listeners.slice().forEach(function (listener) {
        listener.call(list, event)
      })
      if (typeof list.onchange === 'function') {
        list.onchange(event)
      }
    })
  }

  window.matchMedia = function (query) {
    var found = SCHEME.exec(String(query))
    if (!found) {
      return real(query)
    }
    var scheme = found[1].toLowerCase()
    var list = {
      media: String(query),
      scheme: scheme,
      matches: scheme === 'dark' ? dark() : !dark(),
      listeners: [],
      onchange: null,
      addEventListener: function (type, listener) {
        if (type === 'change' && typeof listener === 'function') {
          list.listeners.push(listener)
        }
      },
      removeEventListener: function (type, listener) {
        list.listeners = list.listeners.filter(function (l) { return l !== listener })
      },
      addListener: function (listener) { list.addEventListener('change', listener) },
      removeListener: function (listener) { list.removeEventListener('change', listener) },
      dispatchEvent: function () { return false },
    }
    lists.push(list)
    return list
  }

  window.addEventListener('storage', function (event) {
    if (event.key === KEY || event.key === null) {
      refresh()
    }
  })
  system.addEventListener('change', refresh)
})()
