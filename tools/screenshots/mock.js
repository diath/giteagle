// Stands in for GitEagle's Tauri backend, so the built UI runs in a plain
// browser. Commands are answered from data.json, which generate.py writes
// from the backend's real responses for the demo repository.
//
// The query string sets the theme, a zoom for HiDPI captures, how long to
// hold back the page's load event (Firefox captures the page then), and the
// steps that open the screen to capture, separated by `||`:
//
//   click:<selector>            clicks the first visible match
//   click:<selector>@<n>        clicks the nth visible match, counting from 0
//   clicktext:<selector>=><text> clicks the first visible match containing text
//   type:<selector>=><text>     types text into an input
//   enter:<selector>            presses Enter in an input
//   key:<key>                   presses a key, e.g. y
//   wait:<ms>                   waits
//
// Missing responses and failed steps are posted to /report for generate.py
// to print.
;(() => {
  const params = new URLSearchParams(location.search)
  const theme = params.get('theme') || 'darkBlack'
  const zoom = Number(params.get('zoom') || 1)
  const holdMs = Number(params.get('hold') || 3000)
  const steps = (params.get('steps') || '').split('||').filter(Boolean)

  // Loaded synchronously, so the data is there before the app's first command.
  const request = new XMLHttpRequest()
  request.open('GET', './data.json', false)
  request.send()
  const { tabs, recent, responses } = JSON.parse(request.responseText)

  const projects = tabs.map((workdir, index) => ({
    id: index + 1,
    name: workdir.split('/').pop(),
    workdir,
  }))
  const now = Math.floor(Date.now() / 1000)

  function report(message) {
    console.warn(message)
    fetch('/report', { method: 'POST', body: message }).catch(() => {})
  }

  /** JSON with object keys sorted, as dump.rs writes the arguments. */
  function sortedJson(value) {
    if (Array.isArray(value)) {
      return `[${value.map(sortedJson).join(',')}]`
    }
    if (value !== null && typeof value === 'object') {
      const keys = Object.keys(value).sort()
      return `{${keys.map((key) => `${JSON.stringify(key)}:${sortedJson(value[key])}`).join(',')}}`
    }
    return JSON.stringify(value === undefined ? null : value)
  }

  // Answers that don't depend on the repository. Every tab gets the demo
  // repository's responses, but only the first one is ever shown.
  const fixed = {
    restore_session: () => ({ projects, active: 1, failures: [] }),
    get_settings: () => ({
      theme,
      fontSize: 'normal',
      recentLimit: 10,
      recentProjects: recent,
      openProjects: tabs,
      activeProject: tabs[0],
      blameBackend: 'systemGit',
    }),
    set_settings: () => null,
    show_project: () => null,
    close_project: () => null,
    get_avatar: () => null,
    get_fetch_status: () => ({ inProgress: false, lastSuccess: now - 140, lastError: null }),
    'plugin:event|listen': () => Math.floor(Math.random() * 1e9),
    'plugin:event|unlisten': () => null,
  }

  function answer(command, args) {
    if (command in fixed) {
      return fixed[command](args)
    }
    const rest = { ...args }
    delete rest.project
    // A reload of a list's first page names the head it started from, which
    // gets the same page.
    if (rest.cursor && rest.cursor.skip === 0) {
      rest.cursor = null
    }
    const key = `${command} ${sortedJson(rest)}`
    if (!(key in responses)) {
      report(`no response for ${key}`)
      throw { kind: 'notFound', message: `The mock has no response for ${key}` }
    }
    const value = responses[key]
    if (value !== null && typeof value === 'object' && '__error' in value) {
      const error = value.__error
      throw typeof error === 'string' ? { kind: 'git', message: error } : error
    }
    return structuredClone(value)
  }

  let nextCallback = 1
  window.__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: { label: 'main' },
      currentWebview: { windowLabel: 'main', label: 'main' },
    },
    transformCallback(callback, once) {
      const id = nextCallback++
      window[`_${id}`] = (payload) => {
        if (once) {
          delete window[`_${id}`]
        }
        return callback && callback(payload)
      }
      return id
    },
    unregisterCallback(id) {
      delete window[`_${id}`]
    },
    convertFileSrc: (path) => path,
    invoke(command, args) {
      // Answered a moment later, like a real round trip, so the app loads in
      // its usual order.
      return new Promise((resolve, reject) => {
        setTimeout(() => {
          try {
            resolve(answer(command, args || {}))
          } catch (error) {
            reject(error)
          }
        }, 5)
      })
    },
  }
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} }

  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

  function isVisible(element) {
    return element.getClientRects().length > 0 && getComputedStyle(element).visibility !== 'hidden'
  }

  // Views the app has left stay mounted but hidden, so only visible matches count.
  async function find(selector, index, text) {
    for (let tries = 0; tries < 200; tries++) {
      const matches = [...document.querySelectorAll(selector)]
        .filter(isVisible)
        .filter((element) => text === null || element.textContent.includes(text))
      if (matches.length > index) {
        return matches[index]
      }
      await sleep(50)
    }
    throw new Error(`nothing matches ${selector}${text === null ? '' : ` containing "${text}"`}`)
  }

  function parseStep(step) {
    const colon = step.indexOf(':')
    const action = step.slice(0, colon)
    let target = step.slice(colon + 1)
    let text = null
    const arrow = target.indexOf('=>')
    if (arrow !== -1) {
      text = target.slice(arrow + 2)
      target = target.slice(0, arrow)
    }
    let index = 0
    const match = /^(.*)@(\d+)$/.exec(target)
    if (match) {
      target = match[1]
      index = Number(match[2])
    }
    return { action, target, text, index }
  }

  async function runStep(step) {
    const { action, target, text, index } = parseStep(step)
    if (action === 'wait') {
      await sleep(Number(target))
      return
    }
    if (action === 'key') {
      const event = new KeyboardEvent('keydown', { key: target, bubbles: true, cancelable: true })
      ;(document.activeElement || document.body).dispatchEvent(event)
      return
    }
    const element = await find(target, index, action === 'clicktext' ? text : null)
    if (action === 'click' || action === 'clicktext') {
      element.click()
    } else if (action === 'type') {
      element.focus()
      element.value = text
      element.dispatchEvent(new Event('input', { bubbles: true }))
    } else if (action === 'enter') {
      element.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }))
    } else {
      throw new Error(`unknown action ${action}`)
    }
  }

  async function runSteps() {
    for (const step of steps) {
      try {
        await runStep(step)
      } catch (error) {
        report(`step ${step} failed: ${error.message}`)
      }
      // Gives the app time to load and draw what the step opened.
      await sleep(250)
    }
    // A focused field would show its focus ring in the capture.
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur()
    }
  }

  document.addEventListener('DOMContentLoaded', () => {
    if (zoom !== 1) {
      const style = document.createElement('style')
      style.textContent = `html { zoom: ${zoom}; }`
      document.head.append(style)
    }
    // The page's load event waits for this frame, which the server answers
    // only after the hold, so the steps and highlighting finish first.
    const hold = document.createElement('iframe')
    hold.hidden = true
    hold.src = `/delay?ms=${holdMs}`
    document.body.append(hold)
    setTimeout(runSteps, 600)
  })
})()
