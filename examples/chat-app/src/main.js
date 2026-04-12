import { availability, createSession } from 'tauri-plugin-foundation-models-api'

const dot       = document.getElementById('dot')
const statusLbl = document.getElementById('status-label')
const messages  = document.getElementById('messages')
const promptEl  = document.getElementById('prompt')
const sendBtn   = document.getElementById('send-btn')

let session = null
let busy    = false
let scrollQueued = false

function setStatus(state, label) {
  dot.className = 'status-dot ' + state
  statusLbl.textContent = label
}

function scrollToBottom() {
  if (scrollQueued) return
  scrollQueued = true
  requestAnimationFrame(() => {
    messages.scrollTop = messages.scrollHeight
    scrollQueued = false
  })
}

function addBubble(role, text = '') {
  const row = document.createElement('div')
  row.className = `bubble-row ${role}`
  const bubble = document.createElement('div')
  bubble.className = 'bubble'
  bubble.textContent = text
  row.appendChild(bubble)
  messages.appendChild(row)
  scrollToBottom()
  return bubble
}

function addNotice(text, isError = false) {
  const el = document.createElement('div')
  el.className = 'notice' + (isError ? ' error-notice' : '')
  el.textContent = text
  messages.appendChild(el)
  scrollToBottom()
}

function autoResize() {
  promptEl.style.height = 'auto'
  promptEl.style.height = Math.min(promptEl.scrollHeight, 140) + 'px'
}

async function init() {
  try {
    const status = await availability()
    if (!status.available) {
      setStatus('error', 'Unavailable')
      addNotice(`Apple Intelligence unavailable: ${status.reason}`, true)
      return
    }

    session = await createSession({
      instructions: 'You are a helpful, concise assistant. Keep responses clear and to the point.',
    })

    setStatus('ready', 'On-device model ready')
    addNotice('Apple Intelligence is running on-device. Start chatting.')
    promptEl.disabled = false
    sendBtn.disabled  = false
    promptEl.focus()
  } catch (err) {
    setStatus('error', 'Error')
    addNotice(`Failed to start: ${err}`, true)
  }
}

async function send() {
  const text = promptEl.value.trim()
  if (!text || busy || !session) return

  busy = true
  promptEl.value = ''
  promptEl.style.height = 'auto'
  promptEl.disabled = true
  sendBtn.disabled  = true
  setStatus('busy', 'Thinking…')

  addBubble('user', text)

  const aiBubble = addBubble('ai')
  aiBubble.classList.add('streaming')

  try {
    await session.respondStream(text, (chunk) => {
      aiBubble.textContent += chunk
      scrollToBottom()
    })
  } catch (err) {
    aiBubble.textContent = `[Error: ${err}]`
    addNotice('Something went wrong — the session may need to be restarted.', true)
  } finally {
    aiBubble.classList.remove('streaming')
    busy = false
    promptEl.disabled = false
    sendBtn.disabled  = false
    setStatus('ready', 'On-device model ready')
    promptEl.focus()
  }
}

sendBtn.addEventListener('click', send)

promptEl.addEventListener('input', autoResize)

promptEl.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    send()
  }
})

init()
